use crate::effects::Effect;

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
];

pub fn lookup(id: &str) -> Option<&'static EventInfo> {
    ACT1_EVENTS.iter().find(|e| e.id == id)
}

/// Get all Act 1 event IDs.
pub fn act1_event_ids() -> Vec<&'static str> {
    ACT1_EVENTS.iter().map(|e| e.id).collect()
}
