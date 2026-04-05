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

/// Triggers that can override a state machine transition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SmTrigger {
    /// When this monster's block reaches 0 from damage, transition to next_state.
    OnBlockBreak { next_state: u8 },
}

/// A single state in a monster state machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmState {
    pub move_index: u8,
    pub next_state: u8,
    /// Triggers that can override next_state while in this state.
    pub triggers: &'static [SmTrigger],
}

/// How a monster selects its next move.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MovePattern {
    /// Die roll mapping: 1-3 → indices[0], 4-6 → indices[1]
    DieRoll2([u8; 2]),
    /// Die roll mapping: 1-2 → indices[0], 3-4 → indices[1], 5-6 → indices[2]
    DieRoll3([u8; 3]),
    /// First turn uses `first`, all subsequent turns use `repeat`.
    FirstThenRepeat { first: u8, repeat: u8 },
    /// Always the same move.
    Fixed(u8),
    /// Cycle through a fixed sequence of move indices, repeating from the start.
    Sequence(&'static [u8]),
    /// State machine: each state maps to a move index and a default next state.
    /// The monster's `move_index` field tracks the current state.
    StateMachine {
        states: &'static [SmState],
    },
}

impl MovePattern {
    /// Notify the state machine that block was broken. Returns the new state
    /// if a trigger fired, or None if no transition occurred.
    pub fn on_block_broken(&self, current_state: u8) -> Option<u8> {
        if let MovePattern::StateMachine { states, .. } = self {
            if let Some(state) = states.get(current_state as usize) {
                for trigger in state.triggers {
                    match trigger {
                        SmTrigger::OnBlockBreak { next_state } => return Some(*next_state),
                    }
                }
            }
        }
        None
    }
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
    // ── Phase 2: Sequence/StateMachine/Escape monsters ──
    MonsterInfo {
        id: "BGGremlinNob",
        moves: &[
            MonsterMove {
                name: "Bellow",
                effects: &[Effect::ApplyPower { target: EffectTarget::_Self, power_id: "BGAnger", amount: 1 }],
            },
            MonsterMove {
                name: "Skull Bash",
                effects: &[Effect::Damage(3)],
            },
        ],
        pattern: MovePattern::FirstThenRepeat { first: 0, repeat: 1 },
        starting_effects: &[],
    },
    MonsterInfo {
        id: "BGLooter",
        moves: &[
            MonsterMove {
                name: "Mug",
                effects: &[Effect::Damage(2)],
            },
            MonsterMove {
                name: "Smoke Bomb",
                effects: &[
                    Effect::Damage(3),
                    Effect::MonsterBlock(1),
                ],
            },
            MonsterMove {
                name: "Escape",
                effects: &[Effect::StealGold(2), Effect::MonsterEscape],
            },
        ],
        pattern: MovePattern::Sequence(&[0, 1, 2]),
        starting_effects: &[],
    },
    MonsterInfo {
        id: "BGGremlinSneaky",
        moves: &[
            MonsterMove {
                name: "Puncture",
                effects: &[Effect::Damage(2)],
            },
        ],
        pattern: MovePattern::Fixed(0),
        starting_effects: &[],
    },
    MonsterInfo {
        id: "BGGremlinAngry",
        moves: &[
            MonsterMove {
                name: "Scratch",
                effects: &[Effect::Damage(1)],
            },
        ],
        pattern: MovePattern::Fixed(0),
        starting_effects: &[Effect::ApplyPower { target: EffectTarget::_Self, power_id: "Angry", amount: 1 }],
    },
    MonsterInfo {
        id: "BGSentry",
        moves: &[
            MonsterMove {
                name: "Daze",
                effects: &[Effect::AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 1 }],
            },
            MonsterMove {
                name: "Beam",
                effects: &[Effect::Damage(2)],
            },
        ],
        pattern: MovePattern::DieRoll2([0, 1]),
        starting_effects: &[],
    },
    MonsterInfo {
        id: "BGLagavulin",
        moves: &[
            // Move 0: Sleep (no effects — waking up)
            MonsterMove {
                name: "Sleep",
                effects: &[],
            },
            // Move 1: Strong Attack
            MonsterMove {
                name: "Strong Attack",
                effects: &[Effect::Damage(4)],
            },
            // Move 2: Siphon Soul — Weak(2) to player + Strength(1) to self
            MonsterMove {
                name: "Siphon Soul",
                effects: &[
                    Effect::ApplyPower { target: EffectTarget::TargetEnemy, power_id: "BGWeakened", amount: 2 },
                    Effect::ApplyPower { target: EffectTarget::_Self, power_id: "Strength", amount: 1 },
                ],
            },
        ],
        // State machine: Sleep(0) → Attack(1) → Attack(2) → Debuff(3) → Attack(1) ...
        pattern: MovePattern::StateMachine {
            states: &[
                SmState { move_index: 0, next_state: 1, triggers: &[] }, // State 0: Sleep → state 1
                SmState { move_index: 1, next_state: 2, triggers: &[] }, // State 1: Attack → state 2
                SmState { move_index: 1, next_state: 3, triggers: &[] }, // State 2: Attack → state 3
                SmState { move_index: 2, next_state: 1, triggers: &[] }, // State 3: Debuff → state 1
            ],
        },
        starting_effects: &[],
    },
    // ── Phase 3: Bosses ──
    MonsterInfo {
        id: "BGHexaghost",
        moves: &[
            // Phase 1: Sear — 1 dmg + 1 Burn
            MonsterMove {
                name: "Sear",
                effects: &[
                    Effect::Damage(1),
                    Effect::AddCardToPile { card_id: "BGBurn", pile: Pile::Discard, count: 1 },
                ],
            },
            // Phase 2: Tackle — 2 dmg x2 + 1 Burn
            MonsterMove {
                name: "Tackle",
                effects: &[
                    Effect::Damage(2),
                    Effect::Damage(2),
                    Effect::AddCardToPile { card_id: "BGBurn", pile: Pile::Discard, count: 1 },
                ],
            },
            // Phase 3: Inflame — 2 Burns (no damage)
            MonsterMove {
                name: "Inflame",
                effects: &[
                    Effect::AddCardToPile { card_id: "BGBurn", pile: Pile::Discard, count: 2 },
                ],
            },
            // Phase 4: Strengthen — 3 dmg + 5 block
            MonsterMove {
                name: "Strengthen",
                effects: &[
                    Effect::Damage(3),
                    Effect::MonsterBlock(5),
                ],
            },
            // Phase 5: Sear — 2 dmg + 1 Burn
            MonsterMove {
                name: "Sear",
                effects: &[
                    Effect::Damage(2),
                    Effect::AddCardToPile { card_id: "BGBurn", pile: Pile::Discard, count: 1 },
                ],
            },
            // Phase 6: Inferno — 3 dmg x2 + 2 Burns + Strength
            MonsterMove {
                name: "Inferno",
                effects: &[
                    Effect::Damage(3),
                    Effect::Damage(3),
                    Effect::AddCardToPile { card_id: "BGBurn", pile: Pile::Discard, count: 2 },
                    Effect::ApplyPower { target: EffectTarget::_Self, power_id: "Strength", amount: 1 },
                ],
            },
        ],
        pattern: MovePattern::Sequence(&[0, 1, 2, 3, 4, 5]),
        starting_effects: &[],
    },
    MonsterInfo {
        id: "BGTheGuardian",
        moves: &[
            // Move 0: Whirlwind + Charge Up — 2 dmg + 5 block
            MonsterMove {
                name: "Whirlwind",
                effects: &[
                    Effect::Damage(2),
                    Effect::MonsterBlock(5),
                ],
            },
            // Move 1: Fierce Bash — 6 dmg
            MonsterMove {
                name: "Fierce Bash",
                effects: &[Effect::Damage(6)],
            },
            // Move 2: Close Up — apply Sharp Hide
            MonsterMove {
                name: "Close Up",
                effects: &[Effect::ApplyPower { target: EffectTarget::_Self, power_id: "BGSharpHide", amount: 1 }],
            },
            // Move 3: Roll Attack — 2 dmg
            MonsterMove {
                name: "Roll Attack",
                effects: &[Effect::Damage(2)],
            },
            // Move 4: Twin Slam — 4 dmg + Strength, remove Sharp Hide
            MonsterMove {
                name: "Twin Slam",
                effects: &[
                    Effect::Damage(4),
                    Effect::ApplyPower { target: EffectTarget::_Self, power_id: "Strength", amount: 1 },
                    Effect::ApplyPower { target: EffectTarget::_Self, power_id: "BGSharpHide", amount: i16::MIN },
                ],
            },
        ],
        pattern: MovePattern::StateMachine {
            states: &[
                // Attack Mode
                SmState { move_index: 0, next_state: 1, triggers: &[] },                                          // 0: Whirlwind → Fierce Bash
                SmState { move_index: 1, next_state: 0, triggers: &[SmTrigger::OnBlockBreak { next_state: 2 }] },  // 1: Fierce Bash → Whirlwind (or Close Up on block break)
                // Defensive Mode
                SmState { move_index: 2, next_state: 3, triggers: &[] },                                           // 2: Close Up → Roll Attack
                SmState { move_index: 3, next_state: 4, triggers: &[] },                                           // 3: Roll Attack → Twin Slam
                SmState { move_index: 4, next_state: 0, triggers: &[] },                                           // 4: Twin Slam → Whirlwind (back to Attack Mode)
            ],
        },
        starting_effects: &[],
    },
];

