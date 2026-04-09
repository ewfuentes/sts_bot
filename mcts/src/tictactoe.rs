use crate::GameState;

/// A tic-tac-toe board. X always goes first.
#[derive(Debug, Clone, PartialEq)]
pub struct TicTacToe {
    /// 3x3 board: 0 = empty, 1 = X, 2 = O
    board: [u8; 9],
    /// Whose turn: 1 = X, 2 = O
    current_player: u8,
}

/// A move: place your mark at position 0-8.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TicTacToeAction(pub u8);

impl TicTacToe {
    pub fn new() -> Self {
        TicTacToe {
            board: [0; 9],
            current_player: 1,
        }
    }

    pub fn current_player(&self) -> u8 {
        self.current_player
    }

    fn winner(&self) -> Option<u8> {
        const LINES: [[usize; 3]; 8] = [
            [0, 1, 2], [3, 4, 5], [6, 7, 8], // rows
            [0, 3, 6], [1, 4, 7], [2, 5, 8], // cols
            [0, 4, 8], [2, 4, 6],             // diags
        ];
        for line in &LINES {
            let a = self.board[line[0]];
            if a != 0 && a == self.board[line[1]] && a == self.board[line[2]] {
                return Some(a);
            }
        }
        None
    }

    fn is_full(&self) -> bool {
        self.board.iter().all(|&c| c != 0)
    }
}

impl GameState for TicTacToe {
    type Action = TicTacToeAction;

    fn available_actions(&self) -> Vec<TicTacToeAction> {
        if self.is_terminal() {
            return vec![];
        }
        self.board
            .iter()
            .enumerate()
            .filter(|(_, c)| **c == 0)
            .map(|(i, _)| TicTacToeAction(i as u8))
            .collect()
    }

    fn apply(&mut self, action: &TicTacToeAction) {
        self.board[action.0 as usize] = self.current_player;
        self.current_player = if self.current_player == 1 { 2 } else { 1 };
    }

    fn is_terminal(&self) -> bool {
        self.winner().is_some() || self.is_full()
    }

    fn terminal_value(&self) -> f64 {
        // Value from player 1 (X) perspective.
        match self.winner() {
            Some(1) => 1.0, // X wins
            Some(2) => 0.0, // O wins
            _ => 0.5,       // draw
        }
    }
}

impl std::fmt::Display for TicTacToe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in 0..3 {
            for col in 0..3 {
                let c = match self.board[row * 3 + col] {
                    1 => 'X',
                    2 => 'O',
                    _ => '.',
                };
                write!(f, "{}", c)?;
                if col < 2 { write!(f, " ")?; }
            }
            if row < 2 { writeln!(f)?; }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use crate::{MctsConfig, RandomRolloutEvaluator, search};

    #[test]
    fn new_game_has_9_actions() {
        let game = TicTacToe::new();
        assert_eq!(game.available_actions().len(), 9);
        assert!(!game.is_terminal());
    }

    #[test]
    fn x_wins_top_row() {
        let mut game = TicTacToe::new();
        game.apply(&TicTacToeAction(0)); // X
        game.apply(&TicTacToeAction(3)); // O
        game.apply(&TicTacToeAction(1)); // X
        game.apply(&TicTacToeAction(4)); // O
        game.apply(&TicTacToeAction(2)); // X wins

        assert!(game.is_terminal());
        assert_eq!(game.winner(), Some(1));
        // Current player is O (whose turn it would be).
        // Last player was X who won, so reward = 1.0.
        assert_eq!(game.terminal_value(), 1.0);
    }

    #[test]
    fn draw_game() {
        let mut game = TicTacToe::new();
        // X O X
        // X X O
        // O X O
        for &pos in &[0, 1, 2, 4, 3, 5, 7, 6, 8] {
            game.apply(&TicTacToeAction(pos));
        }
        assert!(game.is_terminal());
        assert_eq!(game.winner(), None);
        assert_eq!(game.terminal_value(), 0.5);
    }

    #[test]
    fn o_wins() {
        let mut game = TicTacToe::new();
        game.apply(&TicTacToeAction(0)); // X
        game.apply(&TicTacToeAction(3)); // O
        game.apply(&TicTacToeAction(1)); // X
        game.apply(&TicTacToeAction(4)); // O
        game.apply(&TicTacToeAction(8)); // X (doesn't win)
        game.apply(&TicTacToeAction(5)); // O wins (row 3-4-5)

        assert!(game.is_terminal());
        assert_eq!(game.winner(), Some(2));
        // O won → value from X's perspective = 0.0
        assert_eq!(game.terminal_value(), 0.0);
    }

