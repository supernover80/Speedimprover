use si_core::{fixtures, model::ProcessUsage};
use si_probe::{ScanResult, ScanStep, SystemProbe, fixture::FixtureProbe, generic::*};

#[test]
fn normalize_cpu_divides_by_cores() {
    assert_eq!(normalize_cpu(400.0, 8), 50.0);
    assert_eq!(normalize_cpu(900.0, 8), 100.0);
    assert_eq!(normalize_cpu(50.0, 0), 0.0);
}
#[test]
fn select_processes_unions_cpu_and_memory_tops() {
    let p = |pid, cpu, mem| ProcessUsage {
        pid,
        name: format!("p{pid}"),
        cpu_pct: cpu,
        mem_mb: mem,
        gpu_pct: None,
    };
    let mut all: Vec<_> = (0..40).map(|i| p(i, i as f32, 10 + i as u64)).collect();
    all.push(p(100, 0.0, 9000));
    let sel = select_processes(all);
    assert_eq!(sel.len(), 26);
    assert!(sel.iter().any(|x| x.pid == 100));
    assert_eq!(sel[0].pid, 39);
}
#[test]
fn generic_probe_reads_this_machine() {
    let mut steps = vec![];
    let r = GenericProbe::new().scan(&mut |s| steps.push(s));
    assert!(r.snapshot.cpu.logical_cores > 0 && r.snapshot.memory.total_mb > 0);
    assert!(!r.snapshot.processes.is_empty());
    assert_eq!(steps.first(), Some(&ScanStep::Hardware));
}
#[test]
fn generic_sample_has_cpu_and_ram() {
    let mut p = GenericProbe::new();
    p.sample();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let s = p.sample();
    assert!(s.ram_total_mb > 0 && (0.0..=100.0).contains(&s.cpu_total_pct));
}
#[test]
fn fixture_probe_loads_snapshot_or_scan_result() {
    let dir = std::env::temp_dir().join(format!("si-probe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let snap = dir.join("snap.json");
    std::fs::write(
        &snap,
        serde_json::to_string(&fixtures::healthy_desktop()).unwrap(),
    )
    .unwrap();
    let full = dir.join("full.json");
    std::fs::write(
        &full,
        serde_json::to_string(&ScanResult {
            snapshot: fixtures::struggling_laptop(),
            ..Default::default()
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        FixtureProbe::load(&snap)
            .unwrap()
            .scan(&mut |_| {})
            .snapshot,
        fixtures::healthy_desktop()
    );
    assert_eq!(
        FixtureProbe::load(&full)
            .unwrap()
            .scan(&mut |_| {})
            .snapshot
            .machine
            .hostname,
        "LAPTOP-STUDENT"
    );
}
