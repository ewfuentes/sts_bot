/// An encounter definition: what monsters appear and their configuration.
#[derive(Debug, Clone)]
pub struct EncounterMonster {
    pub id: &'static str,
    pub hp: u16,
    pub move_index: u8,
}

#[derive(Debug, Clone)]
pub struct EncounterInfo {
    pub id: &'static str,
    pub monsters: &'static [EncounterMonster],
}

/// Which pool an encounter belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterPool {
    Weak,
    Strong,
    Elite,
    Boss,
}

// ── Act 1 (Exordium) encounters ──

static ACT1_WEAK: &[EncounterInfo] = &[
    EncounterInfo {
        id: "BoardGame:Easy Small Slimes",
        monsters: &[
            EncounterMonster { id: "BGSpikeSlime_S", hp: 3, move_index: 0 },
            EncounterMonster { id: "BGAcidSlime_M", hp: 5, move_index: 0 },
        ],
    },
    EncounterInfo {
        id: "BoardGame:Jaw Worm (Easy)",
        monsters: &[EncounterMonster { id: "BGJawWorm", hp: 8, move_index: 0 }],
    },
    EncounterInfo {
        id: "BoardGame:Cultist",
        monsters: &[EncounterMonster { id: "BGCultist", hp: 9, move_index: 0 }],
    },
    EncounterInfo {
        id: "BoardGame:2 Louse",
        monsters: &[
            EncounterMonster { id: "BGRedLouse", hp: 3, move_index: 0 },
            EncounterMonster { id: "BGGreenLouse", hp: 3, move_index: 0 },
        ],
    },
];

static ACT1_STRONG: &[EncounterInfo] = &[
    EncounterInfo {
        id: "BoardGame:Cultist and SpikeSlime",
        monsters: &[
            EncounterMonster { id: "BGCultist", hp: 9, move_index: 0 },
            EncounterMonster { id: "BGSpikeSlime_M", hp: 5, move_index: 0 },
        ],
    },
    EncounterInfo {
        id: "BoardGame:Cultist and Louse",
        monsters: &[
            EncounterMonster { id: "BGCultist", hp: 9, move_index: 0 },
            EncounterMonster { id: "BGGreenLouse", hp: 3, move_index: 0 },
        ],
    },
    EncounterInfo {
        id: "BoardGame:Fungi Beasts",
        monsters: &[
            EncounterMonster { id: "BGFungiBeast", hp: 5, move_index: 0 },
            EncounterMonster { id: "BGFungiBeast", hp: 5, move_index: 0 },
        ],
    },
    EncounterInfo {
        id: "BoardGame:Slime Trio",
        monsters: &[
            EncounterMonster { id: "BGSpikeSlime_S", hp: 3, move_index: 0 },
            EncounterMonster { id: "BGAcidSlime_M", hp: 5, move_index: 0 },
            EncounterMonster { id: "BGSpikeSlime_M", hp: 5, move_index: 0 },
        ],
    },
    EncounterInfo {
        id: "BoardGame:3 Louse (Hard)",
        monsters: &[
            EncounterMonster { id: "BGRedLouse", hp: 4, move_index: 0 },
            EncounterMonster { id: "BGGreenLouse", hp: 3, move_index: 0 },
            EncounterMonster { id: "BGRedLouse", hp: 3, move_index: 0 },
        ],
    },
    EncounterInfo {
        id: "BoardGame:Large Slime",
        monsters: &[EncounterMonster { id: "BGAcidSlime_L", hp: 8, move_index: 0 }],
    },
    EncounterInfo {
        id: "BoardGame:Sneaky Gremlin Team",
        monsters: &[EncounterMonster { id: "BGGremlinSneaky", hp: 2, move_index: 0 }],
    },
    EncounterInfo {
        id: "BoardGame:Angry Gremlin Team",
        monsters: &[EncounterMonster { id: "BGGremlinAngry", hp: 4, move_index: 0 }],
    },
    EncounterInfo {
        id: "BoardGame:Blue Slaver",
        monsters: &[EncounterMonster { id: "BGBlueSlaver", hp: 10, move_index: 0 }],
    },
    EncounterInfo {
        id: "BoardGame:Red Slaver",
        monsters: &[EncounterMonster { id: "BGRedSlaver", hp: 10, move_index: 0 }],
    },
    EncounterInfo {
        id: "BoardGame:Looter",
        monsters: &[EncounterMonster { id: "BGLooter", hp: 7, move_index: 0 }],
    },
    EncounterInfo {
        id: "BoardGame:Jaw Worm (Medium)",
        monsters: &[EncounterMonster { id: "BGJawWorm", hp: 8, move_index: 0 }],
    },
];

static ACT1_ELITE: &[EncounterInfo] = &[
    EncounterInfo {
        id: "BoardGame:Gremlin Nob",
        monsters: &[EncounterMonster { id: "BGGremlinNob", hp: 14, move_index: 0 }],
    },
    EncounterInfo {
        id: "BoardGame:Lagavulin",
        monsters: &[EncounterMonster { id: "BGLagavulin", hp: 22, move_index: 0 }],
    },
    EncounterInfo {
        id: "BoardGame:3 Sentries",
        monsters: &[
            EncounterMonster { id: "BGSentry", hp: 8, move_index: 0 },
            EncounterMonster { id: "BGSentry", hp: 8, move_index: 0 },
            EncounterMonster { id: "BGSentry", hp: 8, move_index: 0 },
        ],
    },
];

static ACT1_BOSS: &[EncounterInfo] = &[
    EncounterInfo {
        id: "BoardGame:TheGuardian",
        monsters: &[EncounterMonster { id: "BGTheGuardian", hp: 40, move_index: 0 }],
    },
    EncounterInfo {
        id: "BoardGame:Hexaghost",
        monsters: &[EncounterMonster { id: "BGHexaghost", hp: 36, move_index: 0 }],
    },
    EncounterInfo {
        id: "BoardGame:SlimeBoss",
        monsters: &[EncounterMonster { id: "BGSlimeBoss", hp: 22, move_index: 0 }],
    },
];

pub fn lookup(id: &str) -> Option<&'static EncounterInfo> {
    ACT1_WEAK.iter()
        .chain(ACT1_STRONG.iter())
        .chain(ACT1_ELITE.iter())
        .chain(ACT1_BOSS.iter())
        .find(|e| e.id == id)
}

/// Get the list of encounter IDs for a given act and pool.
pub fn encounter_pool(act: u8, pool: EncounterPool) -> Vec<&'static str> {
    let encounters = match (act, pool) {
        (1, EncounterPool::Weak) => ACT1_WEAK,
        (1, EncounterPool::Strong) => ACT1_STRONG,
        (1, EncounterPool::Elite) => ACT1_ELITE,
        (1, EncounterPool::Boss) => ACT1_BOSS,
        _ => return vec![],
    };
    encounters.iter().map(|e| e.id).collect()
}
