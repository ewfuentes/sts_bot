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
                .expect("no actions aggregated");
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

#[pymodule]
pub fn sts_simulator(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGameState>()?;
    m.add_class::<MctsWorker>()?;
    Ok(())
}
