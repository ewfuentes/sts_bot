use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pyo3::types::PyDict;

use crate::action::Action;
use crate::mcts_adapter::{StsState, make_combat_state};
use crate::screen::Screen;
use crate::state::GameState;

use mcts::{GameState as MctsGameState, MctsTree};

#[pyclass(name = "GameState")]
#[derive(Clone)]
pub struct PyGameState {
    pub(crate) inner: GameState,
}

#[pymethods]
impl PyGameState {
    /// Create a GameState from a JSON string (translator output format).
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner = GameState::from_json(json)
            .map_err(|e| PyValueError::new_err(format!("Invalid JSON: {e}")))?;
        Ok(PyGameState { inner })
    }

    /// Apply an action (JSON string) to advance the state.
    fn apply(&mut self, action_json: &str) -> PyResult<()> {
        let action: Action = serde_json::from_str(action_json)
            .map_err(|e| PyValueError::new_err(format!("Invalid action JSON: {e}")))?;
        self.inner.apply(&action);
        Ok(())
    }

    /// Return the current state as a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {e}")))
    }

    /// Return available actions as a JSON array string.
    fn available_actions_json(&self) -> PyResult<String> {
        let actions = self.inner.available_actions();
        serde_json::to_string(&actions)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {e}")))
    }

    /// Determinize all unordered pools by shuffling with the given seed.
    /// After this, pool-dependent actions (card rewards, relics, etc.) can be simulated.
    fn determinize(&mut self, seed: u64) {
        self.inner.determinize(seed);
    }

    /// Return a dict of key state fields for quick comparison.
    fn summary(&self) -> PyResult<String> {
        let s = &self.inner;
        let summary = serde_json::json!({
            "hp": s.hp,
            "max_hp": s.max_hp,
            "gold": s.gold,
            "floor": s.floor,
            "act": s.act,
            "deck": s.deck,
            "relics": s.relics,
            "potions": s.potions,
            "screen": s.current_screen().to_summary_json(),
        });
        serde_json::to_string(&summary)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {e}")))
    }
}

// --- MctsWorker: batched MCTS for Python-orchestrated training ---

/// Raw terminal outcome — no feature engineering, just the facts.
#[derive(Clone)]
struct TerminalResult {
    victory: bool,
    hp: u16,
    max_hp: u16,
    floor: u8,
    timed_out: bool,
}

/// One game session: base state + N root-parallel trees + trajectory recording.
struct GameSession {
    base_state: StsState,
    roots: Vec<MctsTree<StsState>>,
    /// (state_before_action, action_taken). Terminal state has action=None.
    trajectory: Vec<(GameState, Option<Action>)>,
    seed: u64,
    turn: u64,
    /// None = in progress, Some = terminal.
    result: Option<TerminalResult>,
}

/// Tracks a pending leaf awaiting a neural network value.
struct PendingLeaf {
    game_idx: usize,
    root_idx: usize,
    leaf_node_idx: usize,
}

impl GameSession {
    fn build_trees(base_state: &StsState, num_roots: usize, seed: u64, turn: u64) -> Vec<MctsTree<StsState>> {
        (0..num_roots)
            .map(|j| {
                let mut det_state = base_state.clone();
                det_state.inner.determinize(seed.wrapping_add(j as u64).wrapping_add(turn * 1000));
                MctsTree::new(det_state)
            })
            .collect()
    }
}

#[pyclass]
struct MctsWorker {
    sessions: Vec<GameSession>,
    pending_leaves: Vec<PendingLeaf>,
    num_roots: usize,
    exploration_constant: f64,
    max_steps: u64,
}

