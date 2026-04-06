use crate::effects::Effect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerTrigger {
    OnExhaust,
    OnDraw { card_type: crate::card_db::CardType },
    OnShuffle,
    PlayerEndOfTurn,
    PlayerStartOfTurn,
    MonsterEndOfTurn,
    OnGainBlock,
    /// Monster took HP damage from any source (CurlUp).
    MonsterOnDamaged,
    /// Monster was hit by a player Attack card (Angry).
    MonsterOnAttacked,
    MonsterOnDeath,
    /// Player played a card of the given type (e.g. Skill triggers BGAnger, Attack triggers SharpHide).
    PlayerOnPlay { card_type: crate::card_db::CardType },
}

#[derive(Debug, Clone)]
pub struct TriggeredEffect {
    pub trigger: PowerTrigger,
    /// Effects queued to the back. The power's `amount` is substituted for any
    /// effect that uses it (e.g. Block(0) becomes Block(amount)).
    pub effects: &'static [Effect],
    /// Effects pushed to the front of the queue (execute before next trigger collection).
    /// Used for self-removal so multi-hit attacks don't double-trigger.
    pub front_effects: &'static [Effect],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerModifier {
    PreventBlockDecay,
    PreventDraw,
    RepeatAttack,
    RepeatSkill,
    SkillsCostZero,
    SkillsExhaust,
}

pub struct PowerInfo {
    pub id: &'static str,
    pub triggers: &'static [TriggeredEffect],
    pub modifiers: &'static [PowerModifier],
}

static POWERS: &[PowerInfo] = &[
    PowerInfo {
        id: "FeelNoPain",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnExhaust,
            effects: &[Effect::Block(0)],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGDarkEmbrace",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnExhaust,
            effects: &[Effect::Draw(0)],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "Evolve",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnDraw { card_type: crate::card_db::CardType::Status },
            effects: &[Effect::Draw(0)],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "FireBreathing",
        triggers: &[
            TriggeredEffect {
                trigger: PowerTrigger::OnDraw { card_type: crate::card_db::CardType::Status },
                effects: &[Effect::DamageFixedAll(0)],
                front_effects: &[],
            },
            TriggeredEffect {
                trigger: PowerTrigger::OnDraw { card_type: crate::card_db::CardType::Curse },
                effects: &[Effect::DamageFixedAll(0)],
                front_effects: &[],
            },
        ],
        modifiers: &[],
    },
    PowerInfo {
        id: "Metallicize",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerEndOfTurn,
            effects: &[Effect::Block(0)],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGCombust",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerEndOfTurn,
            effects: &[Effect::DamageFixedAll(0)],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGBerserk",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnExhaust,
            effects: &[Effect::DamageFixedAll(0)],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "DemonForm",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerStartOfTurn,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "Strength", amount: 0 }],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "Barricade",
        triggers: &[],
        modifiers: &[PowerModifier::PreventBlockDecay],
    },
    PowerInfo {
        id: "BGJuggernaut",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnGainBlock,
            effects: &[Effect::DamageFixedTargetSelect { amount: 0, reason: crate::screen::TargetReason::Pending }],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGDoubleAttack",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerEndOfTurn,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "BGDoubleAttack", amount: i16::MIN }],
            front_effects: &[],
        }],
        modifiers: &[PowerModifier::RepeatAttack],
    },
    PowerInfo {
        id: "BGBurst",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerEndOfTurn,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "BGBurst", amount: i16::MIN }],
            front_effects: &[],
        }],
        modifiers: &[PowerModifier::RepeatSkill],
    },
    PowerInfo {
        id: "BGIntangible",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerStartOfTurn,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "BGIntangible", amount: i16::MIN }],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGCorruption",
        triggers: &[],
        modifiers: &[PowerModifier::SkillsCostZero, PowerModifier::SkillsExhaust],
    },
    PowerInfo {
        id: "Ritual",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::MonsterEndOfTurn,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "Strength", amount: 0 }],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "NoDrawPower",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerEndOfTurn,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "NoDrawPower", amount: -1 }],
            front_effects: &[],
        }],
        modifiers: &[PowerModifier::PreventDraw],
    },
    // Monster reactive powers
    PowerInfo {
        id: "BGCurlUp",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::MonsterOnDamaged,
            effects: &[Effect::MonsterBlock(0)],
            front_effects: &[
                Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "BGCurlUp", amount: i16::MIN },
            ],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGSporeCloud",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::MonsterOnDeath,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::Player, power_id: "BGVulnerable", amount: 0 }],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "Angry",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::MonsterOnAttacked,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "Strength", amount: 0 }],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGSharpHide",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerOnPlay { card_type: crate::card_db::CardType::Attack },
            effects: &[Effect::DamageFixed(0)],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGAnger",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerOnPlay { card_type: crate::card_db::CardType::Skill },
            effects: &[Effect::DamageFixed(0)],
            front_effects: &[],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGSplit",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::MonsterOnDeath,
            effects: &[
                Effect::SpawnMonster { id: "BGAcidSlime_L", hp: 12 },
                Effect::SpawnMonster { id: "BGAcidSlime_M", hp: 5 },
                Effect::SpawnMonster { id: "BGSpikeSlime_M", hp: 5 },
                Effect::MonsterEscape,
            ],
            front_effects: &[],
        }],
        modifiers: &[],
    },
];

