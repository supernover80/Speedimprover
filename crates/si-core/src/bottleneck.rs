//! "What exactly limits my FPS?" — a verdict with the evidence behind it.
//!
//! With PresentMon running, every frame says how long the CPU and the GPU
//! were busy; whichever is busy for (almost) the whole frame is the limit.
//! Without it, the load-based classifier in [`crate::live`] is used. Either
//! way, memory, thermal and clock problems found in the load samples come
//! first, because they are the root cause when present.

use serde::Serialize;

use crate::frames::FrameSample;
use crate::live::{Bottleneck, LiveSample, classify};

/// Load samples considered (about ten seconds).
const WINDOW: usize = 10;
/// Frames with busy times needed for a per-frame verdict.
const MIN_FRAMES: usize = 30;
/// Share of the frame time at which a part counts as the limit.
const LIMIT_SHARE: f64 = 0.90;
/// Below this share a part is clearly waiting.
const WAITING_SHARE: f64 = 0.85;
/// FPS within this fraction of the refresh rate counts as locked to it.
const REFRESH_TOLERANCE: f64 = 0.03;
const HOT_CPU_C: f32 = 95.0;
const HOT_GPU_C: f32 = 90.0;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvidenceItem {
    pub label: String,
    pub value: String,
    /// 0–1, drawn as a bar.
    pub share: Option<f32>,
    /// This item is why the verdict was reached.
    pub limiting: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AnalysisSource {
    /// Measured per frame (number of frames).
    Frames(usize),
    /// Estimated from load samples (number of samples).
    Load(usize),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BottleneckReport {
    pub verdict: Bottleneck,
    pub headline: String,
    pub source: AnalysisSource,
    pub evidence: Vec<EvidenceItem>,
    pub actions: Vec<String>,
}

/// Mean of the values present.
fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, n) = values.fold((0.0, 0usize), |(s, n), v| (s + v, n + 1));
    (n > 0).then(|| sum / n as f64)
}

fn gb(mb: f64) -> String {
    format!("{:.1} GB", mb / 1024.0)
}

struct FrameMeans {
    count: usize,
    frame: f64,
    cpu: f64,
    gpu: f64,
}

fn frame_means(frames: &[FrameSample]) -> Option<FrameMeans> {
    let full: Vec<(f64, f64, f64)> = frames
        .iter()
        .filter_map(|f| Some((f.frame_ms, f.cpu_busy_ms?, f.gpu_busy_ms?)))
        .collect();
    if full.len() < MIN_FRAMES {
        return None;
    }
    let n = full.len() as f64;
    Some(FrameMeans {
        count: full.len(),
        frame: full.iter().map(|f| f.0).sum::<f64>() / n,
        cpu: full.iter().map(|f| f.1).sum::<f64>() / n,
        gpu: full.iter().map(|f| f.2).sum::<f64>() / n,
    })
}

fn frame_verdict(m: &FrameMeans) -> Bottleneck {
    let gpu_share = m.gpu / m.frame;
    let cpu_share = m.cpu / m.frame;
    if gpu_share >= LIMIT_SHARE {
        Bottleneck::GpuBound
    } else if cpu_share >= LIMIT_SHARE && gpu_share < WAITING_SHARE {
        Bottleneck::CpuBound
    } else if cpu_share < WAITING_SHARE && gpu_share < WAITING_SHARE {
        Bottleneck::FrameCapped
    } else {
        Bottleneck::Balanced
    }
}

fn headline(verdict: Bottleneck, frames: Option<&FrameMeans>, max_hz: Option<u32>) -> String {
    match verdict {
        Bottleneck::InsufficientData => "Collecting data…".into(),
        Bottleneck::VramLimited => "Video memory (VRAM) is full".into(),
        Bottleneck::RamLimited => "System RAM is full".into(),
        Bottleneck::CpuThrottled => "The CPU is throttling".into(),
        Bottleneck::GpuThrottled => "The GPU is throttling".into(),
        Bottleneck::GpuBound => "The GPU is the bottleneck".into(),
        Bottleneck::CpuBound => "The CPU is the bottleneck".into(),
        Bottleneck::Balanced => "No single bottleneck".into(),
        Bottleneck::FrameCapped => match frames {
            Some(m) => {
                let fps = 1000.0 / m.frame;
                match max_hz.filter(|&hz| {
                    hz > 0 && (fps - f64::from(hz)).abs() <= f64::from(hz) * REFRESH_TOLERANCE
                }) {
                    Some(hz) => {
                        format!("Frame rate is capped at your monitor's {hz} Hz refresh rate")
                    }
                    None => format!("Frame rate is capped at about {fps:.0} FPS"),
                }
            }
            None => "Frame rate is capped".into(),
        },
    }
}

