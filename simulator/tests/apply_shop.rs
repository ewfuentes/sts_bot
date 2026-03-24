use sts_simulator::{Action, GameState, Screen};

fn make_shop_state(gold: u16) -> GameState {
    let json = serde_json::json!({
        "hp": 50, "max_hp": 80, "gold": gold, "floor": 5, "act": 1, "ascension": 0,
        "deck": [
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": false},
        ],
        "relics": [],
        "potions": [null, null],
        "actions": [],
        "screen": {
            "type": "shop",
            "cards": [
                {"id": "BGCarnage", "name": "Carnage", "cost": 2, "type": "ATTACK", "upgraded": false, "price": 10},
                {"id": "BGBludgeon", "name": "Bludgeon", "cost": 3, "type": "ATTACK", "upgraded": false, "price": 20},
            ],
            "relics": [
                {"id": "BGAkabeko", "name": "Akabeko", "price": 15},
            ],
            "potions": [
                {"id": "BGFirePotion", "name": "Fire Potion", "price": 5},
            ],
            "purge_cost": 7
        }
    });
    GameState::from_json(&json.to_string()).unwrap()
}

#[test]
fn shop_available_actions_filters_by_gold() {
    let state = make_shop_state(12);
    let actions = state.available_actions();
    // Can afford: Carnage (10), Fire Potion (5), Purge (7), Leave
    // Cannot afford: Bludgeon (20), Akabeko (15)
    let buy_cards: Vec<_> = actions.iter().filter(|a| matches!(a, Action::BuyCard { .. })).collect();
    let buy_relics: Vec<_> = actions.iter().filter(|a| matches!(a, Action::BuyRelic { .. })).collect();
    let buy_potions: Vec<_> = actions.iter().filter(|a| matches!(a, Action::BuyPotion { .. })).collect();
    let purges: Vec<_> = actions.iter().filter(|a| matches!(a, Action::Purge { .. })).collect();
    let leaves: Vec<_> = actions.iter().filter(|a| matches!(a, Action::LeaveShop)).collect();

    assert_eq!(buy_cards.len(), 1); // Carnage only
    assert_eq!(buy_relics.len(), 0);
    assert_eq!(buy_potions.len(), 1);
    assert_eq!(purges.len(), 1);
    assert_eq!(leaves.len(), 1);
}

#[test]
fn shop_all_affordable_with_enough_gold() {
    let state = make_shop_state(100);
    let actions = state.available_actions();
    // Everything affordable + leave
    let count = actions.len();
    // 2 cards + 1 relic + 1 potion + 1 purge + 1 leave = 6
    assert_eq!(count, 6);
}

#[test]
fn buy_card_deducts_gold_and_adds_to_deck() {
    let mut state = make_shop_state(100);
    let action = state.available_actions().into_iter()
        .find(|a| matches!(a, Action::BuyCard { card, .. } if card.id == "BGCarnage"))
        .unwrap();
    state.apply(&action);

    assert_eq!(state.gold, 90);
    assert_eq!(state.deck.len(), 3);
    assert!(state.deck.iter().any(|c| c.id == "BGCarnage"));
    // Card should be removed from shop
    if let Screen::Shop { cards, .. } = &state.screen {
        assert_eq!(cards.len(), 1); // only Bludgeon remains
    } else {
        panic!("Expected Shop screen");
    }
}

#[test]
fn buy_relic_adds_to_relics() {
    let mut state = make_shop_state(100);
    let action = state.available_actions().into_iter()
        .find(|a| matches!(a, Action::BuyRelic { .. }))
        .unwrap();
    state.apply(&action);

    assert_eq!(state.gold, 85);
    assert!(state.relics.iter().any(|r| r.id == "BGAkabeko"));
    if let Screen::Shop { relics, .. } = &state.screen {
        assert!(relics.is_empty());
    } else {
        panic!("Expected Shop screen");
    }
}

#[test]
fn buy_potion_adds_to_slot() {
    let mut state = make_shop_state(100);
    let action = state.available_actions().into_iter()
        .find(|a| matches!(a, Action::BuyPotion { .. }))
        .unwrap();
    state.apply(&action);

    assert_eq!(state.gold, 95);
    assert!(state.potions.iter().any(|p| p.as_ref().map_or(false, |p| p.id == "BGFirePotion")));
}

#[test]
fn purge_opens_grid() {
    let mut state = make_shop_state(100);
    let action = state.available_actions().into_iter()
        .find(|a| matches!(a, Action::Purge { .. }))
        .unwrap();
    state.apply(&action);

    assert_eq!(state.gold, 93);
    match &state.screen {
        Screen::Grid { purpose, cards } => {
            assert_eq!(purpose, "purge");
            assert_eq!(cards.len(), 2);
        }
        other => panic!("Expected Grid screen, got {:?}", other),
    }
}

#[test]
fn leave_shop_completes() {
    let mut state = make_shop_state(100);
    state.apply(&Action::LeaveShop);
    assert!(matches!(state.screen, Screen::Complete));
    assert_eq!(state.gold, 100); // no gold deducted
}

#[test]
fn cannot_buy_if_too_poor() {
    let state = make_shop_state(3);
    let actions = state.available_actions();
    // Only leave should be available (nothing affordable)
    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0], Action::LeaveShop));
}
