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
        available_nodes: Vec<MapChoice>,
    },
    Combat {},
    CardReward {
        cards: Vec<Card>,
    },
    CombatRewards {
        rewards: Vec<Reward>,
    },
    BossRelic {
        relics: Vec<crate::types::Relic>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reward {
    #[serde(rename = "type")]
    pub reward_type: String,
    pub gold: Option<u16>,
    pub relic: Option<crate::types::Relic>,
    pub potion: Option<crate::types::Potion>,
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
