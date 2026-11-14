#![cfg(windows)]
#[test]
fn wmi_fills_cpu_and_drivers() {
    let mut s = si_core::model::SystemSnapshot::default();
    let mut w = vec![];
    si_probe::win::wmi::collect_wmi(&mut s, &mut w);
    assert!(!s.cpu.name.is_empty() && s.cpu.logical_cores > 0, "{w:?}");
    assert!(!s.drivers.is_empty());
}
#[test]
fn scan_survives_failing_collectors() {
    let mut w = vec![];
    let rows: Vec<si_probe::mapping::Win32Processor> =
        si_probe::win::wmi::query_in(r"root\DoesNotExist", &mut w, si_probe::ScanStep::Hardware);
    assert!(rows.is_empty() && w.len() == 1);
}
#[test]
#[ignore = "needs a real GPU"]
fn real_gpu_detected() {
    assert!(!si_probe::win::dxgi::enumerate_gpus().unwrap().is_empty());
}
#[test]
fn windows_probe_full_scan() {
    let r = si_probe::default_probe().scan(&mut |_| {});
    assert!(r.snapshot.machine.os_build >= 19041);
    assert_ne!(r.snapshot.os.power_plan, si_core::model::PowerPlan::Unknown);
}
#[test]
fn windows_sample_has_cpu_perf() {
    let mut p = si_probe::default_probe();
    p.sample();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let s = p.sample();
    assert!(
        s.cpu_perf_pct.is_some(),
        "English PDH counter missing: {s:?}"
    );
}
