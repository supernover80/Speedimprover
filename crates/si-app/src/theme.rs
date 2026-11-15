//! Colours, shapes and fonts: a neutral, Windows-style look.

use std::sync::Arc;

use egui::{
    Color32, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, RichText,
    Stroke, Visuals, vec2,
};
use serde::{Deserialize, Serialize};

/// Card corner radius.
pub const CARD_RADIUS: u8 = 8;
/// Button / chip corner radius.
pub const PILL_RADIUS: u8 = 6;
/// Inner padding of cards.
pub const CARD_PADDING: i8 = 14;
/// Gap between cards.
pub const GAP: f32 = 12.0;
/// Font family for headings (Segoe UI Semibold on Windows).
pub const HEADING_FAMILY: &str = "heading";

const SEGOE_UI: &str = r"C:\Windows\Fonts\segoeui.ttf";
const SEGOE_UI_SEMIBOLD: &str = r"C:\Windows\Fonts\seguisb.ttf";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    /// Follow Windows' light/dark setting.
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub bg: Color32,
    pub sidebar: Color32,
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
            bg: Color32::from_rgb(0x11, 0x13, 0x18),
            sidebar: Color32::from_rgb(0x16, 0x19, 0x1F),
            card: Color32::from_rgb(0x1B, 0x1F, 0x27),
            card_raised: Color32::from_rgb(0x23, 0x28, 0x33),
            border: Color32::from_rgb(0x2C, 0x32, 0x40),
            text: Color32::from_rgb(0xE6, 0xE8, 0xEC),
            text_muted: Color32::from_rgb(0x9A, 0xA3, 0xB2),
            accent: Color32::from_rgb(0x3B, 0x82, 0xF6),
            accent_strong: Color32::from_rgb(0x25, 0x63, 0xEB),
            success: Color32::from_rgb(0x22, 0xA0, 0x6B),
            warning: Color32::from_rgb(0xD9, 0x82, 0x2B),
            danger: Color32::from_rgb(0xE5, 0x48, 0x4D),
        }
    }

    pub const fn light() -> Self {
        Self {
            bg: Color32::from_rgb(0xF5, 0xF6, 0xF8),
            sidebar: Color32::from_rgb(0xEC, 0xEE, 0xF2),
            card: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            card_raised: Color32::from_rgb(0xF0, 0xF2, 0xF5),
            border: Color32::from_rgb(0xDD, 0xE1, 0xE7),
            text: Color32::from_rgb(0x1B, 0x1F, 0x24),
            text_muted: Color32::from_rgb(0x5B, 0x64, 0x72),
            accent: Color32::from_rgb(0x25, 0x63, 0xEB),
            accent_strong: Color32::from_rgb(0x1D, 0x4E, 0xD8),
            success: Color32::from_rgb(0x1A, 0x7F, 0x52),
            warning: Color32::from_rgb(0xB5, 0x65, 0x1D),
            danger: Color32::from_rgb(0xC9, 0x37, 0x2C),
        }
    }

    /// The palette for a theme choice; `system_dark` is Windows' setting.
    pub const fn resolve(choice: ThemeChoice, system_dark: bool) -> Self {
        match choice {
            ThemeChoice::Dark => Self::dark(),
            ThemeChoice::Light => Self::light(),
            ThemeChoice::System if system_dark => Self::dark(),
            ThemeChoice::System => Self::light(),
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
    v.window_corner_radius = CornerRadius::same(CARD_RADIUS);
    v.extreme_bg_color = p.card_raised;
    v.faint_bg_color = p.card_raised;
    v.override_text_color = Some(p.text);
    v.hyperlink_color = p.accent;
    v.selection.bg_fill = p.accent_strong;
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
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
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.hovered.weak_bg_fill = p.border;
    v.widgets.hovered.bg_fill = p.border;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.active.weak_bg_fill = p.accent_strong;
    v.widgets.active.bg_fill = p.accent_strong;
    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| s.spacing.button_padding = vec2(10.0, 4.0));
}

/// Segoe UI (the Windows UI font) when available, the Phosphor outline
/// icons, and a semibold family for headings.
pub fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    let defaults = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut proportional = defaults.clone();
    let mut heading = defaults;
    if let Ok(bytes) = std::fs::read(SEGOE_UI) {
        fonts
            .font_data
            .insert("segoe-ui".into(), Arc::new(FontData::from_owned(bytes)));
        proportional.insert(0, "segoe-ui".into());
        heading.insert(0, "segoe-ui".into());
    }
    if let Ok(bytes) = std::fs::read(SEGOE_UI_SEMIBOLD) {
        fonts.font_data.insert(
            "segoe-ui-semibold".into(),
            Arc::new(FontData::from_owned(bytes)),
        );
        heading.insert(0, "segoe-ui-semibold".into());
    }
    fonts
        .families
        .insert(FontFamily::Proportional, proportional);
    fonts
        .families
        .insert(FontFamily::Name(HEADING_FAMILY.into()), heading);
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    // Icons are also needed in headings.
    if let (Some(icons), Some(heading)) = (
        fonts.families.get(&FontFamily::Proportional).cloned(),
        fonts
            .families
            .get_mut(&FontFamily::Name(HEADING_FAMILY.into())),
    ) {
        for font in icons {
            if !heading.contains(&font) {
                heading.push(font);
            }
        }
    }
    ctx.set_fonts(fonts);
}

/// Heading text in the semibold family (the normal font until the fonts
/// installed on the first frame take effect).
pub fn heading(ui: &egui::Ui, text: impl Into<String>, size: f32) -> RichText {
    let family = FontFamily::Name(HEADING_FAMILY.into());
    let ready = ui
        .ctx()
        .fonts(|f| f.definitions().families.contains_key(&family));
    let family = if ready {
        family
    } else {
        FontFamily::Proportional
    };
    RichText::new(text).font(FontId::new(size, family))
}
