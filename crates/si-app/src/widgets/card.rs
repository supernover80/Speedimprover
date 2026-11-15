//! Cards, banners and columns.

use egui::{
    Align, Button, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke, Ui, vec2,
};

use crate::theme::{CARD_PADDING, CARD_RADIUS, GAP, Palette, heading};

pub struct CardResponse {
    /// The top-right action link (e.g. "View All") was clicked.
    pub action_clicked: bool,
}

/// A card filling the available width: title top-left, optional action link
/// top-right, then the contents.
pub fn card(
    ui: &mut Ui,
    p: &Palette,
    title: &str,
    action: Option<&str>,
    add: impl FnOnce(&mut Ui),
) -> CardResponse {
    let mut action_clicked = false;
    Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(CARD_RADIUS))
        .inner_margin(Margin::same(CARD_PADDING))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(heading(ui, title, 15.0).color(p.text));
                if let Some(action) = action {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let link =
                            Button::new(RichText::new(action).size(12.0).color(p.text_muted))
                                .frame(false);
                        action_clicked = ui.add(link).clicked();
                    });
                }
            });
            ui.add_space(10.0);
            add(ui);
        });
    CardResponse { action_clicked }
}

/// A full-width notice with an icon (e.g. "Restart your PC…").
pub fn banner(ui: &mut Ui, p: &Palette, icon: &str, color: Color32, text: &str) {
    Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, color))
        .corner_radius(CornerRadius::same(CARD_RADIUS))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).size(18.0).color(color));
                ui.label(RichText::new(text).strong());
            });
        });
}

/// A fixed-width, top-down column (for side-by-side cards).
pub fn column(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
        ui.set_width(width);
        ui.spacing_mut().item_spacing.y = GAP;
        add(ui);
    });
}
