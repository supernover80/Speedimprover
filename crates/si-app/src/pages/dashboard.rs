//! The dashboard: the mockup's three columns mapped to PC data (spec §9).

use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Label, Layout, Rect, RichText, ScrollArea, Sense,
    Ui, WidgetInfo, WidgetType, vec2,
};
use egui_phosphor::regular as icons;
use si_core::findings::Severity;
use si_core::model::{DiskMedia, Store};

use crate::nav::Page;
use crate::pages::drivers::{apply_pill_action, row_pill};
use crate::state::AppState;
use crate::theme::{GAP, Palette, ThemeChoice};
use crate::widgets::card::{card, gradient_card};
use crate::widgets::chart::line_chart;
use crate::widgets::donut::donut;
use crate::widgets::format::{format_mb, severity_color, tier_badge};
use crate::widgets::pill::{badge, status_dot, toggle};

const MAX_DRIVER_ROWS: usize = 7;
const MAX_FAVORITES: usize = 4;
const MAX_FINDING_TILES: usize = 12;
/// Tiles per row; drops to 3 when the column is narrow so words don't break.
const FINDING_TILES_PER_ROW: usize = 4;
const NARROW_TILES_PER_ROW: usize = 3;
const NARROW_COLUMN: f32 = 460.0;
const RING_SIZE: f32 = 96.0;
const LIVE_CHART_SECONDS: usize = 60;
const SCROLLBAR_ROOM: f32 = 14.0;

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        // Leave room on the right for the overlay scrollbar.
        let usable = ui.available_width() - 2.0 * GAP - SCROLLBAR_ROOM;
        let widths = [usable * 0.24, usable * 0.38, usable * 0.38];
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            column(ui, widths[0], |ui| {
                profile_card(ui, state);
                theme_card(ui, state);
                summary_card(ui, state);
                live_card(ui, state);
            });
            column(ui, widths[1], |ui| {
                drivers_card(ui, state);
                graphics_card(ui, state);
            });
            column(ui, widths[2], |ui| {
                favorites_card(ui, state);
                fps_killers_card(ui, state);
                banner(ui, state);
            });
        });
    });
}

fn column(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(
        vec2(width, ui.available_height()),
        Layout::top_down(Align::Min),
        |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.y = GAP;
            add(ui);
        },
    );
}

/// A label that shortens itself with "…" instead of wrapping.
fn truncated(ui: &mut Ui, text: RichText) {
    ui.add(Label::new(text).truncate());
}

fn profile_card(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    card(ui, &p, "System Profile", None, |ui| {
        ui.vertical_centered(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());
            ui.painter()
                .circle_filled(rect.center(), 36.0, p.accent_strong);
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                icons::DESKTOP_TOWER,
                FontId::proportional(32.0),
                Color32::WHITE,
            );
            ui.add_space(8.0);
            let Some(scan) = &state.scan else {
                ui.label(RichText::new("Scanning…").color(p.text_muted));
                return;
            };
            let snap = &scan.snapshot;
            ui.label(RichText::new(&snap.machine.hostname).size(16.0).strong());
            let gpu = snap.primary_gpu().map_or("No GPU", |g| g.name.as_str());
            truncated(
                ui,
                RichText::new(format!("{} · {gpu}", snap.cpu.name))
                    .size(11.0)
                    .color(p.text_muted),
            );
            ui.add_space(8.0);
            if let Some(hw) = state.hardware {
                badge(ui, &tier_badge(hw.overall), p.accent_strong, Color32::WHITE);
            }
            ui.add_space(6.0);
            badge(
                ui,
                &format!("Health {}/100", state.health),
                p.card_raised,
                p.text,
            );
        });
    });
}

fn theme_card(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    card(ui, &p, "Dark Theme", None, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Appearance").color(p.text_muted));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mut dark = state.settings.theme == ThemeChoice::Dark;
                if toggle(ui, &p, &mut dark, "Dark mode").changed() {
                    state.set_theme(if dark {
                        ThemeChoice::Dark
                    } else {
                        ThemeChoice::Light
                    });
                }
            });
        });
    });
}

fn summary_row(ui: &mut Ui, p: &Palette, name: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(name).color(p.text_muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            truncated(ui, RichText::new(value).strong());
        });
    });
}

