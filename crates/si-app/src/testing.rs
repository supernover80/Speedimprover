//! Helpers for tests and demo mode.

use si_core::model::{InstalledGame, Store, SystemSnapshot};
use si_probe::ScanResult;

use crate::settings::Settings;
use crate::state::AppState;
use crate::worker::WorkerEvent;

/// Three installed games: CS2 and Cyberpunk 2077 on Steam, Fortnite on Epic.
pub fn demo_games() -> Vec<InstalledGame> {
    let game = |store, id: &str, name: &str, dir: &str| InstalledGame {
        store,
        id: id.into(),
        name: name.into(),
        install_dir: dir.into(),
        size_bytes: None,
    };
    vec![
        game(
            Store::Steam,
            "730",
            "Counter-Strike 2",
            r"C:\Program Files (x86)\Steam\steamapps\common\Counter-Strike Global Offensive",
        ),
        game(
            Store::Steam,
            "1091500",
            "Cyberpunk 2077",
            r"C:\Program Files (x86)\Steam\steamapps\common\Cyberpunk 2077",
        ),
        game(
            Store::Epic,
            "Fortnite",
            "Fortnite",
            r"C:\Program Files\Epic Games\Fortnite",
        ),
    ]
}

/// A state as it looks right after a scan of `snapshot` finished.
pub fn state_with(snapshot: SystemSnapshot, games: Vec<InstalledGame>) -> AppState {
    let mut state = AppState::new(Settings::default());
    state.apply_event(WorkerEvent::ScanDone(Box::new(ScanResult {
        snapshot,
        games,
        warnings: Vec::new(),
    })));
    state.effects.clear();
    state
}
