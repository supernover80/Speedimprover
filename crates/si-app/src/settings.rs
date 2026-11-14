//! User settings (`settings.toml`) and where the app keeps its files.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use si_core::games::recommend::Goal;

use crate::theme::ThemeChoice;

pub const DEFAULT_CATALOG_URL: &str =
    "https://raw.githubusercontent.com/supernover80/speedimprover/main/data/driver_catalog.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeChoice,
    /// Game keys such as `steam:730` or `epic:Fortnite`.
    pub favorites: Vec<String>,
    pub goal: Goal,
    pub presentmon_path: Option<PathBuf>,
    pub catalog_url: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::default(),
            favorites: Vec::new(),
            goal: Goal::default(),
            presentmon_path: None,
            catalog_url: DEFAULT_CATALOG_URL.to_string(),
        }
    }
}

impl Settings {
    /// Reads `path`; a missing or unreadable file gives the defaults.
    pub fn load(path: &Path) -> Settings {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Settings::default();
        };
        toml::from_str(&text).unwrap_or_else(|e| {
            tracing::warn!("ignoring unreadable settings file {}: {e}", path.display());
            Settings::default()
        })
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

/// Files the app reads and writes.
#[derive(Debug, Clone, PartialEq)]
pub struct AppPaths {
    pub config_file: PathBuf,
    pub last_scan: PathBuf,
    pub log_file: PathBuf,
}

impl AppPaths {
    /// The per-user config and data folders for SpeedImprover.
    pub fn from_project_dirs() -> Option<Self> {
        let dirs = directories::ProjectDirs::from("com", "SpeedImprover", "SpeedImprover")?;
        Some(Self {
            config_file: dirs.config_dir().join("settings.toml"),
            last_scan: dirs.data_dir().join("last_scan.json"),
            log_file: dirs.data_dir().join("speedimprover.log"),
        })
    }

    /// Everything in one folder (tests, fallback).
    pub fn in_dir(dir: &Path) -> Self {
        Self {
            config_file: dir.join("settings.toml"),
            last_scan: dir.join("last_scan.json"),
            log_file: dir.join("speedimprover.log"),
        }
    }
}
