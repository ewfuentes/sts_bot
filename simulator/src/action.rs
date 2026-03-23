use serde::{Deserialize, Serialize};

use crate::map::MapNodeKind;
use crate::types::Card;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    TravelTo {
        kind: MapNodeKind,
        label: String,
        choice_index: u8,
    },
    PickNeowBlessing {
        label: String,
        choice_index: u8,
        #[serde(default)]
        reward_type: Option<String>,
        #[serde(default)]
        drawback: Option<String>,
    },
    PickEventOption {
        label: String,
        choice_index: u8,
        #[serde(default)]
        reward_type: Option<String>,
        #[serde(default)]
        drawback: Option<String>,
    },
    TakeCard {
        card: Card,
        choice_index: u8,
    },
    SkipCardReward,
    TakeReward {
        choice_index: u8,
    },
    PickBossRelic {
        choice_index: u8,
    },
    SkipBossRelic,
    BuyCard {
        card: Card,
        price: u16,
        choice_index: u8,
    },
    BuyRelic {
        relic: String,
        price: u16,
        choice_index: u8,
    },
    BuyPotion {
        potion: String,
        price: u16,
        choice_index: u8,
    },
    Purge {
        price: u16,
        choice_index: u8,
    },
    LeaveShop,
    Rest {
        choice_index: u8,
    },
    Smith {
        choice_index: u8,
    },
    OpenChest {
        choice_index: u8,
    },
    PickGridCard {
        card: Card,
        choice_index: u8,
    },
    PickHandCard {
        card: Card,
        choice_index: u8,
    },
    PickCustomScreenOption {
        label: String,
        choice_index: u8,
    },
    Proceed,
    Skip,
}

impl Action {
    pub fn to_commod_command(&self) -> String {
        match self {
            Action::TravelTo { choice_index, .. } => format!("choose {}", choice_index),
            Action::PickNeowBlessing { choice_index, .. } => format!("choose {}", choice_index),
            Action::PickEventOption { choice_index, .. } => format!("choose {}", choice_index),
            Action::TakeCard { choice_index, .. } => format!("choose {}", choice_index),
            Action::SkipCardReward => "skip".to_string(),
            Action::TakeReward { choice_index, .. } => format!("choose {}", choice_index),
            Action::PickBossRelic { choice_index, .. } => format!("choose {}", choice_index),
            Action::SkipBossRelic => "skip".to_string(),
            Action::BuyCard { choice_index, .. } => format!("choose {}", choice_index),
            Action::BuyRelic { choice_index, .. } => format!("choose {}", choice_index),
            Action::BuyPotion { choice_index, .. } => format!("choose {}", choice_index),
            Action::Purge { choice_index, .. } => format!("choose {}", choice_index),
            Action::LeaveShop => "return".to_string(),
            Action::Rest { choice_index, .. } => format!("choose {}", choice_index),
            Action::Smith { choice_index, .. } => format!("choose {}", choice_index),
            Action::OpenChest { choice_index, .. } => format!("choose {}", choice_index),
            Action::PickGridCard { choice_index, .. } => format!("choose {}", choice_index),
            Action::PickHandCard { choice_index, .. } => format!("choose {}", choice_index),
            Action::PickCustomScreenOption { choice_index, .. } => format!("choose {}", choice_index),
            Action::Proceed => "proceed".to_string(),
            Action::Skip => "skip".to_string(),
        }
    }
}
