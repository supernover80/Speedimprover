//! Colours and styling (spec §9): dark purple dashboard, light variant.

use egui::{Color32, Context, CornerRadius, FontDefinitions, Stroke, Visuals};
use serde::{Deserialize, Serialize};

/// Card corner radius.
pub const CARD_RADIUS: u8 = 14;
/// Pill button / chip corner radius.
pub const PILL_RADIUS: u8 = 10;
/// Inner padding of cards.
pub const CARD_PADDING: i8 = 16;
/// Gap between cards.
pub const GAP: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub bg: Color32,
    pub card: Color32,
    pub card_raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_strong: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
}

impl Palette {
    pub const fn dark() -> Self {
        Self {
            bg: Color32::from_rgb(0x0F, 0x0F, 0x14),
            card: Color32::from_rgb(0x17, 0x17, 0x1F),
            card_raised: Color32::from_rgb(0x1F, 0x1F, 0x2A),
            border: Color32::from_rgb(0x2A, 0x2A, 0x36),
            text: Color32::from_rgb(0xEC, 0xEC, 0xF1),
            text_muted: Color32::from_rgb(0x9A, 0x9A, 0xAE),
            accent: Color32::from_rgb(0xA7, 0x8B, 0xFA),
            accent_strong: Color32::from_rgb(0x8B, 0x5C, 0xF6),
            success: Color32::from_rgb(0x22, 0xC5, 0x5E),
            warning: Color32::from_rgb(0xF5, 0x9E, 0x0B),
            danger: Color32::from_rgb(0xEF, 0x44, 0x44),
        }
    }

    pub const fn light() -> Self {
        Self {
            bg: Color32::from_rgb(0xF4, 0xF4, 0xF8),
            card: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            card_raised: Color32::from_rgb(0xEE, 0xEE, 0xF4),
            border: Color32::from_rgb(0xDC, 0xDC, 0xE6),
            text: Color32::from_rgb(0x15, 0x15, 0x1C),
            text_muted: Color32::from_rgb(0x5C, 0x5C, 0x70),
            accent: Color32::from_rgb(0x7C, 0x3A, 0xED),
            accent_strong: Color32::from_rgb(0x6D, 0x28, 0xD9),
            success: Color32::from_rgb(0x16, 0xA3, 0x4A),
            warning: Color32::from_rgb(0xD9, 0x77, 0x06),
            danger: Color32::from_rgb(0xDC, 0x26, 0x26),
        }
    }

    pub const fn for_choice(choice: ThemeChoice) -> Self {
        match choice {
            ThemeChoice::Dark => Self::dark(),
            ThemeChoice::Light => Self::light(),
        }
    }

    fn is_dark(&self) -> bool {
        self.bg.r() < 0x80
    }
}

/// Applies the palette to egui's visuals.
pub fn apply(ctx: &Context, p: &Palette) {
    let mut v = if p.is_dark() {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.panel_fill = p.bg;
    v.window_fill = p.card;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.extreme_bg_color = p.card_raised;
    v.faint_bg_color = p.card_raised;
    v.override_text_color = Some(p.text);
    v.hyperlink_color = p.accent;
    v.selection.bg_fill = p.accent_strong;
    v.selection.stroke = Stroke::new(1.0, p.text);
    let radius = CornerRadius::same(PILL_RADIUS);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = radius;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.inactive.weak_bg_fill = p.card_raised;
    v.widgets.inactive.bg_fill = p.card_raised;
    v.widgets.hovered.weak_bg_fill = p.border;
    v.widgets.hovered.bg_fill = p.border;
    v.widgets.active.weak_bg_fill = p.accent_strong;
    v.widgets.active.bg_fill = p.accent_strong;
    ctx.set_visuals(v);
}

/// Adds the Phosphor outline icon font (used for the sidebar and card icons).
pub fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
}
