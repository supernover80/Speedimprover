//! What `sysinfo` can read on any OS: CPU, memory, disks, processes and live
//! load. The Windows probe uses this for the same parts and adds the rest.

use std::collections::HashSet;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use si_core::live::LiveSample;
use si_core::model::{DiskInfo, DiskMedia, ProcessUsage, SystemSnapshot};
use sysinfo::{Disks, ProcessesToUpdate, System};

use crate::{ScanResult, ScanStep, ScanWarning, SystemProbe};

const MB: u64 = 1024 * 1024;
const GB: f64 = 1024.0 * 1024.0 * 1024.0;
/// How many processes each "top" list keeps.
const TOP_PROCESSES: usize = 25;
/// Processes reported in each live sample.
const LIVE_TOP_PROCESSES: usize = 8;

/// Share of the whole CPU from a per-core percentage (`400 %` on 8 cores = 50 %).
pub fn normalize_cpu(raw_pct: f32, logical_cores: u32) -> f32 {
    if logical_cores == 0 {
        return 0.0;
    }
    (raw_pct / logical_cores as f32).clamp(0.0, 100.0)
}

/// Top 25 by CPU ∪ top 25 by memory, deduplicated, sorted by CPU (then memory).
pub fn select_processes(all: Vec<ProcessUsage>) -> Vec<ProcessUsage> {
    let mut by_cpu = all.clone();
    by_cpu.sort_by(|a, b| b.cpu_pct.total_cmp(&a.cpu_pct));
    let mut by_mem = all;
    by_mem.sort_by_key(|p| std::cmp::Reverse(p.mem_mb));
    let mut seen = HashSet::new();
    let mut selected: Vec<ProcessUsage> = by_cpu
        .into_iter()
        .take(TOP_PROCESSES)
        .chain(by_mem.into_iter().take(TOP_PROCESSES))
        .filter(|p| seen.insert(p.pid))
        .collect();
    selected.sort_by(|a, b| {
        b.cpu_pct
            .total_cmp(&a.cpu_pct)
            .then(b.mem_mb.cmp(&a.mem_mb))
    });
    selected
}

pub struct GenericProbe {
    sys: Option<System>,
    init_error: Option<String>,
    started: Instant,
}

impl Default for GenericProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericProbe {
    pub fn new() -> Self {
        let (sys, init_error) = match System::new() {
            Ok(mut sys) => {
                sys.refresh_cpu_all();
                (Some(sys), None)
            }
            Err(e) => (None, Some(e.to_string())),
        };
        Self {
            sys,
            init_error,
            started: Instant::now(),
        }
    }

    /// One process from the last refresh (used to name GPU-heavy processes).
    pub fn process_usage(&self, pid: u32) -> Option<ProcessUsage> {
        let sys = self.sys.as_ref()?;
        let p = sys.process(sysinfo::Pid::from_u32(pid))?;
        Some(ProcessUsage {
            pid,
            name: p.name().to_string_lossy().into_owned(),
            cpu_pct: normalize_cpu(p.cpu_usage(), self.logical_cores()),
            mem_mb: p.memory() / MB,
            gpu_pct: None,
        })
    }

    fn logical_cores(&self) -> u32 {
        self.sys.as_ref().map_or(0, |s| s.cpus().len() as u32)
    }

    /// Fills the parts of `step` that sysinfo can read. Steps it cannot help
    /// with (displays, Windows settings, drivers, games) are left alone.
    pub fn collect_step(
        &mut self,
        step: ScanStep,
        snap: &mut SystemSnapshot,
        warnings: &mut Vec<ScanWarning>,
    ) {
        if self.sys.is_none() {
            if matches!(
                step,
                ScanStep::Hardware | ScanStep::Memory | ScanStep::Processes
            ) {
                let why = self.init_error.as_deref().unwrap_or("unknown error");
                warnings.push(ScanWarning::new(
                    step,
                    format!("sysinfo unavailable: {why}"),
                ));
            }
            return;
        }
        match step {
            ScanStep::Hardware => self.collect_hardware(snap),
            ScanStep::Memory => self.collect_memory(snap),
            ScanStep::Storage => collect_disks(snap, warnings),
            ScanStep::Processes => self.collect_processes(snap),
            ScanStep::Displays | ScanStep::OsSettings | ScanStep::Drivers | ScanStep::Games => {}
        }
    }

