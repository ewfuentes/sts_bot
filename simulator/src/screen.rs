use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::effects::{Effect, HandSelectAction};
use crate::map::MapNodeKind;
use crate::types::{Card, Monster, Power};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Screen {
    Neow {
        options: Vec<EventOption>,
    },
    Event {
        event_id: String,
        event_name: String,
        options: Vec<EventOption>,
    },
    Map {
        #[serde(default)]
        current_node: usize,
        available_nodes: Vec<MapChoice>,
    },
    Combat {
        #[serde(default)]
        encounter: String,
        #[serde(default)]
        monsters: Vec<Monster>,
        #[serde(default)]
        hand: Vec<HandCard>,
        #[serde(default)]
        draw_pile: Vec<Card>,
        #[serde(default)]
        discard_pile: Vec<Card>,
        #[serde(default)]
        exhaust_pile: Vec<Card>,
        #[serde(default)]
        player_block: u16,
        #[serde(default)]
        player_energy: u8,
        #[serde(default)]
        player_powers: Vec<Power>,
        #[serde(default)]
        turn: u16,
        /// The die roll for this turn (1-6). None if not yet rolled.
        #[serde(default)]
        die_roll: Option<u8>,
        /// Queue of effects waiting to execute. Each entry is (effect, target_index).
        /// Target is Some for single-target effects, None for AoE/self/untargeted.
        #[serde(skip)]
        effect_queue: VecDeque<(Effect, Option<u8>)>,
    },
    CardReward {
        cards: Vec<Card>,
    },
    CombatRewards {
        rewards: Vec<Reward>,
    },
    BossRelic {
        relics: Vec<crate::types::Relic>,
        #[serde(default)]
        cards: Vec<Card>,
    },
    Shop {
        cards: Vec<ShopCard>,
        relics: Vec<ShopRelic>,
        potions: Vec<ShopPotion>,
        purge_cost: Option<u16>,
    },
    Rest {
        options: Vec<String>,
    },
    Treasure,
    Grid {
        purpose: String,
        cards: Vec<Card>,
    },
    HandSelect {
        min_cards: u8,
        max_cards: u8,
        /// Each entry is (hand_index, card) — the hand_index refers to
        /// the card's position in the combat hand.
        cards: Vec<(u8, Card)>,
        /// Hand indices selected so far (applied when screen resolves).
        #[serde(skip)]
        picked_indices: Vec<u8>,
        #[serde(skip)]
        action: HandSelectAction,
    },
    DiscardSelect {
        cards: Vec<(u8, Card)>,
    },
    ChoiceSelect {
        #[serde(skip)]
        choices: Vec<(String, Vec<Effect>)>,
        #[serde(skip)]
        target_index: Option<u8>,
        /// Energy to deduct for each choice (empty = no energy cost).
        #[serde(skip)]
        energy_costs: Vec<u8>,
    },
    CustomScreen {
        screen_enum: String,
        options: Vec<String>,
    },
    GameOver {
        victory: bool,
    },
    Complete,
    ShopRoom,
    #[serde(rename = "main_menu")]
    MainMenu,
    Error {
        message: String,
    },
    Unknown {
        raw_screen_type: String,
    },
}

impl Screen {
    pub fn new_combat(encounter: impl Into<String>) -> Self {
        Screen::Combat {
            encounter: encounter.into(),
            monsters: vec![],
            hand: vec![],
            draw_pile: vec![],
            discard_pile: vec![],
            exhaust_pile: vec![],
            player_block: 0,
            player_energy: 0,
            player_powers: vec![],
            turn: 0,
            die_roll: None,
            effect_queue: VecDeque::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventOption {
    pub label: String,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub reward_type: Option<String>,
    #[serde(default)]
    pub drawback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapChoice {
    pub label: String,
    pub kind: MapNodeKind,
    #[serde(default)]
    pub node_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reward {
    #[serde(rename = "type")]
    pub reward_type: String,
    pub gold: Option<u16>,
    pub relic: Option<crate::types::Relic>,
    pub potion: Option<crate::types::Potion>,
}

impl Reward {
    pub fn gold(amount: u16) -> Self {
        Reward { reward_type: "GOLD".into(), gold: Some(amount), relic: None, potion: None }
    }
    pub fn card() -> Self {
        Reward { reward_type: "CARD".into(), gold: None, relic: None, potion: None }
    }
    pub fn upgraded_card() -> Self {
        Reward { reward_type: "UPGRADED_CARD".into(), gold: None, relic: None, potion: None }
    }
    pub fn rare_card() -> Self {
        Reward { reward_type: "RARE_CARD".into(), gold: None, relic: None, potion: None }
    }
    pub fn potion(p: crate::types::Potion) -> Self {
        Reward { reward_type: "POTION".into(), gold: None, relic: None, potion: Some(p) }
    }
    pub fn relic(r: crate::types::Relic) -> Self {
        Reward { reward_type: "RELIC".into(), gold: None, relic: Some(r), potion: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopCard {
    #[serde(flatten)]
    pub card: Card,
    pub price: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopRelic {
    pub id: String,
    pub name: String,
    pub price: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopPotion {
    pub id: String,
    pub name: String,
    pub price: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandCard {
    #[serde(flatten)]
    pub card: Card,
}
