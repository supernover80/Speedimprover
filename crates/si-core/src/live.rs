//! Live monitoring: one sample per second, and a verdict on what limits FPS
//! right now (spec §6).

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::model::ProcessUsage;

/// Samples the verdict looks at.
const WINDOW: usize = 10;
/// Fewer samples than this ⇒ `InsufficientData`.
const MIN_SAMPLES: usize = 5;

/// One reading of the machine's load. `None` = metric not available.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LiveSample {
    /// Milliseconds since sampling started.
    pub t_ms: u64,
    /// Whole-CPU load, 0–100.
    pub cpu_total_pct: f32,
    /// Load of the busiest logical core, 0–100.
    pub cpu_max_core_pct: f32,
    /// `% Processor Performance`: below 100 means the CPU runs under its base clock.
    pub cpu_perf_pct: Option<f32>,
    pub gpu_pct: Option<f32>,
    pub vram_used_mb: Option<u64>,
    pub vram_total_mb: Option<u64>,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub gpu_temp_c: Option<f32>,
    /// CPU temperature in °C.
    pub cpu_temp_c: Option<f32>,
    /// True when `cpu_temp_c` is the CPU's own sensor (LibreHardwareMonitor);
    /// false for the approximate ACPI thermal-zone reading.
    pub cpu_temp_exact: bool,
    /// GPU is thermally throttling (NVML; `None` on non-NVIDIA GPUs).
    pub gpu_throttling: Option<bool>,
    pub top_processes: Vec<ProcessUsage>,
}

/// What is limiting FPS, in the priority order the classifier checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bottleneck {
    InsufficientData,
    VramLimited,
    RamLimited,
    CpuThrottled,
    GpuThrottled,
    GpuBound,
    CpuBound,
    FrameCapped,
    Balanced,
}

impl Bottleneck {
    /// One-line explanation shown in the UI (exact spec copy).
    pub fn explanation(&self) -> &'static str {
        match self {
            Bottleneck::InsufficientData => "Collecting data…",
            Bottleneck::VramLimited => {
                "Video memory is full — lower texture quality or resolution."
            }
            Bottleneck::RamLimited => "System RAM is nearly full — close background apps.",
            Bottleneck::CpuThrottled => {
                "CPU is slowing down (heat or power limit) — check cooling and power plan."
            }
            Bottleneck::GpuThrottled => {
                "GPU is throttling from heat — check cooling and fan curve."
            }
            Bottleneck::GpuBound => {
                "GPU is the limit — lower resolution, enable upscaling or reduce GPU-heavy settings."
            }
            Bottleneck::CpuBound => {
                "CPU is the limit — lower CPU-heavy settings (view distance, crowds, physics) and close background apps."
            }
            Bottleneck::FrameCapped => {
                "Neither CPU nor GPU is maxed — a frame cap or V-Sync is probably limiting FPS."
            }
            Bottleneck::Balanced => "No single bottleneck — the system is well balanced.",
        }
    }
}

/// Average of the values that are present; `None` if none are.
fn avg(window: &[LiveSample], f: impl Fn(&LiveSample) -> Option<f64>) -> Option<f64> {
    let values: Vec<f64> = window.iter().filter_map(f).collect();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

/// Classifies the last 10 samples. Missing metrics make their condition false.
pub fn classify(samples: &[LiveSample]) -> Bottleneck {
    let w = &samples[samples.len().saturating_sub(WINDOW)..];
    if w.len() < MIN_SAMPLES {
        return Bottleneck::InsufficientData;
    }
    let cpu_total = avg(w, |s| Some(s.cpu_total_pct.into())).unwrap_or(0.0);
    let busiest_core = avg(w, |s| Some(s.cpu_max_core_pct.into())).unwrap_or(0.0);
    let cpu_perf = avg(w, |s| s.cpu_perf_pct.map(f64::from));
    let gpu = avg(w, |s| s.gpu_pct.map(f64::from));
    let vram_used = avg(w, |s| s.vram_used_mb.map(|v| v as f64));
    let vram_total = avg(w, |s| s.vram_total_mb.filter(|&t| t > 0).map(|v| v as f64));
    let ram_used = avg(w, |s| (s.ram_total_mb > 0).then_some(s.ram_used_mb as f64));
    let ram_total = avg(w, |s| (s.ram_total_mb > 0).then_some(s.ram_total_mb as f64));
    let throttled = w.iter().filter(|s| s.gpu_throttling == Some(true)).count();

    let at_least = |value: Option<f64>, limit: f64| value.is_some_and(|v| v >= limit);
    let below = |value: Option<f64>, limit: f64| value.is_some_and(|v| v < limit);

    if let (Some(used), Some(total)) = (vram_used, vram_total) {
        if used >= 0.95 * total {
            return Bottleneck::VramLimited;
        }
    }
    if let (Some(used), Some(total)) = (ram_used, ram_total) {
        if used >= 0.92 * total {
            return Bottleneck::RamLimited;
        }
    }
    if below(cpu_perf, 80.0) && cpu_total >= 50.0 {
        return Bottleneck::CpuThrottled;
    }
    if throttled * 2 >= w.len() {
        return Bottleneck::GpuThrottled;
    }
    if at_least(gpu, 95.0) {
        return Bottleneck::GpuBound;
    }
    if below(gpu, 80.0) && (busiest_core >= 90.0 || cpu_total >= 90.0) {
        return Bottleneck::CpuBound;
    }
    if below(gpu, 80.0) && busiest_core < 90.0 {
        return Bottleneck::FrameCapped;
    }
    Bottleneck::Balanced
}

/// Ring buffer of recent samples (the UI keeps 120 = two minutes).
#[derive(Debug, Clone, Default)]
pub struct LiveHistory {
    capacity: usize,
    samples: VecDeque<LiveSample>,
}

impl LiveHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            samples: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, s: LiveSample) {
        if self.capacity == 0 {
            return;
        }
        while self.samples.len() >= self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(s);
    }

    pub fn latest(&self) -> Option<&LiveSample> {
        self.samples.back()
    }

    /// Oldest first.
    pub fn samples(&self) -> impl Iterator<Item = &LiveSample> {
        self.samples.iter()
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn verdict(&self) -> Bottleneck {
        let start = self.samples.len().saturating_sub(WINDOW);
        let recent: Vec<LiveSample> = self.samples.range(start..).cloned().collect();
        classify(&recent)
    }
}
