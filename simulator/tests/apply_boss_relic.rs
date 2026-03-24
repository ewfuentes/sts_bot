use sts_simulator::{Action, GameState, Screen};

fn make_boss_relic_state() -> GameState {
    let json = serde_json::json!({
        "hp": 50, "max_hp": 80, "gold": 10, "floor": 15, "act": 1, "ascension": 0,
        "deck": [
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
        ],
        "relics": [],
        "potions": [],
        "actions": [],
        "screen": {
            "type": "boss_relic",
            "relics": [
                {"id": "BGBlackStar", "name": "Black Star", "counter": -1, "clickable": false, "pulsing": false},
                {"id": "BGSneckoEye", "name": "Snecko Eye", "counter": -1, "clickable": false, "pulsing": false},
                {"id": "BGTinyHouse", "name": "Tiny House", "counter": -1, "clickable": false, "pulsing": false},
            ]
        }
    });
    GameState::from_json(&json.to_string()).unwrap()
}

#[test]
fn boss_relic_available_actions() {
    let state = make_boss_relic_state();
    let actions = state.available_actions();
    // 3 pick options + 1 skip
    assert_eq!(actions.len(), 4);
    assert!(matches!(&actions[0], Action::PickBossRelic { choice_index: 0 }));
    assert!(matches!(&actions[1], Action::PickBossRelic { choice_index: 1 }));
    assert!(matches!(&actions[2], Action::PickBossRelic { choice_index: 2 }));
    assert!(matches!(&actions[3], Action::SkipBossRelic));
}

#[test]
fn pick_boss_relic_adds_to_relics() {
    let mut state = make_boss_relic_state();
    state.apply(&Action::PickBossRelic { choice_index: 1 });

    assert_eq!(state.relics.len(), 1);
    assert_eq!(state.relics[0].id, "BGSneckoEye");
    assert!(matches!(state.screen, Screen::Complete));
}

#[test]
fn skip_boss_relic() {
    let mut state = make_boss_relic_state();
    state.apply(&Action::SkipBossRelic);

    assert!(state.relics.is_empty());
    assert!(matches!(state.screen, Screen::Complete));
}
