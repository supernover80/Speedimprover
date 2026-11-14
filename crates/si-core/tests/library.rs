use si_core::{games::library::*, model::Store};

const FOLDERS: &str = r#""libraryfolders"
{
	"0" { "path" "C:\\Program Files (x86)\\Steam" "label" "" }
	"1" { "path" "D:\\SteamLibrary" "label" "" }
}"#;
fn acf(appid: &str, flags: &str) -> String {
    format!(
        "\"AppState\"\n{{\n\"appid\" \"{appid}\"\n\"name\" \"Counter-Strike 2\"\n\"StateFlags\" \"{flags}\"\n\"installdir\" \"Counter-Strike Global Offensive\"\n\"SizeOnDisk\" \"37000000000\"\n}}"
    )
}

#[test]
fn library_folders_lists_all_paths() {
    assert_eq!(
        parse_steam_library_folders(FOLDERS),
        ["C:\\Program Files (x86)\\Steam", "D:\\SteamLibrary"]
    );
}
#[test]
fn app_manifest_builds_install_dir() {
    let g = parse_steam_app_manifest(&acf("730", "4"), "D:\\SteamLibrary\\").unwrap();
    assert_eq!(
        (g.store, g.id.as_str(), g.name.as_str()),
        (Store::Steam, "730", "Counter-Strike 2")
    );
    assert_eq!(
        g.install_dir,
        "D:\\SteamLibrary\\steamapps\\common\\Counter-Strike Global Offensive"
    );
    assert_eq!(g.size_bytes, Some(37_000_000_000));
}
#[test]
fn skips_redistributables_and_incomplete() {
    assert!(parse_steam_app_manifest(&acf("228980", "4"), "C:\\Steam").is_none());
    assert!(parse_steam_app_manifest(&acf("730", "1026"), "C:\\Steam").is_none());
}
#[test]
fn epic_manifest_parses() {
    let j = r#"{"DisplayName":"Fortnite","InstallLocation":"C:\\Program Files\\Epic Games\\Fortnite","AppName":"Fortnite","InstallSize":40000000000,"bIsIncompleteInstall":false}"#;
    let g = parse_epic_manifest(j).unwrap();
    assert_eq!(
        (g.store, g.id.as_str(), g.install_dir.as_str()),
        (
            Store::Epic,
            "Fortnite",
            "C:\\Program Files\\Epic Games\\Fortnite"
        )
    );
}
#[test]
fn malformed_inputs_are_skipped() {
    assert!(parse_steam_library_folders("garbage {").is_empty());
    assert!(parse_steam_app_manifest("\"AppState\" { \"appid\" \"730\" }", "C:\\Steam").is_none());
    assert!(parse_epic_manifest("{not json").is_none());
    assert!(parse_epic_manifest(r#"{"DisplayName":"X","InstallLocation":"C:\\X","AppName":"X","bIsIncompleteInstall":true}"#).is_none());
}
