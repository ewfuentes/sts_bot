use crate::effects::{Effect, EffectTarget, HandSelectAction, Pile};

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
        id: "BoardGame:BGSkillPotion",
        effects: &[Effect::ApplyPower { target: EffectTarget::_Self, power_id: "BGBurst", amount: 1 }],
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
    PotionInfo {
        id: "BoardGame:BGBloodPotion",
        effects: &[Effect::Heal(2)],
    },
    PotionInfo {
        id: "BoardGame:BGElixirPotion",
        effects: &[Effect::SelectFromHand { min: 3, max: 3, action: HandSelectAction::Exhaust }],
    },
    PotionInfo {
        id: "BoardGame:BGAncientPotion",
        effects: &[
            Effect::ApplyPower { target: EffectTarget::_Self, power_id: "BGWeakened", amount: i16::MIN },
            Effect::ApplyPower { target: EffectTarget::_Self, power_id: "BGVulnerable", amount: i16::MIN },
        ],
    },
    PotionInfo {
        id: "BoardGame:BGSneckoOil",
        effects: &[
            Effect::Draw(5),
            Effect::AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 2 },
        ],
    },
    PotionInfo {
        id: "BoardGame:BGLiquidMemories",
        effects: &[Effect::SelectFromDiscardToHand { cost_override: Some(0) }],
    },
];

pub fn lookup(id: &str) -> Option<&'static PotionInfo> {
    POTIONS.iter().find(|p| p.id == id)
}
