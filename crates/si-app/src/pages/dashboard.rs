//! Overview: summary tiles, the performance issues, system details, driver
//! status and favourite games.

use egui::{Align, Button, Layout, RichText, ScrollArea, Ui, vec2};
use egui_phosphor::regular as icons;
use si_core::bottleneck::{AnalysisSource, BottleneckReport};
use si_core::drivers::DriverStatus;
use si_core::findings::{Finding, Severity};
use si_core::live::Bottleneck;
use si_core::model::{DiskMedia, Store};

use crate::nav::Page;
use crate::pages::drivers::RESTART_TEXT;
use crate::state::AppState;
use crate::theme::{GAP, Palette, heading};
use crate::widgets::card::{banner, card, column};
use crate::widgets::format::{format_mb, severity_color};
use crate::widgets::pill::{PillKind, badge, pill};

const MAX_ISSUE_ROWS: usize = 8;
const MAX_FAVORITES: usize = 6;
const SCROLLBAR_ROOM: f32 = 14.0;
/// Keeps the four summary tiles the same height.
const TILE_BODY_HEIGHT: f32 = 56.0;

/// "Measured from 312 frames" / "Estimated from 10 seconds of system load".
pub fn source_text(report: &BottleneckReport) -> String {
    match (report.verdict, report.source) {
        (Bottleneck::InsufficientData, _) => "Waiting for live data".into(),
        (_, AnalysisSource::Frames(n)) => format!("Measured from {n} frames"),
        (_, AnalysisSource::Load(n)) => format!("Estimated from {n} seconds of system load"),
    }
}

/// `62 °C`, `62 °C (approx.)` or `—`.
pub fn temperature_text(celsius: Option<f32>, exact: bool) -> String {
    match celsius {
        Some(t) if exact => format!("{t:.0} °C"),
        Some(t) => format!("{t:.0} °C (approx.)"),
        None => "—".into(),
    }
}

fn severity_name(s: Severity) -> &'static str {
    match s {
        Severity::High => "High",
        Severity::Medium => "Medium",
        Severity::Low => "Low",
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let report = state.bottleneck_report();
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        let width = ui.available_width() - SCROLLBAR_ROOM;
        ui.set_max_width(width);
        ui.spacing_mut().item_spacing.y = GAP;
        header(ui, state);
        if state.reboot_required {
            let p = state.palette;
            banner(ui, &p, icons::ARROW_CLOCKWISE, p.warning, RESTART_TEXT);
        }
        tiles(ui, state, &report, width);
        let left = (width - GAP) * 0.62;
        let right = width - GAP - left;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            column(ui, left, |ui| {
                issues_card(ui, state);
                favorites_card(ui, state);
            });
            column(ui, right, |ui| {
                system_card(ui, state);
                drivers_card(ui, state);
            });
        });
    });
}

