use std::collections::HashMap;
use std::time::Instant;

use mcts::GameState as MctsGameState;
use rand::SeedableRng;
use rayon::prelude::*;
use sts_simulator::{
    GameState as StsGameState, Monster, MonsterState, Screen,
    encounter_db, monster_db,
    mcts_adapter::{StsState, StsRandomEvaluator},
};

fn make_combat(seed: u64, encounter_id: &str) -> StsGameState {
    let json = serde_json::json!({
        "hp": 8, "max_hp": 8, "gold": 5, "floor": 1, "act": 1, "ascension": 0,
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
        ],
        "relics": [
            {"id": "BoardGame:BurningBlood", "name": "Burning Blood", "counter": -1},
        ],
        "potions": [null, null],
        "screen": {
            "type": "combat",
            "encounter": encounter_id,
        }
    });

    let mut state = StsGameState::from_json(&json.to_string()).unwrap();

    // Populate monsters from encounter_db
    if let Some(enc) = encounter_db::lookup(encounter_id) {
        if let Screen::Combat { monsters, .. } = state.current_screen_mut() {
            for em in enc.monsters {
                monsters.push(Monster {
                    id: em.id.to_string(),
                    name: em.id.to_string(),
                    hp: em.hp,
                    max_hp: em.hp,
                    block: 0,
                    intent: "UNKNOWN".to_string(),
                    damage: None,
                    hits: 1,
                    powers: vec![],
                    state: MonsterState::Alive,
                    move_index: em.move_index,
                    pattern: monster_db::MovePattern::default(),
                });
            }
        }
    }

    // Seed the combat RNG so different seeds produce different games
    if let Screen::Combat { rng, .. } = state.current_screen_mut() {
        *rng = sts_simulator::Rng::from_seed(seed);
    }
    state.determinize(seed);
    state.start_combat();
    state.apply_monster_starting_effects();
    state
}

fn main() {
    let encounter = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "BoardGame:Jaw Worm (Easy)".to_string());
    let num_iterations: u32 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);
    let seed: u64 = std::env::args()
        .nth(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);

    println!("MCTS combat vs {} ({} iterations, seed {})", encounter, num_iterations, seed);

    let inner_state = make_combat(seed, &encounter);
    let state = StsState::new(inner_state);

    // Show initial state
    if let Screen::Combat { monsters, hand, player_energy, die_roll, .. } = state.inner.current_screen() {
        println!("HP: {}/{}", state.inner.hp, state.inner.max_hp);
        println!("Energy: {}", player_energy);
        println!("Die roll: {:?}", die_roll);
        println!("Monsters:");
        for m in monsters {
            println!("  {} - HP: {}/{}, Intent: {} (dmg: {:?})", m.name, m.hp, m.max_hp, m.intent, m.damage);
        }
        println!("Hand ({} cards):", hand.len());
        for hc in hand {
            println!("  {} (cost {})", hc.card.name, hc.card.cost);
        }
    }

    let num_roots = 10u64;
    let config = mcts::MctsConfig {
        num_iterations,
        exploration_constant: 1.41,
    };

    // Play out the combat using MCTS with root parallelism
    let mut current = state;
    let mut turn = 0;

    while !current.is_terminal() {
        let actions = current.available_actions();
        if actions.is_empty() {
            println!("No actions available!");
            break;
        }

        let start = Instant::now();

        // Run parallel searches with different seeds, each re-determinizing
        let trees: Vec<_> = (0..num_roots).into_par_iter().map(|root_seed| {
            let mut root_state = current.clone();
            // Re-determinize with a different seed per root
            root_state.inner.determinize(seed.wrapping_add(root_seed).wrapping_add(turn as u64 * 1000));
            let evaluator = StsRandomEvaluator;
            let mut rng = rand::rngs::SmallRng::seed_from_u64(seed.wrapping_add(root_seed));
            mcts::search(&config, &root_state, &evaluator, &mut rng)
        }).collect();

        // Aggregate visit counts across all trees
        let mut aggregated: HashMap<String, (u32, f64)> = HashMap::new();
        for tree in &trees {
            for (action, visits) in tree.root_action_visits() {
                let key = format!("{:?}", action);
                let entry = aggregated.entry(key).or_insert((0, 0.0));
                entry.0 += visits;
                // Find value for this action
                if let Some(node) = tree.nodes.iter().find(|n| n.action.as_ref() == Some(&action)) {
                    entry.1 += node.total_value;
                }
            }
        }

        // Find best action by total visits across all roots
        let best_key = aggregated.iter()
            .max_by_key(|(_, (v, _))| *v)
            .map(|(k, _)| k.clone())
            .unwrap();

        // Find the actual Action object from the first tree that has it
        let best = trees[0].root_action_visits().into_iter()
            .find(|(a, _)| format!("{:?}", a) == best_key)
            .or_else(|| {
                // Try other trees if first doesn't have it
                trees.iter().flat_map(|t| t.root_action_visits()).find(|(a, _)| format!("{:?}", a) == best_key)
            })
            .map(|(a, _)| a)
            .unwrap();

        let elapsed = start.elapsed();
        let total_nodes: usize = trees.iter().map(|t| t.nodes.len()).sum();

        // Print the decision
        println!("\n--- Turn {} ---", turn);
        println!("Screen: {:?}", std::mem::discriminant(current.inner.current_screen()));
        println!("Search took {:.1}ms ({} total nodes across {} roots)",
            elapsed.as_secs_f64() * 1000.0, total_nodes, num_roots);
        println!("Action visits (aggregated):");
        let mut sorted: Vec<_> = aggregated.iter().collect();
        sorted.sort_by(|a, b| b.1.0.cmp(&a.1.0));
        for (key, (visits, value)) in &sorted {
            let avg_val = if *visits > 0 { value / *visits as f64 } else { 0.0 };
            let marker = if **key == best_key { " <<<" } else { "" };
            println!("  {:>5} visits (val {:.3})  {}{}", visits, avg_val, key, marker);
        }

        current.apply(&best);
        turn += 1;

        // Show combat state after action
        if let Screen::Combat { monsters, hand, player_block, player_energy, .. } = current.inner.current_screen() {
            println!("HP: {}/{} Block: {}", current.inner.hp, current.inner.max_hp, player_block);
            for m in monsters {
                if m.state == MonsterState::Alive {
                    println!("  {} HP: {}/{} Block: {}", m.name, m.hp, m.max_hp, m.block);
                }
            }
            println!("Energy: {} Hand: {}", player_energy, hand.len());
        }
    }

    // Final result
    println!("\n=== RESULT ===");
    match current.inner.current_screen() {
        Screen::GameOver { victory: true } => println!("VICTORY! HP: {}/{}", current.inner.hp, current.inner.max_hp),
        Screen::GameOver { victory: false } => println!("DEFEAT"),
        other => println!("Ended on screen: {:?}", std::mem::discriminant(other)),
    }
}
