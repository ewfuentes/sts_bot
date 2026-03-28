use std::collections::HashMap;
use std::sync::LazyLock;

use crate::effects::{DamageSource, Effect, EffectTarget, HandFilter, HandSelectAction, Pile};

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

/// Condition that must be met for a card to be playable (beyond energy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayCondition {
    /// All cards in hand must be attacks (Clash).
    HandAllAttacks,
    /// No other attack cards in hand (Signature Move).
    #[allow(dead_code)]
    HandNoOtherAttacks,
    /// Draw pile must be empty (Grand Finale).
    #[allow(dead_code)]
    DrawPileEmpty,
    /// Card can never be played; only triggers on discard (Tactician, Reflex).
    #[allow(dead_code)]
    Never,
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
    pub rebound: bool,
    pub play_condition: Option<PlayCondition>,
    /// Effects triggered when this card is exhausted (by any means).
    pub on_exhaust: Option<&'static [Effect]>,
    // Upgrade overrides (None = same as base)
    pub upgraded_cost: Option<i8>,
    pub upgraded_effects: Option<&'static [Effect]>,
    pub upgraded_exhaust: Option<bool>,
    pub upgraded_ethereal: Option<bool>,
    pub upgraded_on_exhaust: Option<&'static [Effect]>,
}

impl CardInfo {
    /// Create a new card with required fields; optional fields default to None/false.
    const fn new(
        id: &'static str,
        cost: i8,
        card_type: CardType,
        target: CardTarget,
        effects: &'static [Effect],
    ) -> Self {
        Self {
            id,
            cost,
            card_type,
            target,
            effects,
            exhaust: false,
            ethereal: false,
            rebound: false,
            play_condition: None,
            on_exhaust: None,
            upgraded_cost: None,
            upgraded_effects: None,
            upgraded_exhaust: None,
            upgraded_ethereal: None,
            upgraded_on_exhaust: None,
        }
    }

    const fn exhaust(mut self) -> Self {
        self.exhaust = true;
        self
    }

    const fn ethereal(mut self) -> Self {
        self.ethereal = true;
        self
    }

    const fn rebound(mut self) -> Self {
        self.rebound = true;
        self
    }

    const fn on_exhaust(mut self, effects: &'static [Effect]) -> Self {
        self.on_exhaust = Some(effects);
        self
    }

    const fn upgraded_on_exhaust(mut self, effects: &'static [Effect]) -> Self {
        self.upgraded_on_exhaust = Some(effects);
        self
    }

    const fn play_condition(mut self, cond: PlayCondition) -> Self {
        self.play_condition = Some(cond);
        self
    }

    const fn upgraded_cost(mut self, cost: i8) -> Self {
        self.upgraded_cost = Some(cost);
        self
    }

    const fn upgraded_effects(mut self, effects: &'static [Effect]) -> Self {
        self.upgraded_effects = Some(effects);
        self
    }

    #[allow(dead_code)]
    const fn upgraded_exhaust(mut self, val: bool) -> Self {
        self.upgraded_exhaust = Some(val);
        self
    }

    #[allow(dead_code)]
    const fn upgraded_ethereal(mut self, val: bool) -> Self {
        self.upgraded_ethereal = Some(val);
        self
    }

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

