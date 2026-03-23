use sts_simulator::{Action, GameState};

fn make_neow_state(reward_type: &str, drawback: &str) -> GameState {
    let json = format!(r#"{{
        "hp": 10,
        "max_hp": 10,
        "gold": 5,
        "floor": 0,
        "act": 1,
        "ascension": 0,
        "deck": [],
        "relics": [],
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
    GameState::from_json(&json).unwrap()
}

fn pick_first(state: &GameState) -> Action {
    state.available_actions().into_iter().next().unwrap()
}

#[test]
fn neow_four_gold() {
    let mut state = make_neow_state("FOUR_GOLD", "NONE");
    let action = pick_first(&state);
    state.apply(&action);
    assert_eq!(state.gold, 9); // 5 + 4
}

#[test]
fn neow_five_gold() {
    let mut state = make_neow_state("FIVE_GOLD", "NONE");
    let action = pick_first(&state);
    state.apply(&action);
    assert_eq!(state.gold, 10); // 5 + 5
}

#[test]
fn neow_ten_gold() {
    let mut state = make_neow_state("TEN_GOLD", "NONE");
    let action = pick_first(&state);
    state.apply(&action);
    assert_eq!(state.gold, 15); // 5 + 10
}

#[test]
fn neow_drawback_lose_hp() {
    let mut state = make_neow_state("FOUR_GOLD", "LOSE_HP");
    let action = pick_first(&state);
    state.apply(&action);
    assert_eq!(state.hp, 8);   // 10 - 2
    assert_eq!(state.gold, 9); // 5 + 4
}

#[test]
fn neow_drawback_lose_3_hp() {
    let mut state = make_neow_state("FIVE_GOLD", "LOSE_3_HP");
    let action = pick_first(&state);
    state.apply(&action);
    assert_eq!(state.hp, 7);    // 10 - 3
    assert_eq!(state.gold, 10); // 5 + 5
}

#[test]
fn neow_drawback_lose_gold() {
    let mut state = make_neow_state("EIGHT_GOLD", "LOSE_GOLD");
    let action = pick_first(&state);
    state.apply(&action);
    assert_eq!(state.gold, 10); // 5 - 3 + 8
}

#[test]
fn neow_card_gold_combo() {
    let mut state = make_neow_state("CARD_GOLD_COMBO", "NONE");
    let action = pick_first(&state);
    state.apply(&action);
    assert_eq!(state.gold, 10); // 5 + 5
}