fn summary_card(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    card(ui, &p, "System Summary", None, |ui| {
        let Some(scan) = &state.scan else {
            ui.label(RichText::new("Waiting for the first scan…").color(p.text_muted));
            return;
        };
        let snap = &scan.snapshot;
        summary_row(ui, &p, "CPU", snap.cpu.name.clone());
        summary_row(
            ui,
            &p,
            "GPU",
            snap.primary_gpu()
                .map_or_else(|| "Unknown".into(), |g| g.name.clone()),
        );
        let modules = &snap.memory.modules;
        let mut ram = format_mb(snap.memory.total_mb);
        if !modules.is_empty() {
            let sticks = if modules.len() == 1 {
                "stick"
            } else {
                "sticks"
            };
            ram.push_str(&format!(" · {} {sticks}", modules.len()));
            if let Some(speed) = modules
                .iter()
                .map(|m| m.configured_mhz)
                .max()
                .filter(|&s| s > 0)
            {
                ram.push_str(&format!(" · {speed} MT/s"));
            }
        }
        summary_row(ui, &p, "RAM", ram);
        let drive = snap.system_disk().map_or_else(
            || "Unknown".to_string(),
            |d| {
                let media = match d.media {
                    DiskMedia::Nvme => "NVMe",
                    DiskMedia::Ssd => "SSD",
                    DiskMedia::Hdd => "HDD",
                    DiskMedia::Unknown => "Drive",
                };
                format!("{media} · {:.0} GB free", d.free_gb)
            },
        );
        summary_row(ui, &p, "System drive", drive);
    });
}

fn live_card(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    card(ui, &p, "Live", None, |ui| {
        let latest = state.live.latest();
        let ring = RING_SIZE.min(ui.available_width() / 2.0 - 16.0);
        ui.columns(2, |cols| {
            cols[0].vertical_centered(|ui| {
                donut(ui, &p, latest.map(|s| s.cpu_total_pct), "CPU", ring);
            });
            cols[1].vertical_centered(|ui| {
                donut(ui, &p, latest.and_then(|s| s.gpu_pct), "GPU", ring);
            });
        });
        let recent: Vec<_> = state.live.samples().collect();
        let recent = &recent[recent.len().saturating_sub(LIVE_CHART_SECONDS)..];
        let cpu: Vec<[f64; 2]> = recent
            .iter()
            .enumerate()
            .map(|(i, s)| [i as f64, f64::from(s.cpu_total_pct)])
            .collect();
        let gpu: Vec<[f64; 2]> = recent
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.gpu_pct.map(|g| [i as f64, f64::from(g)]))
            .collect();
        line_chart(
            ui,
            "dashboard-live",
            &[("CPU", cpu, p.accent), ("GPU", gpu, p.success)],
            100.0,
            90.0,
        );
    });
}

fn drivers_card(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    let mut open_page = false;
    let mut open_url = None;
    let response = card(ui, &p, "Drivers", Some("View All"), |ui| {
        if state.drivers.is_empty() {
            let text = if state.drivers_checking {
                "Checking for driver updates…"
            } else {
                "Not checked yet"
            };
            ui.label(RichText::new(text).color(p.text_muted));
            return;
        }
        for row in state.drivers.iter().take(MAX_DRIVER_ROWS) {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icons::FILE_TEXT).size(20.0).color(p.accent));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(action) = row_pill(ui, &p, row) {
                        open_url = Some(action);
                    }
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        truncated(ui, RichText::new(&row.driver.device_name).strong());
                        let date = row.driver.date.as_deref().unwrap_or("unknown date");
                        truncated(
                            ui,
                            RichText::new(format!("{} · {date}", row.driver.version))
                                .size(11.0)
                                .color(p.text_muted),
                        );
                    });
                });
            });
            ui.add_space(4.0);
        }
        if state.drivers.len() > MAX_DRIVER_ROWS {
            open_page |= ui
                .link(format!("+{} more", state.drivers.len() - MAX_DRIVER_ROWS))
                .clicked();
        }
    });
    if response.action_clicked || open_page {
        state.set_page(Page::Drivers);
    }
    if let Some(action) = open_url {
        apply_pill_action(state, action);
    }
}

fn graphics_card(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    card(ui, &p, "Graphics Card", None, |ui| {
        let gpu = state.scan.as_ref().and_then(|s| s.snapshot.primary_gpu());
        match gpu {
            Some(g) => {
                let vram = format!("{} VRAM", format_mb(g.vram_mb));
                let driver = format!(
                    "Driver {}",
                    g.driver_version.as_deref().unwrap_or("unknown")
                );
                gradient_card(ui, &p, &[&g.name, &vram, &driver]);
            }
            None => {
                ui.label(RichText::new("No GPU detected").color(p.text_muted));
            }
        }
    });
}

