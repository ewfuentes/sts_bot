use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::pool::Pool;
use crate::reward_deck::{self, Character, RewardDeck};
use crate::screen::{EventOption, Screen, ShopCard, ShopPotion, ShopRelic};
use crate::types::{Card, Potion, Relic};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub hp: u16,
    pub max_hp: u16,
    pub gold: u16,
    pub floor: u8,
    pub act: u8,
    pub ascension: u8,
    pub deck: Vec<Card>,
    pub relics: Vec<Relic>,
    pub potions: Vec<Option<Potion>>,
    pub screen: Screen,
    #[serde(default)]
    pub actions: Vec<Action>,
    /// Reward pools for offline simulation. Not serialized.
    #[serde(skip)]
    pub reward_pools: Option<RewardPools>,
}

/// All the draw-from-top pools used in the Board Game mod.
#[derive(Debug, Clone)]
pub struct RewardPools {
    pub card_deck: Pool,
    pub rare_deck: Pool,
    pub relic_deck: Pool,
    pub boss_relic_deck: Pool,
    pub potion_deck: Pool,
    pub curse_deck: Pool,
    pub colorless_deck: Pool,
}

impl RewardPools {
    /// Create fully-ordered pools from a seed (for offline simulation from scratch).
    pub fn new(character: Character, seed: u64) -> Self {
        let rd = RewardDeck::new(character, seed);
        let rare = reward_deck::build_rare_deck(character, seed.wrapping_add(1));
        let relic = reward_deck::build_relic_deck(seed.wrapping_add(2));
        let boss_relic = reward_deck::build_boss_relic_deck(seed.wrapping_add(3));
        let potion = reward_deck::build_potion_deck(seed.wrapping_add(4));
        let curse = reward_deck::build_curse_deck(seed.wrapping_add(5));
        let colorless = reward_deck::build_colorless_deck(seed.wrapping_add(6));

        RewardPools {
            card_deck: Pool::ordered(rd.cards),
            rare_deck: Pool::ordered(rare.cards),
            relic_deck: Pool::ordered(relic),
            boss_relic_deck: Pool::ordered(boss_relic),
            potion_deck: Pool::ordered(potion.cards),
            curse_deck: Pool::ordered(curse.cards),
            colorless_deck: Pool::ordered(colorless.cards),
        }
    }

    /// Create unordered pools by taking the full pool for a character
    /// and removing items already observed in the game state.
    pub fn from_observed(
        character: Character,
        deck_card_ids: &[String],
        relic_ids: &[String],
        potion_ids: &[String],
    ) -> Self {
        // Build full card pool
        let rd = RewardDeck::new(character, 0); // seed doesn't matter, we discard order
        let mut card_items = rd.cards;
        for id in deck_card_ids {
            if let Some(idx) = card_items.iter().position(|c| c == id) {
                card_items.remove(idx);
            }
        }

        // Rare deck
        let rare = reward_deck::build_rare_deck(character, 0);
        let mut rare_items = rare.cards;
        for id in deck_card_ids {
            if let Some(idx) = rare_items.iter().position(|c| c == id) {
                rare_items.remove(idx);
            }
        }

        // Relic deck — remove relics already obtained
        let relic_all = reward_deck::build_relic_deck(0);
        let relic_items: Vec<String> = relic_all
            .into_iter()
            .filter(|r| !relic_ids.contains(r))
            .collect();

        // Boss relic deck — remove obtained
        let boss_all = reward_deck::build_boss_relic_deck(0);
        let boss_items: Vec<String> = boss_all
            .into_iter()
            .filter(|r| !relic_ids.contains(r))
            .collect();

        // Potion deck
        let potion_all = reward_deck::build_potion_deck(0);
        let potion_items: Vec<String> = potion_all
            .cards
            .into_iter()
            .filter(|p| !potion_ids.contains(p))
            .collect();

        // Curse deck — remove curses already in player's deck
        let curse_all = reward_deck::build_curse_deck(0);
        let mut curse_items = curse_all.cards;
        for id in deck_card_ids {
            if let Some(idx) = curse_items.iter().position(|c| c == id) {
                curse_items.remove(idx);
            }
        }

        // Colorless deck — remove obtained colorless cards
        let colorless_all = reward_deck::build_colorless_deck(0);
        let mut colorless_items = colorless_all.cards;
        for id in deck_card_ids {
            if let Some(idx) = colorless_items.iter().position(|c| c == id) {
                colorless_items.remove(idx);
            }
        }

        RewardPools {
            card_deck: Pool::unordered(card_items),
            rare_deck: Pool::unordered(rare_items),
            relic_deck: Pool::unordered(relic_items),
            boss_relic_deck: Pool::unordered(boss_items),
            potion_deck: Pool::unordered(potion_items),
            curse_deck: Pool::unordered(curse_items),
            colorless_deck: Pool::unordered(colorless_items),
        }
    }

