use serde::{Deserialize, Serialize};

use crate::effects::{Effect, HandSelectAction};
use crate::map::MapNodeKind;
use crate::types::{Card, Monster, Power};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TargetReason {
    Card(Card),
    Power(Power),
    /// Placeholder used in static power_db templates; resolved to a concrete
    /// variant by substitute_amount before the effect executes.
    Pending,
}

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
        /// Per-combat RNG, seeded from the map node's seed.
        #[serde(skip)]
        rng: crate::rng::Rng,
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
        #[serde(default)]
        destination: DiscardSelectDestination,
    },
    ExhaustSelect {
        cards: Vec<(u8, Card)>,
    },
    TargetSelect {
        reason: TargetReason,
        #[serde(skip)]
        effects: Vec<Effect>,
    },
    ChoiceSelect {
        #[serde(skip)]
        choices: Vec<(String, Vec<Effect>)>,
        #[serde(skip)]
        target_index: Option<u8>,
    },
    /// Energy selection screen for X-cost cards. The player chooses how much
    /// energy to spend; the resolved effects are then played via play_card_effects.
    /// This screen is only created at runtime — never deserialized.
    #[serde(skip)]
    XCostSelect {
        per_energy: Vec<Effect>,
        bonus: i16,
        card_type: crate::card_db::CardType,
        target: Option<u8>,
        max_energy: u8,
    },
    /// Cards drawn by Distilled Chaos waiting to be played for free.
    /// Player picks one at a time; each is played before the next pick.
    AutoPlaySelect {
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

impl Screen {
    /// Create a Map screen from a map and a starting node index.
    /// Computes available_nodes from the start node's edges.
    pub fn new_map(map: &crate::map::ActMap, start_node: usize) -> Self {
        let start = &map.nodes[start_node];
        let available_nodes = vec![MapChoice {
            label: format!("{:?} ({},{})", start.kind, start.x, start.y),
            kind: start.kind,
            node_index: start_node,
        }];
        Screen::Map {
            current_node: start_node,
            available_nodes,
        }
    }

    /// Serialize this screen to a JSON value for training/summary use.
    /// Unlike serde Serialize, this handles all variants including those
    /// with non-serializable Effect fields.
    pub fn to_summary_json(&self) -> serde_json::Value {
        match self {
            Screen::XCostSelect { bonus, card_type, target, max_energy, .. } => {
                serde_json::json!({
                    "type": "x_cost_select",
                    "bonus": bonus,
                    "card_type": format!("{:?}", card_type),
                    "target": target,
                    "max_energy": max_energy,
                })
            }
            Screen::AutoPlaySelect { cards } => {
                serde_json::json!({
                    "type": "auto_play_select",
                    "cards": cards,
                })
            }
            Screen::TargetSelect { reason, .. } => {
                serde_json::json!({
                    "type": "target_select",
                    "reason": reason,
                })
            }
            Screen::ChoiceSelect { choices, target_index } => {
                let labels: Vec<&str> = choices.iter().map(|(l, _)| l.as_str()).collect();
                serde_json::json!({
                    "type": "choice_select",
                    "choices": labels,
                    "target_index": target_index,
                })
            }
            Screen::HandSelect { min_cards, max_cards, cards, picked_indices, .. } => {
                serde_json::json!({
                    "type": "hand_select",
                    "min_cards": min_cards,
                    "max_cards": max_cards,
                    "cards": cards,
                    "picked_indices": picked_indices,
                })
            }
            // All other variants serialize fine with serde
            other => serde_json::to_value(other).unwrap_or_else(|e| {
                serde_json::json!({"type": "unknown", "error": e.to_string()})
            }),
        }
    }

    pub fn new_combat(encounter: impl Into<String>, seed: u64) -> Self {
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
            rng: crate::rng::Rng::from_seed(seed),
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscardSelectDestination {
    #[default]
    DrawPile,
    Hand { cost_override: Option<i8> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandCard {
    #[serde(flatten)]
    pub card: Card,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_override: Option<i8>,
}
