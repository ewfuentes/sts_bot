use crate::encounter_db::{self, EncounterPool};
use crate::map::{ActMap, MapNode, MapNodeKind};
use crate::rng::Rng;

const COLS: usize = 7;
const ROWS: usize = 13;

/// Two fixed Act 1 map layouts from the board game.
/// Listed top-to-bottom (index 0 = boss row, index 12 = first encounter row).
/// Reversed during generation so row 0 = bottom.
static LAYOUT_A: [&str; ROWS] = [
    "...B...",
    ".RR.RR.",
    "D.$.D.?",
    "L.M.?.L",
    ".D..L.D",
    "M.?.D.M",
    "T..T.T.",
    ".L..L.D",
    "D.M.D.L",
    "M.L.?.?",
    "?..M..M",
    ".?.?.?.",
    "...M...",
];

static LAYOUT_B: [&str; ROWS] = [
    "...B...",
    ".RR.RR.",
    "?.D.D.?",
    "E.L.?.D",
    "L.?.L.L",
    ".M.D.D.",
    ".T.TT.T",
    "?.D.M.D",
    "D.M.L.L",
    "$.L.?.M",
    "M..M..?",
    ".?.?.?.",
    "...M...",
];

/// Connections for layout A. Each entry: (source "row-col", destinations ["row-col", ...])
static CONNECTIONS_A: &[(&str, &[&str])] = &[
    ("0-3", &["1-1", "1-3", "1-5"]),
    ("1-1", &["2-0"]),
    ("1-3", &["2-3"]),
    ("1-5", &["2-6"]),
    ("2-0", &["3-0"]),
    ("2-3", &["3-2"]),
    ("2-3", &["3-4"]),
    ("2-6", &["3-6"]),
    ("3-0", &["4-0", "4-2"]),
    ("3-2", &["4-2", "4-4"]),
    ("3-4", &["4-4"]),
    ("3-6", &["4-6"]),
    ("4-0", &["5-1"]),
    ("4-2", &["5-4"]),
    ("4-4", &["5-4"]),
    ("4-6", &["5-6"]),
    ("5-1", &["6-0", "6-3"]),
    ("5-4", &["6-3", "6-5"]),
    ("5-6", &["6-5"]),
    ("6-0", &["7-0"]),
    ("6-3", &["7-2", "7-4"]),
    ("6-5", &["7-4", "7-6"]),
    ("7-0", &["8-1"]),
    ("7-2", &["8-1"]),
    ("7-4", &["8-4"]),
    ("7-6", &["8-6"]),
    ("8-1", &["9-0", "9-2"]),
    ("8-4", &["9-2", "9-4"]),
    ("8-6", &["9-6"]),
    ("9-0", &["10-0"]),
    ("9-2", &["10-2"]),
    ("9-4", &["10-4"]),
    ("9-6", &["10-4", "10-6"]),
    ("10-0", &["11-1"]),
    ("10-2", &["11-2"]),
    ("10-4", &["11-4"]),
    ("10-6", &["11-5"]),
    ("11-1", &["12-3"]),
    ("11-2", &["12-3"]),
    ("11-4", &["12-3"]),
    ("11-5", &["12-3"]),
];

static CONNECTIONS_B: &[(&str, &[&str])] = &[
    ("0-3", &["1-1", "1-3", "1-5"]),
    ("1-1", &["2-0"]),
    ("1-3", &["2-3"]),
    ("1-5", &["2-6"]),
    ("2-0", &["3-0", "3-2"]),
    ("2-3", &["3-2", "3-4"]),
    ("2-6", &["3-6"]),
    ("3-0", &["4-0"]),
    ("3-2", &["4-2"]),
    ("3-4", &["4-4"]),
    ("3-6", &["4-6"]),
    ("4-0", &["5-0"]),
    ("4-2", &["5-0", "5-2"]),
    ("4-4", &["5-2", "5-4"]),
    ("4-6", &["5-4", "5-6"]),
    ("5-0", &["6-1"]),
    ("5-2", &["6-1"]),
    ("5-4", &["6-3", "6-4"]),
    ("5-6", &["6-6"]),
    ("6-1", &["7-1", "7-3"]),
    ("6-3", &["7-3"]),
    ("6-4", &["7-5"]),
    ("6-6", &["7-5"]),
    ("7-1", &["8-0", "8-2"]),
    ("7-3", &["8-2", "8-4"]),
    ("7-5", &["8-4", "8-6"]),
    ("8-0", &["9-0"]),
    ("8-2", &["9-2"]),
    ("8-4", &["9-4"]),
    ("8-6", &["9-6"]),
    ("9-0", &["10-0"]),
    ("9-2", &["10-2", "10-4"]),
    ("9-4", &["10-4"]),
    ("9-6", &["10-6"]),
    ("10-0", &["11-1"]),
    ("10-2", &["11-2"]),
    ("10-4", &["11-4"]),
    ("10-6", &["11-5"]),
    ("11-1", &["12-3"]),
    ("11-2", &["12-3"]),
    ("11-4", &["12-3"]),
    ("11-5", &["12-3"]),
];

