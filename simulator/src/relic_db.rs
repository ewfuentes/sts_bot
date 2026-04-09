use crate::effects::Effect;
use crate::power_db::{Trigger, TriggeredEffect};

pub struct RelicInfo {
    pub id: &'static str,
    pub triggers: &'static [TriggeredEffect],
}

static RELICS: &[RelicInfo] = &[
    // ── Start-of-combat relics ──
    RelicInfo {
        id: "BGAnchor",
        triggers: &[TriggeredEffect {
            trigger: Trigger::StartOfCombat,
            effects: &[Effect::Block(2)],
            front_effects: &[],
        }],
    },
    RelicInfo {
        id: "BGBag of Preparation",
        triggers: &[TriggeredEffect {
            trigger: Trigger::StartOfCombat,
            effects: &[Effect::Draw(2)],
            front_effects: &[],
        }],
    },
    RelicInfo {
        id: "BGBlood Vial",
        triggers: &[TriggeredEffect {
            trigger: Trigger::StartOfCombat,
            effects: &[Effect::Heal(1)],
            front_effects: &[],
        }],
    },
    RelicInfo {
        id: "BGLantern",
        triggers: &[TriggeredEffect {
            trigger: Trigger::StartOfCombat,
            effects: &[Effect::GainEnergy(1)],
            front_effects: &[],
        }],
    },
    RelicInfo {
        id: "BGMutagenic Strength",
        triggers: &[TriggeredEffect {
            trigger: Trigger::StartOfCombat,
            effects: &[Effect::GainTemporaryStrength(1)],
            front_effects: &[],
        }],
    },
    // ── End-of-combat relics ──
    RelicInfo {
        id: "BoardGame:BurningBlood",
        triggers: &[TriggeredEffect {
            trigger: Trigger::EndOfCombat,
            effects: &[Effect::Heal(1)],
            front_effects: &[],
        }],
    },
    RelicInfo {
        id: "BGBlack Blood",
        triggers: &[TriggeredEffect {
            trigger: Trigger::EndOfCombat,
            effects: &[Effect::Heal(2)],
            front_effects: &[],
        }],
    },
    RelicInfo {
        id: "BGMeat on the Bone",
        triggers: &[TriggeredEffect {
            trigger: Trigger::EndOfCombat,
            effects: &[Effect::HealToMinHP(4)],
            front_effects: &[],
        }],
    },
    RelicInfo {
        id: "BGGolden Idol",
        triggers: &[TriggeredEffect {
            trigger: Trigger::EndOfCombat,
            effects: &[Effect::GainGold(1)],
            front_effects: &[],
        }],
    },
    // ── End-of-turn relics ──
    RelicInfo {
        id: "BGOrichalcum",
        triggers: &[TriggeredEffect {
            trigger: Trigger::PlayerEndOfTurn,
            effects: &[Effect::OrichalcumBlock],
            front_effects: &[],
        }],
    },
];

/// Collect triggered effects from all relics the player owns.
pub fn collect_relic_triggered_effects(
    trigger: Trigger,
    relics: &[crate::types::Relic],
) -> Vec<(Effect, crate::effects::ResolvedTarget)> {
    let mut result = Vec::new();
    for relic in relics {
        if let Some(info) = lookup(&relic.id) {
            for te in info.triggers {
                if te.trigger == trigger {
                    for effect in te.effects {
                        result.push((effect.clone(), crate::effects::ResolvedTarget::NoTarget));
                    }
                }
            }
        }
    }
    result
}

pub fn lookup(id: &str) -> Option<&'static RelicInfo> {
    RELICS.iter().find(|r| r.id == id)
}
