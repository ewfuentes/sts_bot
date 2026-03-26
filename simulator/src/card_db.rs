use std::collections::HashMap;
use std::sync::LazyLock;

use crate::effects::{Effect, EffectTarget, Pile};

/// How a card targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardTarget {
    Enemy,     // single enemy, requires target selection
    AllEnemy,  // all enemies, no target selection
    _Self,     // player, no target selection
    None,      // no target (powers, some skills)
}

impl CardTarget {
    pub fn has_target(self) -> bool {
        matches!(self, CardTarget::Enemy)
    }
}

/// Card type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardType {
    Attack,
    Skill,
    Power,
    Status,
    Curse,
}

/// Static metadata for a card, looked up by ID.
#[derive(Debug, Clone)]
pub struct CardInfo {
    pub id: &'static str,
    pub cost: i8,
    pub card_type: CardType,
    pub target: CardTarget,
    pub effects: &'static [Effect],
    pub exhaust: bool,
    pub ethereal: bool,
    // Upgrade overrides (None = same as base)
    pub upgraded_cost: Option<i8>,
    pub upgraded_effects: Option<&'static [Effect]>,
    pub upgraded_exhaust: Option<bool>,
    pub upgraded_ethereal: Option<bool>,
}

impl CardInfo {
    /// Get the effective cost for a card (base or upgraded).
    pub fn effective_cost(&self, upgraded: bool) -> i8 {
        if upgraded {
            self.upgraded_cost.unwrap_or(self.cost)
        } else {
            self.cost
        }
    }

    /// Get the effective effects for a card (base or upgraded).
    pub fn effective_effects(&self, upgraded: bool) -> &[Effect] {
        if upgraded {
            self.upgraded_effects.unwrap_or(self.effects)
        } else {
            self.effects
        }
    }

    /// Is this card ethereal (base or upgraded)?
    pub fn is_ethereal(&self, upgraded: bool) -> bool {
        if upgraded {
            self.upgraded_ethereal.unwrap_or(self.ethereal)
        } else {
            self.ethereal
        }
    }

    /// Does this card exhaust (base or upgraded)?
    pub fn does_exhaust(&self, upgraded: bool) -> bool {
        if upgraded {
            self.upgraded_exhaust.unwrap_or(self.exhaust)
        } else {
            self.exhaust
        }
    }
}

/// Look up card info by ID. Returns None for unknown cards.
pub fn lookup(id: &str) -> Option<&'static CardInfo> {
    CARD_DB.get(id)
}

// ── Ironclad cards ──────────────────────────────────────────────────────

use Effect::*;
use EffectTarget::*;

