use serde::{Deserialize, Serialize};

use crate::map::MapNodeKind;
use crate::types::Card;

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
        max_cards: u8,
        cards: Vec<Card>,
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
