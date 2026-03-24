use sts_simulator::{Action, GameState, Screen};

fn make_rest_state(hp: u16, max_hp: u16, deck: Vec<(&str, bool)>) -> GameState {
    let deck_json: Vec<serde_json::Value> = deck
        .iter()
        .map(|(id, upgraded)| {
            serde_json::json!({
                "id": id, "name": id, "cost": 1, "type": "ATTACK", "upgraded": upgraded,
            })
        })
        .collect();

    let json = serde_json::json!({
        "hp": hp, "max_hp": max_hp, "gold": 0, "floor": 5, "act": 1, "ascension": 0,
        "deck": deck_json, "relics": [], "potions": [], "actions": [],
        "screen": { "type": "rest", "options": ["rest", "smith"] }
    });
    GameState::from_json(&json.to_string()).unwrap()
}

#[test]
fn rest_available_actions() {
    let state = make_rest_state(50, 80, vec![("BGStrike_R", false)]);
    let actions = state.available_actions();
    assert_eq!(actions.len(), 2);
    assert!(matches!(&actions[0], Action::Rest { choice_index: 0 }));
    assert!(matches!(&actions[1], Action::Smith { choice_index: 1 }));
}

#[test]
fn rest_heals_30_percent() {
    let mut state = make_rest_state(50, 80, vec![("BGStrike_R", false)]);
    state.apply(&Action::Rest { choice_index: 0 });

    // 80 / 3 = 26 (integer division), 50 + 26 = 76
    assert_eq!(state.hp, 76);
    assert!(matches!(state.screen, Screen::Complete));
}

#[test]
fn rest_heal_capped_at_max_hp() {
    let mut state = make_rest_state(75, 80, vec![("BGStrike_R", false)]);
    state.apply(&Action::Rest { choice_index: 0 });

    // 80 / 3 = 26, 75 + 26 = 101 → capped at 80
    assert_eq!(state.hp, 80);
}

#[test]
fn smith_opens_upgrade_grid() {
    let mut state = make_rest_state(50, 80, vec![
        ("BGStrike_R", false),
        ("BGBash", false),
        ("BGDefend_R", true),  // already upgraded
    ]);
    state.apply(&Action::Smith { choice_index: 1 });

    match &state.screen {
        Screen::Grid { purpose, cards } => {
            assert_eq!(purpose, "upgrade");
            // Only non-upgraded cards should appear
            assert_eq!(cards.len(), 2);
            assert!(cards.iter().any(|c| c.id == "BGStrike_R"));
            assert!(cards.iter().any(|c| c.id == "BGBash"));
        }
        other => panic!("Expected Grid screen, got {:?}", other),
    }
}

#[test]
fn smith_then_upgrade_card() {
    let mut state = make_rest_state(50, 80, vec![
        ("BGStrike_R", false),
        ("BGBash", false),
    ]);
    state.apply(&Action::Smith { choice_index: 1 });

    // Pick BGBash to upgrade
    let bash_action = state
        .available_actions()
        .into_iter()
        .find(|a| matches!(a, Action::PickGridCard { card, .. } if card.id == "BGBash"))
        .unwrap();
    state.apply(&bash_action);

    let bash = state.deck.iter().find(|c| c.id == "BGBash").unwrap();
    assert!(bash.upgraded);
    assert!(!state.deck.iter().find(|c| c.id == "BGStrike_R").unwrap().upgraded);
    assert!(matches!(state.screen, Screen::Complete));
}
