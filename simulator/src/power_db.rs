use crate::effects::Effect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerTrigger {
    OnExhaust,
    OnDraw { card_type: crate::card_db::CardType },
    OnShuffle,
}

#[derive(Debug, Clone)]
pub struct TriggeredEffect {
    pub trigger: PowerTrigger,
    /// Effects to queue. The power's `amount` is substituted for any
    /// effect that uses it (e.g. Block(0) becomes Block(amount)).
    pub effects: &'static [Effect],
}

pub struct PowerInfo {
    pub id: &'static str,
    pub triggers: &'static [TriggeredEffect],
}

static POWERS: &[PowerInfo] = &[
    PowerInfo {
        id: "FeelNoPain",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnExhaust,
            effects: &[Effect::Block(0)], // amount substituted at runtime
        }],
    },
    PowerInfo {
        id: "BGDarkEmbrace",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnExhaust,
            effects: &[Effect::Draw(0)], // amount substituted at runtime
        }],
    },
    PowerInfo {
        id: "Evolve",
        triggers: &[TriggeredEffect {
            trigger: PowerTrigger::OnDraw { card_type: crate::card_db::CardType::Status },
            effects: &[Effect::Draw(0)], // amount substituted at runtime
        }],
    },
    PowerInfo {
        id: "FireBreathing",
        triggers: &[
            TriggeredEffect {
                trigger: PowerTrigger::OnDraw { card_type: crate::card_db::CardType::Status },
                effects: &[Effect::DamageFixedAll(0)], // amount substituted at runtime
            },
            TriggeredEffect {
                trigger: PowerTrigger::OnDraw { card_type: crate::card_db::CardType::Curse },
                effects: &[Effect::DamageFixedAll(0)], // amount substituted at runtime
            },
        ],
    },
];

pub fn lookup(id: &str) -> Option<&'static PowerInfo> {
    POWERS.iter().find(|p| p.id == id)
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

fn substitute_amount(effect: &Effect, amount: i32) -> Effect {
    match effect {
        Effect::Block(0) => Effect::Block(amount as i16),
        Effect::Draw(0) => Effect::Draw(amount as u8),
        Effect::DamageFixedAll(0) => Effect::DamageFixedAll(amount as i16),
        other => other.clone(),
    }
}
