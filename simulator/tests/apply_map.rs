use sts_simulator::{Action, GameState, MapNodeKind, Screen};

fn make_map_state() -> GameState {
    let json = serde_json::json!({
        "hp": 50, "max_hp": 80, "gold": 10, "floor": 3, "act": 1, "ascension": 0,
        "deck": [
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
        ],
        "relics": [],
        "potions": [],
        "actions": [],
        "screen": {
            "type": "map",
            "available_nodes": [
                {"label": "x=1", "kind": "monster"},
                {"label": "x=3", "kind": "rest"},
                {"label": "x=5", "kind": "shop"},
            ]
        }
    });
    GameState::from_json(&json.to_string()).unwrap()
}

#[test]
fn travel_to_monster_advances_floor() {
    let mut state = make_map_state();
    assert_eq!(state.floor, 3);

    state.apply(&Action::TravelTo {
        kind: MapNodeKind::Monster,
        label: "x=1".to_string(),
        choice_index: 0,
    });

    assert_eq!(state.floor, 4);
    assert!(matches!(state.current_screen(), Screen::Combat { .. }));
}

#[test]
fn travel_to_rest() {
    let mut state = make_map_state();
    state.apply(&Action::TravelTo {
        kind: MapNodeKind::Rest,
        label: "x=3".to_string(),
        choice_index: 1,
    });

    assert_eq!(state.floor, 4);
    match state.current_screen() {
        Screen::Rest { options } => {
            assert!(options.contains(&"rest".to_string()));
            assert!(options.contains(&"smith".to_string()));
        }
        other => panic!("Expected Rest screen, got {:?}", other),
    }
}

#[test]
fn travel_to_shop() {
    let mut state = make_map_state();
    state.apply(&Action::TravelTo {
        kind: MapNodeKind::Shop,
        label: "x=5".to_string(),
        choice_index: 2,
    });

    assert_eq!(state.floor, 4);
    assert!(matches!(state.current_screen(), Screen::ShopRoom));
}

#[test]
fn travel_to_elite() {
    let mut state = make_map_state();
    state.apply(&Action::TravelTo {
        kind: MapNodeKind::Elite,
        label: "x=1".to_string(),
        choice_index: 0,
    });

    assert_eq!(state.floor, 4);
    assert!(matches!(state.current_screen(), Screen::Combat { .. }));
}

#[test]
fn travel_to_treasure() {
    let mut state = make_map_state();
    state.apply(&Action::TravelTo {
        kind: MapNodeKind::Treasure,
        label: "x=1".to_string(),
        choice_index: 0,
    });

    assert_eq!(state.floor, 4);
    assert!(matches!(state.current_screen(), Screen::Treasure));
}
