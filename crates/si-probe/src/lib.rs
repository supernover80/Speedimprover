//! SpeedImprover data collection.
//!
//! [`SystemProbe`] is the one interface the app uses. [`generic::GenericProbe`]
//! reads what `sysinfo` can on any OS; the Windows probe adds WMI, registry,
//! DXGI, PDH and NVML data; [`fixture::FixtureProbe`] replays a saved scan.
//! A collector that fails never aborts a scan: it records a [`ScanWarning`]
//! and leaves its fields at their "unknown" defaults.

use serde::{Deserialize, Serialize};
use si_core::drivers::AvailableDriverUpdate;
use si_core::live::LiveSample;
use si_core::model::{InstalledGame, SystemSnapshot};

pub mod catalog;
pub mod fixture;
pub mod generic;
pub mod gpu_counters;
pub mod mapping;
pub mod presentmon;
pub mod temps;
#[cfg(windows)]
pub mod win;
pub mod winsettings;

/// The stages of a scan, in the order they run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStep {
    Hardware,
    Memory,
    Storage,
    Displays,
    OsSettings,
    Processes,
    Drivers,
    Games,
}

impl ScanStep {
    pub const ALL: [ScanStep; 8] = [
        ScanStep::Hardware,
        ScanStep::Memory,
        ScanStep::Storage,
        ScanStep::Displays,
        ScanStep::OsSettings,
        ScanStep::Processes,
        ScanStep::Drivers,
        ScanStep::Games,
    ];

    /// Progress text shown while the step runs.
    pub fn label(self) -> &'static str {
        match self {
            ScanStep::Hardware => "Scanning hardware…",
            ScanStep::Memory => "Scanning memory…",
            ScanStep::Storage => "Scanning storage…",
            ScanStep::Displays => "Scanning displays…",
            ScanStep::OsSettings => "Reading Windows settings…",
            ScanStep::Processes => "Checking background apps…",
            ScanStep::Drivers => "Reading drivers…",
            ScanStep::Games => "Finding games…",
        }
    }
}

/// Something a scan could not read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanWarning {
    pub step: ScanStep,
    pub message: String,
}

impl ScanWarning {
    pub fn new(step: ScanStep, message: impl Into<String>) -> Self {
        Self {
            step,
            message: message.into(),
        }
    }
}

/// Everything one scan produced. `snapshot` is required in JSON, so a bare
/// snapshot file is not mistaken for an (empty) scan result.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScanResult {
    pub snapshot: SystemSnapshot,
    #[serde(default)]
    pub games: Vec<InstalledGame>,
    #[serde(default)]
    pub warnings: Vec<ScanWarning>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Windows API error: {0}")]
    Windows(String),
    #[error("network error: {0}")]
    Network(String),
}

pub trait SystemProbe {
    /// Runs a full scan, calling `progress` as each step starts.
    fn scan(&mut self, progress: &mut dyn FnMut(ScanStep)) -> ScanResult;
    /// One live-monitoring sample; call about once a second.
    fn sample(&mut self) -> LiveSample;
}

/// Builds a probe on the thread that will use it (WMI/COM objects are not `Send`).
pub type ProbeFactory = Box<dyn FnOnce() -> Box<dyn SystemProbe> + Send>;

/// The best probe for this OS.
pub fn default_probe() -> Box<dyn SystemProbe> {
    #[cfg(windows)]
    {
        Box::new(win::WindowsProbe::new())
    }
    #[cfg(not(windows))]
    {
        Box::new(generic::GenericProbe::new())
    }
}

/// Driver updates Windows Update offers (slow: run off the UI thread).
/// Always empty on non-Windows systems.
pub fn search_windows_update_drivers() -> Result<Vec<AvailableDriverUpdate>, ProbeError> {
    #[cfg(windows)]
    {
        win::wua::search_drivers()
    }
    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
}
pub mod install;

/// Installs the given Windows Update driver updates, asking Windows for
/// administrator rights (UAC). Blocks until the install finishes.
pub fn install_driver_updates_elevated(
    update_ids: &[String],
) -> Result<install::InstallReport, ProbeError> {
    #[cfg(windows)]
    {
        win::elevate::run_elevated_install(update_ids)
    }
    #[cfg(not(windows))]
    {
        let _ = update_ids;
        Err(ProbeError::Windows(
            "Driver installation is only available on Windows".into(),
        ))
    }
}

/// Runs inside the elevated helper: creates a restore point, then downloads
/// and installs the updates through Windows Update.
pub fn install_driver_updates_here(update_ids: &[String]) -> install::InstallReport {
    #[cfg(windows)]
    {
        win::install_with_restore_point(update_ids)
    }
    #[cfg(not(windows))]
    {
        install::InstallReport {
            failed: update_ids.to_vec(),
            ..Default::default()
        }
    }
}
