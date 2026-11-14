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
