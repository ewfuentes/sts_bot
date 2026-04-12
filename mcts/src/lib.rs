pub mod tictactoe;

use std::fmt;

use rand::prelude::*;

/// Trait for games compatible with MCTS.
///
/// The MCTS algorithm is generic over any game that implements this trait.
/// `Action` must be `Clone + PartialEq` so the tree can store and compare actions.
pub trait GameState: Clone {
    type Action: Clone + PartialEq + fmt::Debug;

    /// Return all legal actions from this state.
    fn available_actions(&self) -> Vec<Self::Action>;

    /// Apply an action, mutating the state in place.
    fn apply(&mut self, action: &Self::Action);

    /// Is the game over?
    fn is_terminal(&self) -> bool;

    /// The value of this state from player 1's perspective.
    /// Only meaningful when `is_terminal()` is true.
    /// Convention: 1.0 = player 1 wins, 0.0 = player 1 loses, 0.5 = draw.
    fn terminal_value(&self) -> f64;
}

/// A function that estimates the value of a non-terminal state.
/// Returns a value in [0, 1] from the perspective of the player to move.
pub trait Evaluator<S: GameState> {
    fn evaluate(&self, state: &S, rng: &mut impl rand::Rng) -> f64;
}

/// A simple evaluator that performs random rollouts to estimate value.
pub struct RandomRolloutEvaluator {
    pub num_rollouts: u32,
    pub seed: u64,
}

impl<S: GameState> Evaluator<S> for RandomRolloutEvaluator {
    fn evaluate(&self, state: &S, rng: &mut impl rand::Rng) -> f64 {
        let mut state = state.clone();

        while !state.is_terminal() {
            let actions = state.available_actions();
            let action = actions.choose(rng).unwrap();
            state.apply(action);
        }
        state.terminal_value()
    }
}

/// A node in the MCTS search tree.
#[derive(Debug, Clone)]
pub struct MctsNode<S: GameState> {
    /// Game state at this node.
    pub state: S,
    /// Action that led to this node (None for root).
    pub action: Option<S::Action>,
    /// Parent node index (None for root).
    pub parent: Option<usize>,
    /// Child node indices.
    pub children: Vec<usize>,
    /// Number of times this node has been visited.
    pub visits: u32,
    /// Total accumulated value from simulations through this node.
    pub total_value: f64,
    /// Actions not yet expanded from this node.
    pub unexpanded_actions: Vec<S::Action>,
}

impl<S: GameState> MctsNode<S> {
    fn ucb_value(&self, parent_visits: u32, exploration_constant: f64) -> f64 {
        if self.visits == 0 {
            return f64::INFINITY;
        }
        self.total_value / (self.visits as f64) +
           exploration_constant * ((parent_visits as f64).sqrt() / (self.visits as f64)).sqrt()
    }
}

/// The MCTS search tree. All nodes are stored in an arena (Vec).
#[derive(Debug)]
pub struct MctsTree<S: GameState> {
    pub nodes: Vec<MctsNode<S>>,
}

impl<S: GameState> MctsTree<S> {
    /// Create a new tree with a root node for the given state.
    pub fn new(state: S) -> Self {
        let available_actions = state.available_actions();
        let root = MctsNode {
            state,
            action: None,
            parent: None,
            children: Vec::new(),
            visits: 0,
            total_value: 0.0,
            unexpanded_actions: available_actions,
        };
        MctsTree { nodes: vec![root] }
    }

    /// Get the root node index (always 0).
    pub fn root(&self) -> usize {
        0
    }

    /// Add a child node by applying an action to the parent's state.
    /// Removes the action from the parent's unexpanded list.
    /// Returns the new child's index.
    pub fn add_child(&mut self, parent: usize, action: S::Action) -> usize {
        let mut child_state = self.nodes[parent].state.clone();
        child_state.apply(&action);
        let available_actions = child_state.available_actions();
        // Remove from parent's unexpanded actions
        if let Some(pos) = self.nodes[parent].unexpanded_actions.iter().position(|a| a == &action) {
            self.nodes[parent].unexpanded_actions.remove(pos);
        }
        let child_idx = self.nodes.len();
        self.nodes.push(MctsNode {
            state: child_state,
            action: Some(action),
            parent: Some(parent),
            children: Vec::new(),
            visits: 0,
            total_value: 0.0,
            unexpanded_actions: available_actions,
        });
        self.nodes[parent].children.push(child_idx);
        child_idx
    }

    pub fn backprop(&mut self, leaf_idx: usize, leaf_value:f64) {
        let mut maybe_curr_idx = Some(leaf_idx);

        while let Some(curr_idx) = maybe_curr_idx {
            let curr_node = &mut self.nodes[curr_idx];

            curr_node.total_value += leaf_value;
            curr_node.visits += 1;
            maybe_curr_idx = curr_node.parent;
        }
    }

