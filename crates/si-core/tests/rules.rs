use si_core::{findings::*, fixtures, model::*, rules::analyze};

fn ids(s: &SystemSnapshot, g: &[InstalledGame]) -> Vec<&'static str> {
    analyze(s, g).iter().map(|f| f.id).collect()
}
fn find(s: &SystemSnapshot, id: &str) -> Option<Finding> {
    analyze(s, &[]).into_iter().find(|f| f.id == id)
}

#[test]
fn healthy_fixture_has_no_findings() {
    let f = analyze(&fixtures::healthy_desktop(), &[]);
    assert!(f.is_empty(), "{f:#?}");
    assert_eq!(health_score(&f), 100);
}

#[test]
fn struggling_fixture_flags_expected_rules() {
    let mut got = ids(&fixtures::struggling_laptop(), &[]);
    got.sort();
    assert_eq!(
        got,
        [
            "display.refresh_below_max",
            "gpu.driver_age",
            "memory.below_rated",
            "memory.low_total",
            "memory.pressure",
            "memory.single_channel",
            "os.background_recording_on",
            "os.game_mode_off",
            "os.memory_integrity_on",
            "power.on_battery",
            "power.plan_saver",
            "process.hogs",
            "startup.too_many",
            "storage.low_free"
        ]
    );
    assert_eq!(
        health_score(&analyze(&fixtures::struggling_laptop(), &[])),
        0
    );
}

#[test]
fn findings_sorted_by_severity_then_id() {
    let f = analyze(&fixtures::struggling_laptop(), &[]);
    assert!(
        f.windows(2).all(|w| w[0].severity > w[1].severity
            || (w[0].severity == w[1].severity && w[0].id <= w[1].id))
    );
}

#[test]
fn unknowns_produce_no_findings() {
    assert!(analyze(&SystemSnapshot::default(), &[]).is_empty());
}

#[test]
fn fractional_refresh_not_flagged() {
    for (cur, max, flagged) in [
        (59, 60, false),
        (143, 144, false),
        (60, 75, true),
        (60, 144, true),
    ] {
        let mut s = fixtures::healthy_desktop();
        (s.displays[0].current_hz, s.displays[0].max_hz) = (cur, max);
        assert_eq!(
            ids(&s, &[]).contains(&"display.refresh_below_max"),
            flagged,
            "{cur}/{max}"
        );
    }
}

#[test]
fn single_channel_severity_depends_on_discrete_gpu() {
    let mut s = fixtures::healthy_desktop();
    s.memory.modules.truncate(1);
    assert_eq!(
        find(&s, "memory.single_channel").unwrap().severity,
        Severity::Medium
    );
    assert_eq!(
        find(&fixtures::struggling_laptop(), "memory.single_channel")
            .unwrap()
            .severity,
        Severity::High
    );
}

#[test]
fn memory_pressure_thresholds() {
    let mut s = fixtures::healthy_desktop();
    s.memory.used_mb = 26_300; // 0.80
    assert_eq!(
        find(&s, "memory.pressure").unwrap().severity,
        Severity::Medium
    );
    s.memory.used_mb = 28_200; // 0.86
    assert_eq!(
        find(&s, "memory.pressure").unwrap().severity,
        Severity::High
    );
}

#[test]
fn jedec_default_only_on_desktops() {
    let mut s = fixtures::healthy_desktop();
    for m in &mut s.memory.modules {
        (m.rated_mhz, m.configured_mhz) = (4800, 4800);
    }
    assert_eq!(
        find(&s, "memory.jedec_default").unwrap().severity,
        Severity::Low
    );
    s.machine.is_laptop = Some(true);
    assert!(find(&s, "memory.jedec_default").is_none());
}

#[test]
fn process_hogs_lists_top_three_by_cpu() {
    let d = find(&fixtures::struggling_laptop(), "process.hogs")
        .unwrap()
        .detail;
    for name in ["chrome.exe", "OneDrive.exe", "Teams.exe"] {
        assert!(d.contains(name), "{d}");
    }
    assert!(!d.contains("dwm.exe"));
}

#[test]
fn games_on_hdd_uses_longest_mount() {
    let mut s = fixtures::healthy_desktop();
    s.disks.push(DiskInfo {
        mount: "D:\\".into(),
        media: DiskMedia::Hdd,
        total_gb: 2000.0,
        free_gb: 900.0,
    });
    let on_d = InstalledGame {
        name: "Foo".into(),
        install_dir: "d:\\SteamLibrary\\steamapps\\common\\Foo".into(),
        ..Default::default()
    };
    let on_c = InstalledGame {
        name: "Bar".into(),
        install_dir: "C:\\Games\\Bar".into(),
        ..Default::default()
    };
    assert!(ids(&s, &[on_d]).contains(&"storage.games_on_hdd"));
    assert!(!ids(&s, &[on_c]).contains(&"storage.games_on_hdd"));
}

#[test]
fn fixes_and_titles_match_spec() {
    let s = fixtures::struggling_laptop();
    let gm = find(&s, "os.game_mode_off").unwrap();
    assert_eq!(gm.title, "Game Mode is off");
    assert_eq!(gm.fix, Fix::OpenUri("ms-settings:gaming-gamemode".into()));
    assert_eq!(find(&s, "gpu.driver_age").unwrap().fix, Fix::ShowDrivers);
}

#[test]
fn health_score_mixed() {
    let mut s = fixtures::healthy_desktop();
    s.os.game_mode = Some(false); // Medium
    s.os.hags = Some(false); // Low
    assert_eq!(health_score(&analyze(&s, &[])), 89);
}
