use sts_simulator::{Action, Card, GameState, HandCard, Screen};

fn make_card(id: &str, cost: i8, card_type: &str) -> Card {
    Card {
        id: id.to_string(),
        name: id.to_string(),
        cost,
        card_type: card_type.to_string(),
        upgraded: false,
    }
}

fn make_hand_card(id: &str, cost: i8, card_type: &str) -> HandCard {
    HandCard {
        card: make_card(id, cost, card_type),
    }
}

fn combat_state(
    hand: Vec<HandCard>,
    draw_pile: Vec<Card>,
    discard_pile: Vec<Card>,
    exhaust_pile: Vec<Card>,
    energy: u8,
    block: u16,
) -> GameState {
    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": [{"id": "BGJawWorm", "name": "Jaw Worm", "hp": 8, "max_hp": 8,
                          "block": 0, "intent": "ATTACK", "damage": 3, "hits": 1,
                          "powers": [], "is_gone": false}],
            "hand": hand,
            "draw_pile": draw_pile,
            "discard_pile": discard_pile,
            "exhaust_pile": exhaust_pile,
            "player_block": block,
            "player_energy": energy,
            "player_powers": [],
            "turn": 1
        }
    });
    GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap()
}

// ── PlayCard tests ──

#[test]
fn play_card_deducts_energy_and_discards() {
    let hand = vec![
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let mut state = combat_state(hand, vec![], vec![], vec![], 3, 0);

    state.apply(&Action::PlayCard {
        card: make_card("BGStrike_R", 1, "ATTACK"),
        hand_index: 0,
        target_index: Some(0),
        target_name: Some("Jaw Worm".into()),
    });

    if let Screen::Combat { hand, discard_pile, player_energy, .. } = state.current_screen() {
        assert_eq!(hand.len(), 1);
        assert_eq!(hand[0].card.id, "BGDefend_R");
        assert_eq!(*player_energy, 2);
        assert_eq!(discard_pile.len(), 1);
        assert_eq!(discard_pile[0].id, "BGStrike_R");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn play_card_exhaust_goes_to_exhaust_pile() {
    let hand = vec![
        make_hand_card("BGOffering", 0, "SKILL"),
    ];
    let mut state = combat_state(hand, vec![], vec![], vec![], 3, 0);

    state.apply(&Action::PlayCard {
        card: make_card("BGOffering", 0, "SKILL"),
        hand_index: 0,
        target_index: None,
        target_name: None,
    });

    if let Screen::Combat { hand, discard_pile, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(hand.len(), 0);
        assert!(discard_pile.is_empty());
        assert_eq!(exhaust_pile.len(), 1);
        assert_eq!(exhaust_pile[0].id, "BGOffering");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn play_power_card_is_consumed() {
    let hand = vec![
        make_hand_card("BGInflame", 2, "POWER"),
    ];
    let mut state = combat_state(hand, vec![], vec![], vec![], 3, 0);

    state.apply(&Action::PlayCard {
        card: make_card("BGInflame", 2, "POWER"),
        hand_index: 0,
        target_index: None,
        target_name: None,
    });

    if let Screen::Combat { hand, discard_pile, exhaust_pile, player_energy, .. } = state.current_screen() {
        assert_eq!(hand.len(), 0);
        assert!(discard_pile.is_empty()); // Power not in discard
        assert!(exhaust_pile.is_empty()); // Power not in exhaust
        assert_eq!(*player_energy, 1); // 3 - 2 = 1
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn unaffordable_card_not_in_available_actions() {
    let hand = vec![
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGBash", 2, "ATTACK"), // costs 2
    ];
    let mut state = combat_state(hand, vec![], vec![], vec![], 2, 0);

    // Play strike (cost 1), leaving 1 energy
    state.apply(&Action::PlayCard {
        card: make_card("BGStrike_R", 1, "ATTACK"),
        hand_index: 0,
        target_index: Some(0),
        target_name: Some("Jaw Worm".into()),
    });

    // Bash (cost 2) should not appear in available actions with only 1 energy
    let actions = state.available_actions();
    let has_bash = actions.iter().any(|a| matches!(a, Action::PlayCard { card, .. } if card.id == "BGBash"));
    assert!(!has_bash, "Bash should not be playable with 1 energy");
    // EndTurn should still be available
    assert!(actions.iter().any(|a| matches!(a, Action::EndTurn)));
}

// ── EndTurn tests ──

#[test]
fn end_turn_discards_hand_and_draws() {
    let hand = vec![
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let draw_pile = vec![
        make_card("BGBash", 2, "ATTACK"),
        make_card("BGStrike_R", 1, "ATTACK"),
        make_card("BGDefend_R", 1, "SKILL"),
        make_card("BGStrike_R", 1, "ATTACK"),
        make_card("BGDefend_R", 1, "SKILL"),
    ];
    let mut state = combat_state(hand, draw_pile, vec![], vec![], 1, 5);

    state.apply(&Action::EndTurn);

    if let Screen::Combat {
        hand, draw_pile, discard_pile, player_block, player_energy, turn, ..
    } = state.current_screen()
    {
        // Hand should be 5 cards drawn from draw pile
        assert_eq!(hand.len(), 5);
        // Old hand (2 cards) went to discard
        assert_eq!(discard_pile.len(), 2);
        // Draw pile emptied (had 5, drew 5)
        assert_eq!(draw_pile.len(), 0);
        // Block reset
        assert_eq!(*player_block, 0);
        // Energy refilled
        assert_eq!(*player_energy, 3);
        // Turn incremented
        assert_eq!(*turn, 2);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn end_turn_reshuffles_when_draw_pile_small() {
    let hand = vec![
        make_hand_card("BGStrike_R", 1, "ATTACK"),
    ];
    let draw_pile = vec![
        make_card("BGBash", 2, "ATTACK"),
        make_card("BGDefend_R", 1, "SKILL"),
    ];
    let discard_pile = vec![
        make_card("BGStrike_R", 1, "ATTACK"),
        make_card("BGStrike_R", 1, "ATTACK"),
        make_card("BGDefend_R", 1, "SKILL"),
    ];
    let mut state = combat_state(hand, draw_pile, discard_pile, vec![], 0, 0);

    state.apply(&Action::EndTurn);

    if let Screen::Combat { hand, draw_pile, discard_pile, .. } = state.current_screen() {
        // Should have drawn 5 total (2 from draw + reshuffle + 3 more)
        assert_eq!(hand.len(), 5);
        // 1 from old hand went to discard before reshuffle, but discard was reshuffled in
        // Total cards: 1 (old hand) + 2 (draw) + 3 (discard) = 6, drew 5, so 1 left
        assert_eq!(draw_pile.len() + discard_pile.len(), 1);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn end_turn_ethereal_cards_exhaust() {
    let hand = vec![
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("Dazed", -2, "STATUS"),  // ethereal status
    ];
    let draw_pile = vec![
        make_card("BGStrike_R", 1, "ATTACK"),
        make_card("BGStrike_R", 1, "ATTACK"),
        make_card("BGStrike_R", 1, "ATTACK"),
        make_card("BGStrike_R", 1, "ATTACK"),
        make_card("BGStrike_R", 1, "ATTACK"),
    ];
    let mut state = combat_state(hand, draw_pile, vec![], vec![], 3, 0);

    state.apply(&Action::EndTurn);

    if let Screen::Combat { discard_pile, exhaust_pile, .. } = state.current_screen() {
        // Strike went to discard, Dazed went to exhaust
        assert_eq!(discard_pile.len(), 1);
        assert_eq!(discard_pile[0].id, "BGStrike_R");
        assert_eq!(exhaust_pile.len(), 1);
        assert_eq!(exhaust_pile[0].id, "Dazed");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn end_turn_playability_via_available_actions() {
    let hand = vec![];
    let draw_pile = vec![
        make_card("BGBash", 2, "ATTACK"),      // cost 2, target: Enemy
        make_card("BGDefend_R", 1, "SKILL"),    // cost 1, target: Self
        make_card("BGStrike_R", 1, "ATTACK"),   // cost 1, target: Enemy
        make_card("Dazed", -2, "STATUS"),       // unplayable
        make_card("BGBludgeon", 3, "ATTACK"),   // cost 3, target: Enemy
    ];
    let mut state = combat_state(hand, draw_pile, vec![], vec![], 0, 0);

    state.apply(&Action::EndTurn);

    // After end turn, energy is 3. Check available actions.
    let actions = state.available_actions();

    // Dazed (cost -2) should not be playable
    let has_dazed = actions.iter().any(|a| matches!(a, Action::PlayCard { card, .. } if card.id == "Dazed"));
    assert!(!has_dazed, "Dazed should not be playable");

    // BGBash (cost 2) should be playable and target enemy
    let bash_actions: Vec<_> = actions.iter()
        .filter(|a| matches!(a, Action::PlayCard { card, .. } if card.id == "BGBash"))
        .collect();
    assert_eq!(bash_actions.len(), 1); // one target
    assert!(matches!(bash_actions[0], Action::PlayCard { target_index: Some(0), .. }));

    // BGDefend_R (cost 1) should be playable and not target
    let defend_actions: Vec<_> = actions.iter()
        .filter(|a| matches!(a, Action::PlayCard { card, .. } if card.id == "BGDefend_R"))
        .collect();
    assert_eq!(defend_actions.len(), 1);
    assert!(matches!(defend_actions[0], Action::PlayCard { target_index: None, .. }));

    // BGBludgeon (cost 3) should be playable
    let has_bludgeon = actions.iter().any(|a| matches!(a, Action::PlayCard { card, .. } if card.id == "BGBludgeon"));
    assert!(has_bludgeon);
}
