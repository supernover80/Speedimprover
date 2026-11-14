mod common;
use common::*;

fn h() -> Harness<'static, SpeedImproverApp> {
    let mut st = testing::state_with(fixtures::struggling_laptop(), testing::demo_games());
    st.settings.favorites = vec!["steam:730".into()];
    harness(st)
}

#[test]
fn shows_profile_health_and_tier() {
    let h = h();
    for l in [
        "System Profile",
        "LAPTOP-STUDENT",
        "Health 0/100",
        "LOW TIER",
        "7 high-impact issues found",
        "Status: Scan complete",
    ] {
        h.get_by_label(l);
    }
}
#[test]
fn lists_fps_killers_and_opens_analyzer() {
    let mut h = h();
    h.get_by_label("Monitor running below its max refresh rate")
        .click();
    h.run();
    assert_eq!(h.state().state.page, Page::Analyzer);
    assert_eq!(
        h.state().state.expanded_finding,
        Some("display.refresh_below_max")
    );
}
#[test]
fn view_all_opens_drivers() {
    let mut h = h();
    h.get_by_label("Checking for driver updates…");
    h.get_by_label("View All").click();
    h.run();
    assert_eq!(h.state().state.page, Page::Drivers);
}
#[test]
fn favorites_and_add_tile() {
    let mut h = h();
    h.get_by_label("Counter-Strike 2");
    h.get_by_label("Add game").click();
    h.run();
    assert_eq!(h.state().state.page, Page::Games);
}
