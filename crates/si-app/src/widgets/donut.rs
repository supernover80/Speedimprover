//! The ring gauges from the mockup's chart card.

use std::f32::consts::{FRAC_PI_2, TAU};

use egui::{
    Align2, FontId, Response, Sense, Shape, Stroke, Ui, WidgetInfo, WidgetType, pos2, vec2,
};

use crate::theme::Palette;

const RING_WIDTH: f32 = 10.0;

/// A ring filled clockwise from 12 o'clock to `pct` %, with the value in the
/// middle (`"73%"`, or `"—"` when unknown) and `caption` underneath.
pub fn donut(ui: &mut Ui, p: &Palette, pct: Option<f32>, caption: &str, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size + 22.0), Sense::hover());
    let painter = ui.painter();
    let center = pos2(rect.center().x, rect.top() + size / 2.0);
    let radius = size / 2.0 - RING_WIDTH / 2.0;
    painter.circle_stroke(center, radius, Stroke::new(RING_WIDTH, p.border));

    if let Some(value) = pct {
        let fraction = (value / 100.0).clamp(0.0, 1.0);
        if fraction > 0.0 {
            let steps = ((64.0 * fraction).ceil() as usize).max(2);
            let points = (0..=steps)
                .map(|i| {
                    let a = -FRAC_PI_2 + TAU * fraction * i as f32 / steps as f32;
                    center + radius * vec2(a.cos(), a.sin())
                })
                .collect();
            painter.add(Shape::line(points, Stroke::new(RING_WIDTH, p.accent)));
        }
    }

    let text = pct.map_or_else(|| "—".to_string(), |v| format!("{v:.0}%"));
    painter.text(
        center,
        Align2::CENTER_CENTER,
        &text,
        FontId::proportional(size * 0.22),
        p.text,
    );
    painter.text(
        pos2(center.x, rect.bottom() - 8.0),
        Align2::CENTER_CENTER,
        caption,
        FontId::proportional(12.0),
        p.text_muted,
    );
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
    response
}