/// A clickable tile that paints its own text and exposes one accessible node
/// named `label`.
fn tile(ui: &mut Ui, size: egui::Vec2, label: &str, p: &Palette) -> (egui::Response, Rect) {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    let fill = if response.hovered() {
        p.border
    } else {
        p.card_raised
    };
    ui.painter().rect_filled(rect, CornerRadius::same(12), fill);
    (response, rect)
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
    let mut open_game: Option<Option<String>> = None;
    card(ui, &p, "Favorite Games", None, |ui| {
        let slots = (MAX_FAVORITES + 1) as f32;
        let width = (ui.available_width() - (slots - 1.0) * 8.0) / slots;
        let size = vec2(width, 110.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for game in &games {
                let (response, rect) = tile(ui, size, &game.name, &p);
                let painter = ui.painter_at(rect);
                let galley = painter.layout(
                    game.name.clone(),
                    FontId::proportional(13.0),
                    p.text,
                    rect.width() - 16.0,
                );
                painter.galley(rect.left_top() + vec2(8.0, 10.0), galley, p.text);
                let store = match game.store {
                    Store::Steam => "Steam",
                    Store::Epic => "Epic",
                };
                painter.text(
                    rect.left_bottom() + vec2(8.0, -10.0),
                    Align2::LEFT_BOTTOM,
                    store,
                    FontId::proportional(11.0),
                    p.text_muted,
                );
                if response.clicked() {
                    open_game = Some(Some(game.key()));
                }
            }
            let (response, rect) = tile(ui, size, "Add game", &p);
            ui.painter()
                .circle_filled(rect.center(), 18.0, p.accent_strong);
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                icons::PLUS,
                FontId::proportional(20.0),
                Color32::WHITE,
            );
            if response.clicked() {
                open_game = Some(None);
            }
        });
    });
    if let Some(selected) = open_game {
        if selected.is_some() {
            state.selected_game = selected;
        }
        state.set_page(Page::Games);
    }
}

fn fps_killers_card(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    let mut open: Option<&'static str> = None;
    let response = card(ui, &p, "FPS Killers", Some("Open Analyzer"), |ui| {
        ui.horizontal(|ui| match state.scan_step {
            Some(step) => {
                status_dot(ui, p.warning);
                ui.label(RichText::new(step.label()).color(p.text_muted));
            }
            None => {
                status_dot(ui, p.success);
                ui.label(RichText::new("Status: Scan complete").color(p.text_muted));
            }
        });
        ui.add_space(8.0);
        if state.scan.is_some() && state.findings.is_empty() {
            ui.label(RichText::new("No FPS problems found").color(p.success));
            return;
        }
        let tiles_per_row = if ui.available_width() < NARROW_COLUMN {
            NARROW_TILES_PER_ROW
        } else {
            FINDING_TILES_PER_ROW
        };
        let per_row = tiles_per_row as f32;
        let width = (ui.available_width() - (per_row - 1.0) * 8.0) / per_row;
        for row in state
            .findings
            .iter()
            .take(MAX_FINDING_TILES)
            .collect::<Vec<_>>()
            .chunks(tiles_per_row)
        {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for finding in row {
                    let (response, rect) = tile(ui, vec2(width, 92.0), &finding.title, &p);
                    let painter = ui.painter_at(rect);
                    painter.circle_filled(
                        rect.right_top() + vec2(-12.0, 12.0),
                        5.0,
                        severity_color(finding.severity, &p),
                    );
                    let galley = painter.layout(
                        finding.title.clone(),
                        FontId::proportional(12.0),
                        p.text,
                        rect.width() - 30.0,
                    );
                    painter.galley(rect.left_top() + vec2(10.0, 10.0), galley, p.text);
                    if response.clicked() {
                        open = Some(finding.id);
                    }
                }
            });
            ui.add_space(8.0);
        }
    });
    if let Some(id) = open {
        state.expanded_finding = Some(id);
        state.set_page(Page::Analyzer);
    } else if response.action_clicked {
        state.set_page(Page::Analyzer);
    }
}

fn banner(ui: &mut Ui, state: &mut AppState) {
    if state.scan.is_none() {
        return;
    }
    let p = state.palette;
    let high = state
        .findings
        .iter()
        .filter(|f| f.severity == Severity::High)
        .count();
    let (icon, color, text) = if high > 0 {
        (
            icons::BELL_RINGING,
            p.warning,
            format!("{high} high-impact issues found"),
        )
    } else {
        (
            icons::CHECK_CIRCLE,
            p.success,
            "No high-impact issues found".to_string(),
        )
    };
    egui::Frame::new()
        .fill(p.card)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(crate::theme::CARD_RADIUS))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).size(22.0).color(color));
                ui.label(RichText::new(text).strong());
            });
        });
}
