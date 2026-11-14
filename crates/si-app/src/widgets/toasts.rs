//! Short notifications in the bottom-right corner (spec §9).

use egui::{Align2, Context, CornerRadius, Frame, Margin, RichText, Stroke, vec2};

use crate::theme::{CARD_RADIUS, Palette};

/// Seconds a toast stays on screen.
const LIFETIME: f64 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Info,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    /// `egui` time (seconds) when the toast was pushed.
    pub created: f64,
}

#[derive(Debug, Clone, Default)]
pub struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    pub fn push(&mut self, text: impl Into<String>, kind: ToastKind, now: f64) {
        self.items.push(Toast {
            text: text.into(),
            kind,
            created: now,
        });
    }

    pub fn visible(&self, now: f64) -> Vec<&Toast> {
        self.items
            .iter()
            .filter(|t| now - t.created < LIFETIME)
            .collect()
    }

    pub fn show(&mut self, ctx: &Context, p: &Palette, now: f64) {
        self.items.retain(|t| now - t.created < LIFETIME);
        if self.items.is_empty() {
            return;
        }
        egui::Area::new("toasts".into())
            .anchor(Align2::RIGHT_BOTTOM, vec2(-16.0, -16.0))
            .fade_in(false)
            .interactable(false)
            .show(ctx, |ui| {
                for t in &self.items {
                    let (icon, color) = match t.kind {
                        ToastKind::Success => (egui_phosphor::regular::CHECK_CIRCLE, p.success),
                        ToastKind::Info => (egui_phosphor::regular::INFO, p.accent),
                        ToastKind::Error => (egui_phosphor::regular::WARNING_CIRCLE, p.danger),
                    };
                    Frame::new()
                        .fill(p.card_raised)
                        .stroke(Stroke::new(1.0, p.border))
                        .corner_radius(CornerRadius::same(CARD_RADIUS))
                        .inner_margin(Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(icon).size(18.0).color(color));
                                ui.label(RichText::new(&t.text).strong());
                            });
                        });
                    ui.add_space(8.0);
                }
            });
        // Repaint when the oldest toast should disappear.
        let remaining = self
            .items
            .iter()
            .map(|t| LIFETIME - (now - t.created))
            .fold(LIFETIME, f64::min)
            .max(0.05);
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(remaining));
    }
}
