//! Live Monitor: what limits FPS right now, load charts, top processes and
//! real FPS through PresentMon.

use egui::{
    Align, ComboBox, CornerRadius, Frame, Grid, Layout, Margin, RichText, ScrollArea, Stroke, Ui,
};
use egui_phosphor::regular as icons;
use si_core::live::{Bottleneck, LiveSample};

use crate::nav::Page;
use crate::state::AppState;
use crate::theme::{CARD_RADIUS, GAP};
use crate::widgets::card::card;
use crate::widgets::chart::line_chart;
use crate::widgets::format::format_mb;
use crate::widgets::pill::{PillKind, pill};
use crate::worker::WorkerCommand;

const CHART_HEIGHT: f32 = 110.0;
const PROCESS_NAME_WIDTH: f32 = 180.0;
const DESKTOP_WINDOW_MANAGER: &str = "dwm.exe";

/// The process to measure by default: the heaviest GPU user that is not
/// Windows' desktop compositor.
pub fn default_fps_target(sample: &LiveSample) -> Option<String> {
    sample
        .top_processes
        .iter()
        .filter(|p| !p.name.eq_ignore_ascii_case(DESKTOP_WINDOW_MANAGER))
        .filter_map(|p| p.gpu_pct.filter(|&g| g > 0.0).map(|g| (g, p)))
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, p)| p.name.clone())
}

fn percent(used: u64, total: u64) -> Option<f64> {
    (total > 0).then(|| used as f64 * 100.0 / total as f64)
}

pub fn show(ui: &mut Ui, state: &mut AppState) {
    ui.label(RichText::new("Live performance").size(22.0).strong());
    ui.add_space(GAP);
    verdict_banner(ui, state);
    ui.add_space(GAP);

    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        let width = ui.available_width() - GAP - 14.0;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.6, ui.available_height()),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.set_width(width * 0.6);
                    ui.spacing_mut().item_spacing.y = GAP;
                    charts(ui, state);
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(width * 0.4, ui.available_height()),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.set_width(width * 0.4);
                    ui.spacing_mut().item_spacing.y = GAP;
                    fps_panel(ui, state);
                    processes(ui, state);
                },
            );
        });
    });
}

fn verdict_banner(ui: &mut Ui, state: &AppState) {
    let p = state.palette;
    let verdict = state.live.verdict();
    let (icon, color) = match verdict {
        Bottleneck::InsufficientData => (icons::HOURGLASS, p.text_muted),
        Bottleneck::Balanced | Bottleneck::FrameCapped => (icons::CHECK_CIRCLE, p.success),
        Bottleneck::GpuBound | Bottleneck::CpuBound => (icons::GAUGE, p.accent),
        Bottleneck::VramLimited
        | Bottleneck::RamLimited
        | Bottleneck::CpuThrottled
        | Bottleneck::GpuThrottled => (icons::WARNING, p.warning),
    };
    Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, color))
        .corner_radius(CornerRadius::same(CARD_RADIUS))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).size(22.0).color(color));
                ui.label(RichText::new(verdict.explanation()).size(15.0).strong());
            });
        });
}

fn charts(ui: &mut Ui, state: &AppState) {
    let p = state.palette;
    let samples: Vec<&LiveSample> = state.live.samples().collect();
    let latest = state.live.latest();
    let series = |f: &dyn Fn(&LiveSample) -> Option<f64>| -> Vec<[f64; 2]> {
        samples
            .iter()
            .enumerate()
            .filter_map(|(i, s)| f(s).map(|v| [i as f64, v]))
            .collect()
    };
    let value = |v: Option<f64>| v.map_or_else(|| "—".to_string(), |v| format!("{v:.0}%"));

    let cpu = latest.map(|s| f64::from(s.cpu_total_pct));
    card(ui, &p, &format!("CPU {}", value(cpu)), None, |ui| {
        let points = series(&|s| Some(f64::from(s.cpu_total_pct)));
        line_chart(
            ui,
            "live-cpu",
            &[("CPU", points, p.accent)],
            100.0,
            CHART_HEIGHT,
        );
    });
    let gpu = latest.and_then(|s| s.gpu_pct).map(f64::from);
    card(ui, &p, &format!("GPU {}", value(gpu)), None, |ui| {
        let points = series(&|s| s.gpu_pct.map(f64::from));
        line_chart(
            ui,
            "live-gpu",
            &[("GPU", points, p.success)],
            100.0,
            CHART_HEIGHT,
        );
    });
    let ram_title = latest.map_or_else(
        || "RAM —".to_string(),
        |s| {
            format!(
                "RAM {} / {}",
                format_mb(s.ram_used_mb),
                format_mb(s.ram_total_mb)
            )
        },
    );
    card(ui, &p, &ram_title, None, |ui| {
        let points = series(&|s| percent(s.ram_used_mb, s.ram_total_mb));
        line_chart(
            ui,
            "live-ram",
            &[("RAM", points, p.warning)],
            100.0,
            CHART_HEIGHT,
        );
    });
    let vram_title = match latest.and_then(|s| s.vram_used_mb.zip(s.vram_total_mb)) {
        Some((used, total)) => format!("VRAM {} / {}", format_mb(used), format_mb(total)),
        None => "VRAM —".to_string(),
    };
    card(ui, &p, &vram_title, None, |ui| {
        let points = series(&|s| {
            s.vram_used_mb
                .zip(s.vram_total_mb)
                .and_then(|(used, total)| percent(used, total))
        });
        line_chart(
            ui,
            "live-vram",
            &[("VRAM", points, p.danger)],
            100.0,
            CHART_HEIGHT,
        );
    });
}

