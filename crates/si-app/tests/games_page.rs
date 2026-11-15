mod common;
use common::*;
use si_app::pages::games::reason_text;
use si_core::games::{profile::ProfileLibrary, recommend::*};

fn games_state() -> AppState {
    let mut st = testing::state_with(fixtures::healthy_desktop(), testing::demo_games());
    st.page = Page::Games;
    st
}
#[test]
fn shows_recommendations_for_first_profiled_game() {
    let h = harness(games_state());
    h.get_by_label("Your PC: High tier");
    let cs2 = ProfileLibrary::bundled()
        .profiles
        .into_iter()
        .find(|p| p.id == "cs2")
        .unwrap();
    h.get_by_label(&cs2.settings[0].label);
}
#[test]
fn goal_change_is_saved() {
    let mut h = harness(games_state());
    h.get_by_label("Competitive").click();
    h.run();
    assert_eq!(h.state().state.settings.goal, Goal::Competitive);
    assert!(h.state().state.effects.contains(&UiEffect::SaveSettings));
}
#[test]
fn star_toggles_favorite() {
    let mut h = harness(games_state());
    h.get_by_label("Favorite Counter-Strike 2").click();
    h.run();
    assert_eq!(h.state().state.settings.favorites, ["steam:730"]);
}
#[test]
fn reason_texts() {
    assert_eq!(
        reason_text(&Reason::VramCap(6144)),
        "Limited by 6 GB of VRAM"
    );
    assert_eq!(
        reason_text(&Reason::Tier(si_core::tier::Tier::High)),
        "Matches your High-tier PC"
    );
}
#[test]
fn no_games_empty_state() {
    let mut st = testing::state_with(fixtures::healthy_desktop(), vec![]);
    st.page = Page::Games;
    harness(st).get_by_label("No Steam or Epic games found");
}
