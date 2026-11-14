use si_core::{fixtures, model::*};

#[test]
fn fixtures_round_trip() {
    for s in [fixtures::healthy_desktop(), fixtures::struggling_laptop()] {
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<SystemSnapshot>(&json).unwrap(), s);
    }
}

#[test]
fn partial_json_uses_defaults() {
    let s: SystemSnapshot = serde_json::from_str(r#"{"cpu":{"name":"X"}}"#).unwrap();
    assert_eq!(s.cpu.name, "X");
    assert_eq!(s.os.power_plan, PowerPlan::Unknown);
}

#[test]
fn primary_gpu_prefers_discrete_with_most_vram() {
    let mut s = fixtures::struggling_laptop();
    s.gpus.push(GpuInfo {
        name: "NVIDIA GeForce RTX 4060 Laptop GPU".into(),
        vendor: Vendor::Nvidia,
        vram_mb: 8188,
        ..Default::default()
    });
    assert_eq!(s.primary_gpu().unwrap().vram_mb, 8188);
    assert!(s.has_discrete_gpu());
    assert!(!fixtures::struggling_laptop().has_discrete_gpu());
}

#[test]
fn age_days_counts_from_snapshot_time() {
    let s = fixtures::healthy_desktop();
    assert_eq!(s.age_days("2026-10-01"), Some(31));
    assert_eq!(s.age_days("not a date"), None);
}

#[test]
fn system_disk_and_game_key() {
    assert_eq!(
        fixtures::healthy_desktop().system_disk().unwrap().media,
        DiskMedia::Nvme
    );
    let g = InstalledGame {
        store: Store::Epic,
        id: "Fortnite".into(),
        ..Default::default()
    };
    assert_eq!(g.key(), "epic:Fortnite");
}