    #[test]
    fn actions_decrease() {
        let mut game = TicTacToe::new();
        assert_eq!(game.available_actions().len(), 9);
        game.apply(&TicTacToeAction(4));
        assert_eq!(game.available_actions().len(), 8);
    }

    #[test]
    fn mcts_blocks_winning_move() {
        // O has two in a row (positions 3, 4), X should block at 5.
        // . . .
        // O O _  ← X must play 5
        // . . .
        let mut game = TicTacToe::new();
        game.apply(&TicTacToeAction(0)); // X
        game.apply(&TicTacToeAction(3)); // O
        game.apply(&TicTacToeAction(8)); // X
        game.apply(&TicTacToeAction(4)); // O — now O threatens 3-4-5

        // It's X's turn. MCTS should find that position 5 is critical.
        let config = MctsConfig {
            num_iterations: 1000,
            exploration_constant: 1.41,
        };
        let evaluator = RandomRolloutEvaluator { num_rollouts: 1, seed: 42 };
        let mut rng = rand::rngs::SmallRng::seed_from_u64(123);
        let tree = search(&config, &game, &evaluator, &mut rng);

        let best = tree.best_action().unwrap();
        assert_eq!(best, TicTacToeAction(5), "MCTS should block O's winning move at position 5");
    }

    #[test]
    fn mcts_takes_winning_move() {
        // X has two in a row (positions 0, 1), X should win at 2.
        // X X _  ← X should play 2
        // O O .
        // . . .
        let mut game = TicTacToe::new();
        game.apply(&TicTacToeAction(0)); // X
        game.apply(&TicTacToeAction(3)); // O
        game.apply(&TicTacToeAction(1)); // X
        game.apply(&TicTacToeAction(4)); // O

        // It's X's turn. X can win immediately at position 2.
        let config = MctsConfig {
            num_iterations: 1000,
            exploration_constant: 1.41,
        };
        let evaluator = RandomRolloutEvaluator { num_rollouts: 1, seed: 42 };
        let mut rng = rand::rngs::SmallRng::seed_from_u64(123);
        let tree = search(&config, &game, &evaluator, &mut rng);

        let best = tree.best_action().unwrap();
        assert_eq!(best, TicTacToeAction(2), "MCTS should take the winning move at position 2");
    }

    #[test]
    fn mcts_from_empty_board() {
        // Just verify it runs without panicking and returns an action.
        let game = TicTacToe::new();
        let config = MctsConfig {
            num_iterations: 500,
            exploration_constant: 1.41,
        };
        let evaluator = RandomRolloutEvaluator { num_rollouts: 1, seed: 42 };
        let mut rng = rand::rngs::SmallRng::seed_from_u64(123);
        let tree = search(&config, &game, &evaluator, &mut rng);

        let best = tree.best_action();
        assert!(best.is_some(), "MCTS should return an action");
        // Root should have been visited num_iterations times
        assert_eq!(tree.nodes[0].visits, 500);
    }

    #[test]
    fn advance_reuses_subtree() {
        let game = TicTacToe::new();
        let config = MctsConfig {
            num_iterations: 500,
            exploration_constant: 1.41,
        };
        let evaluator = RandomRolloutEvaluator { num_rollouts: 1, seed: 42 };
        let mut rng = rand::rngs::SmallRng::seed_from_u64(123);
        let tree = search(&config, &game, &evaluator, &mut rng);

        let best = tree.best_action().unwrap();
        let advanced = tree.advance(&best).expect("Should find the child");

        // New root should have visits from previous search
        assert!(advanced.nodes[0].visits > 0);
        // New root should have no parent
        assert!(advanced.nodes[0].parent.is_none());
        // New root should have no action (it's the root)
        assert!(advanced.nodes[0].action.is_none());
        // Tree should be smaller than original (pruned siblings)
        assert!(advanced.nodes.len() < tree.nodes.len());
    }

    #[test]
    fn advance_nonexistent_action_returns_none() {
        let game = TicTacToe::new();
        let config = MctsConfig {
            num_iterations: 10,
            exploration_constant: 1.41,
        };
        let evaluator = RandomRolloutEvaluator { num_rollouts: 1, seed: 42 };
        let mut rng = rand::rngs::SmallRng::seed_from_u64(123);
        let tree = search(&config, &game, &evaluator, &mut rng);

        // Position 9 doesn't exist
        assert!(tree.advance(&TicTacToeAction(9)).is_none());
    }
}
