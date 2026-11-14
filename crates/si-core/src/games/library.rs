//! Parsing Steam and Epic library files into [`InstalledGame`]s.
//!
//! Every function takes file contents, so parsing is testable on any OS; the
//! Windows probe finds the files. Bad or partial entries are skipped.

use keyvalues_parser::{Obj, Value, Vdf};

use crate::model::{InstalledGame, Store};

/// Steamworks Common Redistributables: installed with Steam, not a game.
const STEAM_REDISTRIBUTABLES_APPID: &str = "228980";
/// `StateFlags` bit meaning "fully installed".
const STEAM_FULLY_INSTALLED: u64 = 4;

fn first_str<'a>(obj: &'a Obj<'_>, key: &str) -> Option<&'a str> {
    obj.get(key)?.first()?.get_str()
}

fn root_obj(text: &str) -> Option<Obj<'_>> {
    let vdf: Vdf<'_> = keyvalues_parser::parse(text).ok()?.into();
    match vdf.value {
        Value::Obj(obj) => Some(obj),
        Value::Str(_) => None,
    }
}

/// Library paths from `steamapps\libraryfolders.vdf`, in Steam's order.
/// Handles both the current format (`"0" { "path" "…" }`) and the old one
/// (`"1" "D:\\SteamLibrary"`).
pub fn parse_steam_library_folders(vdf: &str) -> Vec<String> {
    let Some(root) = root_obj(vdf) else {
        return Vec::new();
    };
    let mut libraries: Vec<(u32, String)> = root
        .iter()
        .filter_map(|(key, values)| {
            let index: u32 = key.parse().ok()?;
            let path = match values.first()? {
                Value::Str(path) => path.to_string(),
                Value::Obj(folder) => first_str(folder, "path")?.to_string(),
            };
            Some((index, path))
        })
        .collect();
    libraries.sort_by_key(|(index, _)| *index);
    libraries.into_iter().map(|(_, path)| path).collect()
}

/// One game from a `steamapps\appmanifest_<id>.acf` in `library_path`.
pub fn parse_steam_app_manifest(acf: &str, library_path: &str) -> Option<InstalledGame> {
    let app = root_obj(acf)?;
    let appid = first_str(&app, "appid")?;
    let name = first_str(&app, "name")?;
    let installdir = first_str(&app, "installdir")?;
    if appid == STEAM_REDISTRIBUTABLES_APPID {
        return None;
    }
    if let Some(flags) = first_str(&app, "StateFlags") {
        let flags: u64 = flags.parse().ok()?;
        if flags & STEAM_FULLY_INSTALLED == 0 {
            return None;
        }
    }
    let library = library_path.trim_end_matches(['\\', '/']);
    Some(InstalledGame {
        store: Store::Steam,
        id: appid.to_string(),
        name: name.to_string(),
        install_dir: format!("{library}\\steamapps\\common\\{installdir}"),
        size_bytes: first_str(&app, "SizeOnDisk").and_then(|s| s.parse().ok()),
    })
}

/// One game from an Epic launcher `Manifests\*.item` file.
pub fn parse_epic_manifest(json: &str) -> Option<InstalledGame> {
    let item: serde_json::Value = serde_json::from_str(json).ok()?;
    if item.get("bIsIncompleteInstall").and_then(|v| v.as_bool()) == Some(true) {
        return None;
    }
    let field = |key: &str| {
        item.get(key)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
    };
    Some(InstalledGame {
        store: Store::Epic,
        id: field("AppName")?.to_string(),
        name: field("DisplayName")?.to_string(),
        install_dir: field("InstallLocation")?.to_string(),
        size_bytes: item.get("InstallSize").and_then(|v| v.as_u64()),
    })
}
