//! Windows-only collectors and the full [`WindowsProbe`].

use std::panic::{AssertUnwindSafe, catch_unwind};

use si_core::live::LiveSample;
use si_core::model::SystemSnapshot;

use crate::generic::{GenericProbe, now_unix};
use crate::gpu_counters::aggregate_gpu;
use crate::temps::{cpu_temp_from_sensors, gpu_temp_from_sensors, thermal_zone_celsius};
use crate::{ScanResult, ScanStep, ScanWarning, SystemProbe};

pub mod display;
pub mod dxgi;
pub mod elevate;
pub mod games;
pub mod gpu_perf;
pub mod lhm;
pub mod nvml;
pub mod pdh;
pub mod power;
pub mod registry;
pub mod restore;
pub mod wmi;
pub mod wua;

/// Processes added to a live sample for their GPU use.
const LIVE_TOP_GPU_PROCESSES: usize = 8;

pub struct WindowsProbe {
    generic: GenericProbe,
    pdh: Option<pdh::PdhSampler>,
    nvml: Option<nvml::NvmlReader>,
    lhm: lhm::LhmReader,
    /// Dedicated VRAM of the primary GPU, from the last scan or DXGI.
    vram_total_mb: Option<u64>,
    /// LUID of the primary GPU, for its temperature.
    primary_luid: Option<u64>,
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
            pdh: pdh::PdhSampler::new().ok(),
            nvml: nvml::NvmlReader::new(),
            lhm: lhm::LhmReader::new(),
            vram_total_mb: None,
            primary_luid: None,
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
        if let Some(gpu) = snapshot.primary_gpu() {
            self.vram_total_mb = Some(gpu.vram_mb).filter(|&v| v > 0);
            self.primary_luid = gpu.luid;
        }
        ScanResult {
            snapshot,
            games,
            warnings,
        }
    }

    fn sample(&mut self) -> LiveSample {
        let mut sample = self.generic.sample();
        if self.vram_total_mb.is_none() || self.primary_luid.is_none() {
            let gpus = dxgi::enumerate_gpus().unwrap_or_default();
            let primary = gpus
                .iter()
                .filter(|g| !g.is_integrated)
                .max_by_key(|g| g.vram_mb)
                .or(gpus.first());
            self.vram_total_mb = primary.map(|g| g.vram_mb).filter(|&v| v > 0);
            self.primary_luid = primary.and_then(|g| g.luid);
        }
        sample.vram_total_mb = self.vram_total_mb;

        let mut thermal_zones = Vec::new();
        if let Some(pdh) = &mut self.pdh {
            let reading = pdh.collect();
            thermal_zones = reading.thermal_zones;
            sample.cpu_perf_pct = reading.cpu_perf_pct;
            sample.vram_used_mb = reading.vram_used_mb;
            if !reading.gpu_instances.is_empty() {
                let gpu = aggregate_gpu(&reading.gpu_instances);
                sample.gpu_pct = Some(gpu.total_pct);
                for p in &mut sample.top_processes {
                    p.gpu_pct = Some(gpu.per_pid.get(&p.pid).copied().unwrap_or(0.0));
                }
                // Add the heaviest GPU users even if their CPU use is low.
                let mut by_gpu: Vec<(u32, f32)> =
                    gpu.per_pid.into_iter().filter(|&(_, v)| v > 0.0).collect();
                by_gpu.sort_by(|a, b| b.1.total_cmp(&a.1));
                for (pid, pct) in by_gpu.into_iter().take(LIVE_TOP_GPU_PROCESSES) {
                    if sample.top_processes.iter().any(|p| p.pid == pid) {
                        continue;
                    }
                    if let Some(mut p) = self.generic.process_usage(pid) {
                        p.gpu_pct = Some(pct);
                        sample.top_processes.push(p);
                    }
                }
            }
        }

        if let Some(nv) = self.nvml.as_ref().and_then(nvml::NvmlReader::read) {
            sample.gpu_pct = Some(nv.util_pct);
            sample.vram_used_mb = Some(nv.mem_used_mb);
            sample.vram_total_mb = Some(nv.mem_total_mb)
                .filter(|&t| t > 0)
                .or(sample.vram_total_mb);
            sample.gpu_temp_c = Some(nv.temp_c);
            sample.gpu_throttling = Some(nv.throttling);
        }

        // Temperatures: LibreHardwareMonitor when it runs (exact), else the
        // ACPI thermal zone for the CPU and the graphics kernel for the GPU.
        let sensors = self.lhm.read();
        if let Some(t) = cpu_temp_from_sensors(&sensors) {
            sample.cpu_temp_c = Some(t);
            sample.cpu_temp_exact = true;
        } else {
            sample.cpu_temp_c = thermal_zone_celsius(&thermal_zones);
        }
        if sample.gpu_temp_c.is_none() {
            sample.gpu_temp_c = gpu_temp_from_sensors(&sensors)
                .or_else(|| self.primary_luid.and_then(gpu_perf::gpu_temperature));
        }
        sample
    }
}

/// Runs in the elevated helper: restore point, then the Windows Update install.
pub fn install_with_restore_point(update_ids: &[String]) -> crate::install::InstallReport {
    let restore_point = restore::begin();
    let mut report =
        wua::install_drivers(update_ids).unwrap_or_else(|e| crate::install::InstallReport {
            failed: vec![format!("Windows Update error: {e}")],
            ..Default::default()
        });
    if let Some(sequence) = restore_point {
        restore::end(sequence);
    }
    report.restore_point_created = restore_point.is_some();
    report
}