pub fn lookup(id: &str) -> Option<&'static PowerInfo> {
    POWERS.iter().find(|p| p.id == id)
}

/// Check whether any active power has the given modifier.
pub fn has_modifier(modifier: PowerModifier, powers: &[crate::types::Power]) -> bool {
    powers.iter().any(|power| {
        lookup(&power.id)
            .map(|info| info.modifiers.contains(&modifier))
            .unwrap_or(false)
    })
}

/// Find the first active power (amount > 0) that has the given modifier,
/// returning its id.
pub fn find_active_modifier(modifier: PowerModifier, powers: &[crate::types::Power]) -> Option<String> {
    powers.iter().find(|power| {
        power.amount > 0
            && lookup(&power.id)
                .map(|info| info.modifiers.contains(&modifier))
                .unwrap_or(false)
    }).map(|p| p.id.clone())
}

/// Compute the effective cost of a card after applying cost-modifying powers.
/// Pure query — does not consume any powers.
pub fn get_modified_cost(
    base_cost: i8,
    card_type: crate::card_db::CardType,
    powers: &[crate::types::Power],
) -> i8 {
    if base_cost < 0 {
        return base_cost;
    }

    if card_type == crate::card_db::CardType::Skill
        && has_modifier(PowerModifier::SkillsCostZero, powers)
    {
        return 0;
    }

    base_cost
}

/// Compute the effective cost and consume any one-shot cost-modifying powers.
/// Call this at play time (not for playability checks).
pub fn apply_cost_modification(
    base_cost: i8,
    card_type: crate::card_db::CardType,
    powers: &mut Vec<crate::types::Power>,
) -> i8 {
    let cost = get_modified_cost(base_cost, card_type, powers);

    // Future: consume one-shot powers here (FreeAttack, Confusion)

    cost
}

pub struct TriggeredEffects {
    /// Effects to push to the back of the queue (normal ordering).
    pub back: Vec<(Effect, crate::effects::ResolvedTarget)>,
    /// Effects to push to the front of the queue (execute before next trigger collection).
    pub front: Vec<(Effect, crate::effects::ResolvedTarget)>,
}

/// Collect all effects that should fire for the given trigger,
/// scanning both player powers and all living monsters' powers.
pub fn collect_all_triggered_effects(
    trigger: PowerTrigger,
    player_powers: &[crate::types::Power],
    monsters: &[crate::types::Monster],
) -> TriggeredEffects {
    use crate::effects::ResolvedTarget;
    let mut result = TriggeredEffects { back: Vec::new(), front: Vec::new() };

    // Player powers
    collect_from_powers(trigger, player_powers, ResolvedTarget::Player, &mut result);

    // Monster powers
    for (i, monster) in monsters.iter().enumerate() {
        if monster.state == crate::types::MonsterState::Alive {
            collect_from_powers(trigger, &monster.powers, ResolvedTarget::Monster(i as u8), &mut result);
        }
    }

    result
}

/// Collect triggered effects from a single power source.
/// `owner` identifies who holds these powers — used to resolve `_Self` effects
/// (e.g. `ResolvedTarget::Monster(idx)` for a monster's Ritual power,
/// `ResolvedTarget::Player` for a player's DemonForm power).
pub fn collect_triggered_effects(
    trigger: PowerTrigger,
    powers: &[crate::types::Power],
    owner: crate::effects::ResolvedTarget,
) -> TriggeredEffects {
    let mut result = TriggeredEffects { back: Vec::new(), front: Vec::new() };
    collect_from_powers(trigger, powers, owner, &mut result);
    result
}

fn collect_from_powers(
    trigger: PowerTrigger,
    powers: &[crate::types::Power],
    owner: crate::effects::ResolvedTarget,
    result: &mut TriggeredEffects,
) {
    for power in powers {
        if let Some(info) = lookup(&power.id) {
            for te in info.triggers {
                if te.trigger == trigger {
                    for effect in te.effects {
                        result.back.push((substitute_amount(effect, power), owner));
                    }
                    for effect in te.front_effects {
                        result.front.push((substitute_amount(effect, power), owner));
                    }
                }
            }
        }
    }
}


fn substitute_amount(effect: &Effect, power: &crate::types::Power) -> Effect {
    let amt = power.amount;
    match effect {
        Effect::Block(0) => Effect::Block(amt as i16),
        Effect::MonsterBlock(0) => Effect::MonsterBlock(amt as u16),
        Effect::Draw(0) => Effect::Draw(amt as u8),
        Effect::DamageFixedAll(0) => Effect::DamageFixedAll(amt as i16),
        Effect::DamageFixedTargetSelect { amount: 0, reason: crate::screen::TargetReason::Pending } => {
            Effect::DamageFixedTargetSelect {
                amount: amt as i16,
                reason: crate::screen::TargetReason::Power(power.clone()),
            }
        }
        Effect::ApplyPower { target, power_id, amount: 0 } => Effect::ApplyPower {
            target: *target, power_id, amount: amt as i16,
        },
        other => other.clone(),
    }
}
