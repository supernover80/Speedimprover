//! Minimal line charts (no axes or grid), like the mockup's chart card.

use egui::{Color32, Ui, vec2};
use egui_plot::{Line, Plot, PlotPoints};

/// `series`: (name, points `[x, y]`, colour). The y axis spans `0..=y_max`.
pub fn line_chart(
    ui: &mut Ui,
    id: &str,
    series: &[(&str, Vec<[f64; 2]>, Color32)],
    y_max: f64,
    height: f32,
) {
    Plot::new(id)
        .height(height)
        .show_axes(false)
        .show_grid(false)
        .show_background(false)
        .show_x(false)
        .show_y(false)
        .allow_drag(false)
        .allow_zoom(false)
        .allow_scroll(false)
        .allow_boxed_zoom(false)
        .allow_double_click_reset(false)
        .include_y(0.0)
        .include_y(y_max)
        .set_margin_fraction(vec2(0.0, 0.05))
        .show(ui, |plot| {
            for (name, points, color) in series {
                plot.line(
                    Line::new(*name, PlotPoints::from(points.clone()))
                        .color(*color)
                        .width(2.0),
                );
            }
        });
}
