use crate::effects::Effect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerTrigger {
    OnExhaust,
    OnDraw { card_type: crate::card_db::CardType },
    OnShuffle,
    EndOfTurn,
    StartOfTurn,
    OnGainBlock,
}

#[derive(Debug, Clone)]
pub struct TriggeredEffect {
    pub trigger: PowerTrigger,
    /// Effects to queue. The power's `amount` is substituted for any
    /// effect that uses it (e.g. Block(0) becomes Block(amount)).
    pub effects: &'static [Effect],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerModifier {
    PreventBlockDecay,
    PreventDraw,
    RepeatAttack,
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
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGDarkEmbrace",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnExhaust,
            effects: &[Effect::Draw(0)],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "Evolve",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnDraw { card_type: crate::card_db::CardType::Status },
            effects: &[Effect::Draw(0)],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "FireBreathing",
        triggers: &[
            TriggeredEffect {
                trigger: PowerTrigger::OnDraw { card_type: crate::card_db::CardType::Status },
                effects: &[Effect::DamageFixedAll(0)],
            },
            TriggeredEffect {
                trigger: PowerTrigger::OnDraw { card_type: crate::card_db::CardType::Curse },
                effects: &[Effect::DamageFixedAll(0)],
            },
        ],
        modifiers: &[],
    },
    PowerInfo {
        id: "Metallicize",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::EndOfTurn,
            effects: &[Effect::Block(0)],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGCombust",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::EndOfTurn,
            effects: &[Effect::DamageFixedAll(0)],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGBerserk",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnExhaust,
            effects: &[Effect::DamageFixedAll(0)],
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "DemonForm",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::StartOfTurn,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "Strength", amount: 0 }],
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
        }],
        modifiers: &[],
    },
    PowerInfo {
        id: "BGDoubleAttack",
        triggers: &[],
        modifiers: &[PowerModifier::RepeatAttack],
    },
    PowerInfo {
        id: "NoDrawPower",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::EndOfTurn,
            effects: &[Effect::ApplyPower { target: crate::effects::EffectTarget::_Self, power_id: "NoDrawPower", amount: -1 }],
        }],
        modifiers: &[PowerModifier::PreventDraw],
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Power;

    #[test]
    fn lookup_double_attack() {
        let info = lookup("BGDoubleAttack").unwrap();
        assert!(info.modifiers.contains(&PowerModifier::RepeatAttack));
        assert!(info.triggers.is_empty());
    }

    #[test]
    fn find_active_modifier_returns_power_id() {
        let powers = vec![Power { id: "BGDoubleAttack".into(), amount: 1 }];
        let result = find_active_modifier(PowerModifier::RepeatAttack, &powers);
        assert_eq!(result.as_deref(), Some("BGDoubleAttack"));
    }

    #[test]
    fn find_active_modifier_skips_zero_amount() {
        let powers = vec![Power { id: "BGDoubleAttack".into(), amount: 0 }];
        let result = find_active_modifier(PowerModifier::RepeatAttack, &powers);
        assert_eq!(result, None);
    }

    #[test]
    fn find_active_modifier_returns_none_when_absent() {
        let powers = vec![Power { id: "Barricade".into(), amount: 1 }];
        let result = find_active_modifier(PowerModifier::RepeatAttack, &powers);
        assert_eq!(result, None);
    }
}

fn substitute_amount(effect: &Effect, power: &crate::types::Power) -> Effect {
    let amt = power.amount;
    match effect {
        Effect::Block(0) => Effect::Block(amt as i16),
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
