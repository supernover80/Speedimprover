//! Real FPS from PresentMon's CSV output: per-frame timing and statistics.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// Frame-time column names: PresentMon 1.x / `--v1_metrics`, then 2.x.
const FRAME_TIME_COLUMNS: [&str; 2] = ["MsBetweenPresents", "FrameTime"];
/// How long the CPU worked on a frame (PresentMon 2.x).
const CPU_BUSY_COLUMNS: [&str; 2] = ["CPUBusy", "MsCPUBusy"];
/// How long the GPU worked on a frame (PresentMon 2.x; `MsGPUActive` in 1.9).
const GPU_BUSY_COLUMNS: [&str; 3] = ["GPUBusy", "MsGPUBusy", "MsGPUActive"];

/// One presented frame. The busy times come from PresentMon 2.x and say how
/// much of the frame the CPU and the GPU were actually working.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FrameSample {
    pub frame_ms: f64,
    pub cpu_busy_ms: Option<f64>,
    pub gpu_busy_ms: Option<f64>,
}

impl FrameSample {
    pub fn new(frame_ms: f64) -> Self {
        Self {
            frame_ms,
            cpu_busy_ms: None,
            gpu_busy_ms: None,
        }
    }
}

/// Streams PresentMon CSV lines. The first line is the header; every later
/// line yields a frame, or `None` if it has no usable frame time.
#[derive(Debug, Clone, Default)]
pub struct FrameTimeParser {
    header_seen: bool,
    frame: Option<usize>,
    cpu_busy: Option<usize>,
    gpu_busy: Option<usize>,
}

fn find_column(headers: &[&str], names: &[&str]) -> Option<usize> {
    names
        .iter()
        .find_map(|name| headers.iter().position(|h| h.eq_ignore_ascii_case(name)))
}

fn number(fields: &[&str], column: Option<usize>) -> Option<f64> {
    let value: f64 = fields.get(column?)?.trim().parse().ok()?;
    (value.is_finite() && value >= 0.0).then_some(value)
}

impl FrameTimeParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, line: &str) -> Option<FrameSample> {
        let line = line.trim();
        if !self.header_seen {
            self.header_seen = true;
            let headers: Vec<&str> = line.split(',').map(str::trim).collect();
            self.frame = find_column(&headers, &FRAME_TIME_COLUMNS);
            self.cpu_busy = find_column(&headers, &CPU_BUSY_COLUMNS);
            self.gpu_busy = find_column(&headers, &GPU_BUSY_COLUMNS);
            return None;
        }
        let fields: Vec<&str> = line.split(',').collect();
        let frame_ms = number(&fields, self.frame).filter(|&v| v > 0.0)?;
        Some(FrameSample {
            frame_ms,
            cpu_busy_ms: number(&fields, self.cpu_busy),
            gpu_busy_ms: number(&fields, self.gpu_busy),
        })
    }

    /// The frame time of a line (milliseconds).
    pub fn feed_line(&mut self, line: &str) -> Option<f64> {
        self.feed(line).map(|f| f.frame_ms)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FrameStats {
    pub avg_fps: f32,
    /// 1000 / 99th-percentile frame time.
    pub low_1pct_fps: f32,
    pub frames: usize,
}

/// Average FPS and 1 % low over the given frame times; `None` if empty.
pub fn frame_stats(frame_times_ms: &[f64]) -> Option<FrameStats> {
    let n = frame_times_ms.len();
    if n == 0 {
        return None;
    }
    let mean = frame_times_ms.iter().sum::<f64>() / n as f64;
    let mut sorted = frame_times_ms.to_vec();
    sorted.sort_by(f64::total_cmp);
    // Nearest rank: ceil(0.99 · n), computed in integers to avoid float error.
    let rank = (99 * n).div_ceil(100);
    let p99 = sorted[rank.max(1) - 1];
    Some(FrameStats {
        avg_fps: (1000.0 / mean) as f32,
        low_1pct_fps: (1000.0 / p99) as f32,
        frames: n,
    })
}

/// Keeps the most recent frames whose times add up to at most `window_ms`.
#[derive(Debug, Clone)]
pub struct FrameWindow {
    window_ms: f64,
    total_ms: f64,
    frames: VecDeque<FrameSample>,
}

impl FrameWindow {
    pub fn new(window_ms: f64) -> Self {
        Self {
            window_ms,
            total_ms: 0.0,
            frames: VecDeque::new(),
        }
    }

    pub fn push(&mut self, ms: f64) {
        self.push_sample(FrameSample::new(ms));
    }

    pub fn push_sample(&mut self, frame: FrameSample) {
        self.total_ms += frame.frame_ms;
        self.frames.push_back(frame);
        while self.total_ms > self.window_ms {
            match self.frames.pop_front() {
                Some(old) => self.total_ms -= old.frame_ms,
                None => break,
            }
        }
    }

    /// Frame times (milliseconds), oldest first.
    pub fn as_slice(&self) -> Vec<f64> {
        self.frames.iter().map(|f| f.frame_ms).collect()
    }

    pub fn samples(&self) -> Vec<FrameSample> {
        self.frames.iter().copied().collect()
    }
}
