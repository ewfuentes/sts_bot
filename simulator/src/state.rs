use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::action::Action;
use crate::card_db;
use crate::effects::{DamageSource, Effect, EffectTarget, HandFilter, HandSelectAction, Pile};
use crate::map::{ActMap, MapNodeKind};
use crate::pool::Pool;
use crate::reward_deck::{self, Character, RewardDeck};
use crate::screen::{EventOption, HandCard, Screen, ShopCard, ShopPotion, ShopRelic};
use crate::types::{Card, Monster, Potion, Relic};

fn deserialize_screen_stack<'de, D>(deserializer: D) -> Result<Vec<Screen>, D::Error>
where
    D: Deserializer<'de>,
{
    let screen = Screen::deserialize(deserializer)?;
    Ok(vec![screen])
}

fn serialize_screen_stack<S>(stack: &Vec<Screen>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let screen = stack.last().cloned().unwrap_or(Screen::Complete);
    screen.serialize(serializer)
}

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
    #[serde(default)]
    pub map: Option<ActMap>,
    #[serde(
        deserialize_with = "deserialize_screen_stack",
        serialize_with = "serialize_screen_stack"
    )]
    pub screen: Vec<Screen>,
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

    pub fn current_screen(&self) -> &Screen {
        static COMPLETE: Screen = Screen::Complete;
        self.screen.last().unwrap_or(&COMPLETE)
    }

    pub fn current_screen_mut(&mut self) -> &mut Screen {
        if self.screen.is_empty() {
            self.screen.push(Screen::Complete);
        }
        self.screen.last_mut().unwrap()
    }

    pub fn set_screen(&mut self, s: Screen) {
        if self.screen.is_empty() {
            self.screen.push(s);
        } else {
            let last = self.screen.len() - 1;
            self.screen[last] = s;
        }
    }

    /// Find the Combat screen in the stack (it may not be the top screen
    /// if sub-decision screens like HandSelect or DiscardSelect are above it).
    fn find_combat_mut(&mut self) -> Option<&mut Screen> {
        self.screen.iter_mut().rev().find(|s| matches!(s, Screen::Combat { .. }))
    }

    pub fn push_screen(&mut self, s: Screen) {
        self.screen.push(s);
    }

    pub fn pop_screen(&mut self) {
        if self.screen.len() > 1 {
            self.screen.pop();
        } else {
            self.set_screen(Screen::Complete);
        }
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
                            self.set_screen(Screen::Grid {
                                purpose: "purge".to_string(),
                                cards,
                            });
                        }
                        "REMOVE_TWO" => {
                            let cards = self.purgeable_cards();
                            self.set_screen(Screen::Grid {
                                purpose: "purge".to_string(),
                                cards,
                            });
                            // TODO: need to track that 2 cards must be selected
                        }
                        "TRANSFORM_CARD" => {
                            let cards = self.transformable_cards();
                            self.set_screen(Screen::Grid {
                                purpose: "transform".to_string(),
                                cards,
                            });
                        }
                        "TRANSFORM_TWO_CARDS" => {
                            let cards = self.transformable_cards();
                            self.set_screen(Screen::Grid {
                                purpose: "transform".to_string(),
                                cards,
                            });
                            // TODO: need to track that 2 cards must be selected
                        }
                        "UPGRADE_CARD" => {
                            let cards = self.upgradeable_cards();
                            self.set_screen(Screen::Grid {
                                purpose: "upgrade".to_string(),
                                cards,
                            });
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
                            self.set_screen(Screen::CardReward { cards });
                        }
                        "CHOOSE_RARE_CARD" => {
                            // TODO: draw from rare deck instead
                            let cards = if let Some(pools) = &mut self.reward_pools {
                                pools.draw_card_reward(3)
                            } else {
                                vec![]
                            };
                            self.set_screen(Screen::CardReward { cards });
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
                            self.set_screen(Screen::CardReward { cards });
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
                            self.set_screen(Screen::CardReward { cards });
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
                                self.set_screen(Screen::CombatRewards { rewards });
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
                if let Screen::Grid { purpose, .. } = self.current_screen() {
                    let purpose = purpose.clone();
                    match purpose.as_str() {
                        "purge" => {
                            if let Some(idx) = self.deck.iter().position(|c| c == card) {
                                self.deck.remove(idx);
                            }
                            self.pop_screen();
                        }
                        "transform" => {
                            if let Some(idx) = self.deck.iter().position(|c| c == card) {
                                self.deck.remove(idx);
                            }
                            // TODO: add a random replacement card (needs card pool)
                            self.pop_screen();
                        }
                        "upgrade" => {
                            if let Some(c) = self.deck.iter_mut().find(|c| *c == card) {
                                c.upgraded = true;
                            }
                            self.pop_screen();
                        }
                        _ => {}
                    }
                }
            }
            Action::TakeReward { choice_index, .. } => {
                if let Screen::CombatRewards { rewards } = self.current_screen() {
                    let idx = *choice_index as usize;
                    if idx < rewards.len() {
                        let reward = rewards[idx].clone();
                        let mut taken = true;
                        match reward.reward_type.as_str() {
                            "GOLD" => {
                                if let Some(gold) = reward.gold {
                                    self.gold += gold;
                                }
                            }
                            "POTION" => {
                                if let Some(potion) = &reward.potion {
                                    if let Some(slot) = self.potions.iter_mut().find(|p| p.is_none()) {
                                        *slot = Some(potion.clone());
                                    } else {
                                        taken = false;
                                    }
                                }
                            }
                            "RELIC" => {
                                if let Some(relic) = &reward.relic {
                                    self.relics.push(relic.clone());
                                }
                            }
                            "CARD" | "UPGRADED_CARD" | "RARE_CARD" => {
                                // Open card selection sub-screen; reward removed when card is taken
                                let cards = if let Some(pools) = &mut self.reward_pools {
                                    match reward.reward_type.as_str() {
                                        "RARE_CARD" => {
                                            (0..3).filter_map(|_| {
                                                let id = pools.rare_deck.draw()?;
                                                Some(Card { id: id.clone(), name: id, cost: 0, card_type: "UNKNOWN".to_string(), upgraded: false })
                                            }).collect()
                                        }
                                        "UPGRADED_CARD" => {
                                            let mut cards = pools.draw_card_reward(3);
                                            for c in &mut cards {
                                                c.upgraded = true;
                                            }
                                            cards
                                        }
                                        _ => pools.draw_card_reward(3),
                                    }
                                } else {
                                    vec![]
                                };
                                self.push_screen(Screen::CardReward { cards });
                                taken = false; // don't remove yet
                            }
                            _ => {}
                        }
                        if taken {
                            self.remove_reward(idx);
                        }
                    }
                }
            }
            Action::TakeCard { card, .. } => {
                // Remove taken card from reward pool (permanently gone)
                if let Some(pools) = &mut self.reward_pools {
                    pools.card_deck.remove(&card.id);
                    pools.rare_deck.remove(&card.id);
                    pools.colorless_deck.remove(&card.id);
                }
                self.deck.push(card.clone());

                match self.current_screen() {
                    Screen::BossRelic { .. } => {
                        // Clear cards from boss relic screen, check if done
                        if let Screen::BossRelic { relics, cards } = self.current_screen_mut() {
                            cards.clear();
                            if relics.is_empty() {
                                self.pop_screen();
                            }
                        }
                    }
                    Screen::CardReward { .. } => {
                        self.pop_screen();
                        // Remove CARD reward from CombatRewards if present underneath
                        if let Screen::CombatRewards { rewards } = self.current_screen() {
                            if let Some(idx) = rewards.iter().position(|r|
                                matches!(r.reward_type.as_str(), "CARD" | "UPGRADED_CARD" | "RARE_CARD")
                            ) {
                                self.remove_reward(idx);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Action::SkipCardReward => {
                match self.current_screen() {
                    Screen::BossRelic { .. } => {
                        // Clear cards, check if done
                        if let Screen::BossRelic { relics, cards } = self.current_screen_mut() {
                            cards.clear();
                            if relics.is_empty() {
                                self.pop_screen();
                            }
                        }
                    }
                    _ => {
                        self.pop_screen();
                    }
                }
            }
            Action::Rest { .. } => {
                // Heal 30% of max HP (rounded down), capped at max_hp
                let heal = self.max_hp / 3;
                self.hp = (self.hp + heal).min(self.max_hp);
                self.set_screen(Screen::Complete);
            }
            Action::Smith { .. } => {
                // Open grid screen to upgrade a card
                let cards = self.upgradeable_cards();
                self.set_screen(Screen::Grid {
                    purpose: "upgrade".to_string(),
                    cards,
                });
            }
            Action::BuyCard { card, price, .. } => {
                if self.gold >= *price {
                    self.gold -= price;
                    self.deck.push(card.clone());
                    // Remove card from shop
                    if let Screen::Shop { cards, .. } = self.current_screen_mut() {
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
                    if let Screen::Shop { relics, .. } = self.current_screen_mut() {
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
                    if let Screen::Shop { potions, .. } = self.current_screen_mut() {
                        potions.retain(|p| p.id != *potion);
                    }
                }
            }
            Action::Purge { price, .. } => {
                if self.gold >= *price {
                    self.gold -= price;
                    // Mark purge as used (set cost to None so it won't be offered again)
                    if let Screen::Shop { purge_cost, .. } = self.current_screen_mut() {
                        *purge_cost = None;
                    }
                    let cards = self.purgeable_cards();
                    self.push_screen(Screen::Grid {
                        purpose: "purge".to_string(),
                        cards,
                    });
                }
            }
            Action::LeaveShop => {
                self.set_screen(Screen::Complete);
            }
            Action::PickBossRelic { choice_index, .. } => {
                if let Screen::BossRelic { relics, .. } = self.current_screen() {
                    let idx = *choice_index as usize;
                    let relics = relics.clone();
                    if idx < relics.len() {
                        self.relics.push(relics[idx].clone());
                        // Remove all offered boss relics from the pool
                        if let Some(pools) = &mut self.reward_pools {
                            for r in &relics {
                                pools.boss_relic_deck.remove(&r.id);
                            }
                        }
                        // Clear relics from screen, check if done
                        if let Screen::BossRelic { relics, cards } = self.current_screen_mut() {
                            relics.clear();
                            if cards.is_empty() {
                                self.pop_screen();
                            }
                        }
                    }
                }
            }
            Action::SkipBossRelic => {
                // Remove all offered boss relics from the pool even if skipped
                if let Screen::BossRelic { relics, .. } = self.current_screen() {
                    let relics = relics.clone();
                    if let Some(pools) = &mut self.reward_pools {
                        for r in &relics {
                            pools.boss_relic_deck.remove(&r.id);
                        }
                    }
                }
                self.pop_screen();
            }
            Action::OpenChest { .. } => {
                // Draw a relic from the relic pool
                if let Some(pools) = &mut self.reward_pools {
                    if let Some(id) = pools.draw_relic() {
                        self.relics.push(Relic {
                            id: id.clone(),
                            name: id,
                            counter: -1,
                            clickable: false,
                            pulsing: false,
                        });
                    }
                }
                self.set_screen(Screen::Complete);
            }
            Action::TravelTo { kind, .. } => {
                self.floor += 1;
                let kind = *kind;
                let screen = match kind {
                    MapNodeKind::Monster => Screen::new_combat("UNKNOWN_MONSTER"),
                    MapNodeKind::Elite => Screen::new_combat("UNKNOWN_ELITE"),
                    MapNodeKind::Boss => Screen::new_combat("UNKNOWN_BOSS"),
                    MapNodeKind::Rest => Screen::Rest {
                        options: vec!["rest".to_string(), "smith".to_string()],
                    },
                    MapNodeKind::Shop => self.generate_shop(),
                    MapNodeKind::Treasure => Screen::Treasure,
                    MapNodeKind::Event | MapNodeKind::Unknown => Screen::Complete,
                };
                self.set_screen(screen);
            }
            Action::PlayCard { hand_index, target_index, .. } => {
                let hand_idx = *hand_index as usize;

                let target = *target_index;

                if let Some(Screen::Combat {
                    hand, player_energy, player_powers, monsters, effect_queue, ..
                }) = self.screen.last_mut()
                {
                    if hand_idx >= hand.len() {
                        return;
                    }
                    let hc = hand.remove(hand_idx);
                    let card = hc.card;
                    let info = card_db::lookup(&card.id);

                    // Deduct energy
                    let cost = info
                        .map(|i| i.effective_cost(card.upgraded))
                        .unwrap_or(card.cost);
                    if cost >= 0 {
                        *player_energy = player_energy.saturating_sub(cost as u8);
                    }

                    // Queue card effects with target
                    let effects = info
                        .map(|i| i.effective_effects(card.upgraded))
                        .unwrap_or(&[]);
                    for effect in effects {
                        effect_queue.push_back((effect.clone(), target));
                    }

                    // Tick down Weak/Vulnerable after an Attack card resolves.
                    // Snapshot whether these powers exist now, so we don't tick
                    // down stacks that the card itself applies.
                    let is_attack = info
                        .map(|i| i.card_type == card_db::CardType::Attack)
                        .unwrap_or(card.card_type == "ATTACK");
                    if is_attack {
                        let had_weak = player_powers.iter().any(|p| p.id == "BGWeakened");
                        let mut vuln_mask: u8 = 0;
                        match target {
                            Some(t) => {
                                if let Some(m) = monsters.get(t as usize) {
                                    if m.powers.iter().any(|p| p.id == "BGVulnerable") {
                                        vuln_mask |= 1 << t;
                                    }
                                }
                            }
                            None => {
                                for (i, m) in monsters.iter().enumerate() {
                                    if m.powers.iter().any(|p| p.id == "BGVulnerable") {
                                        vuln_mask |= 1 << i;
                                    }
                                }
                            }
                        }
                        effect_queue.push_back((Effect::TickDownAttackPowers { had_weak, vuln_mask }, target));
                    }

                    // Queue disposition as the final effect (after all card effects).
                    // Powers are consumed — no disposition needed.
                    let is_power = info
                        .map(|i| i.card_type == card_db::CardType::Power)
                        .unwrap_or(card.card_type == "POWER");
                    if !is_power {
                        let does_exhaust = info
                            .map(|i| i.does_exhaust(card.upgraded))
                            .unwrap_or(false);
                        let does_rebound = info
                            .map(|i| i.rebound)
                            .unwrap_or(false);
                        effect_queue.push_back((Effect::DisposeCard {
                            card,
                            exhaust: does_exhaust,
                            rebound: does_rebound,
                        }, None));
                    }
                }

                // Drain the effect queue
                self.drain_effect_queue();
            }
            Action::EndTurn => {
                if let Screen::Combat {
                    hand, draw_pile, discard_pile, exhaust_pile,
                    player_block, player_energy, turn, effect_queue, ..
                } = self.current_screen_mut()
                {
                    // 1. Discard hand (ethereal → exhaust)
                    let hand_cards: Vec<HandCard> = hand.drain(..).collect();
                    for hc in hand_cards {
                        let is_ethereal = card_db::lookup(&hc.card.id)
                            .map(|i| i.is_ethereal(hc.card.upgraded))
                            .unwrap_or(false);
                        if is_ethereal {
                            exhaust_card(hc.card, exhaust_pile, effect_queue);
                        } else {
                            discard_pile.push(hc.card);
                        }
                    }

                    // 2. [STUB] Monster turns would go here

                    // 3. Player block → 0
                    *player_block = 0;

                    // 4. [STUB] Turn-start power triggers would go here

                    // 5-6. Draw 5 cards (reshuffle if needed)
                    let draw_count = 5usize;
                    for _ in 0..draw_count {
                        if let Some(card) = draw_card(draw_pile, discard_pile) {
                            hand.push(HandCard { card });
                        }
                    }

                    // 7. Energy → 3
                    *player_energy = 3;

                    // 8. Turn += 1
                    *turn += 1;
                }
            }
            Action::PickHandCard { choice_index, .. } => {
                let idx = *choice_index as usize;
                let done = if let Screen::HandSelect {
                    cards, max_cards, picked_indices, ..
                } = self.current_screen_mut()
                {
                    if idx < cards.len() {
                        let (hand_idx, _) = cards.remove(idx);
                        picked_indices.push(hand_idx);
                    }
                    picked_indices.len() >= *max_cards as usize
                } else {
                    false
                };

                if done {
                    self.resolve_hand_select();
                    self.drain_effect_queue();
                }
            }
            Action::PickChoice { choice_index, .. } => {
                let idx = *choice_index as usize;
                if let Screen::ChoiceSelect { choices, target_index, energy_costs } = self.current_screen() {
                    if idx < choices.len() {
                        let effects = choices[idx].1.clone();
                        let target = *target_index;
                        let energy_cost = energy_costs.get(idx).copied();
                        self.pop_screen();
                        // Deduct energy if this choice has an energy cost (XCost)
                        if let Some(cost) = energy_cost {
                            if let Some(Screen::Combat { player_energy, .. }) = self.find_combat_mut() {
                                *player_energy = player_energy.saturating_sub(cost);
                            }
                        }
                        // Push chosen effects to the front of the queue
                        if let Some(Screen::Combat { effect_queue, .. }) = self.find_combat_mut() {
                            for effect in effects.into_iter().rev() {
                                effect_queue.push_front((effect, target));
                            }
                        }
                        self.drain_effect_queue();
                    }
                }
            }
            Action::PickDiscard { choice_index, .. } => {
                let idx = *choice_index as usize;
                if let Screen::DiscardSelect { cards } = self.current_screen() {
                    if idx < cards.len() {
                        let (discard_idx, _) = cards[idx];
                        self.pop_screen();
                        if let Some(Screen::Combat { discard_pile, draw_pile, .. }) = self.find_combat_mut() {
                            let discard_idx = discard_idx as usize;
                            if discard_idx < discard_pile.len() {
                                let card = discard_pile.remove(discard_idx);
                                draw_pile.push(card);
                            }
                        }
                        self.drain_effect_queue();
                    }
                }
            }
            Action::PickExhaust { choice_index, .. } => {
                let idx = *choice_index as usize;
                if let Screen::ExhaustSelect { cards } = self.current_screen() {
                    if idx < cards.len() {
                        let (exhaust_idx, _) = cards[idx];
                        self.pop_screen();
                        if let Some(Screen::Combat { exhaust_pile, hand, .. }) = self.find_combat_mut() {
                            let exhaust_idx = exhaust_idx as usize;
                            if exhaust_idx < exhaust_pile.len() {
                                let card = exhaust_pile.remove(exhaust_idx);
                                hand.push(HandCard { card });
                            }
                        }
                        self.drain_effect_queue();
                    }
                }
            }
            Action::PickTarget { target_index, .. } => {
                let target = Some(*target_index);
                if let Screen::TargetSelect { card: Some(card), effects, force_exhaust } = self.current_screen() {
                    let effects = effects.clone();
                    let card = card.clone();
                    let force_exhaust = *force_exhaust;
                    self.pop_screen();
                    if let Some(Screen::Combat { effect_queue, .. }) = self.find_combat_mut() {
                        for effect in effects.iter().rev() {
                            effect_queue.push_front((effect.clone(), target));
                        }
                        if force_exhaust {
                            effect_queue.push_back((Effect::DisposeCard {
                                card,
                                exhaust: true,
                                rebound: false,
                            }, None));
                        }
                        // Powers (force_exhaust=false) are consumed — no disposition needed.
                    }
                    self.drain_effect_queue();
                }
            }
            Action::Skip => {
                if matches!(self.current_screen(), Screen::Combat { .. }) {
                    self.finish_combat();
                }
                if matches!(self.current_screen(), Screen::HandSelect { .. }) {
                    self.resolve_hand_select();
                    self.drain_effect_queue();
                }
            }
            Action::DiscardPotion { slot } => {
                let idx = *slot as usize;
                if idx < self.potions.len() {
                    self.potions[idx] = None;
                }
            }
            Action::Proceed => {
                match self.current_screen() {
                    Screen::GameOver { .. } => {
                        // Stay on game over
                    }
                    _ => {
                        self.pop_screen();
                    }
                }
            }
            _ => {}
        }
    }

    /// Remove a reward by index from the current CombatRewards screen.
    fn remove_reward(&mut self, idx: usize) {
        if let Screen::CombatRewards { rewards } = self.current_screen() {
            let mut new_rewards = rewards.clone();
            new_rewards.remove(idx);
            if new_rewards.is_empty() {
                self.set_screen(Screen::Complete);
            } else {
                self.set_screen(Screen::CombatRewards { rewards: new_rewards });
            }
        }
    }

    /// Drain the effect queue on the current Combat screen.
    /// Executes effects until the queue is empty or one needs a sub-decision.
    /// `target_index` is the target from the original PlayCard action.
    fn drain_effect_queue(&mut self) {
        loop {
            // Pop next (effect, target) pair from queue
            let entry = if let Some(Screen::Combat { effect_queue, .. }) = self.find_combat_mut() {
                effect_queue.pop_front()
            } else {
                None
            };

            let Some((effect, target_index)) = entry else { break };

            match self.execute_effect(&effect, target_index) {
                EffectResult::Continue => {}
                EffectResult::Paused => return,
                EffectResult::CombatOver => break,
            }
        }

        // Finalize
        if let Some(Screen::Combat { monsters, .. }) = self.find_combat_mut() {
            if monsters.iter().all(|m| m.is_gone) {
                self.finish_combat();
            }
        }
    }

    /// Execute a single effect. Returns whether to continue draining,
    /// pause for a sub-decision, or stop because combat ended.
    fn execute_effect(&mut self, effect: &Effect, target_index: Option<u8>) -> EffectResult {
        match effect {
            Effect::Damage(amount) => {
                if let Some(idx) = target_index {
                    let idx = idx as usize;
                    if let Some(Screen::Combat { monsters, player_powers, .. }) = self.find_combat_mut() {
                        if idx < monsters.len() && !monsters[idx].is_gone {
                            let dmg = calculate_damage(*amount, player_powers, &monsters[idx].powers);
                            apply_damage_to_monster(&mut monsters[idx], dmg);
                        }
                    }
                }
            }
            Effect::DamageFixed(amount) => {
                // Same as Damage but won't scale with strength when that's implemented
                if let Some(idx) = target_index {
                    let idx = idx as usize;
                    if let Some(Screen::Combat { monsters, .. }) = self.find_combat_mut() {
                        if idx < monsters.len() && !monsters[idx].is_gone {
                            apply_damage_to_monster(&mut monsters[idx], *amount as u16);
                        }
                    }
                }
            }
            Effect::DamageAll(amount) => {
                if let Some(Screen::Combat { monsters, player_powers, .. }) = self.find_combat_mut() {
                    for monster in monsters.iter_mut() {
                        if !monster.is_gone {
                            let dmg = calculate_damage(*amount, player_powers, &monster.powers);
                            apply_damage_to_monster(monster, dmg);
                        }
                    }
                }
            }
            Effect::DamageBasedOn(source) => {
                if let Some(idx) = target_index {
                    let idx = idx as usize;
                    if let Some(Screen::Combat { monsters, player_block, exhaust_pile, hand, player_powers, .. }) = self.find_combat_mut() {
                        let base_amount = match source {
                            DamageSource::ExhaustPileSize => exhaust_pile.len() as i16,
                            DamageSource::CurrentBlock => *player_block as i16,
                            DamageSource::StrikesInHand { base, per_strike } => {
                                let count = hand.iter()
                                    .filter(|hc| hc.card.id.contains("Strike"))
                                    .count() as i16;
                                *base + *per_strike * count
                            }
                            DamageSource::StrengthMultiplier { base, multiplier } => {
                                let str_amount = player_powers.iter()
                                    .find(|p| p.id == "Strength")
                                    .map(|p| p.amount as i16)
                                    .unwrap_or(0);
                                *base + *multiplier * str_amount
                            }
                        };
                        if idx < monsters.len() && !monsters[idx].is_gone {
                            let dmg = calculate_damage(base_amount, player_powers, &monsters[idx].powers);
                            apply_damage_to_monster(&mut monsters[idx], dmg);
                        }
                    }
                }
            }
            Effect::StrengthIfTargetDead(amount) => {
                if let Some(idx) = target_index {
                    let idx = idx as usize;
                    if let Some(Screen::Combat { monsters, player_powers, .. }) = self.find_combat_mut() {
                        if idx < monsters.len() && monsters[idx].is_gone {
                            apply_power(player_powers, "Strength", *amount as i32);
                        }
                    }
                }
            }
            Effect::Block(amount) => {
                if let Some(Screen::Combat { player_block, .. }) = self.find_combat_mut() {
                    *player_block += *amount as u16;
                }
            }
            Effect::DoubleBlock => {
                if let Some(Screen::Combat { player_block, .. }) = self.find_combat_mut() {
                    *player_block *= 2;
                }
            }
            Effect::GainTemporaryStrength(amount) => {
                if let Some(Screen::Combat { player_powers, .. }) = self.find_combat_mut() {
                    let before = player_powers.iter().find(|p| p.id == "Strength")
                        .map(|p| p.amount).unwrap_or(0);
                    apply_power(player_powers, "Strength", *amount as i32);
                    let after = player_powers.iter().find(|p| p.id == "Strength")
                        .map(|p| p.amount).unwrap_or(0);
                    let actual_gain = after - before;
                    if actual_gain > 0 {
                        apply_power(player_powers, "LoseStrength", actual_gain);
                    }
                }
            }
            Effect::DoubleStrength => {
                if let Some(Screen::Combat { player_powers, .. }) = self.find_combat_mut() {
                    let current = player_powers.iter().find(|p| p.id == "Strength")
                        .map(|p| p.amount).unwrap_or(0);
                    if current > 0 {
                        apply_power(player_powers, "Strength", current);
                    }
                }
            }
            Effect::ApplyPower { target, power_id, amount } => {
                if let Some(Screen::Combat { player_powers, monsters, .. }) = self.find_combat_mut() {
                    match target {
                        EffectTarget::TargetEnemy => {
                            if let Some(idx) = target_index {
                                let idx = idx as usize;
                                if idx < monsters.len() && !monsters[idx].is_gone {
                                    apply_power(&mut monsters[idx].powers, power_id, *amount as i32);
                                }
                            }
                        }
                        EffectTarget::_Self => {
                            apply_power(player_powers, power_id, *amount as i32);
                        }
                        EffectTarget::AllEnemies => {
                            for monster in monsters.iter_mut() {
                                if !monster.is_gone {
                                    apply_power(&mut monster.powers, power_id, *amount as i32);
                                }
                            }
                        }
                    }
                }
            }
            Effect::Draw(count) => {
                if let Some(Screen::Combat { hand, draw_pile, discard_pile, .. }) = self.find_combat_mut() {
                    for _ in 0..*count {
                        if let Some(card) = draw_card(draw_pile, discard_pile) {
                            hand.push(HandCard { card });
                        }
                    }
                }
            }
            Effect::GainEnergy(amount) => {
                if let Some(Screen::Combat { player_energy, .. }) = self.find_combat_mut() {
                    *player_energy += amount;
                }
            }
            Effect::LoseHP(amount) => {
                self.hp = self.hp.saturating_sub(*amount);
            }
            Effect::AddCardToPile { card_id, pile, count } => {
                if let Some(Screen::Combat { draw_pile, discard_pile, exhaust_pile, .. }) = self.find_combat_mut() {
                    let new_card = Card {
                        id: card_id.to_string(),
                        name: card_id.to_string(),
                        cost: card_db::lookup(card_id).map(|i| i.cost).unwrap_or(0),
                        card_type: card_db::lookup(card_id)
                            .map(|i| match i.card_type {
                                card_db::CardType::Attack => "ATTACK",
                                card_db::CardType::Skill => "SKILL",
                                card_db::CardType::Power => "POWER",
                                card_db::CardType::Status => "STATUS",
                                card_db::CardType::Curse => "CURSE",
                            })
                            .unwrap_or("STATUS")
                            .to_string(),
                        upgraded: false,
                    };
                    let target_pile = match pile {
                        Pile::Draw => draw_pile,
                        Pile::Discard => discard_pile,
                        Pile::Exhaust => exhaust_pile,
                    };
                    for _ in 0..*count {
                        target_pile.push(new_card.clone());
                    }
                }
            }
            Effect::SelectFromHand { min, max, action } => {
                if let Some(Screen::Combat { hand, discard_pile, exhaust_pile, draw_pile, effect_queue, .. }) = self.find_combat_mut() {
                    if hand.len() <= *min as usize {
                        // Auto-resolve: not enough cards for a real choice
                        let selected: Vec<HandCard> = hand.drain(..).collect();
                        for hc in selected {
                            apply_hand_select_action(*action, hc.card, discard_pile, exhaust_pile, draw_pile, effect_queue);
                        }
                        return EffectResult::Continue;
                    }
                    // Real choice — push HandSelect and pause
                    let cards: Vec<(u8, Card)> = hand.iter().enumerate()
                        .map(|(i, hc)| (i as u8, hc.card.clone())).collect();
                    let action = *action;
                    let min = *min;
                    let max = *max;
                    self.push_screen(Screen::HandSelect {
                        min_cards: min,
                        max_cards: max,
                        cards,
                        picked_indices: vec![],
                        action,
                    });
                    return EffectResult::Paused;
                }
            }
            Effect::SelectFromDiscardToDrawTop => {
                if let Some(Screen::Combat { discard_pile, draw_pile, .. }) = self.find_combat_mut() {
                    if discard_pile.is_empty() {
                        return EffectResult::Continue;
                    }
                    if discard_pile.len() == 1 {
                        // Auto-resolve: only one card
                        let card = discard_pile.pop().unwrap();
                        draw_pile.push(card);
                        return EffectResult::Continue;
                    }
                    let cards: Vec<(u8, Card)> = discard_pile.iter().enumerate()
                        .map(|(i, c)| (i as u8, c.clone())).collect();
                    self.push_screen(Screen::DiscardSelect { cards });
                    return EffectResult::Paused;
                }
            }
            Effect::SelectFromExhaustToHand => {
                if let Some(Screen::Combat { exhaust_pile, hand, .. }) = self.find_combat_mut() {
                    if exhaust_pile.is_empty() {
                        return EffectResult::Continue;
                    }
                    if exhaust_pile.len() == 1 {
                        let card = exhaust_pile.pop().unwrap();
                        hand.push(HandCard { card });
                        return EffectResult::Continue;
                    }
                    let cards: Vec<(u8, Card)> = exhaust_pile.iter().enumerate()
                        .map(|(i, c)| (i as u8, c.clone())).collect();
                    self.push_screen(Screen::ExhaustSelect { cards });
                    return EffectResult::Paused;
                }
            }
            Effect::FlameBarrier(thorns_damage) => {
                if let Some(Screen::Combat { monsters, effect_queue, .. }) = self.find_combat_mut() {
                    for (i, monster) in monsters.iter().enumerate() {
                        if monster.is_gone {
                            continue;
                        }
                        // Only affect monsters that intend to attack (damage >= 0)
                        if let Some(dmg) = monster.damage {
                            if dmg >= 0 {
                                let hit_count = monster.hits;
                                for _ in 0..hit_count {
                                    effect_queue.push_back((
                                        Effect::DamageFixed(*thorns_damage),
                                        Some(i as u8),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Effect::PlayTopOfDraw => {
                if let Some(Screen::Combat { draw_pile, discard_pile, effect_queue, .. }) = self.find_combat_mut() {
                    let card = if let Some(c) = draw_card(draw_pile, discard_pile) {
                        c
                    } else {
                        return EffectResult::Continue;
                    };
                    let info = card_db::lookup(&card.id);
                    // Unplayable cards (Dazed, statuses) have no effects — they
                    // just get exhausted. This matches the game behavior.
                    let effects: Vec<Effect> = info
                        .map(|i| i.effective_effects(card.upgraded).to_vec())
                        .unwrap_or_default();
                    let has_target = info.map(|i| i.target.has_target()).unwrap_or(false);
                    let is_power = info
                        .map(|i| i.card_type == card_db::CardType::Power)
                        .unwrap_or(false);
                    let force_exhaust = !is_power;

                    if has_target {
                        // Need target selection — push screen and pause
                        self.push_screen(Screen::TargetSelect {
                            card: Some(card),
                            effects,
                            force_exhaust,
                        });
                        return EffectResult::Paused;
                    } else {
                        // No target needed — queue effects + disposition directly
                        for effect in effects.iter().rev() {
                            effect_queue.push_front((effect.clone(), None));
                        }
                        if force_exhaust {
                            effect_queue.push_back((Effect::DisposeCard {
                                card: card.clone(),
                                exhaust: true,
                                rebound: false,
                            }, None));
                        }
                        // Powers are consumed (no disposition needed)
                    }
                }
            }
            Effect::ForEachInHand { filter, per_card, exhaust_matched } => {
                if let Some(Screen::Combat { hand, exhaust_pile, effect_queue, .. }) = self.find_combat_mut() {
                    let matches_filter = |card: &Card| {
                        let card_type = card_db::lookup(&card.id)
                            .map(|i| i.card_type)
                            .unwrap_or(card_db::CardType::Skill);
                        match filter {
                            HandFilter::AllCards => true,
                            HandFilter::Attacks => card_type == card_db::CardType::Attack,
                            HandFilter::NonAttacks => card_type != card_db::CardType::Attack,
                        }
                    };

                    // Count matching cards and optionally exhaust them
                    let mut count = 0usize;
                    if *exhaust_matched {
                        let mut kept = Vec::new();
                        for hc in hand.drain(..) {
                            if matches_filter(&hc.card) {
                                count += 1;
                                exhaust_card(hc.card, exhaust_pile, effect_queue);
                            } else {
                                kept.push(hc);
                            }
                        }
                        *hand = kept;
                    } else {
                        count = hand.iter().filter(|hc| matches_filter(&hc.card)).count();
                    }

                    // Push per_card effects N times to front of queue (in order)
                    let total_effects = count * per_card.len();
                    for i in (0..total_effects).rev() {
                        let effect_idx = i % per_card.len();
                        effect_queue.push_front((per_card[effect_idx].clone(), target_index));
                    }
                }
            }
            Effect::ChooseOne(options) => {
                let choices: Vec<(String, Vec<Effect>)> = options
                    .iter()
                    .map(|(label, effects)| (label.to_string(), effects.to_vec()))
                    .collect();
                self.push_screen(Screen::ChoiceSelect {
                    choices,
                    target_index,
                    energy_costs: vec![],
                });
                return EffectResult::Paused;
            }
            Effect::XCost { per_energy, bonus } => {
                if let Some(Screen::Combat { player_energy, .. }) = self.screen.last() {
                    let max_energy = *player_energy;
                    let mut choices = Vec::new();
                    let mut energy_costs = Vec::new();
                    for spend in 0..=max_energy {
                        let reps = spend as i16 + bonus;
                        let label = format!("Spend {}", spend);
                        let mut effects = Vec::new();
                        for _ in 0..reps.max(0) {
                            for e in per_energy.iter() {
                                effects.push(e.clone());
                            }
                        }
                        choices.push((label, effects));
                        energy_costs.push(spend);
                    }
                    self.push_screen(Screen::ChoiceSelect {
                        choices,
                        target_index,
                        energy_costs,
                    });
                    return EffectResult::Paused;
                }
            }
            Effect::ConditionalOnDieRoll { min, max, effects } => {
                if let Some(Screen::Combat { die_roll, effect_queue, .. }) = self.find_combat_mut() {
                    let roll = die_roll.expect("ConditionalOnDieRoll used before die was rolled");
                    if roll >= *min && roll <= *max {
                        for effect in effects.iter().rev() {
                            effect_queue.push_front((effect.clone(), target_index));
                        }
                    }
                }
            }
            Effect::DisposeCard { card, exhaust, rebound } => {
                if let Some(Screen::Combat { discard_pile, exhaust_pile, draw_pile, effect_queue, .. }) = self.find_combat_mut() {
                    if *exhaust {
                        exhaust_card(card.clone(), exhaust_pile, effect_queue);
                    } else if *rebound {
                        draw_pile.push(card.clone());
                    } else {
                        discard_pile.push(card.clone());
                    }
                }
            }
            Effect::TickDownAttackPowers { had_weak, vuln_mask } => {
                if let Some(Screen::Combat { monsters, player_powers, .. }) = self.find_combat_mut() {
                    if *had_weak {
                        apply_power(player_powers, "BGWeakened", -1);
                    }
                    for (i, monster) in monsters.iter_mut().enumerate() {
                        if vuln_mask & (1 << i) != 0 {
                            apply_power(&mut monster.powers, "BGVulnerable", -1);
                        }
                    }
                }
            }
            Effect::Custom(_id) => {
                // Not yet implemented
            }
        }

        // Check for combat end after each effect
        if let Some(Screen::Combat { monsters, effect_queue, .. }) = self.find_combat_mut() {
            if monsters.iter().all(|m| m.is_gone) {
                effect_queue.clear();
                return EffectResult::CombatOver;
            }
        }

        EffectResult::Continue
    }

    /// Resolve a HandSelect screen: apply the action to all picked cards,
    /// then pop the screen.
    fn resolve_hand_select(&mut self) {
        // Extract info from HandSelect before popping
        let (action, mut picked) = if let Screen::HandSelect {
            action, picked_indices, ..
        } = self.current_screen()
        {
            (*action, picked_indices.clone())
        } else {
            return;
        };

        self.pop_screen();

        // Remove picked cards from combat hand in reverse order (so indices stay valid)
        picked.sort();
        picked.reverse();

        if let Some(Screen::Combat { hand, discard_pile, exhaust_pile, draw_pile, effect_queue, .. }) = self.find_combat_mut() {
            for hi in picked {
                let hi = hi as usize;
                if hi < hand.len() {
                    let hc = hand.remove(hi);
                    apply_hand_select_action(action, hc.card, discard_pile, exhaust_pile, draw_pile, effect_queue);
                }
            }
        }
    }

    fn finish_combat(&mut self) {
        if let Screen::Combat { encounter, .. } = self.current_screen() {
            let encounter = encounter.clone();
            let rewards = self.generate_combat_rewards(&encounter);
            self.pop_screen();
            if encounter.contains("BOSS") {
                let boss_relic_screen = self.generate_boss_relic_screen();
                self.push_screen(boss_relic_screen);
                if !rewards.is_empty() {
                    self.push_screen(Screen::CombatRewards { rewards });
                }
            } else {
                self.push_screen(Screen::CombatRewards { rewards });
            }
        }
    }

    fn generate_combat_rewards(&mut self, encounter: &str) -> Vec<crate::screen::Reward> {
        use crate::screen::Reward;
        let mut rewards = Vec::new();

        match encounter {
            "UNKNOWN_MONSTER" => {
                rewards.push(Reward::gold(1));
                if let Some(pools) = &mut self.reward_pools {
                    if let Some(id) = pools.potion_deck.draw() {
                        rewards.push(Reward::potion(Potion { id: id.clone(), name: id }));
                    }
                }
                rewards.push(Reward::card());
            }
            "UNKNOWN_ELITE" => {
                rewards.push(Reward::gold(1));
                rewards.push(Reward::upgraded_card());
                if let Some(pools) = &mut self.reward_pools {
                    if let Some(id) = pools.draw_relic() {
                        rewards.push(Reward::relic(Relic {
                            id: id.clone(), name: id,
                            counter: -1, clickable: false, pulsing: false,
                        }));
                    }
                }
            }
            "UNKNOWN_BOSS" => {
                rewards.push(Reward::gold(3));
            }
            _ => {}
        }

        rewards
    }

    fn generate_shop(&mut self) -> Screen {
        use crate::screen::{ShopCard, ShopRelic, ShopPotion};
        let mut cards = Vec::new();
        let mut relics = Vec::new();
        let mut potions = Vec::new();

        if let Some(pools) = &mut self.reward_pools {
            // 5 cards for sale
            for price in [2, 2, 3, 3, 3] {
                if let Some(id) = pools.card_deck.draw() {
                    cards.push(ShopCard {
                        card: Card { id: id.clone(), name: id, cost: 0, card_type: "UNKNOWN".to_string(), upgraded: false },
                        price: Some(price),
                    });
                }
            }
            // 3 relics
            for price in [7, 7, 8] {
                if let Some(id) = pools.draw_relic() {
                    relics.push(ShopRelic { id: id.clone(), name: id, price: Some(price) });
                }
            }
            // 3 potions
            for price in [2, 2, 3] {
                if let Some(id) = pools.potion_deck.draw() {
                    potions.push(ShopPotion { id: id.clone(), name: id, price: Some(price) });
                }
            }
        }

        Screen::Shop { cards, relics, potions, purge_cost: Some(3) }
    }

    fn generate_boss_relic_screen(&mut self) -> Screen {
        let mut relics = Vec::new();
        let mut cards = Vec::new();
        if let Some(pools) = &mut self.reward_pools {
            for _ in 0..3 {
                if let Some(id) = pools.boss_relic_deck.draw() {
                    relics.push(Relic {
                        id: id.clone(), name: id,
                        counter: -1, clickable: false, pulsing: false,
                    });
                }
            }
            // Rare card reward
            for _ in 0..3 {
                if let Some(id) = pools.rare_deck.draw() {
                    cards.push(Card {
                        id: id.clone(), name: id, cost: 0,
                        card_type: "UNKNOWN".to_string(), upgraded: false,
                    });
                }
            }
        }
        Screen::BossRelic { relics, cards }
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
        let mut actions = match self.current_screen() {
            Screen::Neow { options } => neow_actions(options),
            Screen::Event { options, .. } => event_actions(options),
            Screen::Map { available_nodes, .. } => map_actions(available_nodes),
            Screen::CardReward { cards } => card_reward_actions(cards),
            Screen::CombatRewards { rewards } => combat_reward_actions(rewards),
            Screen::Grid { cards, .. } => grid_actions(cards),
            Screen::Rest { options } => rest_actions(options),
            Screen::Shop { cards, relics, potions, purge_cost } => {
                let has_potion_slot = self.potions.iter().any(|p| p.is_none());
                shop_actions(cards, relics, potions, *purge_cost, self.gold, has_potion_slot)
            }
            Screen::BossRelic { relics, cards } => boss_relic_actions(relics, cards),
            Screen::Combat { hand, monsters, effect_queue, player_energy, draw_pile, .. } => {
                assert!(effect_queue.is_empty(), "Effect queue should be empty when generating actions");
                combat_actions(hand, monsters, *player_energy, draw_pile)
            }
            Screen::HandSelect { cards, picked_indices, min_cards, max_cards, .. } => {
                hand_select_actions(cards, picked_indices.len() as u8, *min_cards, *max_cards)
            }
            Screen::DiscardSelect { cards } => {
                cards.iter().enumerate().map(|(i, (_, card))| {
                    Action::PickDiscard { card: card.clone(), choice_index: i as u8 }
                }).collect()
            }
            Screen::ExhaustSelect { cards } => {
                cards.iter().enumerate().map(|(i, (_, card))| {
                    Action::PickExhaust { card: card.clone(), choice_index: i as u8 }
                }).collect()
            }
            Screen::TargetSelect { card: Some(card), .. } => {
                // Generate one PickTarget per live monster
                if let Some(Screen::Combat { monsters, .. }) = self.screen.iter().rev()
                    .find(|s| matches!(s, Screen::Combat { .. }))
                {
                    monsters.iter().enumerate()
                        .filter(|(_, m)| !m.is_gone)
                        .map(|(i, m)| Action::PickTarget {
                            card: card.clone(),
                            target_index: i as u8,
                            target_name: m.name.clone(),
                        })
                        .collect()
                } else {
                    vec![]
                }
            }
            Screen::ChoiceSelect { choices, .. } => {
                choices.iter().enumerate().map(|(i, (label, _))| {
                    Action::PickChoice { label: label.clone(), choice_index: i as u8 }
                }).collect()
            }
            Screen::Complete | Screen::ShopRoom => vec![Action::Proceed],
            Screen::GameOver { .. } => vec![Action::Proceed],
            Screen::Treasure => vec![Action::OpenChest { choice_index: 0 }],
            _ => vec![],
        };

        // Potion discard available on most screens
        if !matches!(self.current_screen(), Screen::GameOver { .. }) {
            for (i, potion) in self.potions.iter().enumerate() {
                if potion.is_some() {
                    actions.push(Action::DiscardPotion { slot: i as u8 });
                }
            }
        }

        actions
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

fn boss_relic_actions(relics: &[crate::types::Relic], cards: &[Card]) -> Vec<Action> {
    let mut actions = Vec::new();
    // Can pick one relic (if any remain)
    for (i, _) in relics.iter().enumerate() {
        actions.push(Action::PickBossRelic { choice_index: i as u8 });
    }
    // Can take one card (if any remain)
    for (i, card) in cards.iter().enumerate() {
        actions.push(Action::TakeCard { card: card.clone(), choice_index: i as u8 });
    }
    if !cards.is_empty() {
        actions.push(Action::SkipCardReward);
    }
    // Can skip the whole thing
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

fn combat_actions(hand: &[HandCard], monsters: &[Monster], energy: u8, draw_pile: &[Card]) -> Vec<Action> {
    let mut actions = Vec::new();
    let live_monsters: Vec<(u8, &Monster)> = monsters
        .iter()
        .enumerate()
        .filter(|(_, m)| !m.is_gone)
        .map(|(i, m)| (i as u8, m))
        .collect();

    // Precompute hand-level conditions for play predicates
    let all_attacks = hand.iter().all(|hc| hc.card.card_type == "ATTACK");
    let attack_count = hand.iter().filter(|hc| hc.card.card_type == "ATTACK").count();
    let draw_pile_empty = draw_pile.is_empty();

    for (i, hc) in hand.iter().enumerate() {
        let info = card_db::lookup(&hc.card.id);
        let cost = info
            .map(|i| i.effective_cost(hc.card.upgraded))
            .unwrap_or(hc.card.cost);
        let is_x_cost = cost == -1;
        if !is_x_cost && (cost < 0 || cost > energy as i8) {
            continue;
        }
        // Check play condition
        if let Some(cond) = info.and_then(|i| i.play_condition) {
            use card_db::PlayCondition::*;
            let met = match cond {
                HandAllAttacks => all_attacks,
                HandNoOtherAttacks => attack_count <= 1,
                DrawPileEmpty => draw_pile_empty,
                Never => false,
            };
            if !met {
                continue;
            }
        }
        let has_target = info.map(|i| i.target.has_target()).unwrap_or(false);
        if has_target {
            for &(mi, ref m) in &live_monsters {
                actions.push(Action::PlayCard {
                    card: hc.card.clone(),
                    hand_index: i as u8,
                    target_index: Some(mi),
                    target_name: Some(m.name.clone()),
                });
            }
        } else {
            actions.push(Action::PlayCard {
                card: hc.card.clone(),
                hand_index: i as u8,
                target_index: None,
                target_name: None,
            });
        }
    }

    actions.push(Action::EndTurn);
    actions
}

enum EffectResult {
    Continue,
    Paused,
    CombatOver,
}

/// Deal damage to a monster, accounting for its block.
fn apply_damage_to_monster(monster: &mut crate::types::Monster, damage: u16) {
    if damage <= monster.block {
        monster.block -= damage;
    } else {
        let remaining = damage - monster.block;
        monster.block = 0;
        monster.hp = monster.hp.saturating_sub(remaining);
    }
    if monster.hp == 0 {
        monster.is_gone = true;
    }
}

fn get_power_amount(powers: &[crate::types::Power], power_id: &str) -> i32 {
    powers.iter().find(|p| p.id == power_id).map(|p| p.amount).unwrap_or(0)
}

/// Calculate damage after applying attacker and defender power modifiers.
/// Order: atDamageGive (Strength) → atDamageReceive (Vulnerable ×2).
fn calculate_damage(base: i16, attacker_powers: &[crate::types::Power], defender_powers: &[crate::types::Power]) -> u16 {
    let mut dmg = base as f32;

    dmg += get_power_amount(attacker_powers, "Strength") as f32;

    let attacker_weak = get_power_amount(attacker_powers, "BGWeakened") > 0;
    let defender_vuln = get_power_amount(defender_powers, "BGVulnerable") > 0;

    if attacker_weak && !defender_vuln {
        dmg -= 1.0;
    } else if !attacker_weak && defender_vuln {
        dmg *= 2.0;
    }

    dmg.floor().max(0.0) as u16
}

/// Maximum value for the Strength power.
const MAX_STRENGTH: i32 = 8;

/// Add or stack a power on a creature's power list.
/// Strength is capped at MAX_STRENGTH.
fn apply_power(powers: &mut Vec<crate::types::Power>, power_id: &str, amount: i32) {
    if let Some(existing) = powers.iter_mut().find(|p| p.id == power_id) {
        existing.amount += amount;
        if power_id == "Strength" {
            existing.amount = existing.amount.min(MAX_STRENGTH);
        }
    } else {
        let capped = if power_id == "Strength" { amount.min(MAX_STRENGTH) } else { amount };
        powers.push(crate::types::Power {
            id: power_id.to_string(),
            amount: capped,
        });
    }
    // Remove the power if it dropped to 0 or below
    if let Some(pos) = powers.iter().position(|p| p.id == power_id && p.amount <= 0) {
        powers.remove(pos);
    }
}

fn hand_select_actions(cards: &[(u8, Card)], cards_picked: u8, min_cards: u8, max_cards: u8) -> Vec<Action> {
    let mut actions = Vec::new();
    if cards_picked < max_cards {
        for (i, (_hand_idx, card)) in cards.iter().enumerate() {
            actions.push(Action::PickHandCard {
                card: card.clone(),
                choice_index: i as u8,
            });
        }
    }
    if cards_picked >= min_cards {
        actions.push(Action::Skip);
    }
    actions
}

fn apply_hand_select_action(
    action: HandSelectAction,
    card: Card,
    discard_pile: &mut Vec<Card>,
    exhaust_pile: &mut Vec<Card>,
    draw_pile: &mut Vec<Card>,
    effect_queue: &mut std::collections::VecDeque<(Effect, Option<u8>)>,
) {
    match action {
        HandSelectAction::Exhaust => exhaust_card(card, exhaust_pile, effect_queue),
        HandSelectAction::Discard => discard_pile.push(card),
        HandSelectAction::PutOnTopOfDraw => draw_pile.push(card),
        HandSelectAction::Upgrade => {
            // TODO: upgrade the card and put it back in hand
        }
    }
}

/// Draw a card from the draw pile, reshuffling discard into draw if needed.
/// Returns None if both piles are empty.
fn draw_card(draw_pile: &mut Vec<Card>, discard_pile: &mut Vec<Card>) -> Option<Card> {
    if draw_pile.is_empty() && !discard_pile.is_empty() {
        draw_pile.append(discard_pile);
        draw_pile.reverse();
    }
    draw_pile.pop()
}

/// Move a card to the exhaust pile and queue any on-exhaust effects.
fn exhaust_card(
    card: Card,
    exhaust_pile: &mut Vec<Card>,
    effect_queue: &mut std::collections::VecDeque<(Effect, Option<u8>)>,
) {
    if let Some(info) = card_db::lookup(&card.id) {
        if let Some(effects) = info.effective_on_exhaust(card.upgraded) {
            for effect in effects.iter().rev() {
                effect_queue.push_front((effect.clone(), None));
            }
        }
    }
    exhaust_pile.push(card);
}

