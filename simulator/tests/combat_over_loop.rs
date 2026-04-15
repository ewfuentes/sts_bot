/// Regression test: CombatOver re-triggers infinitely when Distilled Chaos
/// auto-plays a card that kills the last monster.

use sts_simulator::{GameState, Action};

#[test]
fn distilled_chaos_kill_does_not_loop() {
    let json = serde_json::json!({
        "hp": 8, "max_hp": 8, "gold": 5, "floor": 1, "act": 1, "ascension": 0,
        "deck": [
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
        ],
        "relics": [
            {"id": "BoardGame:BurningBlood", "name": "Burning Blood", "counter": -1},
        ],
        "potions": [
            {"id": "BoardGame:BGDistilledChaos", "name": "Distilled Chaos"},
            null,
        ],
        "screen": {
            "type": "combat",
            "encounter": "BoardGame:Test",
        }
    });

    let mut state = GameState::from_json(&json.to_string()).unwrap();

    // Populate two 1-HP monsters
    if let sts_simulator::Screen::Combat { monsters, rng, .. } = state.current_screen_mut() {
        *rng = sts_simulator::Rng::from_seed(42);
        monsters.push(sts_simulator::Monster {
            id: "BGGremlinSneaky".to_string(),
            name: "BGGremlinSneaky".to_string(),
            hp: 1, max_hp: 1, block: 0,
            intent: "ATTACK".to_string(),
            damage: Some(1), hits: 1,
            powers: vec![],
            state: sts_simulator::MonsterState::Alive,
            move_index: 0,
            pattern: sts_simulator::monster_db::MovePattern::default(),
        });
        monsters.push(sts_simulator::Monster {
            id: "BGGremlinSneaky".to_string(),
            name: "BGGremlinSneaky".to_string(),
            hp: 1, max_hp: 1, block: 0,
            intent: "ATTACK".to_string(),
            damage: Some(1), hits: 1,
            powers: vec![],
            state: sts_simulator::MonsterState::Alive,
            move_index: 0,
            pattern: sts_simulator::monster_db::MovePattern::default(),
        });
    }

    state.determinize(42);
    state.start_combat();
    state.apply_monster_starting_effects();

    // Use the Distilled Chaos potion
    let action: Action = serde_json::from_str(
        r#"{"type":"use_potion","slot":0,"label":"BoardGame:BGDistilledChaos"}"#
    ).unwrap();
    state.apply(&action);

    // Auto-play the drawn cards — pick whatever is offered until combat ends
    for _ in 0..10 {
        let actions = state.available_actions();
        if actions.is_empty() {
            break;
        }
        eprintln!("Screen: {:?}, actions: {:?}", std::mem::discriminant(state.current_screen()), actions);
        state.apply(&actions[0]);
    }

    eprintln!("Final screen: {:?}", std::mem::discriminant(state.current_screen()));
}
