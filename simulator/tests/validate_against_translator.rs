use std::fs;
use std::path::Path;
use sts_simulator::{Action, GameState};

/// Load a translated JSON fixture, deserialize it, call available_actions(),
/// and compare against the actions the Python translator produced.
fn validate_fixture(path: &str) {
    let json = fs::read_to_string(path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e));
    let state = GameState::from_json(&json).unwrap_or_else(|e| panic!("Failed to deserialize {}: {}", path, e));

    let rust_actions = state.available_actions();
    let translator_actions = &state.actions;

    // Compare action types
    let rust_types: Vec<&str> = rust_actions.iter().map(action_type).collect();
    let translator_types: Vec<&str> = translator_actions.iter().map(action_type).collect();

    assert_eq!(
        rust_types, translator_types,
        "Action type mismatch in {}\n  Rust:       {:?}\n  Translator: {:?}",
        path, rust_types, translator_types
    );

    // Compare commod commands
    for (i, (rust, translator)) in rust_actions.iter().zip(translator_actions.iter()).enumerate() {
        let rust_cmd = rust.to_commod_command();
        let translator_cmd = translator.to_commod_command();
        assert_eq!(
            rust_cmd, translator_cmd,
            "Command mismatch at action {} in {}\n  Rust:       {} ({:?})\n  Translator: {} ({:?})",
            i, path, rust_cmd, rust, translator_cmd, translator
        );
    }
}

fn action_type(action: &Action) -> &str {
    match action {
        Action::TravelTo { .. } => "travel_to",
        Action::PickNeowBlessing { .. } => "pick_neow_blessing",
        Action::PickEventOption { .. } => "pick_event_option",
        Action::TakeCard { .. } => "take_card",
        Action::SkipCardReward => "skip_card_reward",
        Action::TakeReward { .. } => "take_reward",
        Action::PickBossRelic { .. } => "pick_boss_relic",
        Action::SkipBossRelic => "skip_boss_relic",
        Action::BuyCard { .. } => "buy_card",
        Action::BuyRelic { .. } => "buy_relic",
        Action::BuyPotion { .. } => "buy_potion",
        Action::Purge { .. } => "purge",
        Action::LeaveShop => "leave_shop",
        Action::Rest { .. } => "rest",
        Action::Smith { .. } => "smith",
        Action::OpenChest { .. } => "open_chest",
        Action::PickGridCard { .. } => "pick_grid_card",
        Action::PickHandCard { .. } => "pick_hand_card",
        Action::PickCustomScreenOption { .. } => "pick_custom_screen_option",
        Action::Proceed => "proceed",
        Action::Skip => "skip",
    }
}

#[test]
fn validate_neow() {
    validate_fixture("tests/fixtures/neow.json");
}

#[test]
fn validate_map() {
    validate_fixture("tests/fixtures/map.json");
}

#[test]
fn validate_run_fixtures() {
    let run_dir = Path::new("tests/fixtures/run");
    if !run_dir.exists() {
        return;
    }
    let mut entries: Vec<_> = fs::read_dir(run_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let path = entry.path();
        let filename = path.file_name().unwrap().to_str().unwrap();

        // Skip combat fixtures for now
        if filename.contains("combat") {
            println!("Skipping combat fixture: {}", filename);
            continue;
        }

        println!("Validating: {}", filename);
        validate_fixture(path.to_str().unwrap());
    }
}
