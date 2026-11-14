use si_probe::gpu_counters::*;

#[test]
fn parses_engine_instance() {
    let e =
        parse_gpu_engine_instance("pid_1234_luid_0x00000000_0x0000D1A5_phys_0_eng_0_engtype_3D")
            .unwrap();
    assert_eq!(
        (e.pid, e.luid.as_str(), e.is_3d),
        (1234, "0x00000000_0x0000D1A5", true)
    );
    assert!(
        !parse_gpu_engine_instance("pid_1_luid_0x0_0x1_phys_0_eng_3_engtype_Copy")
            .unwrap()
            .is_3d
    );
    assert!(parse_gpu_engine_instance("_Total").is_none());
}
#[test]
fn aggregates_3d_per_adapter_and_pid() {
    let r = [
        (
            "pid_10_luid_0x0_0xA_phys_0_eng_0_engtype_3D".to_string(),
            40.0,
        ),
        (
            "pid_11_luid_0x0_0xA_phys_0_eng_0_engtype_3D".to_string(),
            30.0,
        ),
        (
            "pid_10_luid_0x0_0xA_phys_0_eng_2_engtype_Copy".to_string(),
            90.0,
        ),
        (
            "pid_12_luid_0x0_0xB_phys_0_eng_0_engtype_3D".to_string(),
            5.0,
        ),
    ];
    let a = aggregate_gpu(&r);
    assert_eq!(a.total_pct, 70.0);
    assert_eq!(a.per_pid[&10], 40.0);
}
#[test]
fn total_is_clamped() {
    let r = [
        (
            "pid_1_luid_0x0_0xA_phys_0_eng_0_engtype_3D".to_string(),
            80.0,
        ),
        (
            "pid_2_luid_0x0_0xA_phys_0_eng_0_engtype_3D".to_string(),
            60.0,
        ),
    ];
    assert_eq!(aggregate_gpu(&r).total_pct, 100.0);
}