    /// Determinize all pools by shuffling with the provided function.
    pub fn determinize(&mut self, shuffle_fn: &mut dyn FnMut(&mut Vec<String>)) {
        self.card_deck.determinize(shuffle_fn);
        self.rare_deck.determinize(shuffle_fn);
        self.relic_deck.determinize(shuffle_fn);
        self.boss_relic_deck.determinize(shuffle_fn);
        self.potion_deck.determinize(shuffle_fn);
        self.curse_deck.determinize(shuffle_fn);
        self.colorless_deck.determinize(shuffle_fn);
    }

    /// Draw N cards from the reward deck for a card reward screen.
    /// Returns empty vec if pools are unordered.
    pub fn draw_card_reward(&mut self, count: usize) -> Vec<Card> {
        (0..count)
            .filter_map(|_| {
                let id = self.card_deck.draw()?;
                Some(Card {
                    id: id.clone(),
                    name: id,
                    cost: 0, // TODO: look up actual cost
                    card_type: "UNKNOWN".to_string(),
                    upgraded: false,
                })
            })
            .collect()
    }

    /// Draw the next relic from the relic deck.
    pub fn draw_relic(&mut self) -> Option<String> {
        self.relic_deck.draw()
    }
}

impl GameState {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let mut state: Self = serde_json::from_str(json)?;

        // Build unordered reward pools from observed state if possible
        if state.reward_pools.is_none() {
            if let Some(character) = state.infer_character() {
                let deck_ids: Vec<String> = state.deck.iter().map(|c| c.id.clone()).collect();
                let relic_ids: Vec<String> = state.relics.iter().map(|r| r.id.clone()).collect();
                let potion_ids: Vec<String> = state
                    .potions
                    .iter()
                    .filter_map(|p| p.as_ref().map(|p| p.id.clone()))
                    .collect();
                state.reward_pools =
                    Some(RewardPools::from_observed(character, &deck_ids, &relic_ids, &potion_ids));
            }
        }

