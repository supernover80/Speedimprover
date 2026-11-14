use si_core::{
    games::{profile::*, recommend::*},
    model::*,
    tier::*,
};

const P: &str = r#"
id = "test"
name = "Test Game"
steam_appid = 1
epic_app_name = ""
[[settings]]
label = "Shadows"
impact = "high"
low = "L"
mid = "M"
high = "H"
ultra = "U"
competitive = "Off"
[[settings]]
label = "Textures"
impact = "low"
vram_sensitive = true
low = "L"
mid = "M"
high = "H"
ultra = "U"
"#;
fn hw(overall: Tier, vram: u64) -> HardwareProfile {
    HardwareProfile {
        gpu: overall,
        cpu: overall,
        overall,
        vram_mb: vram,
    }
}
fn rec(goal: Goal, overall: Tier, vram: u64) -> Vec<(String, Reason)> {
    recommend(&GameProfile::parse(P).unwrap(), &hw(overall, vram), goal)
        .into_iter()
        .map(|r| (r.value, r.reason))
        .collect()
}

#[test]
fn balanced_uses_overall_tier() {
    assert_eq!(
        rec(Goal::Balanced, Tier::High, 12_000),
        [
            ("H".to_string(), Reason::Tier(Tier::High)),
            ("H".to_string(), Reason::Tier(Tier::High))
        ]
    );
}
#[test]
fn competitive_override_then_one_tier_down() {
    assert_eq!(
        rec(Goal::Competitive, Tier::High, 12_000),
        [
            ("Off".to_string(), Reason::Competitive),
            ("M".to_string(), Reason::Competitive)
        ]
    );
}
#[test]
fn quality_boosts_only_low_impact() {
    assert_eq!(
        rec(Goal::Quality, Tier::High, 12_000),
        [
            ("H".to_string(), Reason::Tier(Tier::High)),
            ("U".to_string(), Reason::QualityBoost)
        ]
    );
}
#[test]
fn vram_cap_applies_and_wins_over_boost() {
    assert_eq!(
        rec(Goal::Balanced, Tier::Ultra, 6_000)[1],
        ("M".to_string(), Reason::VramCap(6_000))
    );
    assert_eq!(
        rec(Goal::Quality, Tier::High, 5_000)[1],
        ("M".to_string(), Reason::VramCap(5_000))
    );
}
#[test]
fn tiers_saturate() {
    assert_eq!(rec(Goal::Quality, Tier::Ultra, 16_000)[1].0, "U");
    assert_eq!(rec(Goal::Competitive, Tier::Low, 16_000)[1].0, "L");
}
#[test]
fn profile_parsing_and_lookup() {
    let p = GameProfile::parse(P).unwrap();
    assert_eq!((p.steam_appid, p.epic_app_name.clone()), (Some(1), None));
    let lib = ProfileLibrary::bundled();
    let cs2 = InstalledGame {
        store: Store::Steam,
        id: "730".into(),
        ..Default::default()
    };
    let fn_ = InstalledGame {
        store: Store::Epic,
        id: "Fortnite".into(),
        ..Default::default()
    };
    assert_eq!(lib.for_game(&cs2).unwrap().id, "cs2");
    assert_eq!(lib.for_game(&fn_).unwrap().id, "fortnite");
}
#[test]
fn bundled_profiles_are_complete() {
    let lib = ProfileLibrary::bundled();
    assert_eq!(lib.profiles.len(), 8);
    let mut ids: Vec<_> = lib.profiles.iter().map(|p| p.id.clone()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 8);
    for p in &lib.profiles {
        assert!(
            p.steam_appid.is_some() || p.epic_app_name.is_some(),
            "{}",
            p.id
        );
        assert!(p.settings.len() >= 6, "{}", p.id);
        assert!(
            p.settings
                .iter()
                .all(|s| [&s.low, &s.mid, &s.high, &s.ultra]
                    .iter()
                    .all(|v| !v.is_empty())),
            "{}",
            p.id
        );
    }
}
