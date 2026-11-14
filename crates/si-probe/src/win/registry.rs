//! Registry reads: Windows version, gaming settings, startup apps, Steam path.

use std::path::PathBuf;

use si_core::model::SystemSnapshot;
use winreg::HKEY;
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

use crate::winsettings::{
    game_mode_from, hags_from, os_name_for_build, recording_from, startup_enabled,
};

const CURRENT_VERSION: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
const GAME_BAR: &str = r"Software\Microsoft\GameBar";
const GAME_DVR: &str = r"Software\Microsoft\Windows\CurrentVersion\GameDVR";
const GRAPHICS_DRIVERS: &str = r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers";
const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const STARTUP_APPROVED_RUN: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const STEAM: &str = r"Software\Valve\Steam";

fn dword(root: HKEY, path: &str, name: &str) -> Option<u32> {
    RegKey::predef(root)
        .open_subkey(path)
        .ok()?
        .get_value::<u32, _>(name)
        .ok()
}

fn string(root: HKEY, path: &str, name: &str) -> Option<String> {
    RegKey::predef(root)
        .open_subkey(path)
        .ok()?
        .get_value::<String, _>(name)
        .ok()
}

/// Build number and a display name like "Windows 11 24H2".
pub fn collect_os_info(snap: &mut SystemSnapshot) {
    let Some(build) = string(HKEY_LOCAL_MACHINE, CURRENT_VERSION, "CurrentBuildNumber")
        .and_then(|b| b.trim().parse::<u32>().ok())
    else {
        return;
    };
    snap.machine.os_build = build;
    let version = string(HKEY_LOCAL_MACHINE, CURRENT_VERSION, "DisplayVersion").unwrap_or_default();
    snap.machine.os_name = format!("{} {}", os_name_for_build(build), version.trim())
        .trim()
        .to_string();
}

/// Game Mode, background recording, HAGS and the startup-app count.
pub fn collect_os_settings(snap: &mut SystemSnapshot) {
    snap.os.game_mode = game_mode_from(dword(HKEY_CURRENT_USER, GAME_BAR, "AutoGameModeEnabled"));
    snap.os.background_recording = recording_from(dword(
        HKEY_CURRENT_USER,
        GAME_DVR,
        "HistoricalCaptureEnabled",
    ));
    snap.os.hags = hags_from(dword(HKEY_LOCAL_MACHINE, GRAPHICS_DRIVERS, "HwSchMode"));
    snap.os.startup_apps = count_startup_apps();
}

/// Enabled entries under both `Run` keys; `None` if neither key can be read.
fn count_startup_apps() -> Option<u32> {
    let mut total = None;
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let Ok(run) = RegKey::predef(root).open_subkey(RUN) else {
            continue;
        };
        let approved = RegKey::predef(root).open_subkey(STARTUP_APPROVED_RUN).ok();
        let enabled = run
            .enum_values()
            .flatten()
            .filter(|(name, _)| {
                let state = approved.as_ref().and_then(|a| a.get_raw_value(name).ok());
                startup_enabled(state.as_ref().map(|v| v.bytes.as_ref()))
            })
            .count() as u32;
        *total.get_or_insert(0) += enabled;
    }
    total
}

/// Steam's install folder, if Steam is installed for this user.
pub fn steam_root() -> Option<PathBuf> {
    string(HKEY_CURRENT_USER, STEAM, "SteamPath").map(PathBuf::from)
}
