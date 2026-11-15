mod common;
use common::*;

fn h() -> Harness<'static, SpeedImproverApp> {
    let mut st = testing::state_with(fixtures::struggling_laptop(), testing::demo_games());
    st.settings.favorites = vec!["steam:730".into()];
    harness(st)
}

#[test]
fn shows_summary_tiles() {
    let h = h();
    for label in [
        "LAPTOP-STUDENT",
        "Health",
        "0/100",
        "7 high · 5 medium · 2 low",
        "Bottleneck",
        "Collecting data…",
        "CPU",
        "GPU",
        "Performance issues",
        "Low tier",
    ] {
        h.get_by_label(label);
    }
}

#[test]
fn issue_opens_issues_page_expanded() {
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
fn drivers_summary_links_to_drivers() {
    let mut h = h();
    h.get_by_label("Checking for driver updates…");
    h.get_by_label("View drivers").click();
    h.run();
    assert_eq!(h.state().state.page, Page::Drivers);
}

#[test]
fn favorites_and_manage_link() {
    let mut h = h();
    h.get_by_label("Counter-Strike 2");
    h.get_by_label("Manage").click();
    h.run();
    assert_eq!(h.state().state.page, Page::Games);
}

#[test]
fn restart_banner_on_overview() {
    let mut st = testing::state_with(fixtures::healthy_desktop(), vec![]);
    st.reboot_required = true;
    harness(st).get_by_label("Restart your PC to finish installing drivers.");
}
