use sts_simulator::{Action, GameState, MapNodeKind, Screen};

#[test]
fn deserialize_neow_fixture() {
    let json = include_str!("fixtures/neow.json");
    let state = GameState::from_json(json).expect("Failed to deserialize neow fixture");

    assert!(state.hp > 0);
    assert!(state.max_hp > 0);
    assert_eq!(state.floor, 0);
    assert_eq!(state.act, 1);
    assert!(!state.deck.is_empty());
    assert!(!state.relics.is_empty());

    match state.current_screen() {
        Screen::Event { event_name, options, .. } => {
            assert_eq!(event_name, "Neow");
            assert!(!options.is_empty());
        }
        other => panic!("Expected Event screen, got {:?}", other),
    }
}

#[test]
fn neow_available_actions() {
    let json = include_str!("fixtures/neow.json");
    let state = GameState::from_json(json).unwrap();

    let actions = state.available_actions();

    // Should have one action per non-disabled option
    let non_disabled = match state.current_screen() {
        Screen::Event { options, .. } => options.iter().filter(|o| !o.disabled).count(),
        _ => panic!("Expected Event screen"),
    };
    assert_eq!(actions.len(), non_disabled);

    // All should be PickEventOption
    for action in &actions {
        match action {
            Action::PickEventOption { .. } => {}
            other => panic!("Expected PickEventOption, got {:?}", other),
        }
    }
}

#[test]
fn neow_action_to_commod_command() {
    let json = include_str!("fixtures/neow.json");
    let state = GameState::from_json(json).unwrap();
    let actions = state.available_actions();

    assert_eq!(actions[0].to_commod_command(), "choose 0");
}

#[test]
fn deserialize_map_fixture() {
    let json = include_str!("fixtures/map.json");
    let state = GameState::from_json(json).expect("Failed to deserialize map fixture");

    match state.current_screen() {
        Screen::Map { available_nodes, .. } => {
            assert!(!available_nodes.is_empty());
        }
        other => panic!("Expected Map screen, got {:?}", other),
    }
}

#[test]
fn map_available_actions() {
    let json = include_str!("fixtures/map.json");
    let state = GameState::from_json(json).unwrap();

    let actions = state.available_actions();
    let node_count = match state.current_screen() {
        Screen::Map { available_nodes, .. } => available_nodes.len(),
        _ => panic!("Expected Map screen"),
    };
    assert_eq!(actions.len(), node_count);

    for action in &actions {
        match action {
            Action::TravelTo { .. } => {}
            other => panic!("Expected TravelTo, got {:?}", other),
        }
    }
}

#[test]
fn map_action_to_commod_command() {
    let json = include_str!("fixtures/map.json");
    let state = GameState::from_json(json).unwrap();
    let actions = state.available_actions();

    assert_eq!(actions[0].to_commod_command(), "choose 0");
}
