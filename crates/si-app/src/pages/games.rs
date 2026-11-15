//! Games: installed Steam/Epic games and the recommended settings for each.

use std::sync::OnceLock;

use egui::{
    Align, Button, Color32, CornerRadius, Frame, Label, Layout, Margin, RichText, ScrollArea, Ui,
    WidgetInfo, WidgetType,
};
use egui_phosphor::regular as icons;
use si_core::games::profile::{Impact, ProfileLibrary};
use si_core::games::recommend::{Goal, Reason, recommend};
use si_core::model::{InstalledGame, Store};

use crate::state::{AppState, UiEffect};
use crate::theme::{GAP, Palette};
use crate::widgets::card::card;
use crate::widgets::format::format_mb;
use crate::widgets::pill::status_dot;

const SETTING_TILES_PER_ROW: usize = 3;
const GOALS: [(Goal, &str); 3] = [
    (Goal::Competitive, "Competitive"),
    (Goal::Balanced, "Balanced"),
    (Goal::Quality, "Quality"),
];

fn profiles() -> &'static ProfileLibrary {
    static PROFILES: OnceLock<ProfileLibrary> = OnceLock::new();
    PROFILES.get_or_init(ProfileLibrary::bundled)
}

/// Tooltip explaining why a value was recommended.
pub fn reason_text(r: &Reason) -> String {
    match r {
        Reason::Tier(t) => format!("Matches your {}-tier PC", t.label()),
        Reason::VramCap(mb) => format!("Limited by {} of VRAM", format_mb(*mb)),
        Reason::Competitive => "Competitive: favors FPS".to_string(),
        Reason::QualityBoost => "Cheap on FPS, raised for quality".to_string(),
    }
}

fn impact_color(i: Impact, p: &Palette) -> Color32 {
    match i {
        Impact::High => p.danger,
        Impact::Medium => p.warning,
        Impact::Low => p.success,
    }
}

fn store_name(s: Store) -> &'static str {
    match s {
        Store::Steam => "Steam",
        Store::Epic => "Epic",
    }
}

pub fn show(ui: &mut Ui, state: &mut AppState) {
    let p = state.palette;
    let hardware = state.hardware.unwrap_or_default();
    ui.label(
        RichText::new(format!("Your PC: {} tier", hardware.overall.label()))
            .size(22.0)
            .strong(),
    );
    ui.add_space(GAP);

    let games: Vec<InstalledGame> = state.scan.iter().flat_map(|s| s.games.clone()).collect();
    if games.is_empty() {
        let text = if state.scan.is_some() {
            "No Steam or Epic games found"
        } else {
            "Scanning…"
        };
        ui.label(RichText::new(text).color(p.text_muted));
        return;
    }
    if state.selected_game.is_none() {
        state.selected_game = games
            .iter()
            .find(|g| profiles().for_game(g).is_some())
            .or(games.first())
            .map(InstalledGame::key);
    }

    let width = ui.available_width() - GAP;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        ui.allocate_ui_with_layout(
            egui::vec2(width * 0.34, ui.available_height()),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_width(width * 0.34);
                game_list(ui, state, &games);
            },
        );
        ui.allocate_ui_with_layout(
            egui::vec2(width * 0.66, ui.available_height()),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_width(width * 0.66);
                recommendations(ui, state, &games);
            },
        );
    });
}

fn game_list(ui: &mut Ui, state: &mut AppState, games: &[InstalledGame]) {
    let p = state.palette;
    card(ui, &p, "Installed games", None, |ui| {
        ScrollArea::vertical()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for game in games {
                    let key = game.key();
                    ui.horizontal(|ui| {
                        let favorite = state.settings.favorites.contains(&key);
                        let star =
                            Button::new(RichText::new(icons::STAR).size(18.0).color(if favorite {
                                p.warning
                            } else {
                                p.text_muted
                            }))
                            .frame(false);
                        let response = ui.add(star);
                        response.widget_info(|| {
                            WidgetInfo::labeled(
                                WidgetType::Button,
                                true,
                                format!("Favorite {}", game.name),
                            )
                        });
                        if response.clicked() {
                            if favorite {
                                state.settings.favorites.retain(|k| k != &key);
                            } else {
                                state.settings.favorites.push(key.clone());
                            }
                            state.effects.push(UiEffect::SaveSettings);
                        }
                        let selected = state.selected_game.as_deref() == Some(key.as_str());
                        if ui.selectable_label(selected, &game.name).clicked() {
                            state.selected_game = Some(key.clone());
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(store_name(game.store))
                                    .size(11.0)
                                    .color(p.text_muted),
                            );
                        });
                    });
                }
            });
    });
}

fn recommendations(ui: &mut Ui, state: &mut AppState, games: &[InstalledGame]) {
    let p = state.palette;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Goal").color(p.text_muted));
        for (goal, name) in GOALS {
            if ui
                .selectable_label(state.settings.goal == goal, name)
                .clicked()
                && state.settings.goal != goal
            {
                state.settings.goal = goal;
                state.effects.push(UiEffect::SaveSettings);
            }
        }
    });
    ui.add_space(GAP);

    let Some(game) = games.iter().find(|g| Some(g.key()) == state.selected_game) else {
        return;
    };
    let hardware = state.hardware.unwrap_or_default();
    card(
        ui,
        &p,
        &format!("Recommended settings · {}", game.name),
        None,
        |ui| {
            let Some(profile) = profiles().for_game(game) else {
                ui.label(
                    RichText::new(format!("No profile yet for {}", game.name)).color(p.text_muted),
                );
                return;
            };
            let recs = recommend(profile, &hardware, state.settings.goal);
            let per_row = SETTING_TILES_PER_ROW as f32;
            let tile_width = (ui.available_width() - (per_row - 1.0) * 8.0) / per_row;
            for row in recs.chunks(SETTING_TILES_PER_ROW) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for rec in row {
                        Frame::new()
                            .fill(p.card_raised)
                            .corner_radius(CornerRadius::same(12))
                            .inner_margin(Margin::same(10))
                            .show(ui, |ui| {
                                // The row is left-to-right; each tile stacks vertically.
                                ui.vertical(|ui| {
                                    ui.set_width(tile_width - 20.0);
                                    ui.horizontal(|ui| {
                                        status_dot(ui, impact_color(rec.impact, &p));
                                        ui.add(
                                            Label::new(
                                                RichText::new(&rec.label)
                                                    .size(12.0)
                                                    .color(p.text_muted),
                                            )
                                            .truncate(),
                                        );
                                    });
                                    ui.label(RichText::new(&rec.value).size(15.0).strong());
                                });
                            })
                            .response
                            .on_hover_text(reason_text(&rec.reason));
                    }
                });
                ui.add_space(8.0);
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for (impact, name) in [
                    (Impact::High, "High FPS cost"),
                    (Impact::Medium, "Medium"),
                    (Impact::Low, "Low"),
                ] {
                    status_dot(ui, impact_color(impact, &p));
                    ui.label(RichText::new(name).size(11.0).color(p.text_muted));
                    ui.add_space(8.0);
                }
            });
        },
    );
}
