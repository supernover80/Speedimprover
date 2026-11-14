mod common;
use common::*;

#[test]
fn settings_load_defaults_and_round_trip() {
    let dir = std::env::temp_dir().join(format!("si-app-{}", std::process::id()));
    let paths = AppPaths::in_dir(&dir);
    assert_eq!(Settings::load(&paths.config_file), Settings::default());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&paths.config_file, "not = [valid").unwrap();
    assert_eq!(Settings::load(&paths.config_file), Settings::default());
    let s = Settings {
        favorites: vec!["steam:730".into()],
        ..Default::default()
    };
    s.save(&paths.config_file).unwrap();
    assert_eq!(Settings::load(&paths.config_file), s);
}

#[test]
fn scan_done_computes_findings_and_requests_drivers() {
    let mut st = AppState::new(Settings::default());
    st.apply_event(WorkerEvent::ScanDone(Box::new(si_probe::ScanResult {
        snapshot: fixtures::struggling_laptop(),
        ..Default::default()
    })));
    assert_eq!((st.findings.len(), st.health), (14, 0));
    assert!(st.hardware.is_some());
    assert!(
        st.toasts
            .visible(st.now)
            .iter()
            .any(|t| t.text == "Scan complete")
    );
    assert!(
        st.effects
            .contains(&UiEffect::Send(WorkerCommand::CheckDrivers {
                catalog_url: DEFAULT_CATALOG_URL.into()
            }))
    );
}

#[test]
fn live_follows_page() {
    let mut st = testing::state_with(fixtures::healthy_desktop(), vec![]);
    st.set_page(Page::Live);
    assert_eq!(
        st.effects.pop(),
        Some(UiEffect::Send(WorkerCommand::StartLive))
    );
    assert!(st.live_running);
    st.set_page(Page::Drivers);
    assert_eq!(
        st.effects.pop(),
        Some(UiEffect::Send(WorkerCommand::StopLive))
    );
}

#[test]
fn sidebar_navigates() {
    let mut h = harness(testing::state_with(fixtures::healthy_desktop(), vec![]));
    h.get_by_label("Settings").click();
    h.run();
    assert_eq!(h.state().state.page, Page::Settings);
    h.get_by_label("Download PresentMon");
}

#[test]
fn worker_scans_and_samples_with_fixture_probe() {
    let cfg = WorkerConfig {
        make_probe: Box::new(|| {
            Box::new(si_probe::fixture::FixtureProbe::new(si_probe::ScanResult {
                snapshot: fixtures::healthy_desktop(),
                ..Default::default()
            }))
        }),
        wu_search: || Ok(vec![]),
        last_scan_path: std::env::temp_dir().join(format!("si-last-{}.json", std::process::id())),
    };
    let w = spawn_worker(cfg, egui::Context::default());
    w.tx.send(WorkerCommand::Scan).unwrap();
    w.tx.send(WorkerCommand::StartLive).unwrap();
    w.tx.send(WorkerCommand::CheckDrivers {
        catalog_url: "http://127.0.0.1:9/x.json".into(),
    })
    .unwrap();
    let (mut done, mut sample, mut drivers) = (false, false, false);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !(done && sample && drivers) && std::time::Instant::now() < deadline {
        match w.rx.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(WorkerEvent::ScanDone(_)) => done = true,
            Ok(WorkerEvent::Sample(_)) => sample = true,
            Ok(WorkerEvent::Drivers { catalog, .. }) => {
                drivers = true;
                assert_eq!(catalog, si_probe::catalog::CatalogSource::Bundled);
            }
            _ => {}
        }
    }
    assert!(done && sample && drivers);
    w.tx.send(WorkerCommand::Shutdown).unwrap();
}
