use sts_simulator::{Action, Card, GameState, Screen};

fn make_state_with_deck(cards: Vec<(&str, &str, bool)>) -> GameState {
    let deck: Vec<serde_json::Value> = cards
        .iter()
        .map(|(id, card_type, upgraded)| {
            serde_json::json!({
                "id": id,
                "name": id,
                "cost": 1,
                "type": card_type,
                "upgraded": upgraded,
            })
        })
        .collect();

    let json = serde_json::json!({
        "hp": 10,
        "max_hp": 10,
        "gold": 5,
        "floor": 0,
        "act": 1,
        "ascension": 0,
        "deck": deck,
        "relics": [],
        "potions": [],
        "actions": [],
        "screen": {
            "type": "event",
            "event_id": "",
            "event_name": "Neow",
            "options": [{
                "label": "test",
                "disabled": false,
                "reward_type": "REMOVE_CARD",
                "drawback": "NONE",
            }]
        }
    });
    GameState::from_json(&json.to_string()).unwrap()
}

#[test]
fn remove_card_opens_grid() {
    let mut state = make_state_with_deck(vec![
        ("BGStrike_R", "ATTACK", false),
        ("BGDefend_R", "SKILL", false),
        ("BGBash", "ATTACK", false),
    ]);
    assert_eq!(state.deck.len(), 3);

    let action = state.available_actions()[0].clone();
    state.apply(&action);

    match state.current_screen() {
        Screen::Grid { purpose, cards } => {
            assert_eq!(purpose, "purge");
            assert_eq!(cards.len(), 3);
        }
        other => panic!("Expected Grid screen, got {:?}", other),
    }
}

#[test]
fn remove_card_from_grid() {
    let mut state = make_state_with_deck(vec![
        ("BGStrike_R", "ATTACK", false),
        ("BGDefend_R", "SKILL", false),
        ("BGBash", "ATTACK", false),
    ]);

    // Pick REMOVE_CARD blessing
    let action = state.available_actions()[0].clone();
    state.apply(&action);

    // Now on Grid screen — pick BGDefend_R to remove
    let actions = state.available_actions();
    assert_eq!(actions.len(), 3);

    // Find the Defend action
    let defend_action = actions
        .iter()
        .find(|a| matches!(a, Action::PickGridCard { card, .. } if card.id == "BGDefend_R"))
        .unwrap()
        .clone();

    state.apply(&defend_action);

    // Defend should be removed from deck
    assert_eq!(state.deck.len(), 2);
    assert!(state.deck.iter().all(|c| c.id != "BGDefend_R"));
    assert!(matches!(state.current_screen(), Screen::Complete));
}

#[test]
fn upgrade_card_from_grid() {
    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 5, "floor": 0, "act": 1, "ascension": 0,
        "deck": [
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGBash", "name": "Bash", "cost": 2, "type": "ATTACK", "upgraded": false},
        ],
        "relics": [], "potions": [], "actions": [],
        "screen": {
            "type": "event", "event_id": "", "event_name": "Neow",
            "options": [{"label": "test", "disabled": false, "reward_type": "UPGRADE_CARD", "drawback": "NONE"}]
        }
    });
    let mut state = GameState::from_json(&json.to_string()).unwrap();

    // Pick UPGRADE_CARD blessing
    state.apply(&state.available_actions()[0].clone());

    match state.current_screen() {
        Screen::Grid { purpose, cards } => {
            assert_eq!(purpose, "upgrade");
            assert_eq!(cards.len(), 2);
        }
        other => panic!("Expected Grid screen, got {:?}", other),
    }

    // Pick Bash to upgrade
    let bash_action = state
        .available_actions()
        .into_iter()
        .find(|a| matches!(a, Action::PickGridCard { card, .. } if card.id == "BGBash"))
        .unwrap();

    state.apply(&bash_action);

    // Bash should be upgraded in deck
    let bash = state.deck.iter().find(|c| c.id == "BGBash").unwrap();
    assert!(bash.upgraded);
    assert!(!state.deck.iter().find(|c| c.id == "BGStrike_R").unwrap().upgraded);
    assert!(matches!(state.current_screen(), Screen::Complete));
}

#[test]
fn transform_card_removes_from_deck() {
    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 5, "floor": 0, "act": 1, "ascension": 0,
        "deck": [
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": false},
        ],
        "relics": [], "potions": [], "actions": [],
        "screen": {
            "type": "event", "event_id": "", "event_name": "Neow",
            "options": [{"label": "test", "disabled": false, "reward_type": "TRANSFORM_CARD", "drawback": "NONE"}]
        }
    });
    let mut state = GameState::from_json(&json.to_string()).unwrap();

    state.apply(&state.available_actions()[0].clone());

    match state.current_screen() {
        Screen::Grid { purpose, .. } => assert_eq!(purpose, "transform"),
        other => panic!("Expected Grid screen, got {:?}", other),
    }

    // Pick Strike to transform
    let strike_action = state
        .available_actions()
        .into_iter()
        .find(|a| matches!(a, Action::PickGridCard { card, .. } if card.id == "BGStrike_R"))
        .unwrap();

    state.apply(&strike_action);

    // Strike should be removed (replacement card not added yet — needs card pool)
    assert_eq!(state.deck.len(), 1);
    assert_eq!(state.deck[0].id, "BGDefend_R");
}
