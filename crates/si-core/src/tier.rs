//! Hardware tiers (spec §8): how capable the GPU, the CPU and the PC as a
//! whole are, used to pick game settings.

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::model::{CpuInfo, GpuInfo, SystemSnapshot};

const BUNDLED_GPU_TIERS: &str = include_str!("../../../data/gpu_tiers.toml");
const BUNDLED_CPU_TIERS: &str = include_str!("../../../data/cpu_tiers.toml");

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    #[default]
    Low,
    Mid,
    High,
    Ultra,
}

impl Tier {
    pub fn up(self) -> Self {
        match self {
            Tier::Low => Tier::Mid,
            Tier::Mid => Tier::High,
            Tier::High | Tier::Ultra => Tier::Ultra,
        }
    }

    pub fn down(self) -> Self {
        match self {
            Tier::Low | Tier::Mid => Tier::Low,
            Tier::High => Tier::Mid,
            Tier::Ultra => Tier::High,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Tier::Low => "Low",
            Tier::Mid => "Mid",
            Tier::High => "High",
            Tier::Ultra => "Ultra",
        }
    }
}

/// Name patterns per tier, loaded from `data/*_tiers.toml`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TierTable {
    pub low: Vec<String>,
    pub mid: Vec<String>,
    pub high: Vec<String>,
    pub ultra: Vec<String>,
}

impl TierTable {
    pub fn parse(toml: &str) -> Result<Self, CoreError> {
        Ok(toml::from_str(toml)?)
    }

    pub fn bundled_gpu() -> Self {
        Self::parse(BUNDLED_GPU_TIERS).unwrap_or_default()
    }

    pub fn bundled_cpu() -> Self {
        Self::parse(BUNDLED_CPU_TIERS).unwrap_or_default()
    }

    /// The highest tier with a pattern contained in `name`.
    pub fn best_match(&self, name: &str) -> Option<Tier> {
        let name = normalize(name);
        [
            (Tier::Ultra, &self.ultra),
            (Tier::High, &self.high),
            (Tier::Mid, &self.mid),
            (Tier::Low, &self.low),
        ]
        .into_iter()
        .find(|(_, patterns)| patterns.iter().any(|p| name.contains(&normalize(p))))
        .map(|(tier, _)| tier)
    }
}

/// Lower-case, drop `(r)`, `(tm)`, `(c)` and collapse whitespace.
fn normalize(name: &str) -> String {
    let lower = name
        .to_lowercase()
        .replace("(r)", " ")
        .replace("(tm)", " ")
        .replace("(c)", " ");
    lower.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A token like `6800M` or `7600S`: 3–4 digits followed by `M` or `S`.
fn is_mobile_gpu_token(token: &str) -> bool {
    let Some(digits) = token.strip_suffix(['m', 's']) else {
        return false;
    };
    (3..=4).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit())
}

/// A low-power laptop CPU model like `i5-1235U` or `7520U`.
fn is_u_series_token(token: &str) -> bool {
    let model = ["i3-", "i5-", "i7-", "i9-"]
        .iter()
        .find_map(|p| token.strip_prefix(p))
        .unwrap_or(token);
    model
        .strip_suffix('u')
        .and_then(|rest| rest.bytes().last())
        .is_some_and(|b| b.is_ascii_digit())
}

pub fn gpu_tier(gpu: &GpuInfo, table: &TierTable) -> Tier {
    let tier = table.best_match(&gpu.name).unwrap_or(if gpu.is_integrated {
        Tier::Low
    } else if gpu.vram_mb >= 12 * 1024 {
        Tier::High
    } else if gpu.vram_mb >= 8 * 1024 {
        Tier::Mid
    } else {
        Tier::Low
    });
    let name = normalize(&gpu.name);
    let mobile = name.contains("laptop") || name.split(' ').any(is_mobile_gpu_token);
    if mobile { tier.down() } else { tier }
}

pub fn cpu_tier(cpu: &CpuInfo, table: &TierTable) -> Tier {
    let tier = table
        .best_match(&cpu.name)
        .unwrap_or(if cpu.physical_cores >= 8 {
            Tier::Mid
        } else {
            Tier::Low
        });
    if normalize(&cpu.name).split(' ').any(is_u_series_token) {
        tier.down()
    } else {
        tier
    }
}

/// GPU tier, one lower if the CPU is two or more tiers behind, then capped by
/// RAM (`0` = unknown, no cap).
pub fn overall_tier(gpu: Tier, cpu: Tier, ram_total_mb: u64) -> Tier {
    let mut tier = gpu;
    if (gpu as u8).saturating_sub(cpu as u8) >= 2 {
        tier = tier.down();
    }
    match ram_total_mb {
        0 => tier,
        mb if mb < 7_500 => Tier::Low,
        mb if mb < 15_000 => tier.min(Tier::Mid),
        _ => tier,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub gpu: Tier,
    pub cpu: Tier,
    pub overall: Tier,
    /// VRAM of the primary GPU (`0` if none).
    pub vram_mb: u64,
}

pub fn hardware_profile(s: &SystemSnapshot) -> HardwareProfile {
    let primary = s.primary_gpu();
    let gpu = primary.map_or(Tier::Low, |g| gpu_tier(g, &TierTable::bundled_gpu()));
    let cpu = cpu_tier(&s.cpu, &TierTable::bundled_cpu());
    HardwareProfile {
        gpu,
        cpu,
        overall: overall_tier(gpu, cpu, s.memory.total_mb),
        vram_mb: primary.map_or(0, |g| g.vram_mb),
    }
}