fn header(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    ui.horizontal(|ui| {
        ui.vertical(|ui| match &state.scan {
            Some(scan) => {
                let snap = &scan.snapshot;
                ui.label(heading(ui, &snap.machine.hostname, 22.0));
                let when = chrono::DateTime::from_timestamp(snap.taken_at_unix, 0)
                    .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
                    .unwrap_or_default();
                ui.label(
                    RichText::new(format!("{} · last scan {when}", snap.machine.os_name))
                        .color(p.text_muted),
                );
            }
            None => {
                ui.label(heading(ui, "Scanning your PC…", 22.0));
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if let Some(step) = state.scan_step {
                ui.label(RichText::new(step.label()).color(p.text_muted));
            } else if pill(ui, &p, "Rescan", PillKind::Neutral).clicked() {
                state.request_scan();
            }
        });
    });
}

fn tile(ui: &mut Ui, p: &Palette, label: &str, value: RichText, sub: &str) {
    card(ui, p, label, None, |ui| {
        ui.set_min_height(TILE_BODY_HEIGHT);
        ui.label(value);
        ui.label(RichText::new(sub).size(12.0).color(p.text_muted));
    });
}

fn tiles(ui: &mut Ui, state: &AppState, report: &BottleneckReport, width: f32) {
    let p = state.palette;
    let w = (width - 3.0 * GAP) / 4.0;
    let latest = state.live.latest();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        column(ui, w, |ui| {
            let color = match state.health {
                80.. => p.success,
                50.. => p.warning,
                _ => p.danger,
            };
            let count = |s| state.findings.iter().filter(|f| f.severity == s).count();
            let sub = if state.scan.is_some() {
                format!(
                    "{} high · {} medium · {} low",
                    count(Severity::High),
                    count(Severity::Medium),
                    count(Severity::Low)
                )
            } else {
                "Scanning…".into()
            };
            let value = heading(ui, format!("{}/100", state.health), 26.0).color(color);
            tile(ui, &p, "Health", value, &sub);
        });
        column(ui, w, |ui| {
            let value = heading(ui, &report.headline, 16.0);
            tile(ui, &p, "Bottleneck", value, &source_text(report));
        });
        column(ui, w, |ui| {
            let load = latest.map_or("—".into(), |s| format!("{:.0}%", s.cpu_total_pct));
            let temp = temperature_text(
                latest.and_then(|s| s.cpu_temp_c),
                latest.is_some_and(|s| s.cpu_temp_exact),
            );
            let value = heading(ui, load, 26.0);
            tile(ui, &p, "CPU", value, &format!("Temperature {temp}"));
        });
        column(ui, w, |ui| {
            let load = latest
                .and_then(|s| s.gpu_pct)
                .map_or("—".into(), |g| format!("{g:.0}%"));
            let temp = temperature_text(latest.and_then(|s| s.gpu_temp_c), true);
            let value = heading(ui, load, 26.0);
            tile(ui, &p, "GPU", value, &format!("Temperature {temp}"));
        });
    });
}

fn issue_row(ui: &mut Ui, p: &Palette, f: &Finding) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let color = severity_color(f.severity, p);
        ui.allocate_ui_with_layout(
            vec2(70.0, 22.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                badge(
                    ui,
                    severity_name(f.severity),
                    color.gamma_multiply(0.18),
                    color,
                );
            },
        );
        clicked = ui
            .add(Button::new(RichText::new(&f.title).color(p.text)).frame(false))
            .clicked();
    });
    clicked
}

fn issues_card(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    let mut open: Option<&'static str> = None;
    let response = card(ui, &p, "Performance issues", Some("View all"), |ui| {
        if state.scan.is_none() {
            ui.label(RichText::new("Scanning…").color(p.text_muted));
            return;
        }
        if state.findings.is_empty() {
            ui.label(RichText::new("No performance issues found").color(p.success));
            return;
        }
        for f in state.findings.iter().take(MAX_ISSUE_ROWS) {
            if issue_row(ui, &p, f) {
                open = Some(f.id);
            }
        }
        if state.findings.len() > MAX_ISSUE_ROWS {
            let more = state.findings.len() - MAX_ISSUE_ROWS;
            ui.label(RichText::new(format!("and {more} more")).color(p.text_muted));
        }
    });
    if let Some(id) = open {
        state.expanded_finding = Some(id);
        state.set_page(Page::Analyzer);
    } else if response.action_clicked {
        state.set_page(Page::Analyzer);
    }
}

fn detail_row(ui: &mut Ui, p: &Palette, name: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(name).color(p.text_muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(egui::Label::new(value).truncate());
        });
    });
}

