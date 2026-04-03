/// Monster move effects — what happens when a monster executes a move.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MonsterEffect {
    DamagePlayer(i16),
    DamagePlayerMulti { damage: i16, hits: u8 },
    GainBlock(u16),
    ApplyPowerToSelf { power_id: &'static str, amount: i16 },
    ApplyPowerToPlayer { power_id: &'static str, amount: i16 },
}

/// A single monster move (one entry in the move table).
#[derive(Debug, Clone)]
pub struct MonsterMove {
    pub name: &'static str,
    pub effects: &'static [MonsterEffect],
}

/// How a monster selects its next move.
#[derive(Debug, Clone, Copy)]
pub enum MovePattern {
    /// Die roll mapping: 1-2 → indices[0], 3-4 → indices[1], 5-6 → indices[2]
    DieRoll3([u8; 3]),
    /// First turn uses `first`, all subsequent turns use `repeat`.
    FirstThenRepeat { first: u8, repeat: u8 },
    /// Always the same move.
    Fixed(u8),
}

/// Static definition of a monster type.
pub struct MonsterInfo {
    pub id: &'static str,
    pub moves: &'static [MonsterMove],
    pub pattern: MovePattern,
}

// ── Monster definitions ──

static MONSTERS: &[MonsterInfo] = &[
    MonsterInfo {
        id: "BGSpikeSlime_S",
        moves: &[
            MonsterMove {
                name: "Tackle",
                effects: &[MonsterEffect::DamagePlayer(1)],
            },
        ],
        pattern: MovePattern::Fixed(0),
    },
    MonsterInfo {
        id: "BGCultist",
        moves: &[
            // Move 0: Incantation (attack + gain Strength)
            MonsterMove {
                name: "Incantation",
                effects: &[
                    MonsterEffect::DamagePlayer(1),
                    MonsterEffect::ApplyPowerToSelf { power_id: "Strength", amount: 1 },
                ],
            },
            // Move 1: Dark Strike (attack + gain Strength from Ritual)
            // Ritual is modeled as +1 Strength after each attack rather than
            // a separate triggered power, to avoid first-turn timing issues.
            MonsterMove {
                name: "Dark Strike",
                effects: &[
                    MonsterEffect::DamagePlayer(1),
                    MonsterEffect::ApplyPowerToSelf { power_id: "Strength", amount: 1 },
                ],
            },
        ],
        pattern: MovePattern::FirstThenRepeat { first: 0, repeat: 1 },
    },
    MonsterInfo {
        id: "BGJawWorm",
        moves: &[
            // Move 0: Chomp (attack)
            MonsterMove {
                name: "Chomp",
                effects: &[MonsterEffect::DamagePlayer(3)],
            },
            // Move 1: Thrash (attack + block)
            MonsterMove {
                name: "Thrash",
                effects: &[
                    MonsterEffect::DamagePlayer(2),
                    MonsterEffect::GainBlock(2),
                ],
            },
            // Move 2: Bellow (block + Strength)
            MonsterMove {
                name: "Bellow",
                effects: &[
                    MonsterEffect::GainBlock(2),
                    MonsterEffect::ApplyPowerToSelf { power_id: "Strength", amount: 1 },
                ],
            },
        ],
        // Behavior "sda": rolls 1-2 → Bellow(2), 3-4 → Thrash(1), 5-6 → Chomp(0)
        pattern: MovePattern::DieRoll3([2, 1, 0]),
    },
];

pub fn lookup(id: &str) -> Option<&'static MonsterInfo> {
    MONSTERS.iter().find(|m| m.id == id)
}

/// Determine the next move index based on the monster's pattern and die roll.
pub fn next_move(pattern: MovePattern, die_roll: u8, turn: u16) -> u8 {
    match pattern {
        MovePattern::Fixed(idx) => idx,
        MovePattern::FirstThenRepeat { first, repeat } => {
            if turn == 1 { first } else { repeat }
        }
        MovePattern::DieRoll3(indices) => {
            let bucket = match die_roll {
                1..=2 => 0,
                3..=4 => 1,
                5..=6 => 2,
                _ => 0,
            };
            indices[bucket]
        }
    }
}
