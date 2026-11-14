//! Picks a value for each setting of a game from the PC's tier and the
//! player's goal (spec §8 "Recommendation per setting").

use serde::{Deserialize, Serialize};

use crate::games::profile::{GameProfile, Impact};
use crate::tier::{HardwareProfile, Tier};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Goal {
    /// Highest FPS.
    Competitive,
    #[default]
    Balanced,
    /// Best looks the PC can handle.
    Quality,
}

/// Why a value was chosen (shown as a tooltip).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    Tier(Tier),
    VramCap(u64),
    Competitive,
    QualityBoost,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    pub label: String,
    pub value: String,
    pub impact: Impact,
    pub reason: Reason,
}

/// Highest tier a VRAM-sensitive setting may use.
fn vram_cap(vram_mb: u64) -> Tier {
    match vram_mb {
        mb if mb < 4_096 => Tier::Low,
        mb if mb < 6_144 => Tier::Mid,
        mb if mb < 8_192 => Tier::High,
        _ => Tier::Ultra,
    }
}

/// One recommendation per setting, in the profile's order.
pub fn recommend(p: &GameProfile, hw: &HardwareProfile, goal: Goal) -> Vec<Recommendation> {
    p.settings
        .iter()
        .map(|s| {
            let mut tier = hw.overall;
            let mut boosted = false;
            match goal {
                Goal::Competitive => tier = tier.down(),
                Goal::Balanced => {}
                Goal::Quality if s.impact == Impact::Low => {
                    boosted = tier.up() != tier;
                    tier = tier.up();
                }
                Goal::Quality => {}
            }
            let mut capped = false;
            if s.vram_sensitive {
                let cap = vram_cap(hw.vram_mb);
                if cap < tier {
                    tier = cap;
                    capped = true;
                }
            }
            let tier_value = s.value_for(tier).to_string();
            let (value, reason) = if goal == Goal::Competitive {
                (
                    s.competitive.clone().unwrap_or(tier_value),
                    Reason::Competitive,
                )
            } else if capped {
                (tier_value, Reason::VramCap(hw.vram_mb))
            } else if boosted {
                (tier_value, Reason::QualityBoost)
            } else {
                (tier_value, Reason::Tier(tier))
            };
            Recommendation {
                label: s.label.clone(),
                value,
                impact: s.impact,
                reason,
            }
        })
        .collect()
}
