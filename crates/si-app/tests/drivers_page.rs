mod common;
use common::*;
use si_core::drivers::*;

fn drivers_state() -> AppState {
    let mut st = testing::state_with(fixtures::struggling_laptop(), vec![]);
    st.page = Page::Drivers;
    st.apply_event(WorkerEvent::Drivers {
        rows: evaluate_drivers(&fixtures::struggling_laptop(), &DriverCatalog::empty(), &[]),
        catalog: si_probe::catalog::CatalogSource::Bundled,
        checked_at_unix: 1_793_491_200,
    });
    st.effects.clear();
    st
}
#[test]
fn lists_rows_with_status() {
    let h = harness(drivers_state());
    h.get_by_label("Intel(R) Iris(R) Xe Graphics");
    h.get_by_label("Catalog: built-in");
    assert_eq!(h.get_all_by_label("Check").count(), 2);
}
#[test]
fn check_pill_opens_optional_updates() {
    let mut h = harness(drivers_state());
    h.get_all_by_label("Check").next().unwrap().click();
    h.run();
    assert!(h.state().state.effects.contains(&UiEffect::OpenUrl(
        "ms-settings:windowsupdate-optionalupdates".into()
    )));
}
#[test]
fn check_for_updates_sends_command() {
    let mut h = harness(drivers_state());
    h.get_by_label("Check for updates").click();
    h.run();
    assert!(h.state().state.drivers_checking);
    assert!(matches!(
        h.state().state.effects.last(),
        Some(UiEffect::Send(WorkerCommand::CheckDrivers { .. }))
    ));
}

const ID: &str = "0b5b8d3c-1a2b-4c5d-8e9f-001122334455";

fn windows_update_state() -> AppState {
    let mut st = testing::state_with(fixtures::struggling_laptop(), vec![]);
    st.page = Page::Drivers;
    let wu = [AvailableDriverUpdate {
        hardware_id: "PCI\\VEN_8086&DEV_46A8".into(),
        update_id: ID.into(),
        ..Default::default()
    }];
    st.apply_event(WorkerEvent::Drivers {
        rows: evaluate_drivers(&fixtures::struggling_laptop(), &DriverCatalog::empty(), &wu),
        catalog: si_probe::catalog::CatalogSource::Bundled,
        checked_at_unix: 1_793_491_200,
    });
    st.effects.clear();
    st
}

#[test]
fn install_asks_for_confirmation_then_sends() {
    let mut h = harness(windows_update_state());
    h.get_by_label("Install").click();
    h.run();
    assert_eq!(h.state().state.install_confirm, Some(vec![ID.to_string()]));
    h.get_by_label("Install now").click();
    h.run();
    assert!(h.state().state.drivers_installing);
    assert!(
        h.state()
            .state
            .effects
            .contains(&UiEffect::Send(WorkerCommand::InstallDrivers {
                update_ids: vec![ID.to_string()]
            }))
    );
}

#[test]
fn install_all_offers_every_installable_update() {
    let mut h = harness(windows_update_state());
    h.get_by_label("Install all (1)").click();
    h.run();
    assert_eq!(h.state().state.install_confirm, Some(vec![ID.to_string()]));
}

#[test]
fn install_failure_clears_state() {
    let mut st = windows_update_state();
    st.drivers_installing = true;
    st.apply_event(WorkerEvent::InstallFailed("Installation cancelled".into()));
    assert!(!st.drivers_installing);
    assert!(
        st.toasts
            .visible(st.now)
            .iter()
            .any(|t| t.text == "Installation cancelled")
    );
    st.apply_event(WorkerEvent::DriversInstalled(
        si_probe::install::InstallReport {
            installed: vec!["Intel Graphics".into()],
            reboot_required: true,
            ..Default::default()
        },
    ));
    assert!(st.reboot_required);
    let h = harness(st);
    h.get_by_label("Restart your PC to finish installing drivers.");
}
#[test]
fn warns_when_no_restore_point_was_created() {
    let has_warning = |st: &AppState| {
        st.toasts
            .visible(st.now)
            .iter()
            .any(|t| t.text == si_app::state::NO_RESTORE_POINT_TEXT)
    };
    let report = |restore_point_created| si_probe::install::InstallReport {
        installed: vec!["Intel Graphics".into()],
        restore_point_created,
        ..Default::default()
    };
    let mut st = windows_update_state();
    st.apply_event(WorkerEvent::DriversInstalled(report(false)));
    assert!(has_warning(&st));
    let mut st = windows_update_state();
    st.apply_event(WorkerEvent::DriversInstalled(report(true)));
    assert!(!has_warning(&st));
}
