//! One module per page; each exposes `show(ui, state)`.

use crate::nav::Page;
use crate::state::AppState;

pub mod settings;

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    match state.page {
        Page::Settings => settings::show(ui, state),
        other => {
            ui.heading(other.title());
        }
    }
}
