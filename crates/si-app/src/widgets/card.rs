//! Rounded cards, as in the mockup.

use std::f32::consts::{FRAC_PI_2, PI};

use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{
    Align, Align2, Button, Color32, CornerRadius, FontId, Frame, Layout, Margin, Pos2, Rect,
    RichText, Sense, Shape, Stroke, Ui, pos2, vec2,
};

use crate::theme::{CARD_PADDING, CARD_RADIUS, Palette};

/// Deep purple the gradient card fades into.
const GRADIENT_END: Color32 = Color32::from_rgb(0x4C, 0x1D, 0x95);

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
                ui.label(RichText::new(title).size(16.0).strong().color(p.text));
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

/// Outline of a rounded rectangle, clockwise from the top-left arc.
fn rounded_outline(rect: Rect, r: f32) -> Vec<Pos2> {
    const STEPS: usize = 8;
    let corners = [
        (pos2(rect.left() + r, rect.top() + r), PI),
        (pos2(rect.right() - r, rect.top() + r), PI + FRAC_PI_2),
        (pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (pos2(rect.left() + r, rect.bottom() - r), FRAC_PI_2),
    ];
    corners
        .iter()
        .flat_map(|&(c, start)| {
            (0..=STEPS).map(move |i| {
                let a = start + FRAC_PI_2 * i as f32 / STEPS as f32;
                c + r * vec2(a.cos(), a.sin())
            })
        })
        .collect()
}

/// The purple "bank card": a left-to-right gradient with white text. The first
/// line is the title; the rest sit at the bottom.
pub fn gradient_card(ui: &mut Ui, p: &Palette, lines: &[&str]) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::hover());
    let outline = rounded_outline(rect, f32::from(CARD_RADIUS));
    let color_at = |x: f32| {
        let t = ((x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0);
        p.accent_strong.lerp_to_gamma(GRADIENT_END, t)
    };
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex {
        pos: rect.center(),
        uv: WHITE_UV,
        color: color_at(rect.center().x),
    });
    for &pos in &outline {
        mesh.vertices.push(Vertex {
            pos,
            uv: WHITE_UV,
            color: color_at(pos.x),
        });
    }
    let n = outline.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    let painter = ui.painter_at(rect);
    painter.add(Shape::mesh(mesh));

    let inset = rect.shrink(20.0);
    if let Some(title) = lines.first() {
        painter.text(
            inset.left_top(),
            Align2::LEFT_TOP,
            *title,
            FontId::proportional(20.0),
            Color32::WHITE,
        );
    }
    for (i, line) in lines.iter().skip(1).rev().enumerate() {
        painter.text(
            inset.left_bottom() - vec2(0.0, 20.0 * i as f32),
            Align2::LEFT_BOTTOM,
            *line,
            FontId::proportional(14.0),
            Color32::from_white_alpha(220),
        );
    }
}