fn system_card(ui: &mut Ui, state: &AppState) {
    let p = state.palette;
    card(ui, &p, "System", None, |ui| {
        let Some(scan) = &state.scan else {
            ui.label(RichText::new("Waiting for the first scan…").color(p.text_muted));
            return;
        };
        let snap = &scan.snapshot;
        detail_row(ui, &p, "Windows", snap.machine.os_name.clone());
        detail_row(ui, &p, "Processor", snap.cpu.name.clone());
        let graphics = snap.primary_gpu().map_or_else(
            || "Unknown".into(),
            |g| format!("{} · {}", g.name, format_mb(g.vram_mb)),
        );
        detail_row(ui, &p, "Graphics", graphics);
        let modules = &snap.memory.modules;
        let mut memory = format_mb(snap.memory.total_mb);
        if !modules.is_empty() {
            let sticks = if modules.len() == 1 {
                "stick"
            } else {
                "sticks"
            };
            memory.push_str(&format!(" · {} {sticks}", modules.len()));
            if let Some(speed) = modules
                .iter()
                .map(|m| m.configured_mhz)
                .max()
                .filter(|&s| s > 0)
            {
                memory.push_str(&format!(" · {speed} MT/s"));
            }
        }
        detail_row(ui, &p, "Memory", memory);
        let storage = snap.system_disk().map_or_else(
            || "Unknown".to_string(),
            |d| {
                let media = match d.media {
                    DiskMedia::Nvme => "NVMe SSD",
                    DiskMedia::Ssd => "SSD",
                    DiskMedia::Hdd => "Hard drive",
                    DiskMedia::Unknown => "Drive",
                };
                format!("{media} · {:.0} GB free", d.free_gb)
            },
        );
        detail_row(ui, &p, "Storage", storage);
        if let Some(d) = snap.displays.first() {
            let display = format!(
                "{}×{} · {} Hz (max {} Hz)",
                d.width, d.height, d.current_hz, d.max_hz
            );
            detail_row(ui, &p, "Display", display);
        }
        if let Some(hw) = state.hardware {
            let tier = format!("{} tier", hw.overall.label());
            detail_row(ui, &p, "Performance tier", tier);
        }
    });
}

fn drivers_card(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    let mut install = None;
    let response = card(ui, &p, "Driver status", Some("View drivers"), |ui| {
        if state.drivers_installing {
            ui.label(RichText::new("Installing driver updates…").color(p.text_muted));
            return;
        }
        if state.drivers.is_empty() {
            let text = if state.drivers_checking {
                "Checking for driver updates…"
            } else {
                "Not checked yet"
            };
            ui.label(RichText::new(text).color(p.text_muted));
            return;
        }
        let count =
            |f: fn(&DriverStatus) -> bool| state.drivers.iter().filter(|r| f(&r.status)).count();
        let updates = count(|s| matches!(s, DriverStatus::UpdateAvailable { .. }));
        let outdated = count(|s| matches!(s, DriverStatus::PossiblyOutdated { .. }));
        let (text, color) = match (updates, outdated) {
            (0, 0) => ("No driver updates found".to_string(), p.success),
            (0, n) => (format!("{n} driver(s) may be out of date"), p.warning),
            (n, _) => (format!("{n} driver update(s) available"), p.accent),
        };
        ui.label(RichText::new(text).color(color));
        let installable = state.installable_updates();
        if !installable.is_empty() {
            let label = format!("Install all ({})", installable.len());
            if pill(ui, &p, &label, PillKind::Primary).clicked() {
                install = Some(installable);
            }
        }
    });
    if install.is_some() {
        state.install_confirm = install;
    } else if response.action_clicked {
        state.set_page(Page::Drivers);
    }
}

fn favorites_card(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    let games: Vec<_> = state
        .scan
        .iter()
        .flat_map(|s| s.games.iter())
        .filter(|g| state.settings.favorites.contains(&g.key()))
        .take(MAX_FAVORITES)
        .cloned()
        .collect();
    let mut open: Option<Option<String>> = None;
    let response = card(ui, &p, "Favorite games", Some("Manage"), |ui| {
        if games.is_empty() {
            ui.label(
                RichText::new("Star games on the Games page to pin them here.").color(p.text_muted),
            );
            return;
        }
        for game in &games {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icons::GAME_CONTROLLER).color(p.text_muted));
                let name = Button::new(RichText::new(&game.name).color(p.text)).frame(false);
                if ui.add(name).clicked() {
                    open = Some(Some(game.key()));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let store = match game.store {
                        Store::Steam => "Steam",
                        Store::Epic => "Epic Games",
                    };
                    ui.label(RichText::new(store).size(12.0).color(p.text_muted));
                });
            });
        }
    });
    if let Some(selected) = open {
        if selected.is_some() {
            state.selected_game = selected;
        }
        state.set_page(Page::Games);
    } else if response.action_clicked {
        state.set_page(Page::Games);
    }
}
