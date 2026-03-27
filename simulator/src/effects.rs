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
    /// If the target monster is dead, gain Strength (capped).
    StrengthIfTargetDead(i16),
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
    /// Double the player's current block.
    DoubleBlock,
    /// Gain temporary strength (capped at MAX_STRENGTH). Applies both Strength
    /// and LoseStrength for the clamped amount.
    GainTemporaryStrength(i16),
    /// Double the player's current strength.
    DoubleStrength,
    /// For each card in hand matching the filter, push per_card effects into the queue.
    /// If exhaust_matched is true, exhaust the matched cards.
    ForEachInHand {
        filter: HandFilter,
        per_card: &'static [Effect],
        exhaust_matched: bool,
    },
    /// Present the player with a choice between N named effect lists.
    /// Each entry is (label, effects). The player picks one and those effects are queued.
    ChooseOne(&'static [(&'static str, &'static [Effect])]),
    /// X-cost: present choices for spending 0..=current_energy. Per-energy effects are
    /// repeated (energy_spent + bonus) times. Energy is deducted.
    XCost {
        per_energy: &'static [Effect],
        bonus: i16,
    },
    /// If the current die roll is within [min, max] (inclusive), push effects to the queue.
    ConditionalOnDieRoll {
        min: u8,
        max: u8,
        effects: &'static [Effect],
    },
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

/// Which cards in hand to match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandFilter {
    /// All cards in hand.
    AllCards,
    /// Only attack cards.
    Attacks,
    /// Only non-attack cards.
    NonAttacks,
}

/// Who an effect targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectTarget {
    TargetEnemy,
    _Self,
    AllEnemies,
}
