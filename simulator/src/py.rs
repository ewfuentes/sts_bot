use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

use crate::action::Action;
use crate::state::GameState;

#[pyclass(name = "GameState")]
#[derive(Clone)]
pub struct PyGameState {
    inner: GameState,
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
            "screen": s.screen,
        });
        serde_json::to_string(&summary)
            .map_err(|e| PyValueError::new_err(format!("Serialization error: {e}")))
    }
}

#[pymodule]
pub fn sts_simulator(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGameState>()?;
    Ok(())
}