        Ok(state)
    }

    /// Determinize all unordered pools by shuffling with the given seed.
    /// After this call, all pools are Ordered and can be drawn from.
    pub fn determinize(&mut self, seed: u64) {
        if let Some(pools) = &mut self.reward_pools {
            let mut rng = seed;
            pools.determinize(&mut |items| {
                // Fisher-Yates shuffle with LCG
                for i in (1..items.len()).rev() {
                    rng = rng
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    let j = (rng >> 33) as usize % (i + 1);
                    items.swap(i, j);
                }
            });
        }
    }

    /// Infer the character from starter relics.
    fn infer_character(&self) -> Option<Character> {
        for relic in &self.relics {
            match relic.id.as_str() {
                "BoardGame:BurningBlood" => return Some(Character::Ironclad),
                "BGRing of the Snake" => return Some(Character::Silent),
                "BGCrackedCore" => return Some(Character::Defect),
                "BoardGame:BGMiracles" => return Some(Character::Watcher),
                _ => {}
            }
        }
        None
    }

    pub fn apply(&mut self, action: &Action) {
        match action {
            Action::PickNeowBlessing { drawback, reward_type, .. }
            | Action::PickEventOption { drawback, reward_type, .. } => {
                // Apply drawback
                if let Some(db) = drawback {
                    match db.as_str() {
                        "LOSE_HP" => {
                            self.hp = self.hp.saturating_sub(2);
                        }
                        "LOSE_3_HP" => {
                            self.hp = self.hp.saturating_sub(3);
                        }
                        "LOSE_GOLD" => {
                            self.gold = self.gold.saturating_sub(3);
                        }
                        "CURSE" => {
                            if let Some(pools) = &mut self.reward_pools {
                                if let Some(id) = pools.curse_deck.draw() {
                                    self.deck.push(Card {
                                        id: id.clone(),
                                        name: id,
                                        cost: -2,
                                        card_type: "CURSE".to_string(),
                                        upgraded: false,
                                    });
                                }
                            }
                        }
                        "NONE" | _ => {}
                    }
                }
                // Apply reward
                if let Some(rt) = reward_type {
                    match rt.as_str() {
                        "FOUR_GOLD" => self.gold += 4,
                        "FIVE_GOLD" => self.gold += 5,
                        "EIGHT_GOLD" => self.gold += 8,
                        "TEN_GOLD" => self.gold += 10,
                        "REMOVE_CARD" => {
                            let cards = self.purgeable_cards();
                            self.screen = Screen::Grid {
                                purpose: "purge".to_string(),
                                cards,
                            };
                        }
                        "REMOVE_TWO" => {
                            let cards = self.purgeable_cards();
                            self.screen = Screen::Grid {
                                purpose: "purge".to_string(),
                                cards,
                            };
                            // TODO: need to track that 2 cards must be selected
                        }
                        "TRANSFORM_CARD" => {
                            let cards = self.transformable_cards();
                            self.screen = Screen::Grid {
                                purpose: "transform".to_string(),
                                cards,
                            };
                        }
                        "TRANSFORM_TWO_CARDS" => {
                            let cards = self.transformable_cards();
                            self.screen = Screen::Grid {
                                purpose: "transform".to_string(),
                                cards,
                            };
                            // TODO: need to track that 2 cards must be selected
                        }
                        "UPGRADE_CARD" => {
                            let cards = self.upgradeable_cards();
                            self.screen = Screen::Grid {
                                purpose: "upgrade".to_string(),
                                cards,
                            };
                        }
                        "UPGRADE_TWO_RANDOM" => {
                            // Upgrade 2 random upgradeable cards in deck
                            let upgradeable: Vec<usize> = self.deck.iter()
                                .enumerate()
                                .filter(|(_, c)| !c.upgraded && c.card_type != "CURSE" && c.card_type != "STATUS")
                                .map(|(i, _)| i)
                                .collect();
                            // Pick up to 2 (deterministic based on deck order for now)
                            for &idx in upgradeable.iter().take(2) {
                                self.deck[idx].upgraded = true;
                            }
                        }
                        "CHOOSE_A_CARD" => {
                            let cards = if let Some(pools) = &mut self.reward_pools {
                                pools.draw_card_reward(3)
                            } else {
                                vec![]
                            };
                            self.screen = Screen::CardReward { cards };
                        }
                        "CHOOSE_RARE_CARD" => {
                            // TODO: draw from rare deck instead
                            let cards = if let Some(pools) = &mut self.reward_pools {
                                pools.draw_card_reward(3)
                            } else {
                                vec![]
                            };
                            self.screen = Screen::CardReward { cards };
                        }
                        "CHOOSE_COLORLESS_CARD" => {
                            let cards = if let Some(pools) = &mut self.reward_pools {
                                (0..3).filter_map(|_| {
                                    let id = pools.colorless_deck.draw()?;
                                    Some(Card { id: id.clone(), name: id, cost: 0, card_type: "UNKNOWN".to_string(), upgraded: false })
                                }).collect()
                            } else {
                                vec![]
                            };
                            self.screen = Screen::CardReward { cards };
                        }
                        "CHOOSE_TWO_CARDS" | "CHOOSE_TWO_COLORLESS_CARDS" | "CARD_GOLD_COMBO" => {
                            if rt.as_str() == "CARD_GOLD_COMBO" {
                                self.gold += 5;
                            }
                            // TODO: transition to combat reward screen with card rewards
                            let cards = if let Some(pools) = &mut self.reward_pools {
                                pools.draw_card_reward(3)
                            } else {
                                vec![]
                            };
                            self.screen = Screen::CardReward { cards };
                        }
                        "GET_TWO_RANDOM_CARDS" => {
                            if let Some(pools) = &mut self.reward_pools {
                                let c1 = pools.draw_card_reward(1).pop();
                                let c2 = pools.draw_card_reward(1).pop();
                                if let Some(card) = c1 { self.deck.push(card); }
                                if let Some(card) = c2 { self.deck.push(card); }
                            }
                        }
                        "GET_TWO_RANDOM_COLORLESS_CARDS" => {
                            if let Some(pools) = &mut self.reward_pools {
                                for _ in 0..2 {
                                    if let Some(id) = pools.colorless_deck.draw() {
                                        self.deck.push(Card { id: id.clone(), name: id, cost: 0, card_type: "UNKNOWN".to_string(), upgraded: false });
                                    }
                                }
                            }
                        }
                        "RANDOM_RARE_CARD" => {
                            if let Some(pools) = &mut self.reward_pools {
                                if let Some(id) = pools.rare_deck.draw() {
                                    self.deck.push(Card { id: id.clone(), name: id, cost: 0, card_type: "UNKNOWN".to_string(), upgraded: false });
                                }
                            }
                        }
                        "THREE_POTIONS" => {
                            if let Some(pools) = &mut self.reward_pools {
                                let rewards: Vec<crate::screen::Reward> = (0..3).filter_map(|_| {
                                    let id = pools.potion_deck.draw()?;
                                    Some(crate::screen::Reward {
                                        reward_type: "POTION".to_string(),
                                        gold: None,
                                        relic: None,
                                        potion: Some(Potion { id: id.clone(), name: id }),
                                    })
                                }).collect();
                                self.screen = Screen::CombatRewards { rewards };
                            }
                        }
                        "RELIC" => {
                            if let Some(pools) = &mut self.reward_pools {
                                if let Some(relic_id) = pools.draw_relic() {
                                    self.relics.push(Relic {
                                        id: relic_id.clone(),
                                        name: relic_id,
                                        counter: -1,
                                        clickable: false,
                                        pulsing: false,
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Action::PickGridCard { card, .. } => {
                match &self.screen {
                    Screen::Grid { purpose, .. } => {
                        let purpose = purpose.clone();
                        match purpose.as_str() {
                            "purge" => {
                                if let Some(idx) = self.deck.iter().position(|c| c == card) {
                                    self.deck.remove(idx);
                                }
                                self.screen = Screen::Complete;
                            }
                            "transform" => {
                                if let Some(idx) = self.deck.iter().position(|c| c == card) {
                                    self.deck.remove(idx);
                                }
                                // TODO: add a random replacement card (needs card pool)
                                self.screen = Screen::Complete;
                            }
                            "upgrade" => {
                                if let Some(c) = self.deck.iter_mut().find(|c| *c == card) {
                                    c.upgraded = true;
                                }
                                self.screen = Screen::Complete;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Action::TakeReward { choice_index, .. } => {
                if let Screen::CombatRewards { rewards } = &self.screen {
                    let idx = *choice_index as usize;
                    if idx < rewards.len() {
                        let reward = &rewards[idx];
                        match reward.reward_type.as_str() {
                            "GOLD" => {
                                if let Some(gold) = reward.gold {
                                    self.gold += gold;
                                }
                            }
                            "POTION" => {
                                if let Some(potion) = &reward.potion {
                                    // Add to first empty potion slot
                                    if let Some(slot) = self.potions.iter_mut().find(|p| p.is_none()) {
                                        *slot = Some(potion.clone());
                                    }
                                }
                            }
                            "RELIC" => {
                                if let Some(relic) = &reward.relic {
                                    self.relics.push(relic.clone());
                                }
                            }
                            "CARD" => {
                                // Opens card reward sub-screen
                                // TODO: draw cards from reward deck
                            }
                            _ => {}
                        }
                        // Remove the taken reward
                        let mut new_rewards = rewards.clone();
                        new_rewards.remove(idx);
                        if new_rewards.is_empty() {
                            self.screen = Screen::Complete;
                        } else {
                            self.screen = Screen::CombatRewards { rewards: new_rewards };
                        }
                    }
                }
            }
            Action::TakeCard { card, .. } => {
                self.deck.push(card.clone());
                self.screen = Screen::Complete;
            }
            Action::SkipCardReward => {
                self.screen = Screen::Complete;
            }
            Action::Rest { .. } => {
                // Heal 30% of max HP (rounded down), capped at max_hp
                let heal = self.max_hp / 3;
                self.hp = (self.hp + heal).min(self.max_hp);
                self.screen = Screen::Complete;
            }
            Action::Smith { .. } => {
                // Open grid screen to upgrade a card
                let cards = self.upgradeable_cards();
                self.screen = Screen::Grid {
                    purpose: "upgrade".to_string(),
                    cards,
                };
            }
            Action::BuyCard { card, price, .. } => {
                if self.gold >= *price {
                    self.gold -= price;
                    self.deck.push(card.clone());
                    // Remove card from shop
                    if let Screen::Shop { cards, .. } = &mut self.screen {
                        cards.retain(|c| c.card != *card);
                    }
                }
            }
            Action::BuyRelic { relic, price, .. } => {
                if self.gold >= *price {
                    self.gold -= price;
                    self.relics.push(Relic {
                        id: relic.clone(),
                        name: relic.clone(),
                        counter: -1,
                        clickable: false,
                        pulsing: false,
                    });
                    if let Screen::Shop { relics, .. } = &mut self.screen {
                        relics.retain(|r| r.id != *relic);
                    }
                }
            }
            Action::BuyPotion { potion, price, .. } => {
                if self.gold >= *price {
                    if let Some(slot) = self.potions.iter_mut().find(|p| p.is_none()) {
                        self.gold -= price;
                        *slot = Some(Potion {
                            id: potion.clone(),
                            name: potion.clone(),
                        });
                    }
                    if let Screen::Shop { potions, .. } = &mut self.screen {
                        potions.retain(|p| p.id != *potion);
                    }
                }
            }
            Action::Purge { price, .. } => {
                if self.gold >= *price {
                    self.gold -= price;
                    let cards = self.purgeable_cards();
                    self.screen = Screen::Grid {
                        purpose: "purge".to_string(),
                        cards,
                    };
                }
            }
            Action::LeaveShop => {
                self.screen = Screen::Complete;
            }
            Action::PickBossRelic { choice_index, .. } => {
                if let Screen::BossRelic { relics } = &self.screen {
                    let idx = *choice_index as usize;
                    if idx < relics.len() {
                        let relic = relics[idx].clone();
                        self.relics.push(relic);
                        // Remove all offered boss relics from the pool
                        if let Some(pools) = &mut self.reward_pools {
                            for r in relics {
                                pools.boss_relic_deck.remove(&r.id);
                            }
                        }
                        self.screen = Screen::Complete;
                    }
                }
            }
            Action::SkipBossRelic => {
                // Remove all offered boss relics from the pool even if skipped
                if let Screen::BossRelic { relics } = &self.screen {
                    if let Some(pools) = &mut self.reward_pools {
                        for r in relics {
                            pools.boss_relic_deck.remove(&r.id);
                        }
                    }
                }
                self.screen = Screen::Complete;
            }
            Action::TravelTo { .. } => {
                // TODO: advance floor, enter room
            }
            Action::Proceed => {
                match &self.screen {
                    Screen::CombatRewards { .. } | Screen::Complete | Screen::ShopRoom => {
                        self.screen = Screen::Complete;
                    }
                    Screen::GameOver { .. } => {
                        // Stay on game over
                    }
                    _ => {
                        self.screen = Screen::Complete;
                    }
                }
            }
            _ => {}
        }
    }

    fn purgeable_cards(&self) -> Vec<Card> {
        self.deck
            .iter()
            .filter(|c| c.card_type != "CURSE" || c.id != "AscendersBane")
            .cloned()
            .collect()
    }

    fn transformable_cards(&self) -> Vec<Card> {
        self.purgeable_cards()
    }

    fn upgradeable_cards(&self) -> Vec<Card> {
        self.deck
            .iter()
            .filter(|c| !c.upgraded && c.card_type != "CURSE" && c.card_type != "STATUS")
            .cloned()
            .collect()
    }

    pub fn available_actions(&self) -> Vec<Action> {
        match &self.screen {
            Screen::Neow { options } => neow_actions(options),
            Screen::Event { options, .. } => event_actions(options),
            Screen::Map { available_nodes } => map_actions(available_nodes),
            Screen::CardReward { cards } => card_reward_actions(cards),
            Screen::CombatRewards { rewards } => combat_reward_actions(rewards),
            Screen::Grid { cards, .. } => grid_actions(cards),
            Screen::Rest { options } => rest_actions(options),
            Screen::Shop { cards, relics, potions, purge_cost } => {
                let has_potion_slot = self.potions.iter().any(|p| p.is_none());
                shop_actions(cards, relics, potions, *purge_cost, self.gold, has_potion_slot)
            }
            Screen::BossRelic { relics } => boss_relic_actions(relics),
            Screen::Complete | Screen::ShopRoom => vec![Action::Proceed],
            Screen::GameOver { .. } => vec![Action::Proceed],
            Screen::Treasure => vec![Action::OpenChest { choice_index: 0 }],
            _ => vec![],
        }
    }
}

fn neow_actions(options: &[EventOption]) -> Vec<Action> {
    options
        .iter()
        .enumerate()
        .filter(|(_, opt)| !opt.disabled)
        .map(|(i, opt)| Action::PickNeowBlessing {
            label: opt.label.clone(),
            choice_index: i as u8,
            reward_type: opt.reward_type.clone(),
            drawback: opt.drawback.clone(),
        })
        .collect()
}

fn event_actions(options: &[EventOption]) -> Vec<Action> {
    options
        .iter()
        .enumerate()
        .filter(|(_, opt)| !opt.disabled)
        .map(|(i, opt)| Action::PickEventOption {
            label: opt.label.clone(),
            choice_index: i as u8,
            reward_type: opt.reward_type.clone(),
            drawback: opt.drawback.clone(),
        })
        .collect()
}

fn map_actions(nodes: &[crate::screen::MapChoice]) -> Vec<Action> {
    nodes
        .iter()
        .enumerate()
        .map(|(i, node)| Action::TravelTo {
            kind: node.kind,
            label: node.label.clone(),
            choice_index: i as u8,
        })
        .collect()
}

fn card_reward_actions(cards: &[Card]) -> Vec<Action> {
    let mut actions: Vec<Action> = cards
        .iter()
        .enumerate()
        .map(|(i, card)| Action::TakeCard {
            card: card.clone(),
            choice_index: i as u8,
        })
        .collect();
    actions.push(Action::SkipCardReward);
    actions
}

fn combat_reward_actions(rewards: &[crate::screen::Reward]) -> Vec<Action> {
    let mut actions: Vec<Action> = rewards
        .iter()
        .enumerate()
        .map(|(i, _)| Action::TakeReward {
            choice_index: i as u8,
        })
        .collect();
    actions.push(Action::Proceed);
    actions
}

fn rest_actions(options: &[String]) -> Vec<Action> {
    options
        .iter()
        .enumerate()
        .map(|(i, opt)| match opt.as_str() {
            "rest" => Action::Rest { choice_index: i as u8 },
            "smith" => Action::Smith { choice_index: i as u8 },
            _ => Action::Rest { choice_index: i as u8 },
        })
        .collect()
}

fn shop_actions(
    cards: &[ShopCard],
    relics: &[ShopRelic],
    potions: &[ShopPotion],
    purge_cost: Option<u16>,
    gold: u16,
    has_potion_slot: bool,
) -> Vec<Action> {
    let mut actions = Vec::new();
    for (i, sc) in cards.iter().enumerate() {
        if let Some(price) = sc.price {
            if gold >= price {
                actions.push(Action::BuyCard {
                    card: sc.card.clone(),
                    price,
                    choice_index: i as u8,
                });
            }
        }
    }
    for (i, sr) in relics.iter().enumerate() {
        if let Some(price) = sr.price {
            if gold >= price {
                actions.push(Action::BuyRelic {
                    relic: sr.id.clone(),
                    price,
                    choice_index: (cards.len() + i) as u8,
                });
            }
        }
    }
    if has_potion_slot {
        for (i, sp) in potions.iter().enumerate() {
            if let Some(price) = sp.price {
                if gold >= price {
                    actions.push(Action::BuyPotion {
                        potion: sp.id.clone(),
                        price,
                        choice_index: (cards.len() + relics.len() + i) as u8,
                    });
                }
            }
        }
    }
    if let Some(price) = purge_cost {
        if gold >= price {
            actions.push(Action::Purge {
                price,
                choice_index: (cards.len() + relics.len() + potions.len()) as u8,
            });
        }
    }
    actions.push(Action::LeaveShop);
    actions
}

fn boss_relic_actions(relics: &[crate::types::Relic]) -> Vec<Action> {
    let mut actions: Vec<Action> = relics
        .iter()
        .enumerate()
        .map(|(i, _)| Action::PickBossRelic {
            choice_index: i as u8,
        })
        .collect();
    actions.push(Action::SkipBossRelic);
    actions
}

fn grid_actions(cards: &[Card]) -> Vec<Action> {
    cards
        .iter()
        .enumerate()
        .map(|(i, card)| Action::PickGridCard {
            card: card.clone(),
            choice_index: i as u8,
        })
        .collect()
}
