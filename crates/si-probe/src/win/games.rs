//! Finds the Steam and Epic library folders on this PC.

use std::path::PathBuf;

use si_core::model::InstalledGame;

use crate::ScanWarning;
use crate::win::registry::steam_root;
use crate::winsettings::discover_games_in;

pub fn discover_games() -> (Vec<InstalledGame>, Vec<ScanWarning>) {
    let steam = steam_root();
    let epic = std::env::var_os("ProgramData")
        .map(|p| PathBuf::from(p).join(r"Epic\EpicGamesLauncher\Data\Manifests"));
    discover_games_in(steam.as_deref(), epic.as_deref())
}
