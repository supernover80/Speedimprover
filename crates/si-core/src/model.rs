//! The snapshot of a PC that every other part of SpeedImprover reads.
//!
//! Every field is optional in JSON (`#[serde(default)]`): a collector that
//! fails leaves its part at the default, and rules treat defaults
//! (`None`, `0`, `Unknown`) as "not known" rather than as real values.

use chrono::{DateTime, NaiveDate};
use serde::{Deserialize, Serialize};

/// Everything one scan learned about the machine.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SystemSnapshot {
    /// When the scan ran (Unix seconds). Ages such as driver age are measured
    /// from here, never from "now", so results are reproducible.
    pub taken_at_unix: i64,
    pub machine: MachineInfo,
    pub cpu: CpuInfo,
    pub gpus: Vec<GpuInfo>,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub displays: Vec<DisplayInfo>,
    pub os: OsSettings,
    pub processes: Vec<ProcessUsage>,
    pub drivers: Vec<DriverInfo>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MachineInfo {
    pub hostname: String,
    pub os_name: String,
    pub os_build: u32,
    pub is_laptop: Option<bool>,
    pub on_battery: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CpuInfo {
    pub name: String,
    pub physical_cores: u32,
    pub logical_cores: u32,
    pub base_mhz: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Vendor {
    Nvidia,
    Amd,
    Intel,
    #[default]
    Other,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: Vendor,
    pub vram_mb: u64,
    pub is_integrated: bool,
    pub driver_version: Option<String>,
    /// `YYYY-MM-DD`.
    pub driver_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryInfo {
    /// `0` means unknown.
    pub total_mb: u64,
    pub used_mb: u64,
    pub modules: Vec<MemoryModule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryGen {
    Ddr4,
    Ddr5,
    #[default]
    Other,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryModule {
    pub capacity_mb: u64,
    pub rated_mhz: u32,
    pub configured_mhz: u32,
    pub generation: MemoryGen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskMedia {
    Nvme,
    Ssd,
    Hdd,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DiskInfo {
    /// Mount point, e.g. `C:\`.
    pub mount: String,
    pub media: DiskMedia,
    /// `0.0` means unknown.
    pub total_gb: f64,
    pub free_gb: f64,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub current_hz: u32,
    /// Highest refresh rate available at the current resolution.
    pub max_hz: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerPlan {
    Balanced,
    HighPerformance,
    PowerSaver,
    Ultimate,
    /// A plan whose GUID is not one of the built-in ones.
    Custom,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OsSettings {
    pub game_mode: Option<bool>,
    pub background_recording: Option<bool>,
    /// Hardware-accelerated GPU scheduling.
    pub hags: Option<bool>,
    pub memory_integrity: Option<bool>,
    pub power_plan: PowerPlan,
    pub startup_apps: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProcessUsage {
    pub pid: u32,
    pub name: String,
    /// Share of the whole CPU (100 = every core busy).
    pub cpu_pct: f32,
    pub mem_mb: u64,
    pub gpu_pct: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DriverInfo {
    pub device_name: String,
    /// Device class, e.g. `Display`, `Net`, `Media`.
    pub class: String,
    pub manufacturer: String,
    pub version: String,
    /// `YYYY-MM-DD`.
    pub date: Option<String>,
    pub hardware_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Store {
    #[default]
    Steam,
    Epic,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct InstalledGame {
    pub store: Store,
    /// Steam app id or Epic `AppName`.
    pub id: String,
    pub name: String,
    pub install_dir: String,
    pub size_bytes: Option<u64>,
}

impl InstalledGame {
    /// Stable key used for favorites, e.g. `steam:730` or `epic:Fortnite`.
    pub fn key(&self) -> String {
        let store = match self.store {
            Store::Steam => "steam",
            Store::Epic => "epic",
        };
        format!("{store}:{}", self.id)
    }
}

impl SystemSnapshot {
    /// The GPU games run on: the discrete GPU with the most VRAM, else the first GPU.
    pub fn primary_gpu(&self) -> Option<&GpuInfo> {
        self.gpus
            .iter()
            .filter(|g| !g.is_integrated)
            .max_by_key(|g| g.vram_mb)
            .or_else(|| self.gpus.first())
    }

    pub fn has_discrete_gpu(&self) -> bool {
        self.gpus.iter().any(|g| !g.is_integrated)
    }

    /// The Windows system drive (`C:`).
    pub fn system_disk(&self) -> Option<&DiskInfo> {
        self.disks.iter().find(|d| {
            d.mount
                .get(..2)
                .is_some_and(|p| p.eq_ignore_ascii_case("c:"))
        })
    }

    /// Whole days between `date` (`YYYY-MM-DD`) and the scan time.
    pub fn age_days(&self, date: &str) -> Option<i64> {
        let then = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
        let taken = DateTime::from_timestamp(self.taken_at_unix, 0)?.date_naive();
        Some((taken - then).num_days())
    }
}
