use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub cost: i8,
    #[serde(rename = "type")]
    pub card_type: String,
    pub upgraded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Relic {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub counter: i32,
    #[serde(default)]
    pub clickable: bool,
    #[serde(default)]
    pub pulsing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Potion {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Power {
    pub id: String,
    pub amount: i32,
}

fn default_hits() -> u8 {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Monster {
    pub id: String,
    pub name: String,
    pub hp: u16,
    pub max_hp: u16,
    #[serde(default)]
    pub block: u16,
    #[serde(default)]
    pub intent: String,
    pub damage: Option<i16>,
    #[serde(default = "default_hits")]
    pub hits: u8,
    #[serde(default)]
    pub powers: Vec<Power>,
    #[serde(default)]
    pub is_gone: bool,
    /// Current move index into the monster_db move table.
    #[serde(default)]
    pub move_index: u8,
    /// Per-instance move pattern, copied from monster_db at spawn time.
    /// DieRoll3 indices are shuffled per instance for encounter variety.
    #[serde(skip)]
    pub pattern: crate::monster_db::MovePattern,
}
