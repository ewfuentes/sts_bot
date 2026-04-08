use crate::effects::{DieOutcome, Effect};

pub struct EventInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub options: &'static [EventOptionInfo],
}

#[derive(Debug, Clone, Copy)]
pub enum EventCondition {
    MinGold(u16),
}

pub struct EventOptionInfo {
    pub label: &'static str,
    pub effects: &'static [Effect],
    pub condition: Option<EventCondition>,
}

static ACT1_EVENTS: &[EventInfo] = &[
    EventInfo {
        id: "BGThe Library",
        name: "The Library",
        options: &[
            EventOptionInfo { label: "Read", effects: &[Effect::ChooseCardReward], condition: None },
            EventOptionInfo { label: "Sleep", effects: &[Effect::Heal(3)], condition: None },
        ],
    },
    EventInfo {
        id: "BGGolden Idol",
        name: "Golden Idol",
        options: &[
            EventOptionInfo { label: "Take the Idol", effects: &[Effect::GainRandomRelic, Effect::LoseHP(1)], condition: None },
            EventOptionInfo { label: "Leave", effects: &[], condition: None },
        ],
    },
    EventInfo {
        id: "BGGolden Wing",
        name: "Golden Wing",
        options: &[
            EventOptionInfo { label: "Pray", effects: &[Effect::LoseHP(2), Effect::PurgeFromDeck], condition: None },
            EventOptionInfo { label: "Steal", effects: &[Effect::GainGold(2)], condition: None },
            EventOptionInfo { label: "Leave", effects: &[], condition: None },
        ],
    },
    EventInfo {
        id: "BGBig Fish",
        name: "Big Fish",
        options: &[
            EventOptionInfo { label: "Banana (Heal 2 HP)", effects: &[Effect::Heal(2)], condition: None },
            EventOptionInfo { label: "Donut (Upgrade a Strike)", effects: &[Effect::UpgradeStrike], condition: None },
            EventOptionInfo { label: "Box (Relic + Curse)", effects: &[Effect::GainRandomRelic, Effect::GainRandomCurse], condition: None },
            EventOptionInfo { label: "Remove a Strike", effects: &[Effect::RemoveStrike], condition: None },
        ],
    },
    EventInfo {
        id: "BGThe Cleric",
        name: "The Cleric",
        options: &[
            EventOptionInfo { label: "Heal (1 gold)", effects: &[Effect::LoseGold(1), Effect::Heal(3)], condition: Some(EventCondition::MinGold(1)) },
            EventOptionInfo { label: "Upgrade (2 gold)", effects: &[Effect::LoseGold(2), Effect::UpgradeFromDeck], condition: Some(EventCondition::MinGold(2)) },
            EventOptionInfo { label: "Purify (3 gold)", effects: &[Effect::LoseGold(3), Effect::PurgeFromDeck], condition: Some(EventCondition::MinGold(3)) },
            EventOptionInfo { label: "Leave", effects: &[], condition: None },
        ],
    },
    EventInfo {
        id: "BGBonfire Elementals",
        name: "Bonfire Elementals",
        options: &[
            EventOptionInfo { label: "Offer a Card", effects: &[Effect::BonfireOffer], condition: None },
        ],
    },
    // ── Medium events ──
    EventInfo {
        id: "BGLiving Wall",
        name: "Living Wall",
        options: &[
            EventOptionInfo { label: "Forget (Purge)", effects: &[Effect::PurgeFromDeck], condition: None },
            EventOptionInfo { label: "Change (Transform)", effects: &[Effect::TransformFromDeck], condition: None },
            EventOptionInfo { label: "Grow (Upgrade)", effects: &[Effect::UpgradeFromDeck], condition: None },
        ],
    },
    EventInfo {
        id: "BGTransmorgrifier",
        name: "Transmogrifier",
        options: &[
            EventOptionInfo { label: "Transform 1 Card", effects: &[Effect::TransformFromDeck], condition: None },
            EventOptionInfo { label: "Tempt Fate (Transform 2 + Curse)", effects: &[Effect::TransformFromDeck, Effect::TransformFromDeck, Effect::GainRandomCurse], condition: None },
        ],
    },
    EventInfo {
        id: "BGUpgrade Shrine",
        name: "Upgrade Shrine",
        options: &[
            EventOptionInfo { label: "Upgrade a Card", effects: &[Effect::UpgradeFromDeck], condition: None },
            EventOptionInfo { label: "Gamble (2 dmg, upgrade 1-2 random)", effects: &[Effect::LoseHP(2), Effect::UpgradeRandomCards], condition: None },
        ],
    },
    // ── Die roll events ──
    EventInfo {
        id: "BGAccursed Blacksmith",
        name: "Accursed Blacksmith",
        options: &[
            EventOptionInfo {
                label: "Forge (Upgrade + 2 dmg)",
                effects: &[Effect::LoseHP(2), Effect::UpgradeFromDeck],
                condition: None,
            },
            EventOptionInfo {
                label: "Rummage (Roll for relic)",
                effects: &[Effect::EventDieRoll { seed: 0, outcomes: &[
                    DieOutcome { min: 1, max: 3, effects: &[Effect::GainRandomRelic, Effect::GainRandomCurse] },
                    DieOutcome { min: 4, max: 6, effects: &[Effect::GainRandomRelic] },
                ] }],
                condition: None,
            },
            EventOptionInfo { label: "Leave", effects: &[], condition: None },
        ],
    },
    EventInfo {
        id: "BGWheel of Change",
        name: "Wheel of Change",
        options: &[
            EventOptionInfo {
                label: "Spin the Wheel",
                effects: &[Effect::EventDieRoll { seed: 0, outcomes: &[
                    DieOutcome { min: 1, max: 1, effects: &[Effect::GainGold(4)] },
                    DieOutcome { min: 2, max: 2, effects: &[Effect::GainRandomRelic] },
                    DieOutcome { min: 3, max: 3, effects: &[Effect::FullHeal] },
                    DieOutcome { min: 4, max: 4, effects: &[Effect::GainRandomCurse] },
                    DieOutcome { min: 5, max: 5, effects: &[Effect::PurgeFromDeck] },
                    DieOutcome { min: 6, max: 6, effects: &[Effect::LoseHP(2)] },
                ] }],
                condition: None,
            },
        ],
    },
    EventInfo {
        id: "BGLab",
        name: "Lab",
        options: &[
            EventOptionInfo {
                label: "Search",
                effects: &[Effect::EventDieRoll { seed: 0, outcomes: &[
                    DieOutcome { min: 1, max: 3, effects: &[Effect::GainRandomPotion] },
                    DieOutcome { min: 4, max: 6, effects: &[Effect::GainRandomPotion, Effect::GainRandomPotion] },
                ] }],
                condition: None,
            },
        ],
    },
];

pub fn lookup(id: &str) -> Option<&'static EventInfo> {
    ACT1_EVENTS.iter().find(|e| e.id == id)
}

/// Get all Act 1 event IDs.
pub fn act1_event_ids() -> Vec<&'static str> {
    ACT1_EVENTS.iter().map(|e| e.id).collect()
}
