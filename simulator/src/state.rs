use crate::action::Action;
use crate::map::{ActMap, MapNodeKind};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameState {
    pub ascension: u8,
    pub act: u8,
    pub floor: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub gold: u16,
    pub map: ActMap,
    pub position: Option<usize>,
    pub screen: Screen,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Screen {
    Neow,
    MapSelect,
    Combat,
    Event,
    Rest,
    Shop,
    Treasure,
    CardReward,
    CombatRewards,
    BossReward,
    GameOver { victory: bool },
}

impl GameState {
    pub fn new(ascension: u8) -> Self {
        GameState {
            ascension,
            act: 1,
            floor: 0,
            hp: 80,
            max_hp: 80,
            gold: 99,
            map: ActMap { nodes: vec![] },
            position: None,
            screen: Screen::Neow,
        }
    }

    pub fn available_actions(&self) -> Vec<Action> {
        match &self.screen {
            Screen::Neow => {
                vec![Action::Proceed]
            }
            Screen::MapSelect => {
                self.reachable_nodes()
                    .into_iter()
                    .map(|i| Action::TravelTo {
                        node_index: i,
                        kind: self.map.nodes[i].kind,
                    })
                    .collect()
            }
            Screen::Rest => {
                vec![Action::Rest, Action::Smith]
            }
            Screen::Treasure => {
                vec![Action::OpenChest]
            }
            Screen::Combat
            | Screen::Event
            | Screen::Shop
            | Screen::CardReward
            | Screen::CombatRewards
            | Screen::BossReward => {
                // TODO: real actions for these screens
                vec![Action::Proceed]
            }
            Screen::GameOver { .. } => vec![],
        }
    }

    pub fn apply(&mut self, action: Action) {
        match (&self.screen, &action) {
            (Screen::Neow, Action::Proceed) => {
                self.screen = Screen::MapSelect;
            }
            (Screen::MapSelect, Action::TravelTo { node_index, kind }) => {
                self.position = Some(*node_index);
                self.floor += 1;
                self.enter_room(*kind);
            }
            (Screen::Rest, Action::Rest) => {
                let heal = self.max_hp * 30 / 100;
                self.hp = (self.hp + heal).min(self.max_hp);
                self.screen = Screen::MapSelect;
            }
            (Screen::Rest, Action::Smith) => {
                // TODO: card upgrade selection
                self.screen = Screen::MapSelect;
            }
            (Screen::Treasure, Action::OpenChest) => {
                // TODO: generate chest contents
                self.screen = Screen::MapSelect;
            }
            (Screen::Combat, Action::Proceed) => {
                self.screen = Screen::CombatRewards;
            }
            (Screen::CombatRewards, Action::Proceed) => {
                if self.is_boss_floor() {
                    self.screen = Screen::BossReward;
                } else {
                    self.screen = Screen::MapSelect;
                }
            }
            (Screen::BossReward, Action::Proceed) => {
                self.advance_act();
            }
            (Screen::Event, Action::Proceed)
            | (Screen::Shop, Action::Proceed)
            | (Screen::CardReward, Action::Proceed) => {
                self.screen = Screen::MapSelect;
            }
            _ => {}
        }
    }

    fn enter_room(&mut self, kind: MapNodeKind) {
        self.screen = match kind {
            MapNodeKind::Monster | MapNodeKind::Elite => Screen::Combat,
            MapNodeKind::Rest => Screen::Rest,
            MapNodeKind::Shop => Screen::Shop,
            MapNodeKind::Event => Screen::Event,
            MapNodeKind::Treasure => Screen::Treasure,
            MapNodeKind::Boss => Screen::Combat,
        };
    }

    fn reachable_nodes(&self) -> Vec<usize> {
        match self.position {
            None => {
                // Start of act: any node on row 0
                (0..self.map.nodes.len())
                    .filter(|i| self.map.nodes[*i].y == 0)
                    .collect()
            }
            Some(pos) => self.map.nodes[pos].edges.clone(),
        }
    }

    fn is_boss_floor(&self) -> bool {
        // Acts 1-3 have 15 floors each, boss at floor 15/30/45
        self.floor == self.act as u8 * 15
    }

    fn advance_act(&mut self) {
        if self.act >= 3 {
            self.screen = Screen::GameOver { victory: true };
        } else {
            self.act += 1;
            self.position = None;
            self.map = ActMap { nodes: vec![] }; // TODO: generate new map
            self.screen = Screen::MapSelect;
        }
    }
}
