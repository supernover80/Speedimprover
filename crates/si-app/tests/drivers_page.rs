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
