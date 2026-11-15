//! The sidebar: app name and one labelled entry per page.

use egui::{
    Align2, CornerRadius, FontId, Layout, Rect, RichText, Sense, WidgetInfo, WidgetType, vec2,
};
use egui_phosphor::regular as icons;

use crate::state::AppState;
use crate::theme::{PILL_RADIUS, heading};

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
            Page::Dashboard => "Overview",
            Page::Analyzer => "Issues",
            Page::Live => "Performance",
            Page::Drivers => "Drivers",
            Page::Games => "Games",
            Page::Settings => "Settings",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Page::Dashboard => icons::SQUARES_FOUR,
            Page::Analyzer => icons::LIST_CHECKS,
            Page::Live => icons::CHART_LINE,
            Page::Drivers => icons::CPU,
            Page::Games => icons::GAME_CONTROLLER,
            Page::Settings => icons::GEAR_SIX,
        }
    }
}

const ITEM_HEIGHT: f32 = 36.0;

pub fn sidebar(ui: &mut egui::Ui, state: &mut AppState) {
    let p = state.palette;
    ui.add_space(18.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(RichText::new(icons::GAUGE).size(20.0).color(p.accent));
        ui.label(heading(ui, "SpeedImprover", 16.0));
    });
    ui.add_space(20.0);
    for page in Page::ALL {
        let selected = state.page == page;
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), ITEM_HEIGHT), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, page.title()));
        let item = rect.shrink2(vec2(10.0, 0.0));
        let painter = ui.painter();
        if selected || response.hovered() {
            painter.rect_filled(item, CornerRadius::same(PILL_RADIUS), p.card_raised);
        }
        if selected {
            let bar = Rect::from_min_size(item.left_top() + vec2(0.0, 9.0), vec2(3.0, 18.0));
            painter.rect_filled(bar, CornerRadius::same(2), p.accent);
        }
        let color = if selected { p.text } else { p.text_muted };
        painter.text(
            item.left_center() + vec2(14.0, 0.0),
            Align2::LEFT_CENTER,
            page.icon(),
            FontId::proportional(17.0),
            if selected { p.accent } else { p.text_muted },
        );
        painter.text(
            item.left_center() + vec2(40.0, 0.0),
            Align2::LEFT_CENTER,
            page.title(),
            FontId::proportional(14.0),
            color,
        );
        if response.clicked() {
            state.set_page(page);
        }
        ui.add_space(2.0);
    }
    ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(18.0);
            ui.label(
                RichText::new(concat!("Version ", env!("CARGO_PKG_VERSION")))
                    .size(11.0)
                    .color(p.text_muted),
            );
        });
    });
}
