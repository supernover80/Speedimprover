use si_core::{bottleneck::*, frames::*, live::*};
fn f(frame: f64, cpu: f64, gpu: f64) -> FrameSample {
    FrameSample {
        frame_ms: frame,
        cpu_busy_ms: Some(cpu),
        gpu_busy_ms: Some(gpu),
    }
}
fn load(gpu: f32, core: f32) -> Vec<LiveSample> {
    vec![
        LiveSample {
            gpu_pct: Some(gpu),
            cpu_max_core_pct: core,
            cpu_total_pct: 40.0,
            ram_used_mb: 8000,
            ram_total_mb: 32000,
            ..Default::default()
        };
        10
    ]
}

#[test]
fn parses_busy_columns() {
    let mut p = FrameTimeParser::new();
    p.feed("Application,FrameTime,CPUBusy,GPUBusy");
    assert_eq!(p.feed("g.exe,16.0,6.0,15.5"), Some(f(16.0, 6.0, 15.5)));
}
#[test]
fn gpu_bound_from_frames() {
    let r = analyze_bottleneck(&load(99.0, 50.0), &vec![f(16.4, 7.0, 15.9); 60], Some(144));
    assert_eq!(r.verdict, Bottleneck::GpuBound);
    assert_eq!(r.source, AnalysisSource::Frames(60));
    assert_eq!(r.headline, "The GPU is the bottleneck");
    assert!(
        r.evidence.iter().any(|e| e.label == "GPU busy"
            && e.limiting
            && e.value == "15.9 ms of 16.4 ms per frame")
    );
}
#[test]
fn cpu_bound_from_frames() {
    assert_eq!(
        analyze_bottleneck(&load(60.0, 97.0), &vec![f(10.0, 9.6, 5.0); 60], None).verdict,
        Bottleneck::CpuBound
    );
}
#[test]
fn capped_at_refresh_rate() {
    let r = analyze_bottleneck(&load(40.0, 40.0), &vec![f(6.94, 3.0, 4.0); 60], Some(144));
    assert_eq!(r.verdict, Bottleneck::FrameCapped);
    assert_eq!(
        r.headline,
        "Frame rate is capped at your monitor's 144 Hz refresh rate"
    );
}
#[test]
fn vram_beats_frames() {
    let mut s = load(99.0, 50.0);
    for x in &mut s {
        x.vram_used_mb = Some(7900);
        x.vram_total_mb = Some(8000);
    }
    assert_eq!(
        analyze_bottleneck(&s, &vec![f(16.4, 7.0, 15.9); 60], None).verdict,
        Bottleneck::VramLimited
    );
}
#[test]
fn frames_without_busy_use_load_mode() {
    let frames = vec![
        FrameSample {
            frame_ms: 16.0,
            cpu_busy_ms: None,
            gpu_busy_ms: None
        };
        60
    ];
    let r = analyze_bottleneck(&load(99.0, 50.0), &frames, None);
    assert_eq!(
        (r.verdict, r.source),
        (Bottleneck::GpuBound, AnalysisSource::Load(10))
    );
    assert!(!r.actions.is_empty());
}
#[test]
fn temperatures_are_evidence() {
    let mut s = load(99.0, 50.0);
    for x in &mut s {
        x.cpu_temp_c = Some(96.0);
        x.gpu_temp_c = Some(70.0);
    }
    let r = analyze_bottleneck(&s, &[], None);
    assert!(
        r.evidence
            .iter()
            .any(|e| e.label == "CPU temperature" && e.value == "96 °C (approx.)" && e.limiting)
    );
}
