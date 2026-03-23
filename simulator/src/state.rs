use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::reward_deck::{self, Character, RewardDeck};
use crate::screen::{EventOption, Screen};
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
    pub card_deck: RewardDeck,
    pub rare_deck: RewardDeck,
    pub relic_deck: Vec<String>,
    pub boss_relic_deck: Vec<String>,
    pub potion_deck: RewardDeck,
    pub curse_deck: RewardDeck,
    pub colorless_deck: RewardDeck,
}

impl RewardPools {
    pub fn new(character: Character, seed: u64) -> Self {
        RewardPools {
            card_deck: RewardDeck::new(character, seed),
            rare_deck: reward_deck::build_rare_deck(character, seed.wrapping_add(1)),
            relic_deck: reward_deck::build_relic_deck(seed.wrapping_add(2)),
            boss_relic_deck: reward_deck::build_boss_relic_deck(seed.wrapping_add(3)),
            potion_deck: reward_deck::build_potion_deck(seed.wrapping_add(4)),
            curse_deck: reward_deck::build_curse_deck(seed.wrapping_add(5)),
            colorless_deck: reward_deck::build_colorless_deck(seed.wrapping_add(6)),
        }
    }

    /// Draw N cards from the reward deck for a card reward screen.
    pub fn draw_card_reward(&mut self, count: usize) -> Vec<Card> {
        (0..count)
            .map(|_| {
                let id = self.card_deck.draw().to_string();
                Card {
                    id: id.clone(),
                    name: id,
                    cost: 0, // TODO: look up actual cost
                    card_type: "UNKNOWN".to_string(),
                    upgraded: false,
                }
            })
            .collect()
    }

    /// Draw the next relic from the relic deck.
    pub fn draw_relic(&mut self) -> Option<String> {
        if self.relic_deck.is_empty() {
            None
        } else {
            Some(self.relic_deck.remove(0))
        }
    }
}

impl GameState {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
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
                                let id = pools.curse_deck.draw().to_string();
                                self.deck.push(Card {
                                    id: id.clone(),
                                    name: id,
                                    cost: -2,
                                    card_type: "CURSE".to_string(),
                                    upgraded: false,
                                });
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
                                (0..3).map(|_| {
                                    let id = pools.colorless_deck.draw().to_string();
                                    Card { id: id.clone(), name: id, cost: 0, card_type: "UNKNOWN".to_string(), upgraded: false }
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
                                    let id = pools.colorless_deck.draw().to_string();
                                    self.deck.push(Card { id: id.clone(), name: id, cost: 0, card_type: "UNKNOWN".to_string(), upgraded: false });
                                }
                            }
                        }
                        "RANDOM_RARE_CARD" => {
                            if let Some(pools) = &mut self.reward_pools {
                                let id = pools.rare_deck.draw().to_string();
                                self.deck.push(Card { id: id.clone(), name: id, cost: 0, card_type: "UNKNOWN".to_string(), upgraded: false });
                            }
                        }
                        "THREE_POTIONS" => {
                            if let Some(pools) = &mut self.reward_pools {
                                let rewards: Vec<crate::screen::Reward> = (0..3).map(|_| {
                                    let id = pools.potion_deck.draw().to_string();
                                    crate::screen::Reward {
                                        reward_type: "POTION".to_string(),
                                        gold: None,
                                        relic: None,
                                        potion: Some(Potion { id: id.clone(), name: id }),
                                    }
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
                                self.deck.retain(|c| c != card);
                                self.screen = Screen::Complete;
                            }
                            "transform" => {
                                self.deck.retain(|c| c != card);
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
