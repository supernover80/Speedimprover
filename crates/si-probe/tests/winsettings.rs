use si_core::model::PowerPlan;
use si_probe::winsettings::*;

#[test]
fn power_plan_by_guid() {
    assert_eq!(
        power_plan_from_guid("{8C5E7FDA-E8BF-4A96-9A85-A6E23A8C635C}"),
        PowerPlan::HighPerformance
    );
    assert_eq!(
        power_plan_from_guid("a1841308-3541-4fab-bc81-f71556f20b4a"),
        PowerPlan::PowerSaver
    );
    assert_eq!(
        power_plan_from_guid("00000000-0000-0000-0000-000000000000"),
        PowerPlan::Custom
    );
}
#[test]
fn windows_11_by_build() {
    assert_eq!(os_name_for_build(22631), "Windows 11");
    assert_eq!(os_name_for_build(19045), "Windows 10");
}
#[test]
fn registry_defaults() {
    assert_eq!(game_mode_from(None), Some(true));
    assert_eq!(game_mode_from(Some(0)), Some(false));
    assert_eq!(recording_from(None), Some(false));
    assert_eq!(
        (hags_from(Some(2)), hags_from(Some(1)), hags_from(None)),
        (Some(true), Some(false), None)
    );
}
#[test]
fn startup_approved_bytes() {
    assert!(startup_enabled(None));
    assert!(startup_enabled(Some(&[2, 0, 0])));
    assert!(!startup_enabled(Some(&[3, 0, 0])));
    assert!(!startup_enabled(Some(&[7, 0, 0])));
}
#[test]
fn battery_states() {
    assert_eq!(battery_state(128, 1), (Some(false), Some(false)));
    assert_eq!(battery_state(8, 0), (Some(true), Some(true)));
    assert_eq!(battery_state(255, 255), (None, None));
}
#[test]
fn max_hz_only_at_current_resolution() {
    let modes = [(1920, 1080, 60), (1920, 1080, 144), (1280, 720, 240)];
    assert_eq!(max_hz_for_resolution(&modes, 1920, 1080), 144);
}
#[test]
fn missing_library_folder_is_skipped() {
    let root = std::env::temp_dir().join(format!("si-steam-{}", std::process::id()));
    let apps = root.join("steamapps");
    std::fs::create_dir_all(&apps).unwrap();
    let lib = root.to_string_lossy().replace('\\', "\\\\");
    std::fs::write(apps.join("libraryfolders.vdf"), format!("\"libraryfolders\" {{ \"0\" {{ \"path\" \"{lib}\" }} \"1\" {{ \"path\" \"Z:\\\\Gone\" }} }}")).unwrap();
    std::fs::write(apps.join("appmanifest_730.acf"), "\"AppState\" { \"appid\" \"730\" \"name\" \"Counter-Strike 2\" \"StateFlags\" \"4\" \"installdir\" \"cs2\" }").unwrap();
    let (games, _warnings) = discover_games_in(Some(&root), None);
    assert_eq!(games.len(), 1);
}

#[test]
fn steam_library_spelling_wins_over_root_spelling() {
    let base = std::env::temp_dir().join(format!("si-steam-spelling-{}", std::process::id()));
    let root = base.join("Steam");
    let apps = root.join("steamapps");
    std::fs::create_dir_all(&apps).unwrap();
    // The same folder spelled differently, as libraryfolders.vdf and the
    // registry's SteamPath often disagree.
    let alias = base.join("Steam").join("..").join("Steam");
    let lib = alias.to_string_lossy().replace('\\', "\\\\");
    std::fs::write(
        apps.join("libraryfolders.vdf"),
        format!("\"libraryfolders\" {{ \"0\" {{ \"path\" \"{lib}\" }} }}"),
    )
    .unwrap();
    std::fs::write(
        apps.join("appmanifest_730.acf"),
        "\"AppState\" { \"appid\" \"730\" \"name\" \"Counter-Strike 2\" \"StateFlags\" \"4\" \"installdir\" \"cs2\" }",
    )
    .unwrap();
    let (games, warnings) = discover_games_in(Some(&root), None);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(games.len(), 1);
    assert!(
        games[0].install_dir.starts_with(&*alias.to_string_lossy()),
        "{}",
        games[0].install_dir
    );
}
