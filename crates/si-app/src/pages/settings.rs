//! Settings: theme, PresentMon path, driver catalog URL, rescan.

use std::path::PathBuf;

use egui::RichText;

use crate::settings::DEFAULT_CATALOG_URL;
use crate::state::{AppState, UiEffect};
use crate::theme::{ThemeChoice, heading};

const PRESENTMON_RELEASES: &str = "https://github.com/GameTechDev/PresentMon/releases";

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(heading(ui, "Preferences", 22.0));
    ui.add_space(16.0);

    ui.label(RichText::new("Theme").strong());
    ui.horizontal(|ui| {
        for (choice, name) in [
            (ThemeChoice::System, "System"),
            (ThemeChoice::Light, "Light"),
            (ThemeChoice::Dark, "Dark"),
        ] {
            if ui
                .selectable_label(state.settings.theme == choice, name)
                .clicked()
                && state.settings.theme != choice
            {
                state.set_theme(choice);
            }
        }
    });
    ui.add_space(16.0);

    ui.label(RichText::new("PresentMon path").strong());
    ui.label(
        RichText::new("Used to measure real FPS. Requires administrator rights.")
            .color(state.palette.text_muted),
    );
    let mut path = state
        .settings
        .presentmon_path
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    if ui.text_edit_singleline(&mut path).changed() {
        let trimmed = path.trim();
        state.settings.presentmon_path = (!trimmed.is_empty()).then(|| PathBuf::from(trimmed));
        state.effects.push(UiEffect::SaveSettings);
    }
    ui.hyperlink_to("Download PresentMon", PRESENTMON_RELEASES);
    ui.add_space(16.0);

    ui.label(RichText::new("Driver catalog URL").strong());
    ui.horizontal(|ui| {
        if ui
            .text_edit_singleline(&mut state.settings.catalog_url)
            .changed()
        {
            state.effects.push(UiEffect::SaveSettings);
        }
        if ui.button("Reset").clicked() {
            state.settings.catalog_url = DEFAULT_CATALOG_URL.to_string();
            state.effects.push(UiEffect::SaveSettings);
        }
    });
    ui.add_space(24.0);

    if ui.button("Rescan now").clicked() {
        state.request_scan();
    }
}
