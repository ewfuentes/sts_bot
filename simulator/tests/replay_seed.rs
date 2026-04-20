//! Replay a specific seed with MCTS random rollouts and log each step.
//! Run with: cargo test -p sts_simulator --test replay_seed -- --nocapture

use sts_simulator::mcts_adapter::{StsRandomEvaluator, StsState};
use sts_simulator::{Action, GameState, Screen};

use mcts::{GameState as MctsGameState, Evaluator, MctsTree};
use rand::SeedableRng;
use std::collections::HashMap;

fn replay_game(seed: u64, num_roots: usize, iterations_per_step: usize, max_steps: u64) {
    let inner = GameState::new_ironclad_game(seed);

    // Dump map nodes with encounter/seed info (skipped by serde, visible via Debug)
    if let Some(map) = &inner.map {
        for (i, node) in map.nodes.iter().enumerate() {
            eprintln!("node[{:>2}] {:?}", i, node);
        }
    }

    let mut base_state = StsState::new(inner);
    let exploration_constant = 1.41;

    for turn in 0..max_steps {
        if base_state.is_terminal() {
            let victory = matches!(base_state.inner.current_screen(), Screen::GameOver { victory: true });
            eprintln!(
                "GAME OVER at turn={} floor={} hp={}/{} victory={}",
                turn, base_state.inner.floor, base_state.inner.hp, base_state.inner.max_hp, victory,
            );
            return;
        }

        // Build root-parallel trees
        let mut trees: Vec<MctsTree<StsState>> = (0..num_roots)
            .map(|j| {
                let mut det = base_state.clone();
                det.inner.determinize(seed.wrapping_add(j as u64).wrapping_add(turn * 1000));
                MctsTree::new(det)
            })
            .collect();

        // Run random rollout iterations
        let evaluator = StsRandomEvaluator;
        for tree in trees.iter_mut() {
            let rng_seed = seed.wrapping_add(turn * 1000);
            let mut rng = rand::rngs::SmallRng::seed_from_u64(rng_seed);
            for _ in 0..iterations_per_step {
                let leaf_idx = tree.select_and_expand(exploration_constant);
                let leaf_node = &tree.nodes[leaf_idx];
                let val = if leaf_node.state.is_terminal() {
                    leaf_node.state.terminal_value()
                } else {
                    evaluator.evaluate(&leaf_node.state, &mut rng)
                };
                tree.backprop(leaf_idx, val);
            }
        }

        // Aggregate visits across roots
        let mut aggregated: HashMap<String, (u32, Action)> = HashMap::new();
        for tree in &trees {
            for (action, visits) in tree.root_action_visits() {
                let key = format!("{:?}", action);
                let entry = aggregated.entry(key).or_insert((0, action.clone()));
                entry.0 += visits;
            }
        }

        let (_, (_visits, best_action)) = aggregated
            .iter()
            .max_by_key(|(_, (v, _))| *v)
            .expect("no actions aggregated");

        let screen_info = match base_state.inner.current_screen() {
            Screen::Combat { monsters, player_block, player_energy, turn: combat_turn, .. } => {
                let m_str: Vec<String> = monsters.iter()
                    .map(|m| {
                        let str_amt = m.powers.iter().find(|p| p.id == "Strength").map(|p| p.amount).unwrap_or(0);
                        format!("{}(hp={}/{} blk={} str={} {:?})", m.id, m.hp, m.max_hp, m.block, str_amt, m.state)
                    })
                    .collect();
                format!(" block={} energy={} combat_turn={} monsters=[{}]",
                    player_block, player_energy, combat_turn, m_str.join(", "))
            }
            screen => format!(" screen={:?}", std::mem::discriminant(screen)),
        };
        eprintln!(
            "turn={:<4} floor={:<3} hp={}/{}{}  action={:?}",
            turn, base_state.inner.floor, base_state.inner.hp, base_state.inner.max_hp,
            screen_info, best_action,
        );

        base_state.inner.apply(best_action);
    }

    eprintln!(
        "TIMEOUT at turn={} floor={} hp={}/{}",
        max_steps, base_state.inner.floor, base_state.inner.hp, base_state.inner.max_hp,
    );
}

#[test]
#[ignore] // debug tool — run with: cargo test --release --test replay_seed -- --ignored --nocapture
fn dump_map_5020() {
    let inner = GameState::new_ironclad_game(5020);
    if let Some(map) = &inner.map {
        for (i, node) in map.nodes.iter().enumerate() {
            eprintln!("node[{:>2}] {:?}", i, node);
        }
    }
}

#[test]
#[ignore]
fn replay_seed_5020() {
    replay_game(5020, 10, 1000, 500);
}

#[test]
#[ignore]
fn replay_seed_560129() {
    replay_game(560129, 10, 20, 200);
}
