//! The icon sidebar.

use egui::{Button, RichText, WidgetInfo, WidgetType, vec2};
use egui_phosphor::regular as icons;

use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Dashboard,
    Analyzer,
    Live,
    Drivers,
    Games,
    Settings,
}

impl Page {
    pub const ALL: [Page; 6] = [
        Page::Dashboard,
        Page::Analyzer,
        Page::Live,
        Page::Drivers,
        Page::Games,
        Page::Settings,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Page::Dashboard => "Dashboard",
            Page::Analyzer => "FPS Analyzer",
            Page::Live => "Live Monitor",
            Page::Drivers => "Drivers",
            Page::Games => "Games",
            Page::Settings => "Settings",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Page::Dashboard => icons::HOUSE,
            Page::Analyzer => icons::GAUGE,
            Page::Live => icons::PULSE,
            Page::Drivers => icons::CPU,
            Page::Games => icons::GAME_CONTROLLER,
            Page::Settings => icons::GEAR,
        }
    }
}

/// Icon-only buttons; each exposes its page title for screen readers and tests.
pub fn sidebar(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical_centered(|ui| {
        ui.add_space(16.0);
        ui.label(
            RichText::new(icons::LIGHTNING)
                .size(26.0)
                .color(state.palette.accent),
        );
        ui.add_space(24.0);
        for page in Page::ALL {
            let color = if state.page == page {
                state.palette.accent
            } else {
                state.palette.text_muted
            };
            let response = ui.add(
                Button::new(RichText::new(page.icon()).size(22.0).color(color))
                    .frame(false)
                    .min_size(vec2(44.0, 44.0)),
            );
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, page.title()));
            if response.on_hover_text(page.title()).clicked() {
                state.set_page(page);
            }
            ui.add_space(8.0);
        }
    });
}
