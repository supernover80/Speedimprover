use si_core::{frames::*, live::*};

fn s(gpu: Option<f32>, core: f32, total: f32) -> LiveSample {
    LiveSample {
        cpu_total_pct: total,
        cpu_max_core_pct: core,
        gpu_pct: gpu,
        ram_used_mb: 4000,
        ram_total_mb: 16000,
        ..Default::default()
    }
}
fn n(x: LiveSample, k: usize) -> Vec<LiveSample> {
    vec![x; k]
}

#[test]
fn insufficient_with_four_samples() {
    assert_eq!(
        classify(&n(s(Some(99.0), 50.0, 50.0), 4)),
        Bottleneck::InsufficientData
    );
}
#[test]
fn gpu_bound() {
    assert_eq!(
        classify(&n(s(Some(98.0), 60.0, 40.0), 10)),
        Bottleneck::GpuBound
    );
}
#[test]
fn cpu_bound_by_single_core() {
    assert_eq!(
        classify(&n(s(Some(60.0), 97.0, 40.0), 10)),
        Bottleneck::CpuBound
    );
}
#[test]
fn frame_capped() {
    assert_eq!(
        classify(&n(s(Some(50.0), 40.0, 20.0), 10)),
        Bottleneck::FrameCapped
    );
}
#[test]
fn balanced() {
    assert_eq!(
        classify(&n(s(Some(88.0), 70.0, 50.0), 10)),
        Bottleneck::Balanced
    );
}

#[test]
fn vram_beats_gpu_bound() {
    let x = LiveSample {
        vram_used_mb: Some(7900),
        vram_total_mb: Some(8000),
        ..s(Some(99.0), 50.0, 50.0)
    };
    assert_eq!(classify(&n(x, 10)), Bottleneck::VramLimited);
}
#[test]
fn cpu_throttle() {
    let x = LiveSample {
        cpu_perf_pct: Some(70.0),
        ..s(Some(60.0), 80.0, 60.0)
    };
    assert_eq!(classify(&n(x, 10)), Bottleneck::CpuThrottled);
}
#[test]
fn missing_metrics_never_match() {
    assert_eq!(classify(&n(s(None, 40.0, 20.0), 10)), Bottleneck::Balanced);
}
#[test]
fn uses_only_last_ten() {
    let mut v = n(s(Some(99.0), 50.0, 50.0), 10);
    v.extend(n(s(Some(50.0), 40.0, 20.0), 10));
    assert_eq!(classify(&v), Bottleneck::FrameCapped);
}
#[test]
fn history_caps_capacity() {
    let mut h = LiveHistory::new(120);
    for i in 0..130 {
        h.push(LiveSample {
            t_ms: i * 1000,
            ..Default::default()
        });
    }
    assert_eq!(h.len(), 120);
    assert_eq!(h.samples().next().unwrap().t_ms, 10_000);
}

#[test]
fn parses_v1_and_v2_columns() {
    let mut p = FrameTimeParser::new();
    assert_eq!(p.feed_line("Application,ProcessID,MsBetweenPresents"), None);
    assert_eq!(p.feed_line("game.exe,42,16.6"), Some(16.6));
    let mut p = FrameTimeParser::new();
    p.feed_line("Application,FrameTime,ProcessID");
    assert_eq!(p.feed_line("game.exe,8.3,42"), Some(8.3));
    assert_eq!(p.feed_line("game.exe,NA,42"), None);
}
#[test]
fn unknown_header_yields_nothing() {
    let mut p = FrameTimeParser::new();
    p.feed_line("A,B,C");
    assert_eq!(p.feed_line("1,2,3"), None);
}
#[test]
fn frame_stats_avg_and_one_percent_low() {
    let mut v = vec![10.0; 98];
    v.extend([50.0, 50.0]);
    let st = frame_stats(&v).unwrap();
    assert!((st.avg_fps - 92.59).abs() < 0.01);
    assert_eq!(st.low_1pct_fps, 20.0);
    assert_eq!(frame_stats(&[]), None);
}
#[test]
fn frame_window_keeps_five_seconds() {
    let mut w = FrameWindow::new(5000.0);
    for _ in 0..600 {
        w.push(10.0);
    }
    assert_eq!(w.as_slice().len(), 500);
}
