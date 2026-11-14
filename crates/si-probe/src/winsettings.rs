//! Pure interpretation of raw Windows values (registry DWORDs, power GUIDs,
//! display modes) and filesystem game discovery. Tested on every OS.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use si_core::games::library::{
    parse_epic_manifest, parse_steam_app_manifest, parse_steam_library_folders,
};
use si_core::model::{InstalledGame, PowerPlan};

use crate::{ScanStep, ScanWarning};

const PLAN_BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
const PLAN_HIGH_PERFORMANCE: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
const PLAN_POWER_SAVER: &str = "a1841308-3541-4fab-bc81-f71556f20b4a";
const PLAN_ULTIMATE: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";
const FIRST_WINDOWS_11_BUILD: u32 = 22_000;
/// `SYSTEM_POWER_STATUS.BatteryFlag` values.
const BATTERY_FLAG_NO_BATTERY: u8 = 128;
const BATTERY_FLAG_UNKNOWN: u8 = 255;

/// Identifies the active power plan by GUID (plan names are localized).
pub fn power_plan_from_guid(guid: &str) -> PowerPlan {
    let g = guid
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .to_ascii_lowercase();
    match g.as_str() {
        "" => PowerPlan::Unknown,
        PLAN_BALANCED => PowerPlan::Balanced,
        PLAN_HIGH_PERFORMANCE => PowerPlan::HighPerformance,
        PLAN_POWER_SAVER => PowerPlan::PowerSaver,
        PLAN_ULTIMATE => PowerPlan::Ultimate,
        _ => PowerPlan::Custom,
    }
}

/// Windows 11 still reports `ProductName` = "Windows 10"; the build tells them apart.
pub fn os_name_for_build(build: u32) -> &'static str {
    if build >= FIRST_WINDOWS_11_BUILD {
        "Windows 11"
    } else {
        "Windows 10"
    }
}

/// `GameBar\AutoGameModeEnabled`: missing means on (the Windows default).
pub fn game_mode_from(v: Option<u32>) -> Option<bool> {
    Some(v != Some(0))
}

/// `GameDVR\HistoricalCaptureEnabled`: missing means off.
pub fn recording_from(v: Option<u32>) -> Option<bool> {
    Some(v.is_some_and(|v| v != 0))
}

/// `GraphicsDrivers\HwSchMode`: 2 = on, 1 = off, anything else = unknown.
pub fn hags_from(v: Option<u32>) -> Option<bool> {
    match v {
        Some(2) => Some(true),
        Some(1) => Some(false),
        _ => None,
    }
}

/// A `StartupApproved\Run` entry with an odd first byte (0x03, 0x07) is disabled.
pub fn startup_enabled(approved: Option<&[u8]>) -> bool {
    approved
        .and_then(|bytes| bytes.first())
        .is_none_or(|b| b % 2 == 0)
}

/// `(is_laptop, on_battery)` from `GetSystemPowerStatus`.
pub fn battery_state(battery_flag: u8, ac_line_status: u8) -> (Option<bool>, Option<bool>) {
    match battery_flag {
        BATTERY_FLAG_UNKNOWN => (None, None),
        BATTERY_FLAG_NO_BATTERY => (Some(false), Some(false)),
        _ => (
            Some(true),
            match ac_line_status {
                0 => Some(true),
                1 => Some(false),
                _ => None,
            },
        ),
    }
}

/// Highest refresh rate among display modes `(width, height, hz)` at the given resolution.
pub fn max_hz_for_resolution(modes: &[(u32, u32, u32)], width: u32, height: u32) -> u32 {
    modes
        .iter()
        .filter(|&&(w, h, _)| w == width && h == height)
        .map(|&(_, _, hz)| hz)
        .max()
        .unwrap_or(0)
}

/// Reads a file that may not be UTF-8 (game names, user paths).
fn read_lossy(path: &Path) -> Option<String> {
    fs::read(path)
        .ok()
        .map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// Files in `dir` whose names satisfy `wanted`; empty if `dir` is unreadable.
fn files_in(dir: &Path, wanted: impl Fn(&str) -> bool) -> Vec<std::path::PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(&wanted))
        .collect();
    files.sort();
    files
}

/// Installed Steam and Epic games. `steam_root` is Steam's install folder,
/// `epic_manifests` the launcher's `Data\Manifests` folder; `None` = not installed.
/// Unreadable libraries become warnings; bad manifests are skipped.
pub fn discover_games_in(
    steam_root: Option<&Path>,
    epic_manifests: Option<&Path>,
) -> (Vec<InstalledGame>, Vec<ScanWarning>) {
    let mut games = Vec::new();
    let mut warnings = Vec::new();

    if let Some(root) = steam_root {
        let folders = root.join("steamapps").join("libraryfolders.vdf");
        let mut libraries = read_lossy(&folders)
            .map(|text| parse_steam_library_folders(&text))
            .unwrap_or_default();
        let root_str = root.to_string_lossy().into_owned();
        // Steam's own folder is always a library. It goes last so the
        // spelling in libraryfolders.vdf (the registry's SteamPath often uses
        // `/` and lower case) wins when the same game is seen twice.
        if !libraries.iter().any(|l| Path::new(l) == root) {
            libraries.push(root_str);
        }
        for library in libraries {
            let apps = Path::new(&library).join("steamapps");
            if !apps.is_dir() {
                warnings.push(ScanWarning::new(
                    ScanStep::Games,
                    format!("Steam library not found: {library}"),
                ));
                continue;
            }
            let manifests = files_in(&apps, |n| {
                n.starts_with("appmanifest_") && n.ends_with(".acf")
            });
            games.extend(
                manifests
                    .iter()
                    .filter_map(|m| read_lossy(m))
                    .filter_map(|text| parse_steam_app_manifest(&text, &library)),
            );
        }
    }

    if let Some(dir) = epic_manifests {
        games.extend(
            files_in(dir, |n| n.ends_with(".item"))
                .iter()
                .filter_map(|m| read_lossy(m))
                .filter_map(|text| parse_epic_manifest(&text)),
        );
    }

    let mut seen = HashSet::new();
    games.retain(|g| seen.insert(g.key()));
    games.sort_by_key(|g| g.name.to_lowercase());
    (games, warnings)
}