/// Dark token pool for A0 (no emerald key).
const DARK_TOKENS: &[char] = &['E', 'E', 'E', 'M', 'M', 'M', '?', '?'];
/// Light token pool for A0.
const LIGHT_TOKENS: &[char] = &['M', '?', '$', '$', 'R', 'R', 'R'];

fn parse_coord(s: &str) -> (usize, usize) {
    let mut parts = s.split('-');
    let row: usize = parts.next().unwrap().parse().unwrap();
    let col: usize = parts.next().unwrap().parse().unwrap();
    (row, col)
}

fn node_index(row: usize, col: usize) -> usize {
    row * COLS + col
}

fn symbol_to_kind(ch: char) -> Option<MapNodeKind> {
    match ch {
        'M' => Some(MapNodeKind::Monster),
        'E' => Some(MapNodeKind::Elite),
        'B' => Some(MapNodeKind::Boss),
        'R' => Some(MapNodeKind::Rest),
        '$' => Some(MapNodeKind::Shop),
        '?' => Some(MapNodeKind::Event),
        'T' => Some(MapNodeKind::Treasure),
        _ => None,
    }
}

/// Generate the Act 1 board game map.
/// Returns an ActMap with nodes laid out in a 7×13 grid, and the index of the starting node.
pub fn generate_act1_map(rng: &mut Rng) -> (ActMap, usize) {
    // Pick layout
    let (layout, connections) = if rng.roll_die(2) == 1 {
        (&LAYOUT_A, CONNECTIONS_A)
    } else {
        (&LAYOUT_B, CONNECTIONS_B)
    };

    // Shuffle token pools
    let mut dark_tokens: Vec<char> = DARK_TOKENS.to_vec();
    let mut light_tokens: Vec<char> = LIGHT_TOKENS.to_vec();
    rng.shuffle(&mut dark_tokens);
    rng.shuffle(&mut light_tokens);
    let mut dark_iter = dark_tokens.into_iter();
    let mut light_iter = light_tokens.into_iter();

    // Build nodes — layout is top-to-bottom, reverse to make row 0 = bottom
    let mut nodes: Vec<MapNode> = Vec::with_capacity(ROWS * COLS);
    for row in 0..ROWS {
        let layout_row = ROWS - 1 - row; // reverse: layout index 12 = row 0
        let line = layout[layout_row];
        for (col, ch) in line.chars().enumerate() {
            // Resolve tokens
            let resolved = match ch {
                'D' => dark_iter.next().unwrap_or('M'),
                'L' => light_iter.next().unwrap_or('M'),
                other => other,
            };
            let kind = symbol_to_kind(resolved).unwrap_or(MapNodeKind::Unknown);
            nodes.push(MapNode {
                x: col as u8,
                y: row as u8,
                kind,
                edges: vec![],
                encounter: None,
                seed: rng.derive_seed(),
            });
        }
    }

    // Build edges from connections
    for (src_str, dst_strs) in connections {
        let (src_row, src_col) = parse_coord(src_str);
        let src_idx = node_index(src_row, src_col);
        for dst_str in *dst_strs {
            let (dst_row, dst_col) = parse_coord(dst_str);
            let dst_idx = node_index(dst_row, dst_col);
            nodes[src_idx].edges.push(dst_idx);
        }
    }

    // Assign encounters to monster/elite/boss nodes
    let mut weak_pool = encounter_db::encounter_pool(1, EncounterPool::Weak);
    let mut strong_pool = encounter_db::encounter_pool(1, EncounterPool::Strong);
    let mut elite_pool = encounter_db::encounter_pool(1, EncounterPool::Elite);
    let mut boss_pool = encounter_db::encounter_pool(1, EncounterPool::Boss);
    rng.shuffle(&mut weak_pool);
    rng.shuffle(&mut strong_pool);
    rng.shuffle(&mut elite_pool);
    rng.shuffle(&mut boss_pool);

    let mut event_pool = crate::event_db::act1_event_ids();
    rng.shuffle(&mut event_pool);

    let mut weak_iter = weak_pool.into_iter();
    let mut strong_iter = strong_pool.into_iter();
    let mut elite_iter = elite_pool.into_iter();
    let mut boss_iter = boss_pool.into_iter();
    let mut event_iter = event_pool.into_iter().cycle(); // cycle in case more ? nodes than events
    let mut first_monster = true;

    for node in &mut nodes {
        if node.kind == MapNodeKind::Unknown {
            continue; // empty cell
        }
        node.encounter = match node.kind {
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
            MapNodeKind::Event => event_iter.next().map(|s| s.to_string()),
            _ => None,
        };
    }

    let start_node = node_index(0, 3); // Row 0, col 3 — the first monster
    (ActMap { nodes }, start_node)
}
