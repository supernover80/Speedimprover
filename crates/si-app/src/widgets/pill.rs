//! Pill buttons and status dots.

use egui::{Button, Color32, CornerRadius, Response, RichText, Sense, Ui, vec2};

use crate::theme::{PILL_RADIUS, Palette};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillKind {
    /// Purple, for the main action ("Update").
    Primary,
    /// Light grey, like the mockup's "Download" buttons.
    Neutral,
    /// Amber, for "Check".
    Warning,
    /// Plain muted text; not clickable.
    Muted,
}

pub fn pill(ui: &mut Ui, p: &Palette, text: &str, kind: PillKind) -> Response {
    let (fill, fg) = match kind {
        PillKind::Primary => (p.accent_strong, Color32::WHITE),
        PillKind::Neutral => (
            Color32::from_rgb(0xD9, 0xD9, 0xE0),
            Color32::from_rgb(0x15, 0x15, 0x1C),
        ),
        PillKind::Warning => (p.warning, Color32::from_rgb(0x1A, 0x1A, 0x1A)),
        PillKind::Muted => {
            return ui.label(RichText::new(text).size(12.0).color(p.text_muted));
        }
    };
    ui.add(
        Button::new(RichText::new(text).size(12.0).strong().color(fg))
            .fill(fill)
            .corner_radius(CornerRadius::same(PILL_RADIUS))
            .min_size(vec2(0.0, 24.0)),
    )
}

/// An 8 px coloured dot.
pub fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

/// A small, non-clickable label on a filled background (e.g. `LOW TIER`),
/// sized to its text.
pub fn badge(ui: &mut Ui, text: &str, fill: Color32, fg: Color32) -> Response {
    let font = egui::FontId::proportional(11.0);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, fg);
    let size = galley.size() + vec2(20.0, 8.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(PILL_RADIUS), fill);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, fg);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, text));
    response
}

/// An on/off switch, exposed to assistive tech as a checkbox named `label`.
pub fn toggle(ui: &mut Ui, p: &Palette, on: &mut bool, label: &str) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(vec2(40.0, 22.0), Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let state = *on;
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, state, label));
    let track = if state { p.accent_strong } else { p.border };
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(11), track);
    let x = if state {
        rect.right() - 11.0
    } else {
        rect.left() + 11.0
    };
    painter.circle_filled(egui::pos2(x, rect.center().y), 8.0, Color32::WHITE);
    response
}
