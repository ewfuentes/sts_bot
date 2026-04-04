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
    MonsterOnAttacked,
    MonsterOnDeath,
}

#[derive(Debug, Clone)]
pub struct TriggeredEffect {
    pub trigger: PowerTrigger,
    /// Effects to queue. The power's `amount` is substituted for any
    /// effect that uses it (e.g. Block(0) becomes Block(amount)).
    pub effects: &'static [Effect],
    /// If true, remove this power from the monster after triggering.
    pub remove_after_trigger: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerModifier {
    PreventBlockDecay,
    PreventDraw,
    RepeatAttack,
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
            remove_after_trigger: false,
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGDarkEmbrace",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnExhaust,
            effects: &[Effect::Draw(0)],
            remove_after_trigger: false,
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "Evolve",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnDraw { card_type: crate::card_db::CardType::Status },
            effects: &[Effect::Draw(0)],
            remove_after_trigger: false,
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "FireBreathing",
        triggers: &[
            TriggeredEffect {
                trigger: PowerTrigger::OnDraw { card_type: crate::card_db::CardType::Status },
                effects: &[Effect::DamageFixedAll(0)],
                remove_after_trigger: false,
            },
            TriggeredEffect {
                trigger: PowerTrigger::OnDraw { card_type: crate::card_db::CardType::Curse },
                effects: &[Effect::DamageFixedAll(0)],
                remove_after_trigger: false,
            },
        ],
        modifiers: &[],
    },
    PowerInfo {
        id: "Metallicize",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerEndOfTurn,
            effects: &[Effect::Block(0)],
            remove_after_trigger: false,
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGCombust",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerEndOfTurn,
            effects: &[Effect::DamageFixedAll(0)],
            remove_after_trigger: false,
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGBerserk",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnExhaust,
            effects: &[Effect::DamageFixedAll(0)],
            remove_after_trigger: false,
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "DemonForm",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerStartOfTurn,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "Strength", amount: 0 }],
            remove_after_trigger: false,
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
            remove_after_trigger: false,
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGDoubleAttack",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerEndOfTurn,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "BGDoubleAttack", amount: i16::MIN }],
            remove_after_trigger: false,
        }],
        modifiers: &[PowerModifier::RepeatAttack],
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
            remove_after_trigger: false,
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "NoDrawPower",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::PlayerEndOfTurn,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "NoDrawPower", amount: -1 }],
            remove_after_trigger: false,
        }],
        modifiers: &[PowerModifier::PreventDraw],
    },
    // Monster reactive powers
    PowerInfo {
        id: "BGCurlUp",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::MonsterOnAttacked,
            effects: &[Effect::MonsterBlock(0)],
            remove_after_trigger: true,
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGSporeCloud",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::MonsterOnDeath,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::TargetEnemy, power_id: "BGVulnerable", amount: 0 }],
            remove_after_trigger: false,
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "Angry",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::MonsterOnAttacked,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "Strength", amount: 0 }],
            remove_after_trigger: false,
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

/// Collect all effects that should fire for the given trigger,
/// substituting the power's amount into the effect templates.
pub fn collect_triggered_effects(
    trigger: PowerTrigger,
    powers: &[crate::types::Power],
) -> Vec<Effect> {
    let mut results = Vec::new();
    for power in powers {
        if let Some(info) = lookup(&power.id) {
            for te in info.triggers {
                if te.trigger == trigger {
                    for effect in te.effects {
                        results.push(substitute_amount(effect, power));
                    }
                }
            }
        }
    }
    results
}

/// Collect triggered effects and also return power IDs that should be removed after triggering.
pub fn collect_triggered_effects_with_removal(
    trigger: PowerTrigger,
    powers: &[crate::types::Power],
) -> (Vec<Effect>, Vec<String>) {
    let mut effects = Vec::new();
    let mut to_remove = Vec::new();
    for power in powers {
        if let Some(info) = lookup(&power.id) {
            for te in info.triggers {
                if te.trigger == trigger {
                    for effect in te.effects {
                        effects.push(substitute_amount(effect, power));
                    }
                    if te.remove_after_trigger {
                        to_remove.push(power.id.clone());
                    }
                }
            }
        }
    }
    (effects, to_remove)
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
