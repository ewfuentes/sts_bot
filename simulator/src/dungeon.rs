use crate::encounter_db::{self, EncounterPool};
use crate::map::{ActMap, MapNode, MapNodeKind};

/// Generate an Act 1 dungeon map with encounters assigned to each node.
/// `shuffle` is called to randomize pool order before drawing.
pub fn generate_act1_map(
    map: &ActMap,
    shuffle: &mut dyn FnMut(&mut Vec<&str>),
) -> Vec<MapNode> {
    let mut weak_pool = encounter_db::encounter_pool(1, EncounterPool::Weak);
    let mut strong_pool = encounter_db::encounter_pool(1, EncounterPool::Strong);
    let mut elite_pool = encounter_db::encounter_pool(1, EncounterPool::Elite);
    let mut boss_pool = encounter_db::encounter_pool(1, EncounterPool::Boss);

    shuffle(&mut weak_pool);
    shuffle(&mut strong_pool);
    shuffle(&mut elite_pool);
    shuffle(&mut boss_pool);

    let mut weak_iter = weak_pool.into_iter();
    let mut strong_iter = strong_pool.into_iter();
    let mut elite_iter = elite_pool.into_iter();
    let mut boss_iter = boss_pool.into_iter();

    let mut first_monster = true;

    map.nodes
        .iter()
        .map(|node| {
            let encounter = match node.kind {
                MapNodeKind::Monster => {
                    if first_monster {
                        first_monster = false;
                        weak_iter.next().map(|s| s.to_string())
                    } else {
                        strong_iter.next().map(|s| s.to_string())
                    }
                }
                MapNodeKind::Elite => elite_iter.next().map(|s| s.to_string()),
                MapNodeKind::Boss => boss_iter.next().map(|s| s.to_string()),
                _ => None,
            };
            MapNode {
                x: node.x,
                y: node.y,
                kind: node.kind,
                edges: node.edges.clone(),
                encounter,
            }
        })
        .collect()
}
