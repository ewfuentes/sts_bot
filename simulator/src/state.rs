use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::action::Action;
use crate::card_db;
use crate::encounter_db;
use crate::monster_db;
use crate::power_db;
use crate::effects::{DamageSource, Effect, EffectTarget, HandFilter, HandSelectAction, Pile, ResolvedTarget};
use crate::map::{ActMap, MapNodeKind};
use crate::pool::Pool;
use crate::reward_deck::{self, Character, RewardDeck};
use crate::screen::{EventOption, HandCard, Screen, ShopCard, ShopPotion, ShopRelic, TargetReason};
use crate::types::{Card, Monster, MonsterState, Potion, Relic};

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

    /// Apply pre-battle starting effects for all monsters in combat.
    /// Call this after monsters are populated and before the player's first action.
    /// Initialize combat: copy deck into draw pile, shuffle, set energy, draw opening hand.
    pub fn start_combat(&mut self) {
        let mut shuffled_deck = self.deck.clone();

        if let Some(Screen::Combat {
            draw_pile, player_energy, turn, effect_queue, monsters, die_roll, rng, ..
        }) = self.find_combat_mut()
        {
            // Shuffle the deck using the combat RNG
            rng.shuffle(&mut shuffled_deck);
            *draw_pile = shuffled_deck;
            *player_energy = 3;
            *turn = 1;

            // Roll the die for turn 1
            let roll = rng.roll_die(6);
            *die_roll = Some(roll);

            // Resolve patterns and initial monster intents
            for monster in monsters.iter_mut() {
                if let Some(info) = monster_db::lookup(&monster.id) {
                    // Initialize pattern from monster_db if not already set (e.g. deserialized)
                    if monster.pattern == monster_db::MovePattern::default() {
                        monster.pattern = info.pattern;
                    }
                    let move_idx = monster_db::next_move(monster.pattern, roll, 1, monster.move_index);
                    monster.move_index = move_idx;
                    let actual_move = monster_db::resolve_move_index(monster.pattern, move_idx);
                    update_monster_display(monster, info, actual_move);
                }
            }

            // Draw opening hand of 5
            effect_queue.push_back((Effect::Draw(5), ResolvedTarget::NoTarget));
        }

        self.drain_effect_queue();
    }

    pub fn apply_monster_starting_effects(&mut self) {
        if let Some(Screen::Combat { monsters, effect_queue, .. }) = self.find_combat_mut() {
            for (i, monster) in monsters.iter_mut().enumerate() {
                if let Some(info) = monster_db::lookup(&monster.id) {
                    // Initialize pattern from monster_db if not already set
                    if monster.pattern == monster_db::MovePattern::default() {
                        monster.pattern = info.pattern;
                    }
                    for effect in info.starting_effects {
                        effect_queue.push_back((effect.clone(), ResolvedTarget::Monster(i as u8)));
                    }
                }
            }
        }
        self.drain_effect_queue();
    }

    /// Queue reactive power triggers after a monster takes damage or dies.
    /// `is_attack` indicates the damage came from an Attack effect (triggers Angry)
    /// vs a non-attack Damage effect (only triggers CurlUp).
    fn queue_monster_reactive_triggers(&mut self, monster_idx: u8, result: &DamageResult, kind: DamageKind) {
        if !result.took_damage && !result.died {
            return;
        }
        if let Some(Screen::Combat { monsters, effect_queue, .. }) = self.find_combat_mut() {
            let idx = monster_idx as usize;
            if idx >= monsters.len() {
                return;
            }

            let target = ResolvedTarget::Monster(monster_idx);

            if result.took_damage {
                let triggered = power_db::collect_triggered_effects(
                    power_db::PowerTrigger::MonsterOnDamaged,
                    &monsters[idx].powers,
                    target,
                );
                queue_triggered(effect_queue, triggered);
            }

            if matches!(kind, DamageKind::Attack) && result.took_damage {
                let triggered = power_db::collect_triggered_effects(
                    power_db::PowerTrigger::MonsterOnAttacked,
                    &monsters[idx].powers,
                    target,
                );
                queue_triggered(effect_queue, triggered);
            }

            // Fire death triggers immediately only for monsters that go straight to Dead.
            // DeadPendingSummon monsters have their death triggers deferred to execute_monster_turns.
            if result.died && monsters[idx].state == MonsterState::Dead {
                let triggered = power_db::collect_triggered_effects(
                    power_db::PowerTrigger::MonsterOnDeath,
                    &monsters[idx].powers,
                    target,
                );
                queue_triggered(effect_queue, triggered);
            }

            if result.block_broken {
                if let Some(new_state) = monsters[idx].pattern.on_block_broken(monsters[idx].move_index) {
                    monsters[idx].move_index = new_state;
                    if let Some(info) = monster_db::lookup(&monsters[idx].id) {
                        let actual_move = monster_db::resolve_move_index(monsters[idx].pattern, new_state);
                        update_monster_display(&mut monsters[idx], info, actual_move);
                    }
                }
            }
        }
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
        let mut rng = crate::rng::Rng::from_seed(seed);
        if let Some(pools) = &mut self.reward_pools {
            pools.determinize(&mut |items| {
                rng.shuffle(items);
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
            Action::TravelTo { kind, choice_index, .. } => {
                self.floor += 1;
                let kind = *kind;
                let choice_idx = *choice_index as usize;

                // Look up the encounter ID and node seed from the map
                let (encounter_id, node_seed) = if let Screen::Map { available_nodes, .. } = self.current_screen() {
                    if choice_idx < available_nodes.len() {
                        let node_idx = available_nodes[choice_idx].node_index;
                        if let Some(node) = self.map.as_ref().and_then(|m| m.nodes.get(node_idx)) {
                            (node.encounter.clone(), node.seed)
                        } else {
                            (None, 0)
                        }
                    } else {
                        (None, 0)
                    }
                } else {
                    (None, 0)
                };

                let screen = match kind {
                    MapNodeKind::Monster | MapNodeKind::Elite | MapNodeKind::Boss => {
                        let enc_id = encounter_id.as_deref().unwrap_or(match kind {
                            MapNodeKind::Monster => "UNKNOWN_MONSTER",
                            MapNodeKind::Elite => "UNKNOWN_ELITE",
                            MapNodeKind::Boss => "UNKNOWN_BOSS",
                            _ => unreachable!(),
                        });
                        let mut combat = Screen::new_combat(enc_id, node_seed);
                        // Populate monsters from encounter_db
                        if let Some(enc) = encounter_db::lookup(enc_id) {
                            if let Screen::Combat { monsters, rng, .. } = &mut combat {
                                for em in enc.monsters {
                                    // Copy pattern from monster_db, shuffling die roll indices per instance
                                    let pattern = if let Some(info) = monster_db::lookup(em.id) {
                                        match info.pattern {
                                            monster_db::MovePattern::DieRoll2(indices) => {
                                                let mut shuffled = indices;
                                                rng.shuffle(&mut shuffled);
                                                monster_db::MovePattern::DieRoll2(shuffled)
                                            }
                                            monster_db::MovePattern::DieRoll3(indices) => {
                                                let mut shuffled = indices;
                                                rng.shuffle(&mut shuffled);
                                                monster_db::MovePattern::DieRoll3(shuffled)
                                            }
                                            other => other,
                                        }
                                    } else {
                                        monster_db::MovePattern::default()
                                    };
                                    monsters.push(crate::types::Monster {
                                        id: em.id.to_string(),
                                        name: em.id.to_string(),
                                        hp: em.hp,
                                        max_hp: em.hp,
                                        block: 0,
                                        intent: "UNKNOWN".to_string(),
                                        damage: None,
                                        hits: 1,
                                        powers: vec![],
                                        state: MonsterState::Alive,
                                        move_index: em.move_index,
                                        pattern,
                                    });
                                }
                            }
                        }
                        combat
                    }
                    MapNodeKind::Rest => Screen::Rest {
                        options: vec!["rest".to_string(), "smith".to_string()],
                    },
                    MapNodeKind::Shop => self.generate_shop(),
                    MapNodeKind::Treasure => Screen::Treasure,
                    MapNodeKind::Event | MapNodeKind::Unknown => Screen::Complete,
                };
                self.set_screen(screen);

                // Initialize combat: shuffle deck, draw hand, apply monster starting effects
                if matches!(kind, MapNodeKind::Monster | MapNodeKind::Elite | MapNodeKind::Boss) {
                    self.start_combat();
                    self.apply_monster_starting_effects();
                }
            }
            Action::PlayCard { hand_index, target_index, .. } => {
                let hand_idx = *hand_index as usize;

                let target = match target_index {
                    Some(idx) => ResolvedTarget::Monster(*idx),
                    None => ResolvedTarget::NoTarget,
                };

                if let Some(Screen::Combat {
                    hand, player_energy, player_powers, monsters, effect_queue, ..
                }) = self.screen.last_mut()
                {
                    if hand_idx >= hand.len() {
                        return;
                    }
                    let hc = hand.remove(hand_idx);
                    let card = hc.card;
                    let info = card_db::lookup(&card.id).expect("card not found in card_db");

                    // Deduct energy (with cost modification from powers like Corruption)
                    let base_cost = info.effective_cost(card.upgraded);
                    let cost = power_db::apply_cost_modification(base_cost, info.card_type, player_powers);
                    if cost >= 0 {
                        *player_energy = player_energy.saturating_sub(cost as u8);
                    }

                    let effects = info.effective_effects(card.upgraded);
                    play_card_effects(effects, info.card_type, target, player_powers, monsters, effect_queue);

                    // Queue disposition as the final effect (after all card effects).
                    // Powers are consumed — no disposition needed.
                    if info.card_type != card_db::CardType::Power {
                        let force_exhaust = info.card_type == card_db::CardType::Skill
                            && power_db::has_modifier(power_db::PowerModifier::SkillsExhaust, player_powers);
                        let does_exhaust = info.does_exhaust(card.upgraded) || force_exhaust;
                        let does_rebound = info.rebound;
                        effect_queue.push_back((Effect::DisposeCard {
                            card,
                            exhaust: does_exhaust,
                            rebound: does_rebound,
                        }, ResolvedTarget::NoTarget));
                    }

                    // Fire card-type triggers (e.g. BGAnger on Skill, SharpHide on Attack)
                    let triggered = power_db::collect_all_triggered_effects(
                        power_db::PowerTrigger::PlayerOnPlay { card_type: info.card_type },
                        player_powers,
                        monsters,
                    );
                    queue_triggered(effect_queue, triggered);
                }

                // Drain the effect queue
                self.drain_effect_queue();
            }
            Action::EndTurn => {
                // 1. End-of-turn power triggers (Metallicize, BGCombust, etc.)
                if let Screen::Combat { player_powers, monsters, effect_queue, .. } = self.current_screen_mut() {
                    let triggered = power_db::collect_all_triggered_effects(
                        power_db::PowerTrigger::PlayerEndOfTurn,
                        player_powers,
                        monsters,
                    );
                    queue_triggered(effect_queue, triggered);
                }

                // 2. Discard hand (ethereal → exhaust)
                if let Screen::Combat {
                    hand, discard_pile, effect_queue, ..
                } = self.current_screen_mut()
                {
                    let hand_cards: Vec<HandCard> = hand.drain(..).collect();
                    for hc in hand_cards {
                        let info = card_db::lookup(&hc.card.id);

                        // End-of-turn-in-hand effects (e.g. Burn deals damage)
                        if let Some(effects) = info.and_then(|i| i.on_end_of_turn_in_hand) {
                            for effect in effects {
                                effect_queue.push_back((effect.clone(), ResolvedTarget::Player));
                            }
                        }

                        let is_ethereal = info
                            .map(|i| i.is_ethereal(hc.card.upgraded))
                            .unwrap_or(false);
                        if is_ethereal {
                            effect_queue.push_back((Effect::ExhaustCard { card: hc.card }, ResolvedTarget::NoTarget));
                        } else {
                            discard_pile.push(hc.card);
                        }
                    }
                }

                // Drain end-of-turn + exhaust effects (Metallicize, BGCombust,
                // then FeelNoPain, DarkEmbrace from ethereal exhausts, etc.)
                self.drain_effect_queue();

                // Monster turns (queues effects, drain handles defeat/victory)
                self.execute_monster_turns();
                if !matches!(self.current_screen(), Screen::Combat { .. }) {
                    return;
                }

                if let Screen::Combat {
                    player_block, player_energy, player_powers, turn, effect_queue, die_roll, rng, ..
                } = self.current_screen_mut()
                {
                    // Start of next turn:
                    // 1. Reset energy and block
                    if !power_db::has_modifier(power_db::PowerModifier::PreventBlockDecay, player_powers) {
                        *player_block = 0;
                    }
                    *player_energy = 3;

                    // 2. Turn += 1
                    *turn += 1;

                    // 3. Roll the die for this turn
                    *die_roll = Some(rng.roll_die(6));

                    // 4. Draw 5 cards (reshuffle if needed)
                    effect_queue.push_back((Effect::Draw(5), ResolvedTarget::NoTarget));
                }

                // Drain draw effects (and any on-draw/on-shuffle triggers)
                self.drain_effect_queue();

                // 4. Start-of-turn power triggers (DemonForm, etc.)
                if let Screen::Combat { player_powers, monsters, effect_queue, .. } = self.current_screen_mut() {
                    let triggered = power_db::collect_all_triggered_effects(
                        power_db::PowerTrigger::PlayerStartOfTurn,
                        player_powers,
                        monsters,
                    );
                    queue_triggered(effect_queue, triggered);
                }
                self.drain_effect_queue();
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
                if let Screen::ChoiceSelect { choices, target_index } = self.current_screen() {
                    if idx < choices.len() {
                        let effects = choices[idx].1.clone();
                        let target = match target_index {
                            Some(idx) => ResolvedTarget::Monster(*idx),
                            None => ResolvedTarget::NoTarget,
                        };
                        self.pop_screen();
                        // Push chosen effects to the front of the queue
                        if let Some(Screen::Combat { effect_queue, .. }) = self.find_combat_mut() {
                            for effect in effects.into_iter().rev() {
                                effect_queue.push_front((effect, target));
                            }
                        }
                        self.drain_effect_queue();
                    }
                } else if let Screen::XCostSelect { per_energy, bonus, card_type, target, max_energy } = self.current_screen() {
                    let spend = idx as u8;
                    if spend <= *max_energy {
                        let reps = (spend as i16 + bonus).max(0);
                        let mut resolved_effects = Vec::new();
                        for _ in 0..reps {
                            for e in per_energy.iter() {
                                resolved_effects.push(e.clone());
                            }
                        }
                        let card_type = *card_type;
                        let target = match target {
                            Some(idx) => ResolvedTarget::Monster(*idx),
                            None => ResolvedTarget::NoTarget,
                        };
                        self.pop_screen();
                        // Deduct energy
                        if let Some(Screen::Combat { player_energy, .. }) = self.find_combat_mut() {
                            *player_energy = player_energy.saturating_sub(spend);
                        }
                        // Play the resolved effects through play_card_effects
                        if let Some(Screen::Combat { player_powers, monsters, effect_queue, .. }) = self.find_combat_mut() {
                            play_card_effects(&resolved_effects, card_type, target, player_powers, monsters, effect_queue);
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
                let target = ResolvedTarget::Monster(*target_index);
                if let Screen::TargetSelect { effects, .. } = self.current_screen() {
                    let effects = effects.clone();
                    self.pop_screen();
                    if let Some(Screen::Combat { effect_queue, .. }) = self.find_combat_mut() {
                        for effect in effects.into_iter().rev() {
                            effect_queue.push_front((effect, target));
                        }
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
    /// Executes effects one by one, passing the resolved target along.
    fn drain_effect_queue(&mut self) {
        loop {
            // Pop next (effect, target) pair from queue
            let entry = if let Some(Screen::Combat { effect_queue, .. }) = self.find_combat_mut() {
                effect_queue.pop_front()
            } else {
                None
            };

            let Some((effect, target)) = entry else { break };

            match self.execute_effect(&effect, target) {
                EffectResult::Continue => {}
                EffectResult::Paused => return,
                EffectResult::CombatOver => break,
            }
        }

        // Finalize
        if self.hp == 0 {
            self.set_screen(Screen::GameOver { victory: false });
        } else if let Some(Screen::Combat { monsters, .. }) = self.find_combat_mut() {
            if !monsters.is_empty() && monsters.iter().all(|m| m.state == MonsterState::Dead) {
                self.finish_combat();
            }
        }
    }

    /// Execute a single effect. Returns whether to continue draining,
    /// pause for a sub-decision, or stop because combat ended.
    fn execute_effect(&mut self, effect: &Effect, target: ResolvedTarget) -> EffectResult {
        match effect {
            Effect::DamageToPlayer { base, monster_index } => {
                let idx = *monster_index as usize;
                if let Some(Screen::Combat { monsters, player_powers, player_block, .. }) = self.find_combat_mut() {
                    if idx < monsters.len() {
                        let dmg = calculate_damage(*base, &monsters[idx].powers, player_powers);
                        if dmg <= *player_block {
                            *player_block -= dmg;
                        } else {
                            let remaining = dmg - *player_block;
                            *player_block = 0;
                            self.hp = self.hp.saturating_sub(remaining);
                        }
                    }
                }
            }
            Effect::Damage(amount) => {
                if let ResolvedTarget::Monster(idx) = target {
                    let idx = idx as usize;
                    let mut result = DamageResult { took_damage: false, died: false, block_broken: false };
                    if let Some(Screen::Combat { monsters, player_powers, .. }) = self.find_combat_mut() {
                        if idx < monsters.len() && monsters[idx].state == MonsterState::Alive {
                            let dmg = calculate_damage(*amount, player_powers, &monsters[idx].powers);
                            result = apply_damage_to_monster(&mut monsters[idx], dmg);
                        }
                    }
                    self.queue_monster_reactive_triggers(idx as u8, &result, DamageKind::Attack);
                }
            }
            Effect::DamageFixed(amount) => {
                match target {
                    ResolvedTarget::Monster(idx) => {
                        let idx = idx as usize;
                        let mut result = DamageResult { took_damage: false, died: false, block_broken: false };
                        if let Some(Screen::Combat { monsters, .. }) = self.find_combat_mut() {
                            if idx < monsters.len() && monsters[idx].state == MonsterState::Alive {
                                result = apply_damage_to_monster(&mut monsters[idx], *amount as u16);
                            }
                        }
                        self.queue_monster_reactive_triggers(idx as u8, &result, DamageKind::NonAttack);
                    }
                    ResolvedTarget::Player | ResolvedTarget::NoTarget => {
                        // Blockable damage to the player (e.g. BGAngerPower thorns)
                        let dmg = *amount as u16;
                        if let Some(Screen::Combat { player_block, .. }) = self.find_combat_mut() {
                            if dmg <= *player_block {
                                *player_block -= dmg;
                            } else {
                                let remaining = dmg - *player_block;
                                *player_block = 0;
                                self.hp = self.hp.saturating_sub(remaining);
                            }
                        }
                    }
                }
            }
            Effect::DamageAll(amount) => {
                let mut triggers: Vec<(u8, DamageResult)> = Vec::new();
                if let Some(Screen::Combat { monsters, player_powers, .. }) = self.find_combat_mut() {
                    for (i, monster) in monsters.iter_mut().enumerate() {
                        if monster.state == MonsterState::Alive {
                            let dmg = calculate_damage(*amount, player_powers, &monster.powers);
                            let result = apply_damage_to_monster(monster, dmg);
                            triggers.push((i as u8, result));
                        }
                    }
                }
                for (idx, result) in &triggers {
                    self.queue_monster_reactive_triggers(*idx, result, DamageKind::Attack);
                }
            }
            Effect::DamageBasedOn(source) => {
                if let ResolvedTarget::Monster(idx) = target {
                    let idx = idx as usize;
                    let mut result = DamageResult { took_damage: false, died: false, block_broken: false };
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
                        if idx < monsters.len() && monsters[idx].state == MonsterState::Alive {
                            let dmg = calculate_damage(base_amount, player_powers, &monsters[idx].powers);
                            result = apply_damage_to_monster(&mut monsters[idx], dmg);
                        }
                    }
                    self.queue_monster_reactive_triggers(idx as u8, &result, DamageKind::Attack);
                }
            }
            Effect::StrengthIfTargetDead(amount) => {
                if let ResolvedTarget::Monster(idx) = target {
                    let idx = idx as usize;
                    if let Some(Screen::Combat { monsters, player_powers, .. }) = self.find_combat_mut() {
                        if idx < monsters.len() && monsters[idx].state != MonsterState::Alive {
                            apply_power(player_powers, "Strength", *amount as i32);
                        }
                    }
                }
            }
            Effect::Block(amount) => {
                if let Some(Screen::Combat { player_block, player_powers, effect_queue, .. }) = self.find_combat_mut() {
                    let before = *player_block;
                    *player_block = (*player_block + *amount as u16).min(MAX_BLOCK);
                    let gained = *player_block - before;
                    if gained > 0 {
                        let triggered = power_db::collect_triggered_effects(
                            power_db::PowerTrigger::OnGainBlock,
                            player_powers,
                            ResolvedTarget::Player,
                        );
                        queue_triggered(effect_queue, triggered);
                    }
                }
            }
            Effect::MonsterBlock(amount) => {
                if let ResolvedTarget::Monster(idx) = target {
                    let idx = idx as usize;
                    if let Some(Screen::Combat { monsters, .. }) = self.find_combat_mut() {
                        if idx < monsters.len() && monsters[idx].state == MonsterState::Alive {
                            monsters[idx].block = monsters[idx].block.saturating_add(*amount as u16);
                        }
                    }
                }
            }
            Effect::DoubleBlock => {
                if let Some(Screen::Combat { player_block, player_powers, effect_queue, .. }) = self.find_combat_mut() {
                    let before = *player_block;
                    *player_block = (*player_block * 2).min(MAX_BLOCK);
                    let gained = *player_block - before;
                    if gained > 0 {
                        let triggered = power_db::collect_triggered_effects(
                            power_db::PowerTrigger::OnGainBlock,
                            player_powers,
                            ResolvedTarget::Player,
                        );
                        queue_triggered(effect_queue, triggered);
                    }
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
            Effect::ApplyPower { target: effect_target, power_id, amount } => {
                if let Some(Screen::Combat { player_powers, monsters, .. }) = self.find_combat_mut() {
                    match effect_target {
                        EffectTarget::TargetEnemy => {
                            // Player card targeting an enemy monster
                            if let ResolvedTarget::Monster(idx) = target {
                                let idx = idx as usize;
                                if idx < monsters.len() && monsters[idx].state == MonsterState::Alive {
                                    apply_power(&mut monsters[idx].powers, power_id, *amount as i32);
                                }
                            }
                        }
                        EffectTarget::_Self => {
                            // "Self" resolves based on the owner (who queued the effect)
                            match target {
                                ResolvedTarget::Monster(idx) => {
                                    let idx = idx as usize;
                                    if idx < monsters.len() && monsters[idx].state == MonsterState::Alive {
                                        apply_power(&mut monsters[idx].powers, power_id, *amount as i32);
                                    }
                                }
                                ResolvedTarget::Player | ResolvedTarget::NoTarget => {
                                    apply_power(player_powers, power_id, *amount as i32);
                                }
                            }
                        }
                        EffectTarget::AllEnemies => {
                            for monster in monsters.iter_mut() {
                                if monster.state == MonsterState::Alive {
                                    apply_power(&mut monster.powers, power_id, *amount as i32);
                                }
                            }
                        }
                        EffectTarget::Player => {
                            apply_power(player_powers, power_id, *amount as i32);
                        }
                    }
                }
            }
            Effect::Draw(count) => {
                if let Some(Screen::Combat { effect_queue, .. }) = self.find_combat_mut() {
                    // Push to front in reverse order so they execute in order
                    // before any subsequent effects already in the queue.
                    for _ in 0..*count {
                        effect_queue.push_front((Effect::DrawOneCard, ResolvedTarget::NoTarget));
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
                if let Some(Screen::Combat { hand, discard_pile, draw_pile, effect_queue, .. }) = self.find_combat_mut() {
                    if hand.len() <= *min as usize {
                        // Auto-resolve: not enough cards for a real choice
                        let selected: Vec<HandCard> = hand.drain(..).collect();
                        for hc in selected {
                            apply_hand_select_action(*action, hc.card, discard_pile, draw_pile, effect_queue);
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
                        if monster.state != MonsterState::Alive {
                            continue;
                        }
                        // Only affect monsters that intend to attack (damage >= 0)
                        if let Some(dmg) = monster.damage {
                            if dmg >= 0 {
                                let hit_count = monster.hits;
                                for _ in 0..hit_count {
                                    effect_queue.push_back((
                                        Effect::DamageFixed(*thorns_damage),
                                        ResolvedTarget::Monster(i as u8),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Effect::PlayTopOfDraw => {
                if let Some(Screen::Combat { effect_queue, .. }) = self.find_combat_mut() {
                    effect_queue.push_front((Effect::PlayLastDrawnFromHand, ResolvedTarget::NoTarget));
                    effect_queue.push_front((Effect::DrawOneCard, ResolvedTarget::NoTarget));
                }
            }
            Effect::ForEachInHand { filter, per_card, exhaust_matched } => {
                if let Some(Screen::Combat { hand, effect_queue, .. }) = self.find_combat_mut() {
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
                                effect_queue.push_front((Effect::ExhaustCard { card: hc.card }, ResolvedTarget::NoTarget));
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
                        effect_queue.push_front((per_card[effect_idx].clone(), target));
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
                    target_index: match target {
                        ResolvedTarget::Monster(idx) => Some(idx),
                        _ => None,
                    },
                });
                return EffectResult::Paused;
            }
            Effect::XCost { per_energy, bonus, card_type } => {
                if let Some(Screen::Combat { player_energy, .. }) = self.screen.last() {
                    let max_energy = *player_energy;
                    self.push_screen(Screen::XCostSelect {
                        per_energy: per_energy.to_vec(),
                        bonus: *bonus,
                        card_type: *card_type,
                        target: match target {
                            ResolvedTarget::Monster(idx) => Some(idx),
                            _ => None,
                        },
                        max_energy,
                    });
                    return EffectResult::Paused;
                }
            }
            Effect::ConditionalOnDieRoll { min, max, effects } => {
                if let Some(Screen::Combat { die_roll, effect_queue, .. }) = self.find_combat_mut() {
                    let roll = die_roll.expect("ConditionalOnDieRoll used before die was rolled");
                    if roll >= *min && roll <= *max {
                        for effect in effects.iter().rev() {
                            effect_queue.push_front((effect.clone(), target));
                        }
                    }
                }
            }
            Effect::DisposeCard { card, exhaust, rebound } => {
                if *exhaust {
                    if let Some(Screen::Combat { effect_queue, .. }) = self.find_combat_mut() {
                        effect_queue.push_front((Effect::ExhaustCard { card: card.clone() }, ResolvedTarget::NoTarget));
                    }
                } else if let Some(Screen::Combat { discard_pile, draw_pile, .. }) = self.find_combat_mut() {
                    if *rebound {
                        draw_pile.push(card.clone());
                    } else {
                        discard_pile.push(card.clone());
                    }
                }
            }
            Effect::ExhaustCard { card } => {
                if let Some(Screen::Combat { exhaust_pile, player_powers, effect_queue, .. }) = self.find_combat_mut() {
                    exhaust_card(card.clone(), exhaust_pile, player_powers, effect_queue);
                }
            }
            Effect::TickDownAttackPowers { had_weak, vuln_mask } => {
                if let Some(Screen::Combat { monsters, player_powers, .. }) = self.find_combat_mut() {
                    if *had_weak {
                        apply_power(player_powers, "BGWeakened", -1);
                    }
                    // For single-target attacks, only tick the targeted monster.
                    // For AoE (no specific target), tick all monsters in the mask.
                    let effective_mask = match target {
                        ResolvedTarget::Monster(idx) => vuln_mask & (1 << idx),
                        _ => *vuln_mask,
                    };
                    for (i, monster) in monsters.iter_mut().enumerate() {
                        if effective_mask & (1 << i) != 0 {
                            apply_power(&mut monster.powers, "BGVulnerable", -1);
                        }
                    }
                }
            }
            Effect::DrawOneCard => {
                if let Some(Screen::Combat { draw_pile, discard_pile, hand, player_powers, effect_queue, .. }) = self.find_combat_mut() {
                    if power_db::has_modifier(power_db::PowerModifier::PreventDraw, player_powers) {
                        return EffectResult::Continue;
                    }
                    if draw_pile.is_empty() && !discard_pile.is_empty() {
                        // Shuffle first, then retry the draw
                        effect_queue.push_front((Effect::DrawOneCard, ResolvedTarget::NoTarget));
                        effect_queue.push_front((Effect::ShuffleDiscardIntoDraw, ResolvedTarget::NoTarget));
                        return EffectResult::Continue;
                    }
                    if let Some(card) = draw_pile.pop() {
                        let card_type = card_db::lookup(&card.id)
                            .map(|info| info.card_type);
                        hand.push(HandCard { card });

                        if let Some(ct) = card_type {
                            let triggered = power_db::collect_triggered_effects(
                                power_db::PowerTrigger::OnDraw { card_type: ct },
                                player_powers,
                                ResolvedTarget::Player,
                            );
                            queue_triggered(effect_queue, triggered);
                        }
                    }
                }
            }
            Effect::ShuffleDiscardIntoDraw => {
                if let Some(Screen::Combat { draw_pile, discard_pile, player_powers, effect_queue, .. }) = self.find_combat_mut() {
                    draw_pile.append(discard_pile);
                    draw_pile.reverse();

                    let triggered = power_db::collect_triggered_effects(
                        power_db::PowerTrigger::OnShuffle,
                        player_powers,
                        ResolvedTarget::Player,
                    );
                    queue_triggered(effect_queue, triggered);
                }
            }
            Effect::DamageFixedAll(amount) => {
                let mut triggers: Vec<(u8, DamageResult)> = Vec::new();
                if let Some(Screen::Combat { monsters, .. }) = self.find_combat_mut() {
                    for (i, monster) in monsters.iter_mut().enumerate() {
                        if monster.state == MonsterState::Alive {
                            let result = apply_damage_to_monster(monster, *amount as u16);
                            // Not from an Attack card, so was_attacked stays false
                            triggers.push((i as u8, result));
                        }
                    }
                }
                for (idx, result) in &triggers {
                    self.queue_monster_reactive_triggers(*idx, result, DamageKind::NonAttack);
                }
            }
            Effect::PlayLastDrawnFromHand => {
                if let Some(Screen::Combat { hand, player_powers, monsters, effect_queue, .. }) = self.find_combat_mut() {
                    let card = if let Some(hc) = hand.pop() {
                        hc.card
                    } else {
                        return EffectResult::Continue;
                    };
                    let info = card_db::lookup(&card.id);
                    let has_target = info.map(|i| i.target.has_target()).unwrap_or(false);
                    let is_power = info
                        .map(|i| i.card_type == card_db::CardType::Power)
                        .unwrap_or(false);
                    let is_attack = info
                        .map(|i| i.card_type == card_db::CardType::Attack)
                        .unwrap_or(false);

                    let mut all_effects: Vec<Effect> = info
                        .map(|i| i.effective_effects(card.upgraded).to_vec())
                        .unwrap_or_default();

                    if is_attack {
                        all_effects.push(make_tick_down_attack_powers_effect(player_powers, monsters));
                    }

                    if !is_power {
                        all_effects.push(Effect::DisposeCard {
                            card: card.clone(),
                            exhaust: true,
                            rebound: false,
                        });
                    }

                    if has_target {
                        self.push_screen(Screen::TargetSelect {
                            reason: TargetReason::Card(card.clone()),
                            effects: all_effects,
                        });
                        return EffectResult::Paused;
                    } else {
                        for effect in all_effects.into_iter().rev() {
                            effect_queue.push_front((effect, ResolvedTarget::NoTarget));
                        }
                    }
                }
            }
            Effect::DamageFixedTargetSelect { amount, reason } => {
                self.push_screen(Screen::TargetSelect {
                    reason: reason.clone(),
                    effects: vec![Effect::DamageFixed(*amount)],
                });
                return EffectResult::Paused;
            }
            Effect::TickDownMonsterAttackPowers { monster_had_weak, player_had_vuln } => {
                if let ResolvedTarget::Monster(idx) = target {
                    let idx = idx as usize;
                    if let Some(Screen::Combat { monsters, player_powers, .. }) = self.find_combat_mut() {
                        if *monster_had_weak && idx < monsters.len() {
                            apply_power(&mut monsters[idx].powers, "BGWeakened", -1);
                        }
                        if *player_had_vuln {
                            apply_power(player_powers, "BGVulnerable", -1);
                        }
                    }
                }
            }
            Effect::DecayMonsterBlock => {
                if let ResolvedTarget::Monster(idx) = target {
                    let idx = idx as usize;
                    if let Some(Screen::Combat { monsters, .. }) = self.find_combat_mut() {
                        if idx < monsters.len() && monsters[idx].state == MonsterState::Alive {
                            monsters[idx].block = 0;
                        }
                    }
                }
            }
            Effect::MonsterEscape => {
                if let ResolvedTarget::Monster(idx) = target {
                    let idx = idx as usize;
                    if let Some(Screen::Combat { monsters, .. }) = self.find_combat_mut() {
                        if idx < monsters.len() {
                            monsters[idx].state = MonsterState::Dead;
                        }
                    }
                }
            }
            Effect::StealGold(amount) => {
                self.gold = self.gold.saturating_sub(*amount);
            }
            Effect::SpawnMonster { id, hp } => {
                let mut monster = Monster {
                    id: id.to_string(),
                    name: id.to_string(),
                    hp: *hp,
                    max_hp: *hp,
                    block: 0,
                    intent: "UNKNOWN".to_string(),
                    damage: None,
                    hits: 1,
                    powers: vec![],
                    state: MonsterState::Alive,
                    move_index: 0,
                    pattern: monster_db::MovePattern::default(),
                };
                if let Some(info) = monster_db::lookup(id) {
                    monster.pattern = info.pattern;
                    let actual_move = monster_db::resolve_move_index(monster.pattern, 0);
                    update_monster_display(&mut monster, info, actual_move);
                }
                if let Some(Screen::Combat { monsters, effect_queue, .. }) = self.find_combat_mut() {
                    let spawn_idx = monsters.len() as u8;
                    if let Some(info) = monster_db::lookup(id) {
                        for effect in info.starting_effects {
                            effect_queue.push_back((effect.clone(), ResolvedTarget::Monster(spawn_idx)));
                        }
                    }
                    monsters.push(monster);
                }
            }
            Effect::Custom(_id) => {
                // Not yet implemented
            }
        }

        // Check for combat end after each effect
        if let Some(Screen::Combat { monsters, effect_queue, .. }) = self.find_combat_mut() {
            if !monsters.is_empty() && monsters.iter().all(|m| m.state == MonsterState::Dead) {
                effect_queue.clear();
                return EffectResult::CombatOver;
            }
        }

        // Check for player defeat
        if self.hp == 0 {
            if let Some(Screen::Combat { effect_queue, .. }) = self.find_combat_mut() {
                effect_queue.clear();
            }
            return EffectResult::CombatOver;
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

        if let Some(Screen::Combat { hand, discard_pile, draw_pile, effect_queue, .. }) = self.find_combat_mut() {
            for hi in picked {
                let hi = hi as usize;
                if hi < hand.len() {
                    let hc = hand.remove(hi);
                    apply_hand_select_action(action, hc.card, discard_pile, draw_pile, effect_queue);
                }
            }
        }
    }

    fn execute_monster_turns(&mut self) {
        // Process DeadPendingSummon monsters first (fire their death triggers)
        if let Some(Screen::Combat { monsters, effect_queue, .. }) = self.find_combat_mut() {
            for (i, monster) in monsters.iter().enumerate() {
                if monster.state == MonsterState::DeadPendingSummon {
                    let triggered = power_db::collect_triggered_effects(
                        power_db::PowerTrigger::MonsterOnDeath,
                        &monster.powers,
                        ResolvedTarget::Monster(i as u8),
                    );
                    queue_triggered(effect_queue, triggered);
                }
            }
        }
        self.drain_effect_queue();

        if let Some(Screen::Combat { monsters, player_powers, effect_queue, die_roll, turn, .. }) = self.find_combat_mut() {
            let roll = die_roll.expect("die_roll must be set before EndTurn");
            let current_turn = *turn;

            for (i, monster) in monsters.iter_mut().enumerate() {
                if monster.state != MonsterState::Alive {
                    continue;
                }

                let monster_idx = i as u8;

                if let Some(info) = monster_db::lookup(&monster.id) {
                    // 1. Decay monster block at start of turn
                    effect_queue.push_back((Effect::DecayMonsterBlock, ResolvedTarget::Monster(monster_idx)));

                    // 2. Queue current move effects
                    let move_idx = monster_db::resolve_move_index(info.pattern, monster.move_index) as usize;
                    if let Some(monster_move) = info.moves.get(move_idx) {
                        let is_attack = monster_move.effects.iter().any(|e| matches!(e, Effect::Damage(_)));

                        // Snapshot Weak/Vuln for tick-down after attack
                        let monster_had_weak = is_attack && monster.powers.iter().any(|p| p.id == "BGWeakened");
                        let player_had_vuln = is_attack && player_powers.iter().any(|p| p.id == "BGVulnerable");

                        for effect in monster_move.effects {
                            match effect {
                                Effect::Damage(base) => {
                                    effect_queue.push_back((
                                        Effect::DamageToPlayer { base: *base, monster_index: monster_idx },
                                        ResolvedTarget::Player,
                                    ));
                                }
                                Effect::MonsterBlock(_) | Effect::ApplyPower { .. } => {
                                    effect_queue.push_back((effect.clone(), ResolvedTarget::Monster(monster_idx)));
                                }
                                Effect::AddCardToPile { .. } => {
                                    // Adds status cards to player's piles
                                    effect_queue.push_back((effect.clone(), ResolvedTarget::NoTarget));
                                }
                                Effect::MonsterEscape => {
                                    effect_queue.push_back((effect.clone(), ResolvedTarget::Monster(monster_idx)));
                                }
                                Effect::StealGold(_) => {
                                    effect_queue.push_back((effect.clone(), ResolvedTarget::NoTarget));
                                }
                                _ => {
                                    panic!("Unexpected effect in monster move: {:?}", effect);
                                }
                            }
                        }

                        // Tick down Weak/Vuln after monster attack
                        if is_attack {
                            effect_queue.push_back((
                                Effect::TickDownMonsterAttackPowers { monster_had_weak, player_had_vuln },
                                ResolvedTarget::Monster(monster_idx),
                            ));
                        }
                    }

                    // 3. Queue monster EndOfTurn power triggers (e.g., Ritual → Strength)
                    let triggered = power_db::collect_triggered_effects(
                        power_db::PowerTrigger::MonsterEndOfTurn,
                        &monster.powers,
                        ResolvedTarget::Monster(monster_idx),
                    );
                    queue_triggered(effect_queue, triggered);

                    // 3. Determine next move
                    let next_idx = monster_db::next_move(monster.pattern, roll, current_turn + 1, monster.move_index);
                    monster.move_index = next_idx;
                    let actual_move = monster_db::resolve_move_index(monster.pattern, next_idx);
                    update_monster_display(monster, info, actual_move);
                }
            }
        }

        // Drain all queued effects (damage, block, powers, defeat check all happen here)
        self.drain_effect_queue();
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
            other => {
                // Default rewards based on encounter pool
                let is_elite = other.contains("Elite") || other.contains("Nob") || other.contains("Lagavulin") || other.contains("Sentries");
                if is_elite {
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
                } else {
                    rewards.push(Reward::gold(1));
                    if let Some(pools) = &mut self.reward_pools {
                        if let Some(id) = pools.potion_deck.draw() {
                            rewards.push(Reward::potion(Potion { id: id.clone(), name: id }));
                        }
                    }
                    rewards.push(Reward::card());
                }
            }
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
            Screen::Combat { hand, monsters, effect_queue, player_energy, draw_pile, player_powers, .. } => {
                assert!(effect_queue.is_empty(), "Effect queue should be empty when generating actions");
                combat_actions(hand, monsters, *player_energy, draw_pile, player_powers)
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
            Screen::TargetSelect { reason, .. } => {
                // Generate one PickTarget per live monster
                if let Some(Screen::Combat { monsters, .. }) = self.screen.iter().rev()
                    .find(|s| matches!(s, Screen::Combat { .. }))
                {
                    monsters.iter().enumerate()
                        .filter(|(_, m)| m.state == MonsterState::Alive)
                        .map(|(i, m)| Action::PickTarget {
                            reason: reason.clone(),
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
            Screen::XCostSelect { max_energy, .. } => {
                (0..=*max_energy).map(|spend| {
                    Action::PickChoice { label: format!("Spend {}", spend), choice_index: spend }
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

fn combat_actions(hand: &[HandCard], monsters: &[Monster], energy: u8, draw_pile: &[Card], player_powers: &[crate::types::Power]) -> Vec<Action> {
    let mut actions = Vec::new();
    let live_monsters: Vec<(u8, &Monster)> = monsters
        .iter()
        .enumerate()
        .filter(|(_, m)| m.state == MonsterState::Alive)
        .map(|(i, m)| (i as u8, m))
        .collect();

    // Precompute hand-level conditions for play predicates
    let all_attacks = hand.iter().all(|hc| hc.card.card_type == "ATTACK");
    let attack_count = hand.iter().filter(|hc| hc.card.card_type == "ATTACK").count();
    let draw_pile_empty = draw_pile.is_empty();

    for (i, hc) in hand.iter().enumerate() {
        let info = card_db::lookup(&hc.card.id);
        let base_cost = info
            .map(|i| i.effective_cost(hc.card.upgraded))
            .unwrap_or(hc.card.cost);
        let card_type = info.map(|i| i.card_type).unwrap_or(card_db::CardType::Skill);
        let cost = power_db::get_modified_cost(base_cost, card_type, player_powers);
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
/// Queue triggered effects, applying power self-removals immediately to prevent
/// double-triggering on multi-hit attacks (e.g. CurlUp should only fire once).
/// Queue triggered effects: back effects go to the back of the queue,
/// front effects go to the front (for immediate execution before next trigger).
fn queue_triggered(
    effect_queue: &mut std::collections::VecDeque<(Effect, ResolvedTarget)>,
    triggered: power_db::TriggeredEffects,
) {
    for entry in triggered.back {
        effect_queue.push_back(entry);
    }
    for entry in triggered.front.into_iter().rev() {
        effect_queue.push_front(entry);
    }
}

fn update_monster_display(monster: &mut crate::types::Monster, info: &monster_db::MonsterInfo, move_idx: u8) {
    if let Some(next_move) = info.moves.get(move_idx as usize) {
        let damage_effects: Vec<i16> = next_move.effects.iter()
            .filter_map(|e| if let Effect::Damage(d) = e { Some(*d) } else { None })
            .collect();
        let has_buff = next_move.effects.iter().any(|e| matches!(e,
            Effect::ApplyPower { .. } | Effect::MonsterBlock(_) | Effect::AddCardToPile { .. }));

        if !damage_effects.is_empty() {
            monster.intent = if has_buff { "ATTACK_BUFF".to_string() } else { "ATTACK".to_string() };
            monster.damage = Some(damage_effects[0]);
            monster.hits = damage_effects.len() as u8;
        } else if has_buff {
            monster.intent = "BUFF".to_string();
            monster.damage = None;
            monster.hits = 1;
        } else {
            monster.intent = "UNKNOWN".to_string();
            monster.damage = None;
            monster.hits = 1;
        }
    }
}

struct DamageResult {
    /// Monster lost HP (from any source).
    took_damage: bool,
    /// Monster reached 0 HP and is now gone.
    died: bool,
    /// Monster's block was reduced to 0 by this damage.
    block_broken: bool,
}

/// Whether damage came from an Attack card or a non-attack source.
/// Attack damage goes through the damage pipeline (Strength, Weak, Vulnerable)
/// and triggers MonsterOnAttacked powers (e.g. Angry).
/// Non-attack damage (from powers, status effects) only triggers MonsterOnDamaged (e.g. CurlUp).
enum DamageKind {
    Attack,
    NonAttack,
}

fn apply_damage_to_monster(monster: &mut crate::types::Monster, damage: u16) -> DamageResult {
    let hp_before = monster.hp;
    let had_block = monster.block > 0;
    if damage <= monster.block {
        monster.block -= damage;
    } else {
        let remaining = damage - monster.block;
        monster.block = 0;
        monster.hp = monster.hp.saturating_sub(remaining);
    }
    let took_damage = monster.hp < hp_before;
    let block_broken = had_block && monster.block == 0;
    let died = monster.hp == 0 && monster.state == MonsterState::Alive;
    if died {
        let has_death_triggers = monster.powers.iter().any(|p| {
            power_db::lookup(&p.id)
                .map(|info| info.triggers.iter().any(|te| te.trigger == power_db::PowerTrigger::MonsterOnDeath))
                .unwrap_or(false)
        });
        if has_death_triggers {
            monster.state = MonsterState::DeadPendingSummon;
        } else {
            monster.state = MonsterState::Dead;
        }
    }
    DamageResult { took_damage, died, block_broken }
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
const MAX_BLOCK: u16 = 20;

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
    draw_pile: &mut Vec<Card>,
    effect_queue: &mut std::collections::VecDeque<(Effect, ResolvedTarget)>,
) {
    match action {
        HandSelectAction::Exhaust => {
            effect_queue.push_front((Effect::ExhaustCard { card }, ResolvedTarget::NoTarget));
        }
        HandSelectAction::Discard => discard_pile.push(card),
        HandSelectAction::PutOnTopOfDraw => draw_pile.push(card),
        HandSelectAction::Upgrade => {
            // TODO: upgrade the card and put it back in hand
        }
    }
}

/// Snapshot the current Weak/Vulnerable state and produce a TickDownAttackPowers
/// effect for queuing after an attack resolves.
/// Queue a card's effects, tick-down attack powers, and handle RepeatAttack.
/// Used by both PlayCard (for normal cards) and XCostSelect resolution (for
/// XCost cards once the energy spend is known).
fn play_card_effects(
    effects: &[Effect],
    card_type: card_db::CardType,
    target: ResolvedTarget,
    player_powers: &mut Vec<crate::types::Power>,
    monsters: &[crate::types::Monster],
    effect_queue: &mut std::collections::VecDeque<(Effect, ResolvedTarget)>,
) {
    let has_xcost = effects.iter().any(|e| matches!(e, Effect::XCost { .. }));

    if has_xcost {
        // XCost effects need energy selection before they can resolve.
        // Queue the raw effects; tick-down and RepeatAttack will be handled
        // when play_card_effects is called again with resolved effects.
        for effect in effects {
            effect_queue.push_back((effect.clone(), target));
        }
        return;
    }

    let is_attack = card_type == card_db::CardType::Attack;
    let repeat_count = if is_attack {
        if let Some(repeat_power_id) = power_db::find_active_modifier(
            power_db::PowerModifier::RepeatAttack,
            player_powers,
        ) {
            apply_power(player_powers, &repeat_power_id, -1);
            2
        } else {
            1
        }
    } else {
        1
    };

    for _ in 0..repeat_count {
        for effect in effects {
            effect_queue.push_back((effect.clone(), target));
        }
        if is_attack {
            effect_queue.push_back((make_tick_down_attack_powers_effect(player_powers, monsters), target));
        }
    }
}

fn make_tick_down_attack_powers_effect(
    player_powers: &[crate::types::Power],
    monsters: &[crate::types::Monster],
) -> Effect {
    let had_weak = player_powers.iter().any(|p| p.id == "BGWeakened");
    let mut vuln_mask: u8 = 0;
    for (i, m) in monsters.iter().enumerate() {
        if m.powers.iter().any(|p| p.id == "BGVulnerable") {
            vuln_mask |= 1 << i;
        }
    }
    Effect::TickDownAttackPowers { had_weak, vuln_mask }
}

/// Move a card to the exhaust pile and queue any on-exhaust effects.
fn exhaust_card(
    card: Card,
    exhaust_pile: &mut Vec<Card>,
    player_powers: &[crate::types::Power],
    effect_queue: &mut std::collections::VecDeque<(Effect, ResolvedTarget)>,
) {
    if let Some(info) = card_db::lookup(&card.id) {
        if let Some(effects) = info.effective_on_exhaust(card.upgraded) {
            for effect in effects.iter().rev() {
                effect_queue.push_front((effect.clone(), ResolvedTarget::NoTarget));
            }
        }
    }
    exhaust_pile.push(card);

    let triggered = power_db::collect_triggered_effects(
        power_db::PowerTrigger::OnExhaust,
        player_powers,
        ResolvedTarget::Player,
    );
    queue_triggered(effect_queue, triggered);
}

