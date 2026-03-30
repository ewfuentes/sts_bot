use crate::effects::Effect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerTrigger {
    OnExhaust,
    OnDraw { card_type: crate::card_db::CardType },
    OnShuffle,
    EndOfTurn,
    StartOfTurn,
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
                        results.push(substitute_amount(effect, power.amount));
                    }
                }
            }
        }
    }
    results
}

fn substitute_amount(effect: &Effect, amt: i32) -> Effect {
    match effect {
        Effect::Block(0) => Effect::Block(amt as i16),
        Effect::Draw(0) => Effect::Draw(amt as u8),
        Effect::DamageFixedAll(0) => Effect::DamageFixedAll(amt as i16),
        Effect::ApplyPower { target, power_id, amount: 0 } => Effect::ApplyPower {
            target: *target, power_id, amount: amt as i16,
        },
        other => other.clone(),
    }
}
