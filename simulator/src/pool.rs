use std::collections::VecDeque;

/// A pool of items (cards, relics, potions, etc.) that can be in one of two states:
/// - Unordered: known contents, unknown order. Cannot draw yet.
/// - Ordered: determinized with a concrete order. Can draw from front, items cycle to back.
#[derive(Debug, Clone)]
pub enum Pool {
    /// Known contents, unknown order.
    Unordered(Vec<String>),
    /// Determinized — concrete order. Draw from front, cycles to back.
    Ordered(VecDeque<String>),
}

impl Pool {
    /// Create an unordered pool from a list of item IDs.
    pub fn unordered(items: Vec<String>) -> Self {
        Pool::Unordered(items)
    }

    /// Create an ordered pool from a list of item IDs (already in draw order).
    pub fn ordered(items: Vec<String>) -> Self {
        Pool::Ordered(VecDeque::from(items))
    }

    /// Draw the next item from the pool.
    /// - Ordered: draws from front, cycles to back.
    /// - Unordered: returns "UNKNOWN" (indeterminate draw).
    /// Returns None only if the pool is empty.
    pub fn draw(&mut self) -> Option<String> {
        match self {
            Pool::Ordered(deck) => {
                let item = deck.pop_front()?;
                deck.push_back(item.clone());
                Some(item)
            }
            Pool::Unordered(items) => {
                if items.is_empty() {
                    None
                } else {
                    items.pop();
                    Some("UNKNOWN".to_string())
                }
            }
        }
    }

    /// Remove one instance of an item from the pool (e.g., when a card is taken).
    /// Works on both Unordered and Ordered pools.
    pub fn remove(&mut self, item_id: &str) {
        match self {
            Pool::Unordered(items) => {
                if let Some(idx) = items.iter().position(|i| i == item_id) {
                    items.remove(idx);
                }
            }
            Pool::Ordered(deck) => {
                if let Some(idx) = deck.iter().position(|i| i == item_id) {
                    deck.remove(idx);
                }
            }
        }
    }

    /// Number of items in the pool.
    pub fn len(&self) -> usize {
        match self {
            Pool::Unordered(items) => items.len(),
            Pool::Ordered(deck) => deck.len(),
        }
    }

    /// Whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the contents as a slice (for inspection/comparison).
    /// For Ordered, returns items in draw order.
    pub fn contents(&self) -> Vec<&str> {
        match self {
            Pool::Unordered(items) => items.iter().map(|s| s.as_str()).collect(),
            Pool::Ordered(deck) => deck.iter().map(|s| s.as_str()).collect(),
        }
    }

    /// Whether this pool has been determinized.
    pub fn is_ordered(&self) -> bool {
        matches!(self, Pool::Ordered(_))
    }

    /// Shuffle the pool contents into a concrete order using the provided shuffle function.
    /// Converts Unordered -> Ordered. No-op if already Ordered.
    pub fn determinize(&mut self, shuffle_fn: &mut dyn FnMut(&mut Vec<String>)) {
        match self {
            Pool::Unordered(items) => {
                let mut items = std::mem::take(items);
                shuffle_fn(&mut items);
                *self = Pool::Ordered(VecDeque::from(items));
            }
            Pool::Ordered(_) => {} // already determinized
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unordered_draw_returns_unknown_and_shrinks() {
        let mut pool = Pool::unordered(vec!["a".into(), "b".into()]);
        assert_eq!(pool.draw().as_deref(), Some("UNKNOWN"));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn empty_unordered_draw_returns_none() {
        let mut pool = Pool::unordered(vec![]);
        assert_eq!(pool.draw(), None);
    }

    #[test]
    fn ordered_draw_cycles() {
        let mut pool = Pool::ordered(vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(pool.draw().as_deref(), Some("a"));
        assert_eq!(pool.draw().as_deref(), Some("b"));
        assert_eq!(pool.draw().as_deref(), Some("c"));
        assert_eq!(pool.draw().as_deref(), Some("a")); // cycled
    }

    #[test]
    fn remove_from_unordered() {
        let mut pool = Pool::unordered(vec!["a".into(), "b".into(), "c".into()]);
        pool.remove("b");
        assert_eq!(pool.len(), 2);
        assert!(!pool.contents().contains(&"b"));
    }

    #[test]
    fn remove_from_ordered() {
        let mut pool = Pool::ordered(vec!["a".into(), "b".into(), "c".into()]);
        pool.remove("b");
        assert_eq!(pool.draw().as_deref(), Some("a"));
        assert_eq!(pool.draw().as_deref(), Some("c"));
    }

    #[test]
    fn determinize_shuffles() {
        let mut pool = Pool::unordered(vec!["a".into(), "b".into(), "c".into()]);
        // Reverse as a deterministic "shuffle"
        pool.determinize(&mut |items| items.reverse());
        assert!(pool.is_ordered());
        assert_eq!(pool.draw().as_deref(), Some("c"));
        assert_eq!(pool.draw().as_deref(), Some("b"));
        assert_eq!(pool.draw().as_deref(), Some("a"));
    }
}
