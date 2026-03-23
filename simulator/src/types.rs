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