fn actions(verdict: Bottleneck) -> Vec<String> {
    let lines: &[&str] = match verdict {
        Bottleneck::InsufficientData => &["Keep the game running for a few seconds."],
        Bottleneck::VramLimited => &[
            "Lower texture quality and resolution.",
            "Close other apps that use the GPU, such as browsers playing video or recording software.",
        ],
        Bottleneck::RamLimited => &[
            "Close background apps to free memory.",
            "16 GB or more of RAM is recommended for current games.",
        ],
        Bottleneck::CpuThrottled => &[
            "Check CPU cooling: dust, fan speed and thermal paste.",
            "Use the Balanced or High performance power plan, and keep laptops plugged in.",
        ],
        Bottleneck::GpuThrottled => &[
            "Improve GPU cooling: clean dust, raise the fan curve, improve case airflow.",
            "Cap the frame rate or lower the power target to reduce heat.",
        ],
        Bottleneck::GpuBound => &[
            "Lower the resolution or turn on upscaling (DLSS, FSR or XeSS).",
            "Reduce GPU-heavy settings: shadows, volumetric effects and ray tracing.",
            "See the Games page for settings tuned to your GPU.",
        ],
        Bottleneck::CpuBound => &[
            "Lower CPU-heavy settings: view distance, crowd density and physics.",
            "Close background apps that use CPU time.",
            "Make sure memory runs at its rated speed (XMP/EXPO).",
        ],
        Bottleneck::FrameCapped => &[
            "A frame cap or V-Sync is limiting FPS. Raise or remove it in the game for more FPS.",
            "If the cap matches your monitor's refresh rate, this is the smoothest setting.",
        ],
        Bottleneck::Balanced => &[
            "No single part is holding back FPS. Raise settings for better visuals, or cap FPS for steadier frame times.",
        ],
    };
    lines.iter().map(|s| s.to_string()).collect()
}

