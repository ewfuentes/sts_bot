/// What happens when a card effect resolves.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Damage(i16),
    DamageAll(i16),
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
    /// Player chooses card(s) from hand to exhaust. Pushes a sub-decision screen.
    ExhaustFromHand(u8),
    Custom(&'static str),
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
