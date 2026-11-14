//! FPS Analyzer: every finding, grouped by severity, with its fix.

use egui::{Align, Button, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui};
use si_core::findings::{Category, Finding, Fix, Severity};

use crate::nav::Page;
use crate::state::AppState;
use crate::theme::{CARD_RADIUS, GAP};
use crate::widgets::format::severity_color;
use crate::widgets::pill::{PillKind, pill, status_dot};

const GROUPS: [(Severity, &str); 3] = [
    (Severity::High, "High impact"),
    (Severity::Medium, "Medium impact"),
    (Severity::Low, "Low impact"),
];

fn category_name(c: Category) -> &'static str {
    match c {
        Category::Display => "Display",
        Category::Power => "Power",
        Category::Memory => "Memory",
        Category::Os => "Windows",
        Category::Gpu => "Graphics",
        Category::Storage => "Storage",
        Category::Processes => "Background apps",
        Category::Startup => "Startup",
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("What's costing you FPS").size(22.0).strong());
            ui.label(
                RichText::new(format!("Health score {}/100", state.health)).color(p.text_muted),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if pill(ui, &p, "Rescan", PillKind::Primary).clicked() {
                state.request_scan();
            }
        });
    });
    ui.add_space(GAP);

    if state.scan.is_none() {
        ui.label(RichText::new("Scanning…").color(p.text_muted));
        return;
    }
    if state.findings.is_empty() {
        ui.label(
            RichText::new("No FPS problems found")
                .size(16.0)
                .color(p.success),
        );
        return;
    }

    let findings = state.findings.clone();
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        for (severity, name) in GROUPS {
            let group: Vec<&Finding> = findings.iter().filter(|f| f.severity == severity).collect();
            if group.is_empty() {
                continue;
            }
            ui.label(
                RichText::new(format!("{name} ({})", group.len()))
                    .size(16.0)
                    .strong()
                    .color(severity_color(severity, &p)),
            );
            ui.add_space(6.0);
            for finding in group {
                finding_row(ui, state, finding);
                ui.add_space(8.0);
            }
            ui.add_space(GAP);
        }
    });
}

fn finding_row(ui: &mut Ui, state: &mut AppState, f: &Finding) {
    let p = state.palette;
    let expanded = state.expanded_finding == Some(f.id);
    Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, if expanded { p.accent } else { p.border }))
        .corner_radius(CornerRadius::same(CARD_RADIUS))
        .inner_margin(Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                status_dot(ui, severity_color(f.severity, &p));
                let title = Button::new(RichText::new(&f.title).strong()).frame(false);
                if ui.add(title).clicked() {
                    state.expanded_finding = if expanded { None } else { Some(f.id) };
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(category_name(f.category)).color(p.text_muted));
                });
            });
            if !expanded {
                return;
            }
            ui.add_space(6.0);
            ui.label(&f.detail);
            ui.add_space(8.0);
            match &f.fix {
                Fix::OpenUri(uri) => {
                    if pill(ui, &p, "Fix", PillKind::Primary).clicked() {
                        state.open_url(uri.clone());
                    }
                }
                Fix::ShowDrivers => {
                    if pill(ui, &p, "Open Drivers", PillKind::Primary).clicked() {
                        state.set_page(Page::Drivers);
                    }
                }
                Fix::Manual(text) => {
                    ui.label(RichText::new(format!("How to fix: {text}")).color(p.accent));
                }
            }
        });
}
