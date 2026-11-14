//! Game profiles: the FPS-relevant settings of one game and their value per
//! hardware tier, loaded from `data/games/*.toml` (spec §8).

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::model::{InstalledGame, Store};
use crate::tier::Tier;

const BUNDLED_PROFILES: [&str; 8] = [
    include_str!("../../../../data/games/cs2.toml"),
    include_str!("../../../../data/games/dota2.toml"),
    include_str!("../../../../data/games/apex.toml"),
    include_str!("../../../../data/games/pubg.toml"),
    include_str!("../../../../data/games/cyberpunk2077.toml"),
    include_str!("../../../../data/games/eldenring.toml"),
    include_str!("../../../../data/games/bg3.toml"),
    include_str!("../../../../data/games/fortnite.toml"),
];

/// How much FPS raising this setting costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Impact {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingSpec {
    /// Exactly as in the game's graphics menu.
    pub label: String,
    pub impact: Impact,
    /// Texture/model settings capped by the GPU's VRAM.
    #[serde(default)]
    pub vram_sensitive: bool,
    pub low: String,
    pub mid: String,
    pub high: String,
    pub ultra: String,
    /// Fixed value for the Competitive goal, where players commonly agree on one.
    #[serde(default)]
    pub competitive: Option<String>,
}

impl SettingSpec {
    pub fn value_for(&self, t: Tier) -> &str {
        match t {
            Tier::Low => &self.low,
            Tier::Mid => &self.mid,
            Tier::High => &self.high,
            Tier::Ultra => &self.ultra,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub steam_appid: Option<u32>,
    #[serde(default)]
    pub epic_app_name: Option<String>,
    pub settings: Vec<SettingSpec>,
}

impl GameProfile {
    pub fn parse(toml: &str) -> Result<Self, CoreError> {
        let mut profile: GameProfile = toml::from_str(toml)?;
        profile.epic_app_name = profile.epic_app_name.filter(|n| !n.is_empty());
        Ok(profile)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProfileLibrary {
    pub profiles: Vec<GameProfile>,
}

impl ProfileLibrary {
    /// The profiles compiled into the binary. A profile that fails to parse is
    /// skipped (the test suite checks all of them parse).
    pub fn bundled() -> Self {
        Self {
            profiles: BUNDLED_PROFILES
                .iter()
                .filter_map(|t| GameProfile::parse(t).ok())
                .collect(),
        }
    }

    pub fn for_game(&self, g: &InstalledGame) -> Option<&GameProfile> {
        self.profiles.iter().find(|p| match g.store {
            Store::Steam => p.steam_appid.is_some() && p.steam_appid == g.id.parse().ok(),
            Store::Epic => p.epic_app_name.as_deref() == Some(g.id.as_str()),
        })
    }
}
