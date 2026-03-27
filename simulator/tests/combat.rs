use sts_simulator::{Action, GameState, Screen};

fn load_fixture(path: &str) -> GameState {
    let json = std::fs::read_to_string(path).unwrap();
    GameState::from_json(&json).unwrap()
}

#[test]
fn deserialize_combat_state() {
    let state = load_fixture("tests/fixtures/combat.json");
    assert_eq!(state.hp, 8);
    assert_eq!(state.floor, 1);

    if let Screen::Combat {
        encounter,
        monsters,
        hand,
        draw_pile,
        discard_pile,
        exhaust_pile,
        player_block,
        player_energy,
        player_powers,
        turn,
        ..
    } = state.current_screen()
    {
        assert_eq!(encounter, "BGJawWorm");
        assert_eq!(monsters.len(), 1);
        assert_eq!(monsters[0].name, "Jaw Worm");
        assert_eq!(monsters[0].hp, 8);
        assert_eq!(monsters[0].intent, "ATTACK");
        assert_eq!(monsters[0].damage, Some(3));

        assert_eq!(hand.len(), 5);
        assert_eq!(hand[0].card.id, "BGStrike_R");

        assert_eq!(draw_pile.len(), 2);
        assert!(discard_pile.is_empty());
        assert!(exhaust_pile.is_empty());

        assert_eq!(*player_block, 0);
        assert_eq!(*player_energy, 3);
        assert!(player_powers.is_empty());
        assert_eq!(*turn, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn combat_available_actions_single_monster() {
    let state = load_fixture("tests/fixtures/combat.json");
    let actions = state.available_actions();

    // 3 targeted attacks (Strike x2, Bash) on 1 monster + 2 untargeted (Defend x2) + EndTurn
    let play_cards: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a, Action::PlayCard { .. }))
        .collect();
    assert_eq!(play_cards.len(), 5);

    // Check targeting on first Strike
    if let Action::PlayCard {
        card,
        hand_index,
        target_index,
        target_name,
    } = &play_cards[0]
    {
        assert_eq!(card.id, "BGStrike_R");
        assert_eq!(*hand_index, 0);
        assert_eq!(*target_index, Some(0));
        assert_eq!(target_name.as_deref(), Some("Jaw Worm"));
    }

    // Check untargeted Defend
    if let Action::PlayCard {
        card,
        target_index,
        ..
    } = &play_cards[2]
    {
        assert_eq!(card.id, "BGDefend_R");
        assert_eq!(*target_index, None);
    }

    // EndTurn always present
    assert!(actions.iter().any(|a| matches!(a, Action::EndTurn)));
}

#[test]
fn combat_available_actions_skips_dead_monsters() {
    let state = load_fixture("tests/fixtures/combat_multi_monster.json");
    let actions = state.available_actions();

    // Strike targets only 2 live monsters (indices 0 and 2, not 1 which is_gone)
    let strike_actions: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a, Action::PlayCard { card, .. } if card.id == "BGStrike_R"))
        .collect();
    assert_eq!(strike_actions.len(), 2);

    // Verify target indices skip the dead monster
    if let Action::PlayCard { target_index, .. } = &strike_actions[0] {
        assert_eq!(*target_index, Some(0));
    }
    if let Action::PlayCard { target_index, .. } = &strike_actions[1] {
        assert_eq!(*target_index, Some(2));
    }
}

#[test]
fn combat_actions_match_translator() {
    // Verify the Rust-generated actions match the translator's expected actions
    let state = load_fixture("tests/fixtures/combat.json");
    let rust_actions = state.available_actions();
    let translator_actions = &state.actions;

    // Compare commands
    let rust_cmds: Vec<String> = rust_actions.iter().map(|a| a.to_commod_command()).collect();
    let translator_cmds: Vec<String> = translator_actions.iter().map(|a| a.to_commod_command()).collect();
    assert_eq!(rust_cmds, translator_cmds,
        "Rust: {:?}\nTranslator: {:?}", rust_cmds, translator_cmds);
}

#[test]
fn combat_actions_match_translator_multi_monster() {
    let state = load_fixture("tests/fixtures/combat_multi_monster.json");
    let rust_actions = state.available_actions();
    let translator_actions = &state.actions;

    let rust_cmds: Vec<String> = rust_actions.iter().map(|a| a.to_commod_command()).collect();
    let translator_cmds: Vec<String> = translator_actions.iter().map(|a| a.to_commod_command()).collect();
    assert_eq!(rust_cmds, translator_cmds,
        "Rust: {:?}\nTranslator: {:?}", rust_cmds, translator_cmds);
}

#[test]
fn combat_round_trip_serialization() {
    let state = load_fixture("tests/fixtures/combat.json");
    let json = serde_json::to_string(&state).unwrap();
    let state2 = GameState::from_json(&json).unwrap();

    if let Screen::Combat { monsters, hand, player_energy, .. } = state2.current_screen() {
        assert_eq!(monsters.len(), 1);
        assert_eq!(hand.len(), 5);
        assert_eq!(*player_energy, 3);
    } else {
        panic!("Expected Combat screen after round-trip");
    }
}

#[test]
fn play_card_commod_command() {
    let card = sts_simulator::Card {
        id: "BGStrike_R".into(),
        name: "Strike".into(),
        cost: 1,
        card_type: "ATTACK".into(),
        upgraded: false,
    };

    let untargeted = Action::PlayCard {
        card: card.clone(),
        hand_index: 2,
        target_index: None,
        target_name: None,
    };
    assert_eq!(untargeted.to_commod_command(), "play 3");

    let targeted = Action::PlayCard {
        card,
        hand_index: 0,
        target_index: Some(1),
        target_name: Some("Jaw Worm".into()),
    };
    assert_eq!(targeted.to_commod_command(), "play 1 1");
}

#[test]
fn end_turn_commod_command() {
    assert_eq!(Action::EndTurn.to_commod_command(), "end");
}
