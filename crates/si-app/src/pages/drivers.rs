//! Drivers: every gaming-relevant driver with its update status.

use std::collections::BTreeSet;

use egui::{Align, ComboBox, Grid, Label, Layout, RichText, ScrollArea, Ui};
use egui_phosphor::regular as icons;
use si_core::drivers::DriverStatus;
use si_probe::catalog::CatalogSource;

use crate::state::AppState;
use crate::theme::{GAP, Palette};
use crate::widgets::card::card;
use crate::widgets::format::driver_status_label;
use crate::widgets::pill::{PillKind, pill};

/// Windows Update's "Optional updates" page, where driver updates are listed.
pub const OPTIONAL_UPDATES_URI: &str = "ms-settings:windowsupdate-optionalupdates";
const DEVICE_COLUMN_WIDTH: f32 = 320.0;

/// The status pill for a driver; returns the URL to open when it is clicked.
pub fn status_pill(ui: &mut Ui, p: &Palette, status: &DriverStatus) -> Option<String> {
    let kind = match status {
        DriverStatus::UpdateAvailable { .. } => PillKind::Primary,
        DriverStatus::PossiblyOutdated { .. } => PillKind::Warning,
        DriverStatus::NoUpdateFound => PillKind::Muted,
    };
    if !pill(ui, p, driver_status_label(status), kind).clicked() {
        return None;
    }
    match status {
        DriverStatus::UpdateAvailable { url, .. } => Some(url.clone()),
        DriverStatus::PossiblyOutdated { .. } => Some(OPTIONAL_UPDATES_URI.to_string()),
        DriverStatus::NoUpdateFound => None,
    }
}

fn checked_at_text(unix: i64) -> Option<String> {
    let at = chrono::DateTime::from_timestamp(unix, 0)?.with_timezone(&chrono::Local);
    Some(format!("Checked {}", at.format("%H:%M")))
}

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Driver updates").size(22.0).strong());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if state.drivers_checking {
                ui.label(
                    RichText::new(format!("{} Checking…", icons::HOURGLASS)).color(p.text_muted),
                );
            } else if pill(ui, &p, "Check for updates", PillKind::Primary).clicked() {
                state.request_driver_check();
            }
        });
    });
    ui.add_space(6.0);
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

    let mut open_url = None;
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
                        if let Some(url) = status_pill(ui, &p, &row.status) {
                            open_url = Some(url);
                        }
                        ui.end_row();
                    }
                });
        });
    });
    if let Some(url) = open_url {
        state.open_url(url);
    }
}
