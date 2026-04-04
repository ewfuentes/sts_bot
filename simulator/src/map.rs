#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapNodeKind {
    Monster,
    Elite,
    Rest,
    Shop,
    Event,
    Treasure,
    Boss,
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MapNode {
    pub x: u8,
    pub y: u8,
    pub kind: MapNodeKind,
    pub edges: Vec<usize>,
    /// Hidden encounter ID for monster/elite/boss nodes. Not serialized
    /// so the model can't see which enemies it will face.
    #[serde(skip)]
    pub encounter: Option<String>,
    /// Pre-computed seed for this node's RNG (fights, events, etc.).
    #[serde(skip)]
    pub seed: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActMap {
    pub nodes: Vec<MapNode>,
}
