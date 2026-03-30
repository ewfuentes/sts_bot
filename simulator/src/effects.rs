/// What the damage amount is derived from at resolution time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageSource {
    /// Number of cards in the exhaust pile.
    ExhaustPileSize,
    /// Player's current block.
    CurrentBlock,
    /// Base damage + per_strike bonus for each other "Strike" card in hand.
    StrikesInHand { base: i16, per_strike: i16 },
    /// Base damage + multiplier * current Strength. The result is then passed
    /// through calculate_damage which adds 1x Strength, so multiplier should be
    /// the desired total minus 1 (e.g. 2 for Heavy Blade's 3x scaling).
    StrengthMultiplier { base: i16, multiplier: i16 },
}

/// What happens when a card effect resolves.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Damage(i16),
    /// Damage not affected by strength (thorns, Combust, orbs, etc.)
    DamageFixed(i16),
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
    /// Player chooses a card from the discard pile to put on top of draw pile.
    SelectFromDiscardToDrawTop,
    /// Player chooses a card from the exhaust pile to put in hand.
    SelectFromExhaustToHand,
    /// Draw the top card from the draw pile and play it for free.
    /// Exhausts it (unless it's a Power). If the card targets an enemy,
    /// pushes a TargetSelect screen for target selection.
    PlayTopOfDraw,
    /// Deal damage to each attacking monster, once per hit in their intent.
    FlameBarrier(i16),
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
    /// Dispose a played card to the appropriate pile. Only created dynamically
    /// at runtime (never in static CardInfo definitions).
    DisposeCard {
        card: crate::types::Card,
        exhaust: bool,
        rebound: bool,
    },
    /// Move a card to the exhaust pile and trigger on-exhaust power effects.
    ExhaustCard { card: crate::types::Card },
    /// Draw exactly one card from the draw pile into hand.
    /// If the draw pile is empty, queues ShuffleDiscardIntoDraw + re-queues self.
    /// After drawing, checks for on-draw power triggers (Evolve, FireBreathing).
    DrawOneCard,
    /// Shuffle the discard pile into the draw pile. Fires on-shuffle power triggers.
    ShuffleDiscardIntoDraw,
    /// Deal fixed damage (no Strength scaling) to all non-gone enemies.
    DamageFixedAll(i16),
    /// Pop the last card from hand and play it for free (used by PlayTopOfDraw).
    /// Exhausts non-Power cards. Pushes TargetSelect if the card needs a target.
    PlayLastDrawnFromHand,
    /// Push a TargetSelect screen for the player to choose an enemy target,
    /// then deal fixed damage to the chosen target (used by BGJuggernaut).
    DamageFixedTargetSelect { amount: i16, reason: crate::screen::TargetReason },
    /// After an Attack card resolves: tick down player's BGWeakened and
    /// monsters' BGVulnerable. Only ticks powers that were present before
    /// the card was played. `vuln_mask` is a bitmask of monster indices
    /// that had Vulnerable at queue time.
    TickDownAttackPowers { had_weak: bool, vuln_mask: u8 },
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
