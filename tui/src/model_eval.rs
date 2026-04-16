//! Embedded Python model evaluator for the TUI.
//! Loads a trained StateEncoder checkpoint and scores game states.

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Wraps a Python StateEncoder model for evaluating game states.
pub struct ModelEvaluator {
    /// Python function: takes a list of JSON state strings, returns list of (mean, log_var)
    eval_fn: PyObject,
}

impl ModelEvaluator {
    /// Load a model from a checkpoint file.
    /// The checkpoint must contain {"config": StateEncoderConfig, "state_dict": ...}.
    pub fn load(checkpoint_path: &str) -> Result<Self, String> {
        Python::with_gil(|py| {
            // Run Python setup code that loads the model and returns an eval function
            let code = c"
def _setup_model(checkpoint_path):
    import torch
    from training.model import StateEncoder
    from sts_simulator import GameState

    device = torch.device('cpu')
    ckpt = torch.load(checkpoint_path, map_location=device, weights_only=False)
    config = ckpt['config']
    model = StateEncoder(config).to(device)
    model.load_state_dict(ckpt['state_dict'])
    model.eval()

    def evaluate(state_jsons):
        states = [GameState.from_json(s) for s in state_jsons]
        with torch.no_grad():
            mean, log_var = model(states)
        return list(zip(mean.tolist(), log_var.tolist()))

    return evaluate
";
            let locals = pyo3::types::PyDict::new(py);
            py.run(code, None, Some(&locals))
                .map_err(|e| format!("Failed to define setup function: {e}"))?;

            let setup_fn = locals
                .get_item("_setup_model")
                .map_err(|e| format!("Failed to get setup function: {e}"))?
                .ok_or("_setup_model not found")?;

            let eval_fn = setup_fn
                .call1((checkpoint_path,))
                .map_err(|e| format!("Failed to load model: {e}"))?;

            Ok(ModelEvaluator {
                eval_fn: eval_fn.into(),
            })
        })
    }

    /// Evaluate a single game state. Returns (mean, log_variance).
    #[allow(dead_code)]
    pub fn evaluate(&self, state: &sts_simulator::GameState) -> (f64, f64) {
        Python::with_gil(|py| {
            let json = serde_json::to_string(state).unwrap();
            let json_list = PyList::new(py, &[json]).unwrap();
            let result = self
                .eval_fn
                .call1(py, (json_list,))
                .expect("Model evaluation failed");
            let pairs: Vec<(f64, f64)> = result.extract(py).expect("Failed to extract result");
            pairs[0]
        })
    }

    /// Evaluate multiple game states. Returns vec of (mean, log_variance).
    pub fn evaluate_batch(&self, states: &[sts_simulator::GameState]) -> Vec<(f64, f64)> {
        Python::with_gil(|py| {
            let jsons: Vec<String> = states
                .iter()
                .map(|s| serde_json::to_string(s).unwrap())
                .collect();
            let json_list = PyList::new(py, &jsons).unwrap();
            let result = self
                .eval_fn
                .call1(py, (json_list,))
                .expect("Model evaluation failed");
            result.extract(py).expect("Failed to extract results")
        })
    }
}
