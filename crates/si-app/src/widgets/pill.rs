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
