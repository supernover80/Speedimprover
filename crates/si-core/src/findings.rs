//! What the FPS analysis reports, and the health score derived from it.

use serde::Serialize;

/// How much a finding is likely to cost in FPS. Ordered `Low < Medium < High`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Display,
    Power,
    Memory,
    Os,
    Gpu,
    Storage,
    Processes,
    Startup,
}

/// How the user can fix a finding. The app never changes settings itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Fix {
    /// Open a Windows Settings / Defender page (`ms-settings:…`, `windowsdefender://…`).
    OpenUri(String),
    /// Steps the user performs by hand.
    Manual(String),
    /// Open the app's Drivers page.
    ShowDrivers,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Finding {
    /// Stable rule id, e.g. `display.refresh_below_max`.
    pub id: &'static str,
    pub severity: Severity,
    pub category: Category,
    /// Exact UI copy from the spec.
    pub title: String,
    /// Plain-English explanation with the concrete numbers.
    pub detail: String,
    pub fix: Fix,
}

/// `100 − 15·High − 8·Medium − 3·Low`, floored at 0.
pub fn health_score(findings: &[Finding]) -> u8 {
    let penalty: u32 = findings
        .iter()
        .map(|f| match f.severity {
            Severity::High => 15,
            Severity::Medium => 8,
            Severity::Low => 3,
        })
        .sum();
    100u32.saturating_sub(penalty) as u8
}
