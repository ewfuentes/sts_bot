use crate::action::Action;
use crate::screen::Screen;
use crate::state::GameState as StsGameState;

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
        match self.inner.current_screen() {
            Screen::GameOver { victory: true } => {
                self.inner.hp as f64 / self.inner.max_hp as f64
            }
            Screen::GameOver { victory: false } => 0.0,
            _ => 0.5,
        }
    }
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
            if steps > 500 {
                return 0.5;
            }
        }
        sim.terminal_value()
    }
}
