//! Real FPS from PresentMon's CSV output: frame-time parsing and statistics.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// Frame-time column names: PresentMon 1.x / `--v1_metrics`, then 2.x.
const FRAME_TIME_COLUMNS: [&str; 2] = ["MsBetweenPresents", "FrameTime"];

/// Streams PresentMon CSV lines. The first line is the header; every later
/// line yields its frame time in milliseconds, or `None` if unusable.
#[derive(Debug, Clone, Default)]
pub struct FrameTimeParser {
    header_seen: bool,
    column: Option<usize>,
}

impl FrameTimeParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed_line(&mut self, line: &str) -> Option<f64> {
        let line = line.trim();
        if !self.header_seen {
            self.header_seen = true;
            let headers: Vec<&str> = line.split(',').map(str::trim).collect();
            self.column = FRAME_TIME_COLUMNS
                .iter()
                .find_map(|name| headers.iter().position(|h| h == name));
            return None;
        }
        let value: f64 = line.split(',').nth(self.column?)?.trim().parse().ok()?;
        (value.is_finite() && value > 0.0).then_some(value)
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
    frames: VecDeque<f64>,
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
        self.frames.push_back(ms);
        self.total_ms += ms;
        while self.total_ms > self.window_ms {
            match self.frames.pop_front() {
                Some(old) => self.total_ms -= old,
                None => break,
            }
        }
    }

    pub fn as_slice(&self) -> Vec<f64> {
        self.frames.iter().copied().collect()
    }
}
