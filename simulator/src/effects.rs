/// What the damage amount is derived from at resolution time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageSource {
    /// Number of cards in the exhaust pile.
    ExhaustPileSize,
    /// Player's current block.
    CurrentBlock,
}

/// What happens when a card effect resolves.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Damage(i16),
    DamageAll(i16),
    /// Deal damage to a single target equal to a value derived from game state.
    DamageBasedOn(DamageSource),
    Block(i16),
    ApplyPower {
        target: EffectTarget,
        power_id: &'static str,
        amount: i16,
    },
    Draw(u8),
    GainEnergy(u8),
    LoseHP(u16),
    AddCardToPile {
        card_id: &'static str,
        pile: Pile,
        count: u8,
    },
    /// Player chooses card(s) from hand and applies an action to each.
    SelectFromHand { min: u8, max: u8, action: HandSelectAction },
    Custom(&'static str),
}

/// What to do with cards selected from hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HandSelectAction {
    #[default]
    Exhaust,
    Discard,
    Upgrade,
    PutOnTopOfDraw,
}

/// Which pile to add a card to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pile {
    Draw,
    Discard,
    Exhaust,
}

/// Who an effect targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectTarget {
    TargetEnemy,
    _Self,
    AllEnemies,
}
