//! Two reference snapshots used by tests across the workspace and by the
//! app's demo mode. Both were "taken" on 2026-11-01.

use crate::model::SystemSnapshot;

const HEALTHY_DESKTOP: &str = include_str!("fixtures/healthy_desktop.json");
const STRUGGLING_LAPTOP: &str = include_str!("fixtures/struggling_laptop.json");

/// A well-configured gaming desktop: no FPS findings.
pub fn healthy_desktop() -> SystemSnapshot {
    parse(HEALTHY_DESKTOP)
}

/// A budget laptop with many common FPS problems.
pub fn struggling_laptop() -> SystemSnapshot {
    parse(STRUGGLING_LAPTOP)
}

fn parse(json: &str) -> SystemSnapshot {
    // The JSON is compiled into the binary and covered by tests, so a parse
    // failure is a build defect, not a runtime condition.
    serde_json::from_str(json).expect("embedded fixture JSON is valid")
}
