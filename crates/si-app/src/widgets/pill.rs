//! Buttons, badges and status dots.

use egui::{Button, Color32, CornerRadius, Response, RichText, Sense, Stroke, Ui, vec2};

use crate::theme::{PILL_RADIUS, Palette};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillKind {
    /// Filled accent, for the main action ("Install").
    Primary,
    /// Outlined, for secondary actions ("Rescan", "Download").
    Neutral,
    /// Tinted amber, for "Check".
    Warning,
    /// Plain muted text; not clickable.
    Muted,
}

pub fn pill(ui: &mut Ui, p: &Palette, text: &str, kind: PillKind) -> Response {
    let (fill, fg, stroke) = match kind {
        PillKind::Primary => (p.accent_strong, Color32::WHITE, Stroke::NONE),
        PillKind::Neutral => (p.card_raised, p.text, Stroke::new(1.0, p.border)),
        PillKind::Warning => (
            p.warning.gamma_multiply(0.14),
            p.warning,
            Stroke::new(1.0, p.warning.gamma_multiply(0.6)),
        ),
        PillKind::Muted => {
            return ui.label(RichText::new(text).size(12.0).color(p.text_muted));
        }
    };
    ui.add(
        Button::new(RichText::new(text).size(12.5).color(fg))
            .fill(fill)
            .stroke(stroke)
            .corner_radius(CornerRadius::same(PILL_RADIUS))
            .min_size(vec2(0.0, 26.0)),
    )
}

/// An 8 px coloured dot.
pub fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

/// A small, non-clickable label on a filled background (e.g. `High`), sized
/// to its text.
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