#[pymethods]
impl MctsWorker {
    #[new]
    #[pyo3(signature = (num_games, num_roots, exploration_constant, seed, combat_only=true, encounter=None, max_steps=500))]
    fn new(
        num_games: usize,
        num_roots: usize,
        exploration_constant: f64,
        seed: u64,
        combat_only: bool,
        encounter: Option<String>,
        max_steps: u64,
    ) -> PyResult<Self> {
        let mut sessions = Vec::with_capacity(num_games);
        for i in 0..num_games {
            let seed_i = seed.wrapping_add(i as u64);
            let inner = if combat_only {
                let enc = encounter.as_deref().unwrap_or("BoardGame:Jaw Worm (Easy)");
                make_combat_state(seed_i, enc)
            } else {
                GameState::new_ironclad_game(seed_i)
            };
            let base_state = StsState::new(inner.clone());
            let roots = GameSession::build_trees(&base_state, num_roots, seed_i, 0);
            sessions.push(GameSession {
                base_state,
                roots,
                trajectory: vec![(inner, None)], // initial state, no action yet
                seed: seed_i,
                turn: 0,
                result: None,
            });
        }
        Ok(MctsWorker {
            sessions,
            pending_leaves: Vec::new(),
            num_roots,
            exploration_constant,
            max_steps,
        })
    }

    /// Select and expand one leaf per tree for all active games.
    /// Terminal leaves are backpropped immediately in Rust.
    /// Returns non-terminal leaf states (1:1 with internal pending_leaves).
    fn select_leaves(&mut self) -> Vec<PyGameState> {
        self.pending_leaves.clear();
        let mut leaf_states = Vec::new();

        for (game_idx, session) in self.sessions.iter_mut().enumerate() {
            if session.result.is_some() {
                continue;
            }
            for (root_idx, tree) in session.roots.iter_mut().enumerate() {
                let leaf_idx = tree.select_and_expand(self.exploration_constant);
                let leaf_node = &tree.nodes[leaf_idx];

                if leaf_node.state.is_terminal() {
                    let val = leaf_node.state.terminal_value();
                    tree.backprop(leaf_idx, val);
                } else {
                    self.pending_leaves.push(PendingLeaf {
                        game_idx,
                        root_idx,
                        leaf_node_idx: leaf_idx,
                    });
                    leaf_states.push(PyGameState {
                        inner: leaf_node.state.inner.clone(),
                    });
                }
            }
        }
        leaf_states
    }

    /// Run random rollouts for all active games, parallelized across trees.
    /// Does num_iterations iterations per tree entirely in Rust.
    fn run_random_rollouts(&mut self, num_iterations: usize) {
        use rayon::prelude::*;

        let exploration_constant = self.exploration_constant;

        // Collect mutable refs to all (tree, seed) pairs for active games
        let tree_tasks: Vec<_> = self.sessions.iter_mut()
            .filter(|s| s.result.is_none())
            .flat_map(|session| {
                let seed = session.seed;
                let turn = session.turn;
                session.roots.iter_mut().enumerate().map(move |(root_idx, tree)| {
                    (tree, seed.wrapping_add(root_idx as u64).wrapping_add(turn * 1000))
                })
            })
            .collect();

        tree_tasks.into_par_iter().for_each(|(tree, rng_seed)| {
            use crate::mcts_adapter::StsRandomEvaluator;
            use mcts::Evaluator;
            use rand::SeedableRng;

            let evaluator = StsRandomEvaluator;
            let mut rng = rand::rngs::SmallRng::seed_from_u64(rng_seed);

            for _ in 0..num_iterations {
                let leaf_idx = tree.select_and_expand(exploration_constant);
                let leaf_node = &tree.nodes[leaf_idx];
                let val = if leaf_node.state.is_terminal() {
                    leaf_node.state.terminal_value()
                } else {
                    evaluator.evaluate(&leaf_node.state, &mut rng)
                };
                tree.backprop(leaf_idx, val);
            }
        });
    }

    /// Backprop neural network values into pending leaves.
    fn backprop(&mut self, values: Vec<f64>) -> PyResult<()> {
        if values.len() != self.pending_leaves.len() {
            return Err(PyValueError::new_err(format!(
                "Expected {} values, got {}",
                self.pending_leaves.len(),
                values.len()
            )));
        }
        for (pending, &value) in self.pending_leaves.iter().zip(values.iter()) {
            let tree = &mut self.sessions[pending.game_idx].roots[pending.root_idx];
            tree.backprop(pending.leaf_node_idx, value);
        }
        self.pending_leaves.clear();
        Ok(())
    }