/// The full report. `samples` are live load samples (oldest first), `frames`
/// the PresentMon frames of the last few seconds, `display_max_hz` the main
/// monitor's highest refresh rate.
pub fn analyze_bottleneck(
    samples: &[LiveSample],
    frames: &[FrameSample],
    display_max_hz: Option<u32>,
) -> BottleneckReport {
    let window = &samples[samples.len().saturating_sub(WINDOW)..];
    let load_verdict = classify(samples);
    let frame_data = frame_means(frames);

    let root_cause = matches!(
        load_verdict,
        Bottleneck::VramLimited
            | Bottleneck::RamLimited
            | Bottleneck::CpuThrottled
            | Bottleneck::GpuThrottled
    );
    let (verdict, source) = match &frame_data {
        Some(m) if !root_cause => (frame_verdict(m), AnalysisSource::Frames(m.count)),
        Some(m) => (load_verdict, AnalysisSource::Frames(m.count)),
        None => (load_verdict, AnalysisSource::Load(window.len())),
    };
    let per_frame = frame_data.is_some();

    let mut evidence = Vec::new();
    if let Some(m) = &frame_data {
        evidence.push(EvidenceItem {
            label: "GPU busy".into(),
            value: format!("{:.1} ms of {:.1} ms per frame", m.gpu, m.frame),
            share: Some((m.gpu / m.frame).min(1.0) as f32),
            limiting: verdict == Bottleneck::GpuBound,
        });
        evidence.push(EvidenceItem {
            label: "CPU busy".into(),
            value: format!("{:.1} ms of {:.1} ms per frame", m.cpu, m.frame),
            share: Some((m.cpu / m.frame).min(1.0) as f32),
            limiting: verdict == Bottleneck::CpuBound,
        });
        evidence.push(EvidenceItem {
            label: "Frame rate".into(),
            value: format!("{:.0} FPS average", 1000.0 / m.frame),
            share: None,
            limiting: verdict == Bottleneck::FrameCapped,
        });
    }

    let avg = |f: &dyn Fn(&LiveSample) -> Option<f64>| mean(window.iter().filter_map(f));
    if let Some(gpu) = avg(&|s| s.gpu_pct.map(f64::from)) {
        evidence.push(EvidenceItem {
            label: "GPU load".into(),
            value: format!("{gpu:.0}%"),
            share: Some((gpu / 100.0) as f32),
            limiting: !per_frame && verdict == Bottleneck::GpuBound,
        });
    }
    if let Some(core) = avg(&|s| Some(f64::from(s.cpu_max_core_pct))) {
        evidence.push(EvidenceItem {
            label: "Busiest CPU thread".into(),
            value: format!("{core:.0}%"),
            share: Some((core / 100.0) as f32),
            limiting: !per_frame && verdict == Bottleneck::CpuBound,
        });
    }
    if let Some(total) = avg(&|s| Some(f64::from(s.cpu_total_pct))) {
        evidence.push(EvidenceItem {
            label: "Total CPU load".into(),
            value: format!("{total:.0}%"),
            share: Some((total / 100.0) as f32),
            limiting: false,
        });
    }
    if let Some(perf) = avg(&|s| s.cpu_perf_pct.map(f64::from)) {
        evidence.push(EvidenceItem {
            label: "CPU clock".into(),
            value: format!("{perf:.0}% of base speed"),
            share: Some((perf / 100.0).min(1.0) as f32),
            limiting: verdict == Bottleneck::CpuThrottled,
        });
    }
    if let (Some(used), Some(total)) = (
        avg(&|s| s.vram_used_mb.map(|v| v as f64)),
        avg(&|s| s.vram_total_mb.filter(|&t| t > 0).map(|v| v as f64)),
    ) {
        evidence.push(EvidenceItem {
            label: "Video memory".into(),
            value: format!("{} of {}", gb(used), gb(total)),
            share: Some((used / total).min(1.0) as f32),
            limiting: verdict == Bottleneck::VramLimited,
        });
    }
    if let (Some(used), Some(total)) = (
        avg(&|s| (s.ram_total_mb > 0).then_some(s.ram_used_mb as f64)),
        avg(&|s| (s.ram_total_mb > 0).then_some(s.ram_total_mb as f64)),
    ) {
        evidence.push(EvidenceItem {
            label: "System memory".into(),
            value: format!("{} of {}", gb(used), gb(total)),
            share: Some((used / total).min(1.0) as f32),
            limiting: verdict == Bottleneck::RamLimited,
        });
    }
    if let Some(t) = avg(&|s| s.cpu_temp_c.map(f64::from)) {
        let exact = window.iter().any(|s| s.cpu_temp_exact);
        let t = t as f32;
        evidence.push(EvidenceItem {
            label: "CPU temperature".into(),
            value: format!("{t:.0} °C{}", if exact { "" } else { " (approx.)" }),
            share: None,
            limiting: t >= HOT_CPU_C || verdict == Bottleneck::CpuThrottled,
        });
    }
    if let Some(t) = avg(&|s| s.gpu_temp_c.map(f64::from)) {
        let t = t as f32;
        evidence.push(EvidenceItem {
            label: "GPU temperature".into(),
            value: format!("{t:.0} °C"),
            share: None,
            limiting: t >= HOT_GPU_C || verdict == Bottleneck::GpuThrottled,
        });
    }
    if let Some(top) = window.last().and_then(|s| {
        s.top_processes
            .iter()
            .max_by(|a, b| a.cpu_pct.total_cmp(&b.cpu_pct))
    }) {
        let gpu = top
            .gpu_pct
            .map(|g| format!(", {g:.0}% GPU"))
            .unwrap_or_default();
        evidence.push(EvidenceItem {
            label: "Heaviest app".into(),
            value: format!("{} — {:.0}% CPU{gpu}", top.name, top.cpu_pct),
            share: None,
            limiting: false,
        });
    }

    let mut actions = actions(verdict);
    if !per_frame && verdict != Bottleneck::InsufficientData {
        actions.push("Measure FPS with PresentMon for an exact, per-frame answer.".into());
    }

    BottleneckReport {
        verdict,
        headline: headline(verdict, frame_data.as_ref(), display_max_hz),
        source,
        evidence,
        actions,
    }
}