    /// Get the best action from the root based on visit count.
    pub fn best_action(&self) -> Option<S::Action> {
        let root = &self.nodes[self.root()];
        root.children
            .iter()
            .max_by_key(|&&idx| self.nodes[idx].visits)
            .and_then(|&idx| self.nodes[idx].action.clone())
    }

    /// Get action visit counts from the root (for policy targets).
    pub fn root_action_visits(&self) -> Vec<(S::Action, u32)> {
        let root = &self.nodes[self.root()];
        root.children
            .iter()
            .filter_map(|&idx| {
                let node = &self.nodes[idx];
                node.action.clone().map(|a| (a, node.visits))
            })
            .collect()
    }

    /// Select a leaf via UCB, expand one child if possible.
    /// Returns the leaf node index. Does NOT evaluate or backprop.
    /// If the selected node is terminal, returns it as-is (caller should
    /// check `is_terminal()` and backprop `terminal_value()` directly).
    pub fn select_and_expand(&mut self, exploration_constant: f64) -> usize {
        let (node_idx, maybe_action) = self.select_leaf(exploration_constant);
        if let Some(action) = maybe_action {
            self.add_child(node_idx, action)
        } else {
            node_idx
        }
    }

    /// UCB tree descent. Returns (node_index, action_to_expand_or_None).
    fn select_leaf(&self, exploration_constant: f64) -> (usize, Option<S::Action>) {
        let mut curr_idx = self.root();
        let mut curr_node = &self.nodes[curr_idx];
        while curr_node.unexpanded_actions.is_empty() && !curr_node.state.is_terminal() {
            let best_child_idx = curr_node.children.iter().max_by(|&&idx_1, &&idx_2| {
                let child_1_value = &self.nodes[idx_1].ucb_value(curr_node.visits, exploration_constant);
                let child_2_value = &self.nodes[idx_2].ucb_value(curr_node.visits, exploration_constant);
                child_1_value.partial_cmp(&child_2_value).unwrap()
            }).unwrap();

            curr_idx = *best_child_idx;
            curr_node = &self.nodes[curr_idx];
        }

        (curr_idx,
            if curr_node.state.is_terminal() { None } else { Some(curr_node.unexpanded_actions.first().unwrap().clone()) })
    }

    /// Advance the tree to the child matching the given action.
    /// Returns a new tree rooted at that child, preserving the subtree below it.
    /// Returns None if no child matches the action.
    pub fn advance(&self, action: &S::Action) -> Option<MctsTree<S>> {
        let root = &self.nodes[self.root()];
        let child_idx = root.children.iter()
            .find(|&&idx| self.nodes[idx].action.as_ref() == Some(action))?;

        // BFS copy of the subtree, remapping indices
        let mut new_nodes = Vec::new();
        // Queue of (old_index, new_parent_index)
        let mut queue = std::collections::VecDeque::new();

        // Copy the new root (no parent)
        let old_root = &self.nodes[*child_idx];
        new_nodes.push(MctsNode {
            state: old_root.state.clone(),
            action: None, // root has no action
            parent: None,
            children: Vec::new(),
            visits: old_root.visits,
            total_value: old_root.total_value,
            unexpanded_actions: old_root.unexpanded_actions.clone(),
        });

        // Queue old children with new parent = 0
        for &old_child in &old_root.children {
            queue.push_back((old_child, 0usize));
        }

        while let Some((old_idx, new_parent)) = queue.pop_front() {
            let old_node = &self.nodes[old_idx];
            let new_idx = new_nodes.len();
            new_nodes.push(MctsNode {
                state: old_node.state.clone(),
                action: old_node.action.clone(),
                parent: Some(new_parent),
                children: Vec::new(),
                visits: old_node.visits,
                total_value: old_node.total_value,
                unexpanded_actions: old_node.unexpanded_actions.clone(),
            });
            new_nodes[new_parent].children.push(new_idx);

            for &old_child in &old_node.children {
                queue.push_back((old_child, new_idx));
            }
        }

        Some(MctsTree { nodes: new_nodes })
    }
}

/// MCTS search configuration.
pub struct MctsConfig {
    /// Number of iterations (select → expand → evaluate → backpropagate).
    pub num_iterations: u32,
    /// UCB1 exploration constant. Higher = more exploration.
    pub exploration_constant: f64,
}

impl Default for MctsConfig {
    fn default() -> Self {
        MctsConfig {
            num_iterations: 1000,
            exploration_constant: 1.41,
        }
    }
}

pub fn search<S: GameState>(
    config: &MctsConfig,
    state: &S,
    evaluator: &impl Evaluator<S>,
    rng: &mut impl rand::Rng,
) -> MctsTree<S> {
    let mut tree = MctsTree::<S>::new(state.clone());
    for _iter in 0..config.num_iterations {
        let leaf_idx = tree.select_and_expand(config.exploration_constant);
        let leaf_value = evaluator.evaluate(&tree.nodes[leaf_idx].state, rng);
        tree.backprop(leaf_idx, leaf_value);
    }
    tree
}
