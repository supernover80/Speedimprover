//! Drivers: every gaming-relevant driver, its update status, and one-click
//! installs for updates offered by Windows Update.

use std::collections::BTreeSet;

use egui::{Align, ComboBox, Grid, Label, Layout, Modal, RichText, ScrollArea, Ui};
use egui_phosphor::regular as icons;
use si_core::drivers::{DriverRow, DriverStatus};
use si_probe::catalog::CatalogSource;

use crate::state::AppState;
use crate::theme::{GAP, Palette, heading};
use crate::widgets::card::{banner, card};
use crate::widgets::format::driver_status_label;
use crate::widgets::pill::{PillKind, pill};

/// Windows Update's "Optional updates" page, where driver updates are listed.
pub const OPTIONAL_UPDATES_URI: &str = "ms-settings:windowsupdate-optionalupdates";
const DEVICE_COLUMN_WIDTH: f32 = 320.0;
pub const RESTART_TEXT: &str = "Restart your PC to finish installing drivers.";

/// What clicking a driver's status pill does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PillAction {
    /// Open a vendor page or Windows Settings.
    Open(String),
    /// Install this Windows Update ID (after confirmation).
    Install(String),
}

/// The status pill for a driver row.
pub fn row_pill(ui: &mut Ui, p: &Palette, row: &DriverRow) -> Option<PillAction> {
    let status = &row.status;
    let kind = match status {
        DriverStatus::UpdateAvailable { .. } => PillKind::Primary,
        DriverStatus::PossiblyOutdated { .. } => PillKind::Warning,
        DriverStatus::NoUpdateFound => PillKind::Muted,
    };
    if !pill(ui, p, driver_status_label(status), kind).clicked() {
        return None;
    }
    match (status, &row.install_id) {
        (DriverStatus::UpdateAvailable { .. }, Some(id)) => Some(PillAction::Install(id.clone())),
        (DriverStatus::UpdateAvailable { url, .. }, None) => Some(PillAction::Open(url.clone())),
        (DriverStatus::PossiblyOutdated { .. }, _) => {
            Some(PillAction::Open(OPTIONAL_UPDATES_URI.to_string()))
        }
        (DriverStatus::NoUpdateFound, _) => None,
    }
}

/// Carries out a pill click.
pub fn apply_pill_action(state: &mut AppState, action: PillAction) {
    match action {
        PillAction::Open(url) => state.open_url(url),
        PillAction::Install(id) => state.install_confirm = Some(vec![id]),
    }
}

fn checked_at_text(unix: i64) -> Option<String> {
    let at = chrono::DateTime::from_timestamp(unix, 0)?.with_timezone(&chrono::Local);
    Some(format!("Checked {}", at.format("%H:%M")))
}

/// The "Install driver updates?" dialog; shown over any page while an
/// install waits for confirmation.
pub fn install_dialog(ctx: &egui::Context, state: &mut AppState) {
    let Some(ids) = state.install_confirm.clone() else {
        return;
    };
    let p = state.palette;
    let mut confirmed = false;
    let mut cancelled = false;
    let response = Modal::new(egui::Id::new("install-confirm")).show(ctx, |ui| {
        ui.set_max_width(420.0);
        ui.label(heading(ui, "Install driver updates?", 18.0));
        ui.add_space(8.0);
        ui.label(format!(
            "SpeedImprover will create a System Restore point, then download and install {} update(s) from Windows Update.",
            ids.len()
        ));
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Windows will ask for administrator permission. Your PC may need to restart afterwards.",
            )
            .color(p.text_muted),
        );
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            confirmed = pill(ui, &p, "Install now", PillKind::Primary).clicked();
            cancelled = pill(ui, &p, "Cancel", PillKind::Neutral).clicked();
        });
    });
    if confirmed {
        state.confirm_install();
    } else if cancelled || response.should_close() {
        state.install_confirm = None;
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    ui.horizontal(|ui| {
        ui.label(heading(ui, "Driver updates", 22.0));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if state.drivers_installing {
                ui.label(
                    RichText::new(format!(
                        "{} Installing driver updates… Windows may ask for permission.",
                        icons::HOURGLASS
                    ))
                    .color(p.text_muted),
                );
                return;
            }
            if state.drivers_checking {
                ui.label(
                    RichText::new(format!("{} Checking…", icons::HOURGLASS)).color(p.text_muted),
                );
            } else if pill(ui, &p, "Check for updates", PillKind::Neutral).clicked() {
                state.request_driver_check();
            }
            let installable = state.installable_updates();
            if !installable.is_empty()
                && pill(
                    ui,
                    &p,
                    &format!("Install all ({})", installable.len()),
                    PillKind::Primary,
                )
                .clicked()
            {
                state.install_confirm = Some(installable);
            }
        });
    });
    ui.add_space(6.0);
    if state.reboot_required {
        banner(ui, &p, icons::ARROW_CLOCKWISE, p.warning, RESTART_TEXT);
        ui.add_space(6.0);
    }
    ui.horizontal(|ui| {
        if let Some(source) = state.catalog_source {
            let text = match source {
                CatalogSource::Remote => "Catalog: online",
                CatalogSource::Bundled => "Catalog: built-in",
            };
            ui.label(RichText::new(text).color(p.text_muted));
        }
        if let Some(text) = state.drivers_checked_at.and_then(checked_at_text) {
            ui.label(RichText::new(text).color(p.text_muted));
        }
        let classes: BTreeSet<String> = state
            .drivers
            .iter()
            .map(|r| r.driver.class.clone())
            .collect();
        let mut filter = state.driver_class_filter.clone();
        ComboBox::from_id_salt("driver-class")
            .selected_text(filter.clone().unwrap_or_else(|| "All classes".into()))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut filter, None, "All classes");
                for class in &classes {
                    ui.selectable_value(&mut filter, Some(class.clone()), class);
                }
            });
        state.driver_class_filter = filter;
    });
    ui.add_space(GAP);

    let mut action = None;
    card(ui, &p, "Installed drivers", None, |ui| {
        if state.drivers.is_empty() {
            let text = if state.drivers_checking {
                "Checking for driver updates…"
            } else {
                "Not checked yet"
            };
            ui.label(RichText::new(text).color(p.text_muted));
            return;
        }
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            Grid::new("drivers-table")
                .num_columns(5)
                .striped(true)
                .spacing([20.0, 10.0])
                .show(ui, |ui| {
                    for header in ["Device", "Class", "Version", "Date", "Status"] {
                        ui.label(RichText::new(header).color(p.text_muted));
                    }
                    ui.end_row();
                    let filter = state.driver_class_filter.as_deref();
                    for row in state
                        .drivers
                        .iter()
                        .filter(|r| filter.is_none_or(|f| r.driver.class == f))
                    {
                        ui.allocate_ui_with_layout(
                            egui::vec2(DEVICE_COLUMN_WIDTH, 20.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.add(
                                    Label::new(RichText::new(&row.driver.device_name).strong())
                                        .truncate(),
                                )
                            },
                        );
                        ui.label(&row.driver.class);
                        ui.label(&row.driver.version);
                        ui.label(row.driver.date.as_deref().unwrap_or("—"));
                        if let Some(a) = row_pill(ui, &p, row) {
                            action = Some(a);
                        }
                        ui.end_row();
                    }
                });
        });
    });
    if let Some(a) = action {
        apply_pill_action(state, a);
    }
}
