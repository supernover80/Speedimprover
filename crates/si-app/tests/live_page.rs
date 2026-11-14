mod common;
use common::*;
use si_app::pages::live::default_fps_target;
use si_core::{live::*, model::ProcessUsage};

fn gpu_bound_state() -> AppState {
    let mut st = testing::state_with(fixtures::healthy_desktop(), vec![]);
    st.page = Page::Live;
    for i in 0..10 {
        st.live.push(LiveSample {
            t_ms: i * 1000,
            gpu_pct: Some(99.0),
            cpu_max_core_pct: 60.0,
            ram_total_mb: 32768,
            ram_used_mb: 9000,
            ..Default::default()
        });
    }
    st
}
#[test]
fn shows_verdict() {
    harness(gpu_bound_state()).get_by_label(Bottleneck::GpuBound.explanation());
}
#[test]
fn prompts_for_presentmon() {
    let mut h = harness(gpu_bound_state());
    h.get_by_label("Open Settings").click();
    h.run();
    assert_eq!(h.state().state.page, Page::Settings);
}
#[test]
fn shows_fps_stats() {
    let mut st = gpu_bound_state();
    st.settings.presentmon_path = Some("C:\\Tools\\PresentMon.exe".into());
    st.fps = Some(si_core::frames::FrameStats {
        avg_fps: 92.6,
        low_1pct_fps: 20.0,
        frames: 100,
    });
    let h = harness(st);
    h.get_by_label("Avg 93 FPS");
    h.get_by_label("1% low 20 FPS");
}
#[test]
fn default_target_skips_dwm() {
    let p = |n: &str, g| ProcessUsage {
        name: n.into(),
        gpu_pct: g,
        ..Default::default()
    };
    let s = LiveSample {
        top_processes: vec![
            p("dwm.exe", Some(40.0)),
            p("cs2.exe", Some(35.0)),
            p("chrome.exe", None),
        ],
        ..Default::default()
    };
    assert_eq!(default_fps_target(&s).as_deref(), Some("cs2.exe"));
    assert_eq!(default_fps_target(&LiveSample::default()), None);
}
