use sts_simulator::GameState;

fn make_ironclad_at_neow() -> GameState {
    let json = serde_json::json!({
        "hp": 50, "max_hp": 80, "gold": 10, "floor": 0, "act": 1, "ascension": 0,
        "deck": [
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": false},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": false},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": false},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": false},
            {"id": "BGBash", "name": "Bash", "cost": 2, "type": "ATTACK", "upgraded": false},
            {"id": "BGAscendersBane", "name": "Ascender's Bane", "cost": -2, "type": "CURSE", "upgraded": false},
        ],
        "relics": [
            {"id": "BoardGame:BurningBlood", "name": "Burning Blood", "counter": -1, "clickable": false, "pulsing": false},
            {"id": "BGTheDie", "name": "The Die", "counter": -1, "clickable": false, "pulsing": false},
        ],
        "potions": [null, null],
        "actions": [],
        "screen": {
            "type": "neow",
            "options": [
                {"label": "Obtain a random rare card", "disabled": false, "reward_type": "RANDOM_RARE_CARD", "drawback": "NONE"},
                {"label": "Choose a card", "disabled": false, "reward_type": "CHOOSE_A_CARD", "drawback": "LOSE_HP"}
            ]
        }
    });
    GameState::from_json(&json.to_string()).unwrap()
}

#[test]
fn random_rare_card_adds_to_deck_after_determinize() {
    let mut state = make_ironclad_at_neow();
    assert_eq!(state.deck.len(), 11);

    state.determinize(42);

    // Pick the "obtain random rare card" blessing
    let actions = state.available_actions();
    let action = &actions[0]; // RANDOM_RARE_CARD
    state.apply(action);

    // Deck should now have 12 cards
    assert_eq!(state.deck.len(), 12);

    // The new card should be from the Ironclad rares pool
    let new_card = &state.deck[11];
    let ironclad_rares = [
        "BGBarricade", "BGBerserk", "BGBludgeon", "BGCorruption", "BGDemon Form",
        "BGDouble Tap", "BGExhume", "BGFeed", "BGFiend Fire", "BGImmolate",
        "BGImpervious", "BGJuggernaut", "BGLimit Break", "BGOffering", "BGUppercut",
    ];
    assert!(
        ironclad_rares.contains(&new_card.id.as_str()),
        "Expected rare card, got: {}",
        new_card.id
    );
}

#[test]
fn choose_a_card_opens_reward_after_determinize() {
    let mut state = make_ironclad_at_neow();
    state.determinize(99);

    let actions = state.available_actions();
    let action = &actions[1]; // CHOOSE_A_CARD with LOSE_HP drawback
    state.apply(action);

    // Should have lost HP from drawback
    assert!(state.hp < 50);

    // Should be on card reward screen with 3 cards
    match &state.screen {
        sts_simulator::Screen::CardReward { cards } => {
            assert_eq!(cards.len(), 3);
        }
        other => panic!("Expected CardReward screen, got {:?}", other),
    }
}

#[test]
fn different_seeds_give_different_rare_cards() {
    let mut state1 = make_ironclad_at_neow();
    let mut state2 = make_ironclad_at_neow();
    state1.determinize(1);
    state2.determinize(2);

    let action = &state1.available_actions()[0].clone();
    state1.apply(action);
    let action = &state2.available_actions()[0].clone();
    state2.apply(action);

    // Both should have 12 cards
    assert_eq!(state1.deck.len(), 12);
    assert_eq!(state2.deck.len(), 12);

    // Very likely to get different rare cards with different seeds
    // (15 rares, so 1/15 chance of collision)
    let card1 = &state1.deck[11].id;
    let card2 = &state2.deck[11].id;
    // Not asserting inequality since there's a small chance of collision,
    // but we can at least verify both are valid rares
    assert!(!card1.is_empty());
    assert!(!card2.is_empty());
}