    /// Aggregate visits, pick best action, apply it, rebuild trees.
    /// Returns a list of dicts with info about each stepped game.
    fn step_games<'py>(&mut self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
        use std::collections::HashMap;

        let mut results = Vec::new();

        for session in self.sessions.iter_mut() {
            if session.result.is_some() {
                continue;
            }

            // Aggregate visit counts across all roots
            let mut aggregated: HashMap<String, (u32, Action)> = HashMap::new();
            for tree in &session.roots {
                for (action, visits) in tree.root_action_visits() {
                    let key = format!("{:?}", action);
                    let entry = aggregated.entry(key).or_insert((0, action.clone()));
                    entry.0 += visits;
                }
            }

            // Pick best action by total visits
            let (_, (visits, best_action)) = aggregated
                .iter()
                .max_by_key(|(_, (v, _))| *v)
                .unwrap_or_else(|| {
                    let screen = session.base_state.inner.current_screen();
                    let actions = session.base_state.inner.available_actions();
                    let state_json = serde_json::to_string(&session.base_state.inner).unwrap_or_default();
                    panic!(
                        "no actions aggregated in step_games\n\
                         screen: {:?}\n\
                         available_actions: {:?}\n\
                         turn: {}, seed: {}\n\
                         state_json: {}",
                        std::mem::discriminant(screen),
                        actions,
                        session.turn,
                        session.seed,
                        state_json,
                    );
                });
            let best_action = best_action.clone();
            let best_visits = *visits;

            // Record (state, action) in trajectory
            if let Some(last) = session.trajectory.last_mut() {
                last.1 = Some(best_action.clone());
            }

            // Apply action to base state
            session.base_state.inner.apply(&best_action);

            // Check terminal or step limit
            let finished = if session.base_state.is_terminal() {
                let victory = matches!(session.base_state.inner.current_screen(), Screen::GameOver { victory: true });
                session.result = Some(TerminalResult {
                    victory,
                    hp: session.base_state.inner.hp,
                    max_hp: session.base_state.inner.max_hp,
                    floor: session.base_state.inner.floor,
                    timed_out: false,
                });
                session.trajectory.push((session.base_state.inner.clone(), None));
                true
            } else if session.turn >= self.max_steps {
                eprintln!(
                    "TIMEOUT: seed={}, steps={}, floor={}, hp={}/{}",
                    session.seed, session.turn, session.base_state.inner.floor,
                    session.base_state.inner.hp, session.base_state.inner.max_hp,
                );
                session.result = Some(TerminalResult {
                    victory: false,
                    hp: session.base_state.inner.hp,
                    max_hp: session.base_state.inner.max_hp,
                    floor: session.base_state.inner.floor,
                    timed_out: true,
                });
                session.trajectory.push((session.base_state.inner.clone(), None));
                true
            } else {
                session.trajectory.push((session.base_state.inner.clone(), None));
                session.turn += 1;
                session.roots = GameSession::build_trees(
                    &session.base_state,
                    self.num_roots,
                    session.seed,
                    session.turn,
                );
                false
            };

            let dict = PyDict::new(py);
            let action_json = serde_json::to_string(&best_action)
                .map_err(|e| PyValueError::new_err(format!("Serialization error: {e}")))?;
            dict.set_item("action", action_json)?;
            dict.set_item("visits", best_visits)?;
            dict.set_item("finished", finished)?;
            results.push(dict);
        }

