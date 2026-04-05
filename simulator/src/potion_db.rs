use crate::effects::{Effect, EffectTarget};

pub struct PotionInfo {
    pub id: &'static str,
    pub effects: &'static [Effect],
}

static POTIONS: &[PotionInfo] = &[
    PotionInfo {
        id: "BoardGame:BGBlock Potion",
        effects: &[Effect::Block(2)],
    },
    PotionInfo {
        id: "BoardGame:BGEnergy Potion",
        effects: &[Effect::GainEnergy(2)],
    },
    PotionInfo {
        id: "BoardGame:BGExplosive Potion",
        effects: &[Effect::DamageFixedAll(2)],
    },
    PotionInfo {
        id: "BoardGame:BGSwift Potion",
        effects: &[Effect::Draw(3)],
    },
    PotionInfo {
        id: "BoardGame:BGSteroidPotion",
        effects: &[Effect::GainTemporaryStrength(1)],
    },
    PotionInfo {
        id: "BoardGame:BGAttackPotion",
        effects: &[Effect::ApplyPower { target: EffectTarget::_Self, power_id: "BGDoubleAttack", amount: 1 }],
    },
    PotionInfo {
        id: "BoardGame:BGFire Potion",
        effects: &[Effect::DamageFixedTargetSelect { amount: 4, reason: crate::screen::TargetReason::Pending }],
    },
    PotionInfo {
        id: "BoardGame:BGWeak Potion",
        effects: &[Effect::ApplyPowerTargetSelect { power_id: "BGWeakened", amount: 2 }],
    },
    PotionInfo {
        id: "BoardGame:BGFearPotion",
        effects: &[Effect::ApplyPowerTargetSelect { power_id: "BGVulnerable", amount: 1 }],
    },
    PotionInfo {
        id: "BoardGame:BGCunningPotion",
        effects: &[Effect::ShivTargetSelect, Effect::ShivTargetSelect, Effect::ShivTargetSelect],
    },
];

pub fn lookup(id: &str) -> Option<&'static PotionInfo> {
    POTIONS.iter().find(|p| p.id == id)
}
