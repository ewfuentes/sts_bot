use crate::effects::{Effect, EffectTarget, Pile};

/// A single monster move (one entry in the move table).
#[derive(Debug, Clone)]
pub struct MonsterMove {
    pub name: &'static str,
    /// Effects use the same Effect enum as cards. At execution time:
    /// - Damage(n): calculated with monster as attacker, player as defender,
    ///   then queued as DamageToPlayer
    /// - Block(n): applied to the monster
    /// - ApplyPower with _Self: applied to the monster
    /// - ApplyPower with TargetEnemy: applied to the player
    pub effects: &'static [Effect],
}

/// How a monster selects its next move.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MovePattern {
    /// Die roll mapping: 1-2 → indices[0], 3-4 → indices[1], 5-6 → indices[2]
    DieRoll3([u8; 3]),
    /// First turn uses `first`, all subsequent turns use `repeat`.
    FirstThenRepeat { first: u8, repeat: u8 },
    /// Always the same move.
    Fixed(u8),
}

impl Default for MovePattern {
    fn default() -> Self {
        MovePattern::Fixed(0)
    }
}

/// Static definition of a monster type.
pub struct MonsterInfo {
    pub id: &'static str,
    pub moves: &'static [MonsterMove],
    pub pattern: MovePattern,
    /// Effects applied at the start of combat (pre-battle).
    pub starting_effects: &'static [Effect],
}

// ── Monster definitions ──

