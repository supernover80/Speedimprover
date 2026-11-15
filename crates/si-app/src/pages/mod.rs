//! One module per page; each exposes `show(ui, state)`.

use crate::nav::Page;
use crate::state::AppState;

pub mod analyzer;
pub mod dashboard;
pub mod drivers;
pub mod games;
pub mod live;
pub mod settings;

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    match state.page {
        Page::Dashboard => dashboard::show(ui, state),
        Page::Analyzer => analyzer::show(ui, state),
        Page::Live => live::show(ui, state),
        Page::Drivers => drivers::show(ui, state),
        Page::Settings => settings::show(ui, state),
        Page::Games => games::show(ui, state),
    }
}