        Ok(results)
    }

    /// Return training data from finished games.
    /// Returns (states, actions, results) where results is a list of dicts
    /// with raw terminal info (one per game, not per trajectory step).
    fn get_training_data<'py>(&mut self, py: Python<'py>) -> PyResult<(
        Vec<PyGameState>,
        Vec<Option<String>>,
        Vec<Bound<'py, PyDict>>,
    )> {
        let mut states = Vec::new();
        let mut actions = Vec::new();
        let mut results = Vec::new();

        for session in &self.sessions {
            let result = match &session.result {
                Some(r) => r,
                None => continue,
            };

            let result_dict = PyDict::new(py);
            result_dict.set_item("victory", result.victory)?;
            result_dict.set_item("hp", result.hp)?;
            result_dict.set_item("max_hp", result.max_hp)?;
            result_dict.set_item("floor", result.floor)?;
            result_dict.set_item("timed_out", result.timed_out)?;
            result_dict.set_item("num_steps", session.trajectory.len())?;
            results.push(result_dict);

            for (state, action) in &session.trajectory {
                states.push(PyGameState { inner: state.clone() });
                actions.push(
                    action.as_ref().map(|a| serde_json::to_string(a).unwrap())
                );
            }
        }

        Ok((states, actions, results))
    }

    /// Count of games still in progress.
    fn active_game_count(&self) -> usize {
        self.sessions.iter().filter(|s| s.result.is_none()).count()
    }
}

// --- Batch token extraction for fast featurization ---

/// Pre-extracted token data for a batch of game states.
/// All variable-length fields are padded to the max length in the batch.
/// Padding uses 0 for indices and 0.0 for scalars.
#[pyclass]
#[derive(Clone)]
struct TokenData {
    /// Player scalars: (batch, 4) — [hp, max_hp, gold, floor]
    #[pyo3(get)]
    player_scalars: Vec<Vec<f32>>,
    /// Whether each state is in combat: (batch,)
    #[pyo3(get)]
    in_combat: Vec<bool>,
    /// Combat scalars: (batch, 4) — [block, energy, die_roll, turn]. 0 if not in combat.
    #[pyo3(get)]
    combat_scalars: Vec<Vec<f32>>,
    /// Deck card ID indices: (batch, max_deck). Padded with 0.
    #[pyo3(get)]
    deck_card_indices: Vec<Vec<u32>>,
    /// Deck lengths per state: (batch,)
    #[pyo3(get)]
    deck_lengths: Vec<u32>,
    /// Card pile card ID indices: (batch, max_pile_total). Padded with 0.
    #[pyo3(get)]
    pile_card_indices: Vec<Vec<u32>>,
    /// Card pile type indices (0=hand, 1=draw, 2=discard, 3=exhaust): (batch, max_pile_total)
    #[pyo3(get)]
    pile_type_indices: Vec<Vec<u32>>,
    /// Card pile total lengths per state: (batch,)
    #[pyo3(get)]
    pile_lengths: Vec<u32>,
    /// Number of alive monsters per state: (batch,)
    #[pyo3(get)]
    monster_counts: Vec<u32>,
    /// Monster ID indices: (batch, max_monsters). Padded with 0.
    #[pyo3(get)]
    monster_id_indices: Vec<Vec<u32>>,
    /// Monster position indices (0-3): (batch, max_monsters). Padded with 0.
    #[pyo3(get)]
    monster_position_indices: Vec<Vec<u32>>,
    /// Monster scalars: (batch, max_monsters, 5) — [hp, max_hp, block, damage, hits]. Padded with 0.
    #[pyo3(get)]
    monster_scalars: Vec<Vec<Vec<f32>>>,
    /// Monster intent indices: (batch, max_monsters). 0=UNKNOWN, 1=ATTACK, 2=ATTACK_BUFF, 3=BUFF.
    #[pyo3(get)]
    monster_intent_indices: Vec<Vec<u32>>,
    /// Player power ID indices: (batch, max_player_powers). Padded with 0.
    #[pyo3(get)]
    player_power_indices: Vec<Vec<u32>>,
    /// Player power amounts: (batch, max_player_powers). Padded with 0.
    #[pyo3(get)]
    player_power_amounts: Vec<Vec<f32>>,
    /// Player power counts: (batch,)
    #[pyo3(get)]
    player_power_counts: Vec<u32>,
    /// Monster power ID indices: (batch, max_total_monster_powers). Padded with 0.
    /// Flattened across all alive monsters per state.
    #[pyo3(get)]
    monster_power_indices: Vec<Vec<u32>>,
    /// Monster power amounts: (batch, max_total_monster_powers). Padded with 0.
    #[pyo3(get)]
    monster_power_amounts: Vec<Vec<f32>>,
    /// Monster position for each power token: (batch, max_total_monster_powers). Padded with 0.
    #[pyo3(get)]
    monster_power_positions: Vec<Vec<u32>>,
    /// Monster power total counts: (batch,)
    #[pyo3(get)]
    monster_power_counts: Vec<u32>,
}

