use sts_simulator::{Action, GameState, Screen};

#[test]
fn take_card_adds_to_deck() {
    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 5, "floor": 1, "act": 1, "ascension": 0,
        "deck": [
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
        ],
        "relics": [], "potions": [], "actions": [],
        "screen": {
            "type": "card_reward",
            "cards": [
                {"id": "BGInflame", "name": "Inflame", "cost": 1, "type": "POWER", "upgraded": false},
                {"id": "BGPommelStrike", "name": "Pommel Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            ]
        }
    });
    let mut state = GameState::from_json(&json.to_string()).unwrap();
    assert_eq!(state.deck.len(), 1);

    let actions = state.available_actions();
    assert_eq!(actions.len(), 3); // 2 cards + skip

    // Take Inflame
    let take = actions.iter()
        .find(|a| matches!(a, Action::TakeCard { card, .. } if card.id == "BGInflame"))
        .unwrap()
        .clone();
    state.apply(&take);

    assert_eq!(state.deck.len(), 2);
    assert!(state.deck.iter().any(|c| c.id == "BGInflame"));
    assert!(matches!(state.current_screen(), Screen::Complete));
}

#[test]
fn skip_card_reward() {
    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 5, "floor": 1, "act": 1, "ascension": 0,
        "deck": [
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
        ],
        "relics": [], "potions": [], "actions": [],
        "screen": {
            "type": "card_reward",
            "cards": [
                {"id": "BGInflame", "name": "Inflame", "cost": 1, "type": "POWER", "upgraded": false},
            ]
        }
    });
    let mut state = GameState::from_json(&json.to_string()).unwrap();

    state.apply(&Action::SkipCardReward);

    assert_eq!(state.deck.len(), 1); // unchanged
    assert!(matches!(state.current_screen(), Screen::Complete));
}

#[test]
fn neow_choose_card_opens_card_reward() {
    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 5, "floor": 0, "act": 1, "ascension": 0,
        "deck": [], "relics": [], "potions": [], "actions": [],
        "screen": {
            "type": "event", "event_id": "", "event_name": "Neow",
            "options": [{"label": "test", "disabled": false, "reward_type": "CHOOSE_A_CARD", "drawback": "NONE"}]
        }
    });
    let mut state = GameState::from_json(&json.to_string()).unwrap();

    state.apply(&state.available_actions()[0].clone());

    assert!(matches!(state.current_screen(), Screen::CardReward { .. }));
}