    fn collect_hardware(&mut self, snap: &mut SystemSnapshot) {
        snap.machine.hostname = System::host_name().unwrap_or_default();
        snap.machine.os_name = System::long_os_version()
            .or_else(|_| System::name())
            .unwrap_or_default();
        snap.cpu.physical_cores = System::physical_core_count().unwrap_or(0) as u32;
        snap.cpu.logical_cores = self.logical_cores();
        if let Some(sys) = &self.sys {
            if let Some(cpu) = sys.cpus().first() {
                snap.cpu.name = cpu.brand().trim().to_string();
                snap.cpu.base_mhz = cpu.frequency() as u32;
            }
        }
    }

    fn collect_memory(&mut self, snap: &mut SystemSnapshot) {
        if let Some(sys) = &mut self.sys {
            sys.refresh_memory();
            snap.memory.total_mb = sys.total_memory() / MB;
            snap.memory.used_mb = sys.used_memory() / MB;
        }
    }

    fn collect_processes(&mut self, snap: &mut SystemSnapshot) {
        let logical = self.logical_cores();
        let Some(sys) = &mut self.sys else { return };
        // CPU usage is a difference between two refreshes.
        sys.refresh_processes(ProcessesToUpdate::All, true);
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        sys.refresh_processes(ProcessesToUpdate::All, true);
        snap.processes = select_processes(process_list(sys, logical));
    }
}

fn process_list(sys: &System, logical_cores: u32) -> Vec<ProcessUsage> {
    sys.processes()
        .values()
        .map(|p| ProcessUsage {
            pid: p.pid().as_u32(),
            name: p.name().to_string_lossy().into_owned(),
            cpu_pct: normalize_cpu(p.cpu_usage(), logical_cores),
            mem_mb: p.memory() / MB,
            gpu_pct: None,
        })
        .collect()
}

fn collect_disks(snap: &mut SystemSnapshot, warnings: &mut Vec<ScanWarning>) {
    match Disks::new_with_refreshed_list() {
        Ok(disks) => {
            snap.disks = disks
                .iter()
                .filter(|d| d.total_space() > 0)
                .map(|d| DiskInfo {
                    mount: d.mount_point().to_string_lossy().into_owned(),
                    media: DiskMedia::Unknown,
                    total_gb: d.total_space() as f64 / GB,
                    free_gb: d.available_space() as f64 / GB,
                })
                .collect();
        }
        Err(e) => warnings.push(ScanWarning::new(
            ScanStep::Storage,
            format!("could not list disks: {e}"),
        )),
    }
}

pub(crate) fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

impl SystemProbe for GenericProbe {
    fn scan(&mut self, progress: &mut dyn FnMut(ScanStep)) -> ScanResult {
        let mut snapshot = SystemSnapshot {
            taken_at_unix: now_unix(),
            ..Default::default()
        };
        let mut warnings = Vec::new();
        for step in ScanStep::ALL {
            progress(step);
            self.collect_step(step, &mut snapshot, &mut warnings);
        }
        ScanResult {
            snapshot,
            games: Vec::new(),
            warnings,
        }
    }

    fn sample(&mut self) -> LiveSample {
        let t_ms = self.started.elapsed().as_millis() as u64;
        let logical = self.logical_cores();
        let Some(sys) = &mut self.sys else {
            return LiveSample {
                t_ms,
                ..Default::default()
            };
        };
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let mut top = process_list(sys, logical);
        top.sort_by(|a, b| b.cpu_pct.total_cmp(&a.cpu_pct));
        top.truncate(LIVE_TOP_PROCESSES);
        LiveSample {
            t_ms,
            cpu_total_pct: sys.global_cpu_usage().clamp(0.0, 100.0),
            cpu_max_core_pct: sys
                .cpus()
                .iter()
                .map(|c| c.usage())
                .fold(0.0, f32::max)
                .clamp(0.0, 100.0),
            ram_used_mb: sys.used_memory() / MB,
            ram_total_mb: sys.total_memory() / MB,
            top_processes: top,
            ..Default::default()
        }
    }
}
