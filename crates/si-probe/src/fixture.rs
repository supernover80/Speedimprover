//! Replays a saved scan so the app can run on any OS (`--fixture <file>`).

use std::path::Path;

use si_core::live::LiveSample;
use si_core::model::SystemSnapshot;

use crate::{ProbeError, ScanResult, ScanStep, SystemProbe};

pub struct FixtureProbe {
    result: ScanResult,
    ticks: u64,
}

impl FixtureProbe {
    pub fn new(result: ScanResult) -> Self {
        Self { result, ticks: 0 }
    }

    /// Loads a `ScanResult` JSON file, or a bare `SystemSnapshot` JSON file.
    pub fn load(path: &Path) -> Result<Self, ProbeError> {
        let text = std::fs::read_to_string(path)?;
        let result = match serde_json::from_str::<ScanResult>(&text) {
            Ok(result) => result,
            Err(_) => ScanResult {
                snapshot: serde_json::from_str::<SystemSnapshot>(&text)?,
                ..Default::default()
            },
        };
        Ok(Self::new(result))
    }
}

impl SystemProbe for FixtureProbe {
    /// Returns the file's scan unchanged (its `taken_at_unix` keeps ages reproducible).
    fn scan(&mut self, progress: &mut dyn FnMut(ScanStep)) -> ScanResult {
        for step in ScanStep::ALL {
            progress(step);
        }
        self.result.clone()
    }

    /// A deterministic, gently moving load so the UI looks alive in demo mode.
    fn sample(&mut self) -> LiveSample {
        let t = self.ticks as f64;
        self.ticks += 1;
        let snap = &self.result.snapshot;
        let cpu = (35.0 + 15.0 * (t / 7.0).sin()) as f32;
        let vram_total = snap.primary_gpu().map(|g| g.vram_mb).filter(|&v| v > 0);
        LiveSample {
            t_ms: self.ticks * 1000,
            cpu_total_pct: cpu,
            cpu_max_core_pct: (cpu + 30.0).min(100.0),
            cpu_perf_pct: Some(100.0),
            gpu_pct: Some((70.0 + 25.0 * (t / 5.0).sin()) as f32),
            vram_used_mb: vram_total.map(|v| v * 6 / 10),
            vram_total_mb: vram_total,
            ram_used_mb: snap.memory.used_mb,
            ram_total_mb: snap.memory.total_mb,
            gpu_temp_c: Some((62.0 + 8.0 * (t / 11.0).sin()) as f32),
            gpu_throttling: Some(false),
            top_processes: snap.processes.iter().take(8).cloned().collect(),
        }
    }
}