/// Extract token data from a batch of GameStates.
/// card_ids, monster_ids, and power_ids define the index mappings (must match the embedding tables).
#[pyfunction]
fn extract_token_data(
    states: Vec<PyRef<PyGameState>>,
    card_ids: Vec<String>,
    monster_ids: Vec<String>,
    power_ids: Vec<String>,
) -> PyResult<TokenData> {
    use std::collections::HashMap;

    let card_id_map: HashMap<&str, u32> = card_ids.iter().enumerate()
        .map(|(i, s)| (s.as_str(), i as u32)).collect();
    let num_powers = power_ids.len();
    let power_id_map: HashMap<&str, u32> = power_ids.iter().enumerate()
        .map(|(i, s)| (s.as_str(), i as u32)).collect();
    let intent_map: HashMap<&str, u32> = [
        ("UNKNOWN", 0), ("ATTACK", 1), ("ATTACK_BUFF", 2), ("BUFF", 3),
    ].into_iter().collect();
    let monster_id_map: HashMap<&str, u32> = monster_ids.iter().enumerate()
        .map(|(i, s)| (s.as_str(), i as u32)).collect();

    let batch = states.len();
    let max_monsters = 4usize;

    let mut player_scalars = Vec::with_capacity(batch);
    let mut in_combat = Vec::with_capacity(batch);
    let mut combat_scalars = Vec::with_capacity(batch);
    let mut deck_cards: Vec<Vec<u32>> = Vec::with_capacity(batch);
    let mut pile_cards: Vec<Vec<u32>> = Vec::with_capacity(batch);
    let mut pile_types: Vec<Vec<u32>> = Vec::with_capacity(batch);
    let mut monster_counts = Vec::with_capacity(batch);
    let mut monster_ids_out: Vec<Vec<u32>> = Vec::with_capacity(batch);
    let mut monster_positions: Vec<Vec<u32>> = Vec::with_capacity(batch);
    let mut monster_scalar_out: Vec<Vec<Vec<f32>>> = Vec::with_capacity(batch);
    let mut monster_intents: Vec<Vec<u32>> = Vec::with_capacity(batch);
    let mut player_power_ids: Vec<Vec<u32>> = Vec::with_capacity(batch);
    let mut player_power_amts: Vec<Vec<f32>> = Vec::with_capacity(batch);
    let mut monster_power_ids: Vec<Vec<u32>> = Vec::with_capacity(batch);
    let mut monster_power_amts: Vec<Vec<f32>> = Vec::with_capacity(batch);
    let mut monster_power_pos: Vec<Vec<u32>> = Vec::with_capacity(batch);

    for state_ref in &states {
        let s = &state_ref.inner;

        // Player scalars
        player_scalars.push(vec![s.hp as f32, s.max_hp as f32, s.gold as f32, s.floor as f32]);

        // Deck
        let mut d = Vec::with_capacity(s.deck.len());
        for card in &s.deck {
            let idx = card_id_map.get(card.id.as_str()).copied()
                .ok_or_else(|| PyValueError::new_err(format!("Unknown card ID: {}", card.id)))?;
            d.push(idx);
        }
        deck_cards.push(d);

        // Screen-dependent data
        if let Screen::Combat {
            player_block, player_energy, die_roll, turn,
            hand, draw_pile, discard_pile, exhaust_pile,
            monsters, player_powers, ..
        } = s.current_screen() {
            in_combat.push(true);
            combat_scalars.push(vec![
                *player_block as f32,
                *player_energy as f32,
                die_roll.unwrap_or(0) as f32,
                *turn as f32,
            ]);

            // Card piles: hand=0, draw=1, discard=2, exhaust=3
            let mut pc = Vec::new();
            let mut pt = Vec::new();
            for (pile, type_idx) in [
                (hand.iter().map(|hc| &hc.card).collect::<Vec<_>>(), 0u32),
                (draw_pile.iter().collect::<Vec<_>>(), 1),
                (discard_pile.iter().collect::<Vec<_>>(), 2),
                (exhaust_pile.iter().collect::<Vec<_>>(), 3),
            ] {
                for card in pile {
                    let idx = card_id_map.get(card.id.as_str()).copied()
                        .ok_or_else(|| PyValueError::new_err(format!("Unknown card ID: {}", card.id)))?;
                    pc.push(idx);
                    pt.push(type_idx);
                }
            }
            pile_cards.push(pc);
            pile_types.push(pt);

            // Monsters
            let mut m_ids = Vec::new();
            let mut m_pos = Vec::new();
            let mut m_scalars = Vec::new();
            let mut m_intents = Vec::new();
            for (m_idx, m) in monsters.iter().enumerate().take(max_monsters) {
                if m.state != crate::types::MonsterState::Alive {
                    continue;
                }
                let mid = monster_id_map.get(m.id.as_str()).copied()
                    .ok_or_else(|| PyValueError::new_err(format!("Unknown monster ID: {}", m.id)))?;
                m_ids.push(mid);
                m_pos.push(m_idx as u32);
                m_scalars.push(vec![
                    m.hp as f32,
                    m.max_hp as f32,
                    m.block as f32,
                    m.damage.unwrap_or(0) as f32,
                    m.hits as f32,
                ]);
                m_intents.push(*intent_map.get(m.intent.as_str()).unwrap_or(&0));
            }
            monster_counts.push(m_ids.len() as u32);
            monster_ids_out.push(m_ids);
            monster_positions.push(m_pos);
            monster_scalar_out.push(m_scalars);
            monster_intents.push(m_intents);

            // Monster powers: sparse (id, amount, position) per power
            let mut mp_ids = Vec::new();
            let mut mp_amts = Vec::new();
            let mut mp_pos = Vec::new();
            for (m_idx, m) in monsters.iter().enumerate().take(max_monsters) {
                if m.state != crate::types::MonsterState::Alive {
                    continue;
                }
                for p in &m.powers {
                    if let Some(&idx) = power_id_map.get(p.id.as_str()) {
                        mp_ids.push(idx);
                        mp_amts.push(p.amount as f32);
                        mp_pos.push(m_idx as u32);
                    }
                }
            }
            monster_power_ids.push(mp_ids);
            monster_power_amts.push(mp_amts);
            monster_power_pos.push(mp_pos);

            // Player powers: sparse (id, amount) per power
            let mut pp_ids = Vec::new();
            let mut pp_amts = Vec::new();
            for p in player_powers {
                if let Some(&idx) = power_id_map.get(p.id.as_str()) {
                    pp_ids.push(idx);
                    pp_amts.push(p.amount as f32);
                }
            }
            player_power_ids.push(pp_ids);
            player_power_amts.push(pp_amts);
        } else {
            in_combat.push(false);
            combat_scalars.push(vec![0.0, 0.0, 0.0, 0.0]);
            pile_cards.push(Vec::new());
            pile_types.push(Vec::new());
            monster_counts.push(0);
            monster_ids_out.push(Vec::new());
            monster_positions.push(Vec::new());
            monster_scalar_out.push(Vec::new());
            monster_intents.push(Vec::new());
            monster_power_ids.push(Vec::new());
            monster_power_amts.push(Vec::new());
            monster_power_pos.push(Vec::new());
            player_power_ids.push(Vec::new());
            player_power_amts.push(Vec::new());
        }
    }

    // Pad variable-length fields
    let max_deck = deck_cards.iter().map(|d| d.len()).max().unwrap_or(0);
    let max_pile = pile_cards.iter().map(|p| p.len()).max().unwrap_or(0);
    let max_m = monster_ids_out.iter().map(|m| m.len()).max().unwrap_or(0);

    let deck_lengths: Vec<u32> = deck_cards.iter().map(|d| d.len() as u32).collect();
    let pile_lengths: Vec<u32> = pile_cards.iter().map(|p| p.len() as u32).collect();

    for d in &mut deck_cards { d.resize(max_deck, 0); }
    for p in &mut pile_cards { p.resize(max_pile, 0); }
    for p in &mut pile_types { p.resize(max_pile, 0); }
    for m in &mut monster_ids_out { m.resize(max_m, 0); }
    for m in &mut monster_positions { m.resize(max_m, 0); }
    for m in &mut monster_scalar_out { m.resize(max_m, vec![0.0; 5]); }
    for m in &mut monster_intents { m.resize(max_m, 0); }

    let player_power_counts: Vec<u32> = player_power_ids.iter().map(|v| v.len() as u32).collect();
    let max_pp = player_power_ids.iter().map(|v| v.len()).max().unwrap_or(0);
    for v in &mut player_power_ids { v.resize(max_pp, 0); }
    for v in &mut player_power_amts { v.resize(max_pp, 0.0); }

    let monster_power_counts: Vec<u32> = monster_power_ids.iter().map(|v| v.len() as u32).collect();
    let max_mp = monster_power_ids.iter().map(|v| v.len()).max().unwrap_or(0);
    for v in &mut monster_power_ids { v.resize(max_mp, 0); }
    for v in &mut monster_power_amts { v.resize(max_mp, 0.0); }
    for v in &mut monster_power_pos { v.resize(max_mp, 0); }

    Ok(TokenData {
        player_scalars,
        in_combat,
        combat_scalars,
        deck_card_indices: deck_cards,
        deck_lengths,
        pile_card_indices: pile_cards,
        pile_type_indices: pile_types,
        pile_lengths,
        monster_counts,
        monster_id_indices: monster_ids_out,
        monster_position_indices: monster_positions,
        monster_scalars: monster_scalar_out,
        monster_intent_indices: monster_intents,
        player_power_indices: player_power_ids,
        player_power_amounts: player_power_amts,
        player_power_counts,
        monster_power_indices: monster_power_ids,
        monster_power_amounts: monster_power_amts,
        monster_power_positions: monster_power_pos,
        monster_power_counts,
    })
}

/// Return a sorted list of all known card IDs.
#[pyfunction]
fn all_card_ids() -> Vec<String> {
    crate::card_db::all_card_ids().into_iter().map(|s| s.to_string()).collect()
}

/// Return a sorted list of all known monster IDs.
#[pyfunction]
fn all_monster_ids() -> Vec<String> {
    crate::monster_db::all_monster_ids().into_iter().map(|s| s.to_string()).collect()
}

/// Return a sorted list of all known power IDs.
#[pyfunction]
fn all_power_ids() -> Vec<String> {
    crate::power_db::all_power_ids().into_iter().map(|s| s.to_string()).collect()
}

#[pymodule]
pub fn sts_simulator(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGameState>()?;
    m.add_class::<MctsWorker>()?;
    m.add_class::<TokenData>()?;
    m.add_function(wrap_pyfunction!(all_card_ids, m)?)?;
    m.add_function(wrap_pyfunction!(all_monster_ids, m)?)?;
    m.add_function(wrap_pyfunction!(all_power_ids, m)?)?;
    m.add_function(wrap_pyfunction!(extract_token_data, m)?)?;
    Ok(())
}
