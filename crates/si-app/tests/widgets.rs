mod common;
use common::*;
use si_app::{
    theme::Palette,
    widgets::{card::*, donut::*, format::*},
};

#[test]
fn format_mb_cases() {
    assert_eq!(format_mb(512), "512 MB");
    assert_eq!(format_mb(8188), "8 GB");
    assert_eq!(format_mb(12282), "12 GB");
    assert_eq!(format_mb(7900), "7.7 GB");
}
#[test]
fn badge_and_status_texts() {
    assert_eq!(tier_badge(si_core::tier::Tier::High), "HIGH TIER");
    assert_eq!(
        driver_status_label(&si_core::drivers::DriverStatus::PossiblyOutdated { age_days: 400 }),
        "Check"
    );
    assert_eq!(
        driver_status_label(&si_core::drivers::DriverStatus::NoUpdateFound),
        "No update found"
    );
}
#[test]
fn card_action_is_clickable() {
    let mut h = Harness::new_ui_state(
        |ui, clicked: &mut bool| {
            if card(ui, &Palette::dark(), "Drivers", Some("View All"), |ui| {
                ui.label("row");
            })
            .action_clicked
            {
                *clicked = true;
            }
        },
        false,
    );
    h.get_by_label("Drivers");
    h.get_by_label("View All").click();
    h.run();
    assert!(*h.state());
}
#[test]
fn donut_shows_value_or_dash() {
    let h = Harness::new_ui(|ui| {
        donut(ui, &Palette::dark(), Some(73.4), "CPU", 96.0);
        donut(ui, &Palette::dark(), None, "GPU", 96.0);
    });
    h.get_by_label("73%");
    h.get_by_label("—");
}
#[test]
fn toasts_expire_after_four_seconds() {
    let mut t = si_app::widgets::toasts::Toasts::default();
    t.push(
        "Scan complete",
        si_app::widgets::toasts::ToastKind::Success,
        10.0,
    );
    assert_eq!(t.visible(13.9).len(), 1);
    assert_eq!(t.visible(14.1).len(), 0);
}
