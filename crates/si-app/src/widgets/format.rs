//! Text and colour helpers shared by the pages.

use egui::Color32;
use si_core::drivers::DriverStatus;
use si_core::findings::Severity;
use si_core::tier::Tier;

use crate::theme::Palette;

/// `512 MB`, `8 GB`, `7.7 GB` (one decimal, `.0` dropped).
pub fn format_mb(mb: u64) -> String {
    if mb < 1024 {
        return format!("{mb} MB");
    }
    let gb = format!("{:.1}", mb as f64 / 1024.0);
    format!("{} GB", gb.strip_suffix(".0").unwrap_or(&gb))
}

pub fn severity_color(s: Severity, p: &Palette) -> Color32 {
    match s {
        Severity::High => p.danger,
        Severity::Medium => p.warning,
        Severity::Low => p.accent,
    }
}

/// `HIGH TIER`.
pub fn tier_badge(t: Tier) -> String {
    format!("{} TIER", t.label().to_uppercase())
}

/// Pill text for a driver row.
pub fn driver_status_label(s: &DriverStatus) -> &'static str {
    match s {
        DriverStatus::UpdateAvailable { .. } => "Update",
        DriverStatus::PossiblyOutdated { .. } => "Check",
        DriverStatus::NoUpdateFound => "No update found",
    }
}
