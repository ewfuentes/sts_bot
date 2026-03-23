#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MapNodeKind {
    Monster,
    Elite,
    Rest,
    Shop,
    Event,
    Treasure,
    Boss,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MapNode {
    pub x: u8,
    pub y: u8,
    pub kind: MapNodeKind,
    pub edges: Vec<usize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActMap {
    pub nodes: Vec<MapNode>,
}
