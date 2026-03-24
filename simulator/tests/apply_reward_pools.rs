use sts_simulator::{GameState, RewardPools, Screen};
use sts_simulator::reward_deck::Character;

fn make_neow_state_with_pools(reward_type: &str, drawback: &str) -> GameState {
    let json = format!(r#"{{
        "hp": 10,
        "max_hp": 10,
        "gold": 5,
        "floor": 0,
        "act": 1,
        "ascension": 0,
        "deck": [
            {{"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false}},
            {{"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": false}},
            {{"id": "BGBash", "name": "Bash", "cost": 2, "type": "ATTACK", "upgraded": false}}
        ],
        "relics": [{{"id": "BurningBlood", "name": "Burning Blood"}}],
        "potions": [],
        "actions": [],
        "screen": {{
            "type": "event",
            "event_id": "",
            "event_name": "Neow",
            "options": [
                {{
                    "label": "test option",
                    "disabled": false,
                    "reward_type": "{}",
                    "drawback": "{}"
                }}
            ]
        }}
    }}"#, reward_type, drawback);
    let mut state = GameState::from_json(&json).unwrap();
    state.reward_pools = Some(
        RewardPools::new(Character::Ironclad, 42)
    );
    state
}

#[test]
fn choose_a_card_opens_reward_with_3_cards() {
    let mut state = make_neow_state_with_pools("CHOOSE_A_CARD", "NONE");
    let action = state.available_actions()[0].clone();
    state.apply(&action);

    match state.current_screen() {
        Screen::CardReward { cards } => {
            assert_eq!(cards.len(), 3);
            // Cards should have IDs from the ironclad pool
            for card in cards {
                assert!(!card.id.is_empty());
            }
        }
        other => panic!("Expected CardReward, got {:?}", other),
    }
}

#[test]
fn choose_a_card_then_take() {
    let mut state = make_neow_state_with_pools("CHOOSE_A_CARD", "NONE");
    state.apply(&state.available_actions()[0].clone());

    let initial_deck = state.deck.len();
    let actions = state.available_actions();
    // Should have 3 TakeCard + 1 SkipCardReward
    assert_eq!(actions.len(), 4);

    // Take the first card
    state.apply(&actions[0].clone());
    assert_eq!(state.deck.len(), initial_deck + 1);
    assert!(matches!(state.current_screen(), Screen::Complete));
}

#[test]
fn relic_blessing_adds_relic() {
    let mut state = make_neow_state_with_pools("RELIC", "NONE");
    assert_eq!(state.relics.len(), 1);

    state.apply(&state.available_actions()[0].clone());
    assert_eq!(state.relics.len(), 2);
}

#[test]
fn random_rare_card_adds_to_deck() {
    let mut state = make_neow_state_with_pools("RANDOM_RARE_CARD", "NONE");
    let initial_deck = state.deck.len();

    state.apply(&state.available_actions()[0].clone());
    assert_eq!(state.deck.len(), initial_deck + 1);
}

#[test]
fn get_two_random_cards_adds_to_deck() {
    let mut state = make_neow_state_with_pools("GET_TWO_RANDOM_CARDS", "NONE");
    let initial_deck = state.deck.len();

    state.apply(&state.available_actions()[0].clone());
    assert_eq!(state.deck.len(), initial_deck + 2);
}

#[test]
fn upgrade_two_random() {
    let mut state = make_neow_state_with_pools("UPGRADE_TWO_RANDOM", "NONE");
    let upgradeable_before = state.deck.iter().filter(|c| !c.upgraded).count();

    state.apply(&state.available_actions()[0].clone());

    let upgradeable_after = state.deck.iter().filter(|c| !c.upgraded).count();
    assert_eq!(upgradeable_before - upgradeable_after, 2);
}

#[test]
fn three_potions_opens_combat_rewards() {
    let mut state = make_neow_state_with_pools("THREE_POTIONS", "NONE");
    state.potions = vec![None, None, None]; // 3 empty potion slots

    state.apply(&state.available_actions()[0].clone());

    match state.current_screen() {
        Screen::CombatRewards { rewards } => {
            assert_eq!(rewards.len(), 3);
            for r in rewards {
                assert_eq!(r.reward_type, "POTION");
                assert!(r.potion.is_some());
            }
        }
        other => panic!("Expected CombatRewards, got {:?}", other),
    }

    // Take first potion
    let actions = state.available_actions();
    assert_eq!(actions.len(), 4); // 3 potions + proceed
    state.apply(&actions[0].clone());

    // Should have 1 potion in slot, 2 rewards remaining
    assert!(state.potions[0].is_some());
    match state.current_screen() {
        Screen::CombatRewards { rewards } => assert_eq!(rewards.len(), 2),
        other => panic!("Expected CombatRewards with 2 remaining, got {:?}", other),
    }

    // Proceed to skip remaining
    state.apply(&sts_simulator::Action::Proceed);
    assert!(matches!(state.current_screen(), Screen::Complete));
}

#[test]
fn curse_drawback_adds_curse() {
    let mut state = make_neow_state_with_pools("FOUR_GOLD", "CURSE");
    let initial_deck = state.deck.len();

    state.apply(&state.available_actions()[0].clone());
    assert_eq!(state.deck.len(), initial_deck + 1);
    let curse = state.deck.last().unwrap();
    assert_eq!(curse.card_type, "CURSE");
}

#[test]
fn colorless_card_draws_from_colorless_deck() {
    let mut state = make_neow_state_with_pools("CHOOSE_COLORLESS_CARD", "NONE");
    state.apply(&state.available_actions()[0].clone());

    match state.current_screen() {
        Screen::CardReward { cards } => {
            assert_eq!(cards.len(), 3);
        }
        other => panic!("Expected CardReward, got {:?}", other),
    }
}
