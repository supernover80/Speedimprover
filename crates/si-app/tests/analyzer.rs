mod common;
use common::*;

fn analyzer(s: si_core::model::SystemSnapshot) -> Harness<'static, SpeedImproverApp> {
    let mut st = testing::state_with(s, vec![]);
    st.page = Page::Analyzer;
    harness(st)
}
fn h() -> Harness<'static, SpeedImproverApp> {
    analyzer(fixtures::struggling_laptop())
}
fn h_healthy() -> Harness<'static, SpeedImproverApp> {
    analyzer(fixtures::healthy_desktop())
}

#[test]
fn groups_by_severity() {
    let h = h();
    h.get_by_label("High impact (7)");
    h.get_by_label("Medium impact (5)");
    h.get_by_label("Low impact (2)");
}
#[test]
fn fix_button_opens_settings_uri() {
    let mut h = h();
    h.get_by_label("Game Mode is off").click();
    h.run();
    h.get_by_label("Fix").click();
    h.run();
    assert!(
        h.state()
            .state
            .effects
            .contains(&UiEffect::OpenUrl("ms-settings:gaming-gamemode".into()))
    );
}
#[test]
fn driver_fix_navigates() {
    let mut h = h();
    h.get_by_label("Graphics driver is out of date").click();
    h.run();
    h.get_by_label("Open Drivers").click();
    h.run();
    assert_eq!(h.state().state.page, Page::Drivers);
}
#[test]
fn healthy_pc_shows_empty_state() {
    h_healthy().get_by_label("No FPS problems found");
}