static CARD_DB: LazyLock<HashMap<&'static str, CardInfo>> = LazyLock::new(|| {
    // Only cards whose effects are fully verified against the BG mod source.
    // Cards with missing side effects, play conditions, or custom actions are excluded
    // until those mechanics are implemented.
    let cards: Vec<CardInfo> = vec![
        // ── Starters ──
        CardInfo {
            id: "BGStrike_R", cost: 1, card_type: CardType::Attack, target: CardTarget::Enemy,
            effects: &[Damage(1)], exhaust: false, ethereal: false,
            upgraded_cost: None, upgraded_effects: Some(&[Damage(2)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGDefend_R", cost: 1, card_type: CardType::Skill, target: CardTarget::_Self,
            effects: &[Block(1)], exhaust: false, ethereal: false,
            upgraded_cost: None, upgraded_effects: Some(&[Block(2)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGBash", cost: 2, card_type: CardType::Attack, target: CardTarget::Enemy,
            effects: &[Damage(2), ApplyPower { target: TargetEnemy, power_id: "BGVulnerable", amount: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: None,
            upgraded_effects: Some(&[Damage(4), ApplyPower { target: TargetEnemy, power_id: "BGVulnerable", amount: 2 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        // ── Verified attacks ──
        CardInfo {
            id: "BGCleave", cost: 1, card_type: CardType::Attack, target: CardTarget::AllEnemy,
            effects: &[DamageAll(2)], exhaust: false, ethereal: false,
            upgraded_cost: None, upgraded_effects: Some(&[DamageAll(3)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGClothesline", cost: 2, card_type: CardType::Attack, target: CardTarget::Enemy,
            effects: &[Damage(3), ApplyPower { target: TargetEnemy, power_id: "BGWeakened", amount: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: None,
            upgraded_effects: Some(&[Damage(4), ApplyPower { target: TargetEnemy, power_id: "BGWeakened", amount: 2 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGTwin Strike", cost: 1, card_type: CardType::Attack, target: CardTarget::Enemy,
            effects: &[Damage(1), Damage(1)], exhaust: false, ethereal: false,
            upgraded_cost: None, upgraded_effects: Some(&[Damage(2), Damage(2)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGPommel Strike", cost: 1, card_type: CardType::Attack, target: CardTarget::Enemy,
            effects: &[Damage(2), Draw(1)], exhaust: false, ethereal: false,
            upgraded_cost: None, upgraded_effects: Some(&[Damage(3), Draw(2)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGBludgeon", cost: 3, card_type: CardType::Attack, target: CardTarget::Enemy,
            effects: &[Damage(7)], exhaust: false, ethereal: false,
            upgraded_cost: None, upgraded_effects: Some(&[Damage(10)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGUppercut", cost: 2, card_type: CardType::Attack, target: CardTarget::Enemy,
            effects: &[
                Damage(3),
                ApplyPower { target: TargetEnemy, power_id: "BGWeakened", amount: 1 },
                ApplyPower { target: TargetEnemy, power_id: "BGVulnerable", amount: 1 },
            ],
            exhaust: false, ethereal: false,
            upgraded_cost: None,
            upgraded_effects: Some(&[
                Damage(3),
                ApplyPower { target: TargetEnemy, power_id: "BGWeakened", amount: 2 },
                ApplyPower { target: TargetEnemy, power_id: "BGVulnerable", amount: 2 },
            ]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGCarnage", cost: 2, card_type: CardType::Attack, target: CardTarget::Enemy,
            effects: &[Damage(4)], exhaust: false, ethereal: true,
            upgraded_cost: None, upgraded_effects: Some(&[Damage(6)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGBlood for Blood", cost: 3, card_type: CardType::Attack, target: CardTarget::Enemy,
            effects: &[Damage(4)], exhaust: false, ethereal: false,
            upgraded_cost: None, upgraded_effects: Some(&[Damage(5)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGWild Strike", cost: 1, card_type: CardType::Attack, target: CardTarget::Enemy,
            effects: &[Damage(3), AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: None,
            upgraded_effects: Some(&[Damage(4), AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 1 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGImmolate", cost: 2, card_type: CardType::Attack, target: CardTarget::AllEnemy,
            effects: &[DamageAll(5), AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 2 }],
            exhaust: false, ethereal: false,
            upgraded_cost: None,
            upgraded_effects: Some(&[DamageAll(7), AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 2 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        // ── Verified skills ──
        CardInfo {
            id: "BGPower Through", cost: 1, card_type: CardType::Skill, target: CardTarget::_Self,
            effects: &[Block(3), AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: None,
            upgraded_effects: Some(&[Block(4), AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 1 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGShrug It Off", cost: 1, card_type: CardType::Skill, target: CardTarget::_Self,
            effects: &[Block(2), Draw(1)], exhaust: false, ethereal: false,
            upgraded_cost: None, upgraded_effects: Some(&[Block(3), Draw(1)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGTrue Grit", cost: 1, card_type: CardType::Skill, target: CardTarget::_Self,
            effects: &[Block(1), ExhaustFromHand(1)], exhaust: false, ethereal: false,
            upgraded_cost: None, upgraded_effects: Some(&[Block(2), ExhaustFromHand(1)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGBurning Pact", cost: 1, card_type: CardType::Skill, target: CardTarget::None,
            effects: &[ExhaustFromHand(1), Draw(2)], exhaust: false, ethereal: false,
            upgraded_cost: None, upgraded_effects: Some(&[ExhaustFromHand(1), Draw(3)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        // BGSentinel excluded: has triggerOnExhaust (gain energy) not yet modeled
        CardInfo {
            id: "BGGhostly Armor", cost: 1, card_type: CardType::Skill, target: CardTarget::_Self,
            effects: &[Block(2)], exhaust: false, ethereal: true,
            upgraded_cost: None, upgraded_effects: Some(&[Block(3)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGImpervious", cost: 2, card_type: CardType::Skill, target: CardTarget::_Self,
            effects: &[Block(6)], exhaust: true, ethereal: false,
            upgraded_cost: None, upgraded_effects: Some(&[Block(8)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGDisarm", cost: 1, card_type: CardType::Skill, target: CardTarget::Enemy,
            effects: &[ApplyPower { target: TargetEnemy, power_id: "BGWeakened", amount: 2 }],
            exhaust: true, ethereal: false,
            upgraded_cost: None,
            upgraded_effects: Some(&[ApplyPower { target: TargetEnemy, power_id: "BGWeakened", amount: 3 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGShockwave", cost: 2, card_type: CardType::Skill, target: CardTarget::AllEnemy,
            effects: &[
                ApplyPower { target: AllEnemies, power_id: "BGWeakened", amount: 1 },
                ApplyPower { target: AllEnemies, power_id: "BGVulnerable", amount: 1 },
            ],
            exhaust: true, ethereal: false,
            upgraded_cost: None,
            upgraded_effects: Some(&[
                ApplyPower { target: AllEnemies, power_id: "BGWeakened", amount: 2 },
                ApplyPower { target: AllEnemies, power_id: "BGVulnerable", amount: 2 },
            ]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGSeeing Red", cost: 1, card_type: CardType::Skill, target: CardTarget::None,
            effects: &[GainEnergy(2)], exhaust: true, ethereal: false,
            upgraded_cost: Some(0), upgraded_effects: None,
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGOffering", cost: 0, card_type: CardType::Skill, target: CardTarget::_Self,
            effects: &[LoseHP(1), GainEnergy(2), Draw(3)], exhaust: true, ethereal: false,
            upgraded_cost: None, upgraded_effects: Some(&[LoseHP(1), GainEnergy(2), Draw(5)]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGRupture", cost: 1, card_type: CardType::Skill, target: CardTarget::_Self,
            effects: &[LoseHP(1), ApplyPower { target: _Self, power_id: "Strength", amount: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: Some(0),
            upgraded_effects: Some(&[LoseHP(1), ApplyPower { target: _Self, power_id: "Strength", amount: 2 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        // ── Verified powers (just ApplyPower, no side effects) ──
        CardInfo {
            id: "BGInflame", cost: 2, card_type: CardType::Power, target: CardTarget::_Self,
            effects: &[ApplyPower { target: _Self, power_id: "Strength", amount: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: Some(1), upgraded_effects: None,
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGMetallicize", cost: 1, card_type: CardType::Power, target: CardTarget::_Self,
            effects: &[ApplyPower { target: _Self, power_id: "Metallicize", amount: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: Some(0),
            upgraded_effects: Some(&[ApplyPower { target: _Self, power_id: "Metallicize", amount: 2 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGDemon Form", cost: 3, card_type: CardType::Power, target: CardTarget::None,
            effects: &[ApplyPower { target: _Self, power_id: "DemonForm", amount: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: Some(2), upgraded_effects: None,
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGBarricade", cost: 2, card_type: CardType::Power, target: CardTarget::_Self,
            effects: &[ApplyPower { target: _Self, power_id: "Barricade", amount: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: Some(1), upgraded_effects: None,
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGBerserk", cost: 1, card_type: CardType::Power, target: CardTarget::_Self,
            effects: &[ApplyPower { target: _Self, power_id: "BGBerserk", amount: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: None,
            upgraded_effects: Some(&[ApplyPower { target: _Self, power_id: "BGBerserk", amount: 2 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGCombust", cost: 1, card_type: CardType::Power, target: CardTarget::_Self,
            effects: &[ApplyPower { target: _Self, power_id: "BGCombust", amount: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: None,
            upgraded_effects: Some(&[ApplyPower { target: _Self, power_id: "BGCombust", amount: 2 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGCorruption", cost: 3, card_type: CardType::Power, target: CardTarget::_Self,
            effects: &[ApplyPower { target: _Self, power_id: "BGCorruption", amount: 3 }],
            exhaust: false, ethereal: false,
            upgraded_cost: Some(2), upgraded_effects: None,
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGDark Embrace", cost: 2, card_type: CardType::Power, target: CardTarget::_Self,
            effects: &[ApplyPower { target: _Self, power_id: "BGDarkEmbrace", amount: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: Some(1), upgraded_effects: None,
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGEvolve", cost: 1, card_type: CardType::Power, target: CardTarget::_Self,
            effects: &[ApplyPower { target: _Self, power_id: "Evolve", amount: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: Some(0),
            upgraded_effects: Some(&[ApplyPower { target: _Self, power_id: "Evolve", amount: 2 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGFeel No Pain", cost: 1, card_type: CardType::Power, target: CardTarget::_Self,
            effects: &[ApplyPower { target: _Self, power_id: "FeelNoPain", amount: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: Some(0),
            upgraded_effects: Some(&[ApplyPower { target: _Self, power_id: "FeelNoPain", amount: 2 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGJuggernaut", cost: 2, card_type: CardType::Power, target: CardTarget::_Self,
            effects: &[ApplyPower { target: _Self, power_id: "BGJuggernaut", amount: 1 }],
            exhaust: false, ethereal: false,
            upgraded_cost: None,
            upgraded_effects: Some(&[ApplyPower { target: _Self, power_id: "BGJuggernaut", amount: 2 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "BGFire Breathing", cost: 1, card_type: CardType::Power, target: CardTarget::_Self,
            effects: &[ApplyPower { target: _Self, power_id: "FireBreathing", amount: 2 }],
            exhaust: false, ethereal: false,
            upgraded_cost: None,
            upgraded_effects: Some(&[ApplyPower { target: _Self, power_id: "FireBreathing", amount: 3 }]),
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        // ── Status / Curse ──
        CardInfo {
            id: "Dazed", cost: -2, card_type: CardType::Status, target: CardTarget::None,
            effects: &[], exhaust: false, ethereal: true,
            upgraded_cost: None, upgraded_effects: None,
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "Wound", cost: -2, card_type: CardType::Status, target: CardTarget::None,
            effects: &[], exhaust: false, ethereal: false,
            upgraded_cost: None, upgraded_effects: None,
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
        CardInfo {
            id: "AscendersBane", cost: -2, card_type: CardType::Curse, target: CardTarget::None,
            effects: &[], exhaust: false, ethereal: true,
            upgraded_cost: None, upgraded_effects: None,
            upgraded_exhaust: None, upgraded_ethereal: None,
        },
    ];

    cards.into_iter().map(|c| (c.id, c)).collect()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_strike() {
        let info = lookup("BGStrike_R").unwrap();
        assert_eq!(info.cost, 1);
        assert_eq!(info.card_type, CardType::Attack);
        assert_eq!(info.target, CardTarget::Enemy);
        assert!(info.target.has_target());
        assert_eq!(info.effects, &[Damage(1)]);
        assert_eq!(info.effective_cost(true), 1); // no upgrade cost change
        assert_eq!(info.effective_effects(true), &[Damage(2)]);
    }

    #[test]
    fn lookup_defend() {
        let info = lookup("BGDefend_R").unwrap();
        assert_eq!(info.cost, 1);
        assert_eq!(info.card_type, CardType::Skill);
        assert_eq!(info.target, CardTarget::_Self);
        assert!(!info.target.has_target());
        assert_eq!(info.effects, &[Block(1)]);
    }

    #[test]
    fn lookup_bash() {
        let info = lookup("BGBash").unwrap();
        assert_eq!(info.cost, 2);
        assert_eq!(info.target, CardTarget::Enemy);
        assert_eq!(info.effects.len(), 2);
        assert_eq!(info.effects[0], Damage(2));
    }

    #[test]
    fn lookup_offering() {
        let info = lookup("BGOffering").unwrap();
        assert_eq!(info.cost, 0);
        assert!(info.exhaust);
        assert_eq!(info.effects, &[LoseHP(1), GainEnergy(2), Draw(3)]);
        // Upgraded draws 5
        let up = info.effective_effects(true);
        assert_eq!(up, &[LoseHP(1), GainEnergy(2), Draw(5)]);
    }

    #[test]
    fn lookup_demon_form() {
        let info = lookup("BGDemon Form").unwrap();
        assert_eq!(info.cost, 3);
        assert_eq!(info.effective_cost(true), 2); // upgrade reduces cost
        assert_eq!(info.card_type, CardType::Power);
    }

    #[test]
    fn unknown_card_returns_none() {
        assert!(lookup("Nonexistent").is_none());
    }
}
