use sts_simulator::GameState;

fn make_ironclad_state() -> GameState {
    let json = serde_json::json!({
        "hp": 50, "max_hp": 80, "gold": 10, "floor": 1, "act": 1, "ascension": 0,
        "deck": [
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": false},
            {"id": "BGBash", "name": "Bash", "cost": 2, "type": "ATTACK", "upgraded": false},
        ],
        "relics": [
            {"id": "BoardGame:BurningBlood", "name": "Burning Blood", "counter": -1, "clickable": false, "pulsing": false},
        ],
        "potions": [null],
        "actions": [],
        "screen": {"type": "complete"}
    });
    GameState::from_json(&json.to_string()).unwrap()
}

#[test]
fn from_json_creates_unordered_pools() {
    let state = make_ironclad_state();
    let pools = state.reward_pools.as_ref().expect("pools should be populated");
    assert!(!pools.card_deck.is_ordered());
    assert!(!pools.rare_deck.is_ordered());
    assert!(!pools.relic_deck.is_ordered());
    // Card deck should not contain cards already in the player's deck
    let contents = pools.card_deck.contents();
    assert!(!contents.contains(&"BGBash"));
}

#[test]
fn from_json_removes_obtained_relics() {
    let state = make_ironclad_state();
    let pools = state.reward_pools.as_ref().unwrap();
    let relic_contents = pools.relic_deck.contents();
    assert!(!relic_contents.contains(&"BoardGame:BurningBlood"));
}

#[test]
fn determinize_makes_pools_ordered() {
    let mut state = make_ironclad_state();
    state.determinize(42);
    let pools = state.reward_pools.as_ref().unwrap();
    assert!(pools.card_deck.is_ordered());
    assert!(pools.rare_deck.is_ordered());
    assert!(pools.relic_deck.is_ordered());
    assert!(pools.potion_deck.is_ordered());
}

#[test]
fn determinize_then_draw() {
    let mut state = make_ironclad_state();
    state.determinize(42);
    let pools = state.reward_pools.as_mut().unwrap();
    let cards = pools.draw_card_reward(3);
    assert_eq!(cards.len(), 3);
    // Cards should have IDs from the ironclad pool
    for card in &cards {
        assert!(!card.id.is_empty());
    }
}

#[test]
fn different_seeds_produce_different_orders() {
    let mut state1 = make_ironclad_state();
    let mut state2 = make_ironclad_state();
    state1.determinize(1);
    state2.determinize(2);

    let pools1 = state1.reward_pools.as_mut().unwrap();
    let pools2 = state2.reward_pools.as_mut().unwrap();
    let cards1: Vec<String> = (0..5).filter_map(|_| pools1.card_deck.draw()).collect();
    let cards2: Vec<String> = (0..5).filter_map(|_| pools2.card_deck.draw()).collect();
    // Very unlikely to be the same with different seeds
    assert_ne!(cards1, cards2);
}
