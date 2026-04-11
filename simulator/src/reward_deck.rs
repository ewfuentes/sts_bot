use crate::pools;

/// Which character's card pool to use.
#[derive(Debug, Clone, Copy)]
pub enum Character {
    Ironclad,
    Silent,
    Defect,
    Watcher,
}

/// A circular deck of card IDs. Draw from front, put back at end.
#[derive(Debug, Clone)]
pub struct RewardDeck {
    pub cards: Vec<String>,
    pub position: usize,
}

impl RewardDeck {
    /// Build the reward deck for a character, then shuffle with the given seed.
    pub fn new(character: Character, seed: u64) -> Self {
        let (commons, uncommons, golden_ticket) = match character {
            Character::Ironclad => (
                pools::IRONCLAD_COMMONS,
                pools::IRONCLAD_UNCOMMONS,
                "BGGoldenTicket_R",
            ),
            Character::Silent => (
                pools::SILENT_COMMONS,
                pools::SILENT_UNCOMMONS,
                "BGGoldenTicket_G",
            ),
            Character::Defect => (
                pools::DEFECT_COMMONS,
                pools::DEFECT_UNCOMMONS,
                "BGGoldenTicket_B",
            ),
            Character::Watcher => (
                pools::WATCHER_COMMONS,
                pools::WATCHER_UNCOMMONS,
                "BGGoldenTicket_W",
            ),
        };

        let mut cards = Vec::new();

        // 2 Golden Tickets
        cards.push(golden_ticket.to_string());
        cards.push(golden_ticket.to_string());

        // Commons x2
        for &id in commons {
            cards.push(id.to_string());
            cards.push(id.to_string());
        }

        // Uncommons x1
        for &id in uncommons {
            cards.push(id.to_string());
        }

        // Shuffle
        shuffle(&mut cards, seed);

        RewardDeck { cards, position: 0 }
    }

    /// Draw the next card from the deck. The card cycles back to the end.
    pub fn draw(&mut self) -> &str {
        let card = &self.cards[self.position];
        self.position = (self.position + 1) % self.cards.len();
        card
    }

    /// Remove a specific card ID from the deck (used when a card is obtained).
    pub fn remove(&mut self, card_id: &str) {
        if let Some(idx) = self.cards.iter().position(|c| c == card_id) {
            self.cards.remove(idx);
            if self.position > idx && self.position > 0 {
                self.position -= 1;
            }
            if !self.cards.is_empty() {
                self.position %= self.cards.len();
            }
        }
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}

/// Build the rare card deck for a character.
pub fn build_rare_deck(character: Character, seed: u64) -> RewardDeck {
    let rares = match character {
        Character::Ironclad => pools::IRONCLAD_RARES,
        Character::Silent => pools::SILENT_RARES,
        Character::Defect => pools::DEFECT_RARES,
        Character::Watcher => pools::WATCHER_RARES,
    };

    let mut cards: Vec<String> = rares.iter().map(|&s| s.to_string()).collect();
    shuffle(&mut cards, seed);
    RewardDeck { cards, position: 0 }
}

/// Build the relic deck.
pub fn build_relic_deck(seed: u64) -> Vec<String> {
    let mut relics: Vec<String> = pools::RELICS.iter().map(|&s| s.to_string()).collect();
    shuffle(&mut relics, seed);
    relics
}

/// Build the boss relic deck.
pub fn build_boss_relic_deck(seed: u64) -> Vec<String> {
    let mut relics: Vec<String> = pools::BOSS_RELICS.iter().map(|&s| s.to_string()).collect();
    shuffle(&mut relics, seed);
    relics
}

/// Build the potion deck.
pub fn build_potion_deck(seed: u64) -> RewardDeck {
    let mut potions: Vec<String> = pools::POTIONS.iter().map(|&s| s.to_string()).collect();
    shuffle(&mut potions, seed);
    RewardDeck {
        cards: potions,
        position: 0,
    }
}

/// Build the curse deck.
pub fn build_curse_deck(seed: u64) -> RewardDeck {
    let mut curses: Vec<String> = pools::CURSES.iter().map(|&s| s.to_string()).collect();
    shuffle(&mut curses, seed);
    RewardDeck {
        cards: curses,
        position: 0,
    }
}

/// Build the colorless card deck.
pub fn build_colorless_deck(seed: u64) -> RewardDeck {
    let mut cards = Vec::new();
    for &id in pools::COLORLESS_COMMONS {
        cards.push(id.to_string());
    }
    for &id in pools::COLORLESS_UNCOMMONS {
        cards.push(id.to_string());
    }
    for &id in pools::COLORLESS_RARES {
        cards.push(id.to_string());
    }
    shuffle(&mut cards, seed);
    RewardDeck { cards, position: 0 }
}

/// Simple deterministic shuffle using a seed.
fn shuffle(items: &mut Vec<String>, seed: u64) {
    // Fisher-Yates shuffle with a simple LCG
    let mut rng = seed;
    for i in (1..items.len()).rev() {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (rng >> 33) as usize % (i + 1);
        items.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ironclad_reward_deck_size() {
        let deck = RewardDeck::new(Character::Ironclad, 42);
        // 2 golden tickets + 15 commons * 2 + 28 uncommons = 2 + 30 + 28 = 60
        assert_eq!(deck.len(), 60);
    }

    #[test]
    fn draw_cycles() {
        let mut deck = RewardDeck::new(Character::Ironclad, 42);
        let first = deck.draw().to_string();
        for _ in 0..deck.len() - 1 {
            deck.draw();
        }
        // Should cycle back to the first card
        assert_eq!(deck.draw(), first);
    }

    #[test]
    fn remove_card() {
        let mut deck = RewardDeck::new(Character::Ironclad, 42);
        let initial_len = deck.len();
        let first = deck.draw().to_string();
        deck.remove(&first);
        assert_eq!(deck.len(), initial_len - 1);
    }

    #[test]
    fn relic_deck_size() {
        let relics = build_relic_deck(42);
        assert_eq!(relics.len(), 58);
    }

    #[test]
    fn potion_deck_size() {
        let potions = build_potion_deck(42);
        assert_eq!(potions.len(), 29);
    }
}