fn fps_panel(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    card(ui, &p, "Real FPS", None, |ui| {
        let Some(presentmon) = state.settings.presentmon_path.clone() else {
            ui.label("Set the PresentMon path in Settings to measure real FPS");
            ui.add_space(6.0);
            if pill(ui, &p, "Open Settings", PillKind::Primary).clicked() {
                state.set_page(Page::Settings);
            }
            return;
        };
        let latest = state.live.latest();
        let mut names: Vec<String> = latest
            .map(|s| s.top_processes.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default();
        names.sort_by_key(|n| n.to_lowercase());
        names.dedup();
        if state.fps_target.is_none() {
            state.fps_target = latest.and_then(default_fps_target);
        }
        let mut target = state.fps_target.clone();
        ComboBox::from_label("Game process")
            .selected_text(target.clone().unwrap_or_else(|| "Choose…".into()))
            .show_ui(ui, |ui| {
                for name in &names {
                    ui.selectable_value(&mut target, Some(name.clone()), name);
                }
            });
        state.fps_target = target;
        ui.horizontal(|ui| {
            if pill(ui, &p, "Measure FPS", PillKind::Primary).clicked() {
                if let Some(process_name) = state.fps_target.clone() {
                    state.send(WorkerCommand::StartFps {
                        presentmon: presentmon.clone(),
                        process_name,
                    });
                }
            }
            if pill(ui, &p, "Stop", PillKind::Neutral).clicked() {
                state.send(WorkerCommand::StopFps);
            }
        });
        if let Some(fps) = state.fps {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Avg {:.0} FPS", fps.avg_fps))
                        .size(20.0)
                        .strong(),
                );
                ui.add_space(16.0);
                ui.label(
                    RichText::new(format!("1% low {:.0} FPS", fps.low_1pct_fps))
                        .size(20.0)
                        .color(p.text_muted),
                );
            });
            let points: Vec<[f64; 2]> = state
                .frame_times
                .iter()
                .enumerate()
                .map(|(i, ms)| [i as f64, *ms])
                .collect();
            let worst = state.frame_times.iter().copied().fold(16.7, f64::max);
            line_chart(
                ui,
                "live-frametimes",
                &[("Frame time", points, p.accent)],
                worst,
                80.0,
            );
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new("Requires administrator or Performance Log Users membership")
                .size(11.0)
                .color(p.text_muted),
        );
    });
}

fn processes(ui: &mut Ui, state: &AppState) {
    let p = state.palette;
    card(ui, &p, "Top processes", None, |ui| {
        let Some(latest) = state.live.latest() else {
            ui.label(RichText::new("Collecting data…").color(p.text_muted));
            return;
        };
        let mut rows = latest.top_processes.clone();
        rows.sort_by(|a, b| b.cpu_pct.total_cmp(&a.cpu_pct));
        Grid::new("live-processes")
            .num_columns(4)
            .striped(true)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                for header in ["Process", "CPU", "GPU", "Memory"] {
                    ui.label(RichText::new(header).color(p.text_muted));
                }
                ui.end_row();
                for row in rows {
                    ui.allocate_ui_with_layout(
                        egui::vec2(PROCESS_NAME_WIDTH, 18.0),
                        Layout::left_to_right(Align::Center),
                        |ui| ui.add(egui::Label::new(&row.name).truncate()),
                    );
                    ui.label(format!("{:.0}%", row.cpu_pct));
                    ui.label(
                        row.gpu_pct
                            .map_or_else(|| "—".into(), |g| format!("{g:.0}%")),
                    );
                    ui.label(format_mb(row.mem_mb));
                    ui.end_row();
                }
            });
    });
}
