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
];

pub fn lookup(id: &str) -> Option<&'static PotionInfo> {
    POTIONS.iter().find(|p| p.id == id)
}
