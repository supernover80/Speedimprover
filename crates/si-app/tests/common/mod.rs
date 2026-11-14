#![allow(dead_code, unused_imports)]
//! Shared helpers for si-app UI tests.

pub use egui_kittest::{Harness, kittest::Queryable};
pub use si_app::{app::SpeedImproverApp, nav::Page, settings::*, state::*, testing, worker::*};
pub use si_core::fixtures;

/// The app rendered headless at its default window size.
pub fn harness(state: AppState) -> Harness<'static, SpeedImproverApp> {
    Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_ui_state(
            |ui, app: &mut SpeedImproverApp| app.show(ui),
            SpeedImproverApp::headless(state),
        )
}