static MONSTERS: &[MonsterInfo] = &[
    MonsterInfo {
        id: "BGSpikeSlime_S",
        moves: &[
            MonsterMove {
                name: "Tackle",
                effects: &[Effect::Damage(1)],
            },
        ],
        pattern: MovePattern::Fixed(0),
        starting_effects: &[],
    },
    MonsterInfo {
        id: "BGCultist",
        moves: &[
            // Move 0: Incantation (attack only — Ritual handles all Strength gain)
            MonsterMove {
                name: "Incantation",
                effects: &[Effect::Damage(1)],
            },
            // Move 1: Dark Strike (attack only — Ritual handles Strength gain)
            MonsterMove {
                name: "Dark Strike",
                effects: &[Effect::Damage(1)],
            },
        ],
        pattern: MovePattern::FirstThenRepeat { first: 0, repeat: 1 },
        starting_effects: &[
            Effect::ApplyPower { target: EffectTarget::_Self, power_id: "Ritual", amount: 1 },
        ],
    },
    MonsterInfo {
        id: "BGJawWorm",
        moves: &[
            // Move 0: Chomp (attack)
            MonsterMove {
                name: "Chomp",
                effects: &[Effect::Damage(3)],
            },
            // Move 1: Thrash (attack + block)
            MonsterMove {
                name: "Thrash",
                effects: &[
                    Effect::Damage(2),
                    Effect::MonsterBlock(2),
                ],
            },
            // Move 2: Bellow (block + Strength)
            MonsterMove {
                name: "Bellow",
                effects: &[
                    Effect::MonsterBlock(2),
                    Effect::ApplyPower { target: EffectTarget::_Self, power_id: "Strength", amount: 1 },
                ],
            },
        ],
        // Behavior "sda": rolls 1-2 → Bellow(2), 3-4 → Thrash(1), 5-6 → Chomp(0)
        pattern: MovePattern::DieRoll3([2, 1, 0]),
        starting_effects: &[],
    },
    // ── Phase 1: Die-controlled monsters ──
    MonsterInfo {
        id: "BGAcidSlime_M",
        moves: &[
            MonsterMove {
                name: "Corrosive Spit",
                effects: &[
                    Effect::Damage(2),
                    Effect::AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 1 },
                ],
            },
            MonsterMove {
                name: "Tackle",
                effects: &[Effect::Damage(2)],
            },
            MonsterMove {
                name: "Lick",
                effects: &[Effect::ApplyPower { target: EffectTarget::Player, power_id: "BGWeakened", amount: 1 }],
            },
        ],
        pattern: MovePattern::DieRoll3([0, 1, 2]),
        starting_effects: &[],
    },
    MonsterInfo {
        id: "BGSpikeSlime_M",
        moves: &[
            MonsterMove {
                name: "Flame Tackle",
                effects: &[
                    Effect::Damage(1),
                    Effect::ApplyPower { target: EffectTarget::Player, power_id: "BGVulnerable", amount: 1 },
                ],
            },
            MonsterMove {
                name: "Lick",
                effects: &[
                    Effect::Damage(1),
                    Effect::AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 1 },
                ],
            },
            MonsterMove {
                name: "Tackle",
                effects: &[Effect::Damage(2)],
            },
        ],
        pattern: MovePattern::DieRoll3([0, 1, 2]),
        starting_effects: &[],
    },
    MonsterInfo {
        id: "BGRedLouse",
        moves: &[
            MonsterMove {
                name: "Bite",
                effects: &[Effect::Damage(1)],
            },
            MonsterMove {
                name: "Strengthen",
                effects: &[Effect::ApplyPower { target: EffectTarget::_Self, power_id: "Strength", amount: 1 }],
            },
            MonsterMove {
                name: "Chomp",
                effects: &[Effect::Damage(2)],
            },
        ],
        pattern: MovePattern::DieRoll3([0, 1, 2]),
        starting_effects: &[Effect::ApplyPower { target: EffectTarget::_Self, power_id: "BGCurlUp", amount: 2 }],
    },
    MonsterInfo {
        id: "BGGreenLouse",
        moves: &[
            MonsterMove {
                name: "Bite",
                effects: &[Effect::Damage(1)],
            },
            MonsterMove {
                name: "Spit Web",
                effects: &[Effect::ApplyPower { target: EffectTarget::Player, power_id: "BGWeakened", amount: 1 }],
            },
            MonsterMove {
                name: "Chomp",
                effects: &[Effect::Damage(2)],
            },
        ],
        pattern: MovePattern::DieRoll3([0, 1, 2]),
        starting_effects: &[Effect::ApplyPower { target: EffectTarget::_Self, power_id: "BGCurlUp", amount: 2 }],
    },
    MonsterInfo {
        id: "BGFungiBeast",
        moves: &[
            MonsterMove {
                name: "Bite",
                effects: &[Effect::Damage(2)],
            },
            MonsterMove {
                name: "Grow",
                effects: &[
                    Effect::Damage(1),
                    Effect::ApplyPower { target: EffectTarget::_Self, power_id: "Strength", amount: 1 },
                ],
            },
            MonsterMove {
                name: "Strengthen",
                effects: &[Effect::ApplyPower { target: EffectTarget::_Self, power_id: "Strength", amount: 2 }],
            },
        ],
        pattern: MovePattern::DieRoll3([0, 1, 2]),
        starting_effects: &[Effect::ApplyPower { target: EffectTarget::_Self, power_id: "BGSporeCloud", amount: 1 }],
    },
    MonsterInfo {
        id: "BGBlueSlaver",
        moves: &[
            MonsterMove {
                name: "Rake",
                effects: &[
                    Effect::Damage(2),
                    Effect::ApplyPower { target: EffectTarget::Player, power_id: "BGWeakened", amount: 1 },
                ],
            },
            MonsterMove {
                name: "Stab",
                effects: &[
                    Effect::Damage(2),
                    Effect::AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 1 },
                ],
            },
            MonsterMove {
                name: "Stab",
                effects: &[Effect::Damage(3)],
            },
        ],
        pattern: MovePattern::DieRoll3([0, 1, 2]),
        starting_effects: &[],
    },
    MonsterInfo {
        id: "BGRedSlaver",
        moves: &[
            MonsterMove {
                name: "Stab",
                effects: &[
                    Effect::Damage(2),
                    Effect::AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 1 },
                ],
            },
            MonsterMove {
                name: "Scrape",
                effects: &[
                    Effect::Damage(2),
                    Effect::ApplyPower { target: EffectTarget::Player, power_id: "BGVulnerable", amount: 1 },
                ],
            },
            MonsterMove {
                name: "Stab",
                effects: &[Effect::Damage(3)],
            },
        ],
        pattern: MovePattern::DieRoll3([0, 1, 2]),
        starting_effects: &[],
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