    /// Get the on-exhaust effects for a card (base or upgraded).
    pub fn effective_on_exhaust(&self, upgraded: bool) -> Option<&[Effect]> {
        if upgraded {
            self.upgraded_on_exhaust.or(self.on_exhaust)
        } else {
            self.on_exhaust
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
        CardInfo::new("BGStrike_R", 1, CardType::Attack, CardTarget::Enemy, &[Damage(1)])
            .upgraded_effects(&[Damage(2)]),
        CardInfo::new("BGDefend_R", 1, CardType::Skill, CardTarget::_Self, &[Block(1)])
            .upgraded_effects(&[Block(2)]),
        CardInfo::new("BGBash", 2, CardType::Attack, CardTarget::Enemy,
            &[Damage(2), ApplyPower { target: TargetEnemy, power_id: "BGVulnerable", amount: 1 }])
            .upgraded_effects(&[Damage(4), ApplyPower { target: TargetEnemy, power_id: "BGVulnerable", amount: 2 }]),
        // ── Verified attacks ──
        CardInfo::new("BGCleave", 1, CardType::Attack, CardTarget::AllEnemy, &[DamageAll(2)])
            .upgraded_effects(&[DamageAll(3)]),
        CardInfo::new("BGClothesline", 2, CardType::Attack, CardTarget::Enemy,
            &[Damage(3), ApplyPower { target: TargetEnemy, power_id: "BGWeakened", amount: 1 }])
            .upgraded_effects(&[Damage(4), ApplyPower { target: TargetEnemy, power_id: "BGWeakened", amount: 2 }]),
        CardInfo::new("BGTwin Strike", 1, CardType::Attack, CardTarget::Enemy, &[Damage(1), Damage(1)])
            .upgraded_effects(&[Damage(2), Damage(2)]),
        CardInfo::new("BGPommel Strike", 1, CardType::Attack, CardTarget::Enemy, &[Damage(2), Draw(1)])
            .upgraded_effects(&[Damage(3), Draw(2)]),
        CardInfo::new("BGBludgeon", 3, CardType::Attack, CardTarget::Enemy, &[Damage(7)])
            .upgraded_effects(&[Damage(10)]),
        CardInfo::new("BGUppercut", 2, CardType::Attack, CardTarget::Enemy, &[
                Damage(3),
                ApplyPower { target: TargetEnemy, power_id: "BGWeakened", amount: 1 },
                ApplyPower { target: TargetEnemy, power_id: "BGVulnerable", amount: 1 },
            ])
            .upgraded_effects(&[
                Damage(3),
                ApplyPower { target: TargetEnemy, power_id: "BGWeakened", amount: 2 },
                ApplyPower { target: TargetEnemy, power_id: "BGVulnerable", amount: 2 },
            ]),
        CardInfo::new("BGCarnage", 2, CardType::Attack, CardTarget::Enemy, &[Damage(4)])
            .ethereal()
            .upgraded_effects(&[Damage(6)]),
        CardInfo::new("BGBlood for Blood", 3, CardType::Attack, CardTarget::Enemy, &[Damage(4)])
            .upgraded_effects(&[Damage(5)]),
        CardInfo::new("BGWild Strike", 1, CardType::Attack, CardTarget::Enemy,
            &[Damage(3), AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 1 }])
            .upgraded_effects(&[Damage(4), AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 1 }]),
        CardInfo::new("BGImmolate", 2, CardType::Attack, CardTarget::AllEnemy,
            &[DamageAll(5), AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 2 }])
            .upgraded_effects(&[DamageAll(7), AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 2 }]),
        CardInfo::new("BGBody Slam", 1, CardType::Attack, CardTarget::Enemy,
            &[DamageBasedOn(DamageSource::CurrentBlock)])
            .upgraded_cost(0),
        CardInfo::new("BGRampage", 1, CardType::Attack, CardTarget::Enemy,
            &[DamageBasedOn(DamageSource::ExhaustPileSize)])
            .upgraded_effects(&[
                SelectFromHand { min: 1, max: 1, action: HandSelectAction::Exhaust },
                DamageBasedOn(DamageSource::ExhaustPileSize),
            ]),
        CardInfo::new("BGAnger", 0, CardType::Attack, CardTarget::Enemy, &[Damage(1)])
            .rebound()
            .upgraded_effects(&[Damage(2)]),
        CardInfo::new("BGClash", 0, CardType::Attack, CardTarget::Enemy, &[Damage(3)])
            .play_condition(PlayCondition::HandAllAttacks)
            .upgraded_effects(&[Damage(4)]),
        CardInfo::new("BGIron Wave", 1, CardType::Attack, CardTarget::Enemy, &[Damage(1), Block(1)])
            .upgraded_effects(&[ChooseOne(&[
                ("Spear", &[Damage(2), Block(1)]),
                ("Shield", &[Damage(1), Block(2)]),
            ])]),
        CardInfo::new("BGSever Soul", 2, CardType::Attack, CardTarget::Enemy,
            &[Damage(3), SelectFromHand { min: 1, max: 1, action: HandSelectAction::Exhaust }])
            .upgraded_effects(&[Damage(4), SelectFromHand { min: 1, max: 2, action: HandSelectAction::Exhaust }]),
        // ── Verified skills ──
        CardInfo::new("BGWarcry", 0, CardType::Skill, CardTarget::None,
            &[Draw(2), SelectFromHand { min: 1, max: 1, action: HandSelectAction::PutOnTopOfDraw }])
            .exhaust()
            .upgraded_effects(&[Draw(3), SelectFromHand { min: 1, max: 1, action: HandSelectAction::PutOnTopOfDraw }]),
        CardInfo::new("BGEntrench", 1, CardType::Skill, CardTarget::_Self, &[DoubleBlock])
            .exhaust()
            .upgraded_exhaust(false),
        CardInfo::new("BGLimit Break", 1, CardType::Skill, CardTarget::_Self, &[DoubleStrength])
            .exhaust()
            .upgraded_exhaust(false),
        CardInfo::new("BGRage", 1, CardType::Skill, CardTarget::_Self,
            &[ForEachInHand { filter: HandFilter::Attacks, per_card: &[Block(1)], exhaust_matched: false }])
            .upgraded_cost(0),
        CardInfo::new("BGSecond Wind", 1, CardType::Skill, CardTarget::_Self,
            &[ForEachInHand { filter: HandFilter::NonAttacks, per_card: &[Block(1)], exhaust_matched: true }])
            .upgraded_effects(&[ForEachInHand { filter: HandFilter::NonAttacks, per_card: &[Block(2)], exhaust_matched: true }]),
        CardInfo::new("BGFiend Fire", 2, CardType::Attack, CardTarget::Enemy,
            &[ForEachInHand { filter: HandFilter::AllCards, per_card: &[Damage(1)], exhaust_matched: true }])
            .exhaust()
            .upgraded_effects(&[ForEachInHand { filter: HandFilter::AllCards, per_card: &[Damage(2)], exhaust_matched: true }]),
        CardInfo::new("BGHeadbutt", 1, CardType::Attack, CardTarget::Enemy,
            &[Damage(2), SelectFromDiscardToDrawTop])
            .upgraded_effects(&[Damage(3), SelectFromDiscardToDrawTop]),
        CardInfo::new("BGPerfected Strike", 2, CardType::Attack, CardTarget::Enemy,
            &[DamageBasedOn(DamageSource::StrikesInHand { base: 3, per_strike: 1 })])
            .upgraded_effects(&[DamageBasedOn(DamageSource::StrikesInHand { base: 3, per_strike: 2 })]),
        CardInfo::new("BGHeavy Blade", 2, CardType::Attack, CardTarget::Enemy,
            &[DamageBasedOn(DamageSource::StrengthMultiplier { base: 3, multiplier: 2 })])
            .upgraded_effects(&[DamageBasedOn(DamageSource::StrengthMultiplier { base: 3, multiplier: 4 })]),
        CardInfo::new("BGFlame Barrier", 2, CardType::Skill, CardTarget::_Self,
            &[Block(3), FlameBarrier(1)])
            .upgraded_effects(&[Block(4), FlameBarrier(1)]),
        CardInfo::new("BGHavoc", 1, CardType::Skill, CardTarget::None, &[PlayTopOfDraw])
            .upgraded_cost(0),
        CardInfo::new("BGExhume", 1, CardType::Skill, CardTarget::None, &[SelectFromExhaustToHand])
            .exhaust()
            .upgraded_cost(0),
        CardInfo::new("BGFeed", 1, CardType::Attack, CardTarget::Enemy,
            &[Damage(3), StrengthIfTargetDead(1)])
            .exhaust()
            .upgraded_effects(&[Damage(3), StrengthIfTargetDead(2)]),
        CardInfo::new("BGWhirlwind", -1, CardType::Attack, CardTarget::AllEnemy,
            &[XCost { per_energy: &[DamageAll(1)], bonus: 0 }])
            .upgraded_effects(&[XCost { per_energy: &[DamageAll(1)], bonus: 1 }]),
        CardInfo::new("BGPower Through", 1, CardType::Skill, CardTarget::_Self,
            &[Block(3), AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 1 }])
            .upgraded_effects(&[Block(4), AddCardToPile { card_id: "Dazed", pile: Pile::Draw, count: 1 }]),
        CardInfo::new("BGShrug It Off", 1, CardType::Skill, CardTarget::_Self, &[Block(2), Draw(1)])
            .upgraded_effects(&[Block(3), Draw(1)]),
        CardInfo::new("BGTrue Grit", 1, CardType::Skill, CardTarget::_Self,
            &[Block(1), SelectFromHand { min: 1, max: 1, action: HandSelectAction::Exhaust }])
            .upgraded_effects(&[Block(2), SelectFromHand { min: 1, max: 1, action: HandSelectAction::Exhaust }]),
        CardInfo::new("BGBurning Pact", 1, CardType::Skill, CardTarget::None,
            &[SelectFromHand { min: 1, max: 1, action: HandSelectAction::Exhaust }, Draw(2)])
            .upgraded_effects(&[SelectFromHand { min: 1, max: 1, action: HandSelectAction::Exhaust }, Draw(3)]),
        CardInfo::new("BGBattle Trance", 0, CardType::Skill, CardTarget::_Self,
            &[Draw(3), ApplyPower { target: _Self, power_id: "NoDrawPower", amount: 1 }])
            .upgraded_effects(&[Draw(4), ApplyPower { target: _Self, power_id: "NoDrawPower", amount: 1 }]),
        CardInfo::new("BGFlex", 0, CardType::Skill, CardTarget::_Self,
            &[GainTemporaryStrength(1)])
            .exhaust()
            .upgraded_exhaust(false),
        CardInfo::new("BGSentinel", 1, CardType::Skill, CardTarget::_Self, &[Block(2)])
            .on_exhaust(&[GainEnergy(2)])
            .upgraded_effects(&[Block(3)])
            .upgraded_on_exhaust(&[GainEnergy(3)]),
        CardInfo::new("BGGhostly Armor", 1, CardType::Skill, CardTarget::_Self, &[Block(2)])
            .ethereal()
            .upgraded_effects(&[Block(3)]),
        CardInfo::new("BGImpervious", 2, CardType::Skill, CardTarget::_Self, &[Block(6)])
            .exhaust()
            .upgraded_effects(&[Block(8)]),
        CardInfo::new("BGDisarm", 1, CardType::Skill, CardTarget::Enemy,
            &[ApplyPower { target: TargetEnemy, power_id: "BGWeakened", amount: 2 }])
            .exhaust()
            .upgraded_effects(&[ApplyPower { target: TargetEnemy, power_id: "BGWeakened", amount: 3 }]),
        CardInfo::new("BGShockwave", 2, CardType::Skill, CardTarget::AllEnemy, &[
                ApplyPower { target: AllEnemies, power_id: "BGWeakened", amount: 1 },
                ApplyPower { target: AllEnemies, power_id: "BGVulnerable", amount: 1 },
            ])
            .exhaust()
            .upgraded_effects(&[
                ApplyPower { target: AllEnemies, power_id: "BGWeakened", amount: 2 },
                ApplyPower { target: AllEnemies, power_id: "BGVulnerable", amount: 2 },
            ]),
        CardInfo::new("BGSeeing Red", 1, CardType::Skill, CardTarget::None, &[GainEnergy(2)])
            .exhaust()
            .upgraded_cost(0),
        CardInfo::new("BGOffering", 0, CardType::Skill, CardTarget::_Self, &[LoseHP(1), GainEnergy(2), Draw(3)])
            .exhaust()
            .upgraded_effects(&[LoseHP(1), GainEnergy(2), Draw(5)]),
        CardInfo::new("BGRupture", 1, CardType::Skill, CardTarget::_Self,
            &[LoseHP(1), ApplyPower { target: _Self, power_id: "Strength", amount: 1 }])
            .upgraded_cost(0)
            .upgraded_effects(&[LoseHP(1), ApplyPower { target: _Self, power_id: "Strength", amount: 2 }]),
        CardInfo::new("BGSpot Weakness", 1, CardType::Skill, CardTarget::_Self,
            &[ConditionalOnDieRoll { min: 1, max: 3, effects: &[
                ApplyPower { target: _Self, power_id: "Strength", amount: 1 },
            ]}])
            .upgraded_effects(&[ConditionalOnDieRoll { min: 1, max: 4, effects: &[
                ApplyPower { target: _Self, power_id: "Strength", amount: 1 },
            ]}]),
        CardInfo::new("BGDouble Tap", 1, CardType::Skill, CardTarget::_Self,
            &[ApplyPower { target: _Self, power_id: "BGDoubleAttack", amount: 1 }])
            .upgraded_cost(0),
        // ── Verified powers (just ApplyPower, no side effects) ──
        CardInfo::new("BGInflame", 2, CardType::Power, CardTarget::_Self,
            &[ApplyPower { target: _Self, power_id: "Strength", amount: 1 }])
            .upgraded_cost(1),
        CardInfo::new("BGMetallicize", 1, CardType::Power, CardTarget::_Self,
            &[ApplyPower { target: _Self, power_id: "Metallicize", amount: 1 }])
            .upgraded_cost(0)
            .upgraded_effects(&[ApplyPower { target: _Self, power_id: "Metallicize", amount: 2 }]),
        CardInfo::new("BGDemon Form", 3, CardType::Power, CardTarget::None,
            &[ApplyPower { target: _Self, power_id: "DemonForm", amount: 1 }])
            .upgraded_cost(2),
        CardInfo::new("BGBarricade", 2, CardType::Power, CardTarget::_Self,
            &[ApplyPower { target: _Self, power_id: "Barricade", amount: 1 }])
            .upgraded_cost(1),
        CardInfo::new("BGBerserk", 1, CardType::Power, CardTarget::_Self,
            &[ApplyPower { target: _Self, power_id: "BGBerserk", amount: 1 }])
            .upgraded_effects(&[ApplyPower { target: _Self, power_id: "BGBerserk", amount: 2 }]),
        CardInfo::new("BGCombust", 1, CardType::Power, CardTarget::_Self,
            &[ApplyPower { target: _Self, power_id: "BGCombust", amount: 1 }])
            .upgraded_effects(&[ApplyPower { target: _Self, power_id: "BGCombust", amount: 2 }]),
        CardInfo::new("BGCorruption", 3, CardType::Power, CardTarget::_Self,
            &[ApplyPower { target: _Self, power_id: "BGCorruption", amount: 3 }])
            .upgraded_cost(2),
        CardInfo::new("BGDark Embrace", 2, CardType::Power, CardTarget::_Self,
            &[ApplyPower { target: _Self, power_id: "BGDarkEmbrace", amount: 1 }])
            .upgraded_cost(1),
        CardInfo::new("BGEvolve", 1, CardType::Power, CardTarget::_Self,
            &[ApplyPower { target: _Self, power_id: "Evolve", amount: 1 }])
            .upgraded_cost(0)
            .upgraded_effects(&[ApplyPower { target: _Self, power_id: "Evolve", amount: 2 }]),
        CardInfo::new("BGFeel No Pain", 1, CardType::Power, CardTarget::_Self,
            &[ApplyPower { target: _Self, power_id: "FeelNoPain", amount: 1 }])
            .upgraded_cost(0)
            .upgraded_effects(&[ApplyPower { target: _Self, power_id: "FeelNoPain", amount: 2 }]),
        CardInfo::new("BGJuggernaut", 2, CardType::Power, CardTarget::_Self,
            &[ApplyPower { target: _Self, power_id: "BGJuggernaut", amount: 1 }])
            .upgraded_effects(&[ApplyPower { target: _Self, power_id: "BGJuggernaut", amount: 2 }]),
        CardInfo::new("BGFire Breathing", 1, CardType::Power, CardTarget::_Self,
            &[ApplyPower { target: _Self, power_id: "FireBreathing", amount: 2 }])
            .upgraded_effects(&[ApplyPower { target: _Self, power_id: "FireBreathing", amount: 3 }]),
        // ── Status / Curse ──
        CardInfo::new("Dazed", -2, CardType::Status, CardTarget::None, &[])
            .ethereal(),
        CardInfo::new("Wound", -2, CardType::Status, CardTarget::None, &[]),
        CardInfo::new("AscendersBane", -2, CardType::Curse, CardTarget::None, &[])
            .ethereal(),
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
