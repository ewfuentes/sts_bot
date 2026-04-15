use crate::action::Action;
use crate::screen::Screen;
use crate::state::GameState as StsGameState;
use crate::types::{Monster, MonsterState};

/// Create a combat-only GameState for the given encounter.
pub fn make_combat_state(seed: u64, encounter_id: &str) -> StsGameState {
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
    if let Some(enc) = crate::encounter_db::lookup(encounter_id) {
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
                    pattern: crate::monster_db::MovePattern::default(),
                });
            }
        }
    }

    // Seed the combat RNG so different seeds produce different games
    if let Screen::Combat { rng, .. } = state.current_screen_mut() {
        *rng = crate::rng::Rng::from_seed(seed);
    }
    state.determinize(seed);
    state.start_combat();
    state.apply_monster_starting_effects();
    state
}

/// Wrapper around the StS GameState to implement the mcts::GameState trait.
#[derive(Clone, Debug)]
pub struct StsState {
    pub inner: StsGameState,
}

impl StsState {
    pub fn new(inner: StsGameState) -> Self {
        StsState { inner }
    }
}

impl mcts::GameState for StsState {
    type Action = Action;

    fn available_actions(&self) -> Vec<Action> {
        self.inner.available_actions()
    }

    fn apply(&mut self, action: &Action) {
        self.inner.apply(action);
    }

    fn is_terminal(&self) -> bool {
        matches!(self.inner.current_screen(), Screen::GameOver { .. })
    }

    fn terminal_value(&self) -> f64 {
        progress_value(&self.inner)
    }
}

/// Value heuristic: floor + hp/max_hp.
fn progress_value(state: &StsGameState) -> f64 {
    let hp_fraction = if state.max_hp > 0 {
        state.hp as f64 / state.max_hp as f64
    } else {
        0.0
    };
    state.floor as f64 + hp_fraction
}

/// Random rollout evaluator for StS combat.
pub struct StsRandomEvaluator;

impl mcts::Evaluator<StsState> for StsRandomEvaluator {
    fn evaluate(&self, state: &StsState, rng: &mut impl rand::Rng) -> f64 {
        use mcts::GameState;
        let mut sim = state.clone();
        let mut steps = 0;
        while !sim.is_terminal() {
            let actions = sim.available_actions();
            if actions.is_empty() {
                break;
            }
            let idx = rng.random_range(0..actions.len());
            sim.apply(&actions[idx]);
            steps += 1;
            if steps > 2000 {
                return progress_value(&sim.inner);
            }
        }
        sim.terminal_value()
    }
}
