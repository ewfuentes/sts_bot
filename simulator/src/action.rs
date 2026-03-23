use crate::map::MapNodeKind;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Action {
    TravelTo { node_index: usize, kind: MapNodeKind },
    OpenChest,
    Rest,
    Smith,
    Proceed,
}
