//! Windows-only collectors and the full [`WindowsProbe`].

use std::panic::{AssertUnwindSafe, catch_unwind};

use si_core::live::LiveSample;
use si_core::model::SystemSnapshot;

use crate::generic::{GenericProbe, now_unix};
use crate::{ScanResult, ScanStep, ScanWarning, SystemProbe};

pub mod display;
pub mod dxgi;
pub mod games;
pub mod power;
pub mod registry;
pub mod wmi;

pub struct WindowsProbe {
    generic: GenericProbe,
}

impl Default for WindowsProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsProbe {
    pub fn new() -> Self {
        Self {
            generic: GenericProbe::new(),
        }
    }

    /// The Windows-specific part of one step (after the sysinfo part).
    fn collect_step(
        step: ScanStep,
        snap: &mut SystemSnapshot,
        games: &mut Vec<si_core::model::InstalledGame>,
        warnings: &mut Vec<ScanWarning>,
    ) {
        match step {
            ScanStep::Hardware => {
                registry::collect_os_info(snap);
                wmi::collect_cpu(snap, warnings);
                match dxgi::enumerate_gpus() {
                    Ok(gpus) => snap.gpus = gpus,
                    Err(e) => {
                        warnings.push(ScanWarning::new(step, format!("GPU list unavailable: {e}")))
                    }
                }
                wmi::collect_gpu_drivers(snap, warnings);
                power::collect_battery(snap, warnings);
            }
            ScanStep::Memory => wmi::collect_memory_modules(snap, warnings),
            ScanStep::Storage => wmi::collect_disk_media(snap, warnings),
            ScanStep::Displays => display::collect_displays(snap),
            ScanStep::OsSettings => {
                registry::collect_os_settings(snap);
                power::collect_power_plan(snap, warnings);
                wmi::collect_memory_integrity(snap, warnings);
            }
            ScanStep::Processes => {}
            ScanStep::Drivers => wmi::collect_drivers(snap, warnings),
            ScanStep::Games => {
                let (found, game_warnings) = games::discover_games();
                *games = found;
                warnings.extend(game_warnings);
            }
        }
    }
}

impl SystemProbe for WindowsProbe {
    fn scan(&mut self, progress: &mut dyn FnMut(ScanStep)) -> ScanResult {
        let mut snapshot = SystemSnapshot {
            taken_at_unix: now_unix(),
            ..Default::default()
        };
        let mut games = Vec::new();
        let mut warnings = Vec::new();
        for step in ScanStep::ALL {
            progress(step);
            self.generic
                .collect_step(step, &mut snapshot, &mut warnings);
            // A bug in one collector must not abort the whole scan.
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                let mut step_warnings = Vec::new();
                Self::collect_step(step, &mut snapshot, &mut games, &mut step_warnings);
                step_warnings
            }));
            match outcome {
                Ok(step_warnings) => warnings.extend(step_warnings),
                Err(_) => warnings.push(ScanWarning::new(step, "internal error while scanning")),
            }
        }
        ScanResult {
            snapshot,
            games,
            warnings,
        }
    }

    fn sample(&mut self) -> LiveSample {
        self.generic.sample()
    }
}