pub fn lookup(id: &str) -> Option<&'static MonsterInfo> {
    MONSTERS.iter().find(|m| m.id == id)
}

/// Determine the next move index based on the monster's pattern and die roll.
/// For StateMachine, this returns the *next state* (not the move table index).
/// Use `resolve_move_index` to get the actual move table index from a state.
pub fn next_move(pattern: MovePattern, die_roll: u8, turn: u16, current_state: u8) -> u8 {
    match pattern {
        MovePattern::Fixed(idx) => idx,
        MovePattern::FirstThenRepeat { first, repeat } => {
            if turn == 1 { first } else { repeat }
        }
        MovePattern::DieRoll2(indices) => {
            let bucket = if die_roll <= 3 { 0 } else { 1 };
            indices[bucket]
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
        MovePattern::Sequence(indices) => {
            indices[((turn - 1) as usize) % indices.len()]
        }
        MovePattern::StateMachine { states, .. } => {
            // Return the next state after the current one
            if let Some(state) = states.get(current_state as usize) {
                state.next_state
            } else {
                0
            }
        }
    }
}

/// For StateMachine patterns, resolve the actual move table index from a state.
/// For all other patterns, the move_index IS the move table index.
pub fn resolve_move_index(pattern: MovePattern, move_index: u8) -> u8 {
    match pattern {
        MovePattern::StateMachine { states, .. } => {
            states.get(move_index as usize)
                .map(|s| s.move_index)
                .unwrap_or(0)
        }
        _ => move_index,
    }
}
