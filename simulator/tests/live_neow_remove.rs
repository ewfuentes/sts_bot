use sts_simulator::{Action, GameState, Screen};

#[test]
fn neow_remove_card_matches_live_game() {
    // Load the Neow state captured from the live game
    let before_json = include_str!("fixtures/neow_remove_before.json");
    let mut state = GameState::from_json(before_json).unwrap();

    // Find the REMOVE_CARD action
    let remove_action = state
        .available_actions()
        .into_iter()
        .find(|a| match a {
            Action::PickEventOption { reward_type: Some(rt), .. } => rt == "REMOVE_CARD",
            _ => false,
        })
        .expect("No REMOVE_CARD action found");

    // Apply it
    state.apply(&remove_action);

    // Load what the live game produced
    let grid_json = include_str!("fixtures/neow_remove_grid.json");
    let live_grid_state = GameState::from_json(grid_json).unwrap();

    // Compare: both should be Grid with purpose=purge
    match (state.current_screen(), live_grid_state.current_screen()) {
        (
            Screen::Grid { purpose: sim_purpose, cards: sim_cards },
            Screen::Grid { purpose: live_purpose, cards: live_cards },
        ) => {
            assert_eq!(sim_purpose, live_purpose, "Grid purpose mismatch");
            assert_eq!(
                sim_cards.len(),
                live_cards.len(),
                "Grid card count mismatch: sim={} live={}",
                sim_cards.len(),
                live_cards.len()
            );
            // Compare card IDs
            let sim_ids: Vec<&str> = sim_cards.iter().map(|c| c.id.as_str()).collect();
            let live_ids: Vec<&str> = live_cards.iter().map(|c| c.id.as_str()).collect();
            assert_eq!(sim_ids, live_ids, "Grid card IDs mismatch");
        }
        (sim, live) => panic!(
            "Screen type mismatch:\n  Simulator: {:?}\n  Live game: {:?}",
            sim, live
        ),
    }

    // Compare available actions
    let sim_actions = state.available_actions();
    let live_actions = live_grid_state.available_actions();
    assert_eq!(
        sim_actions.len(),
        live_actions.len(),
        "Action count mismatch: sim={} live={}",
        sim_actions.len(),
        live_actions.len()
    );

    for (i, (sim, live)) in sim_actions.iter().zip(live_actions.iter()).enumerate() {
        assert_eq!(
            sim.to_commod_command(),
            live.to_commod_command(),
            "Action {} command mismatch",
            i
        );
    }
}
