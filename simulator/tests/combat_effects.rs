use sts_simulator::{Action, Card, GameState, HandCard, Monster, Power, Screen, TargetReason};

fn make_card(id: &str, cost: i8, card_type: &str) -> Card {
    Card {
        id: id.to_string(),
        name: id.to_string(),
        cost,
        card_type: card_type.to_string(),
        upgraded: false,
    }
}

fn make_hand_card(id: &str, cost: i8, card_type: &str) -> HandCard {
    HandCard {
        card: make_card(id, cost, card_type),
    }
}

fn make_monster(id: &str, name: &str, hp: u16, block: u16, powers: Vec<Power>) -> Monster {
    Monster {
        id: id.to_string(),
        name: name.to_string(),
        hp,
        max_hp: hp,
        block,
        intent: "ATTACK".to_string(),
        damage: Some(1),
        hits: 1,
        powers,
        is_gone: false,
        move_index: 0,
    }
}

fn play_action(id: &str, cost: i8, card_type: &str, hand_index: u8, target: Option<u8>) -> Action {
    Action::PlayCard {
        card: make_card(id, cost, card_type),
        hand_index,
        target_index: target,
        target_name: target.map(|_| "Target".to_string()),
    }
}

fn combat_state_with_monsters(
    hand: Vec<HandCard>,
    monsters: Vec<Monster>,
    energy: u8,
    block: u16,
    player_powers: Vec<Power>,
) -> GameState {
    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": block,
            "player_energy": energy,
            "player_powers": player_powers,
            "die_roll": 1,
            "turn": 1
        }
    });
    GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap()
}

// ── Damage ──

#[test]
fn strike_deals_damage() {
    let hand = vec![make_hand_card("BGStrike_R", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGStrike_R", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, player_energy, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 7); // 8 - 1 = 7
        assert_eq!(*player_energy, 2);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn damage_blocked_by_monster_block() {
    let hand = vec![make_hand_card("BGBludgeon", 3, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 3, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGBludgeon", 3, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        // Bludgeon deals 7, monster has 3 block: 7 - 3 = 4 damage to HP
        assert_eq!(monsters[0].block, 0);
        assert_eq!(monsters[0].hp, 4); // 8 - 4 = 4
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn damage_kills_last_monster_transitions_to_rewards() {
    let hand = vec![make_hand_card("BGBludgeon", 3, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 5, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGBludgeon", 3, "ATTACK", 0, Some(0)));

    // Killing the last monster should transition to combat rewards
    assert!(matches!(state.current_screen(), Screen::CombatRewards { .. }),
        "Expected CombatRewards, got {:?}", state.current_screen());
}

#[test]
fn damage_kills_one_of_two_monsters_stays_in_combat() {
    let hand = vec![make_hand_card("BGBludgeon", 3, "ATTACK")];
    let monsters = vec![
        make_monster("BGJawWorm", "Jaw Worm", 5, 0, vec![]),
        make_monster("BGGreenLouse", "Louse", 5, 0, vec![]),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGBludgeon", 3, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert!(monsters[0].is_gone);
        assert!(!monsters[1].is_gone);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn twin_strike_hits_twice() {
    let hand = vec![make_hand_card("BGTwin Strike", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGTwin Strike", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 6); // 8 - 1 - 1 = 6
    } else {
        panic!("Expected Combat screen");
    }
}

// ── DamageAll ──

#[test]
fn cleave_damages_all_enemies() {
    let hand = vec![make_hand_card("BGCleave", 1, "ATTACK")];
    let monsters = vec![
        make_monster("BGGreenLouse", "Louse A", 5, 0, vec![]),
        make_monster("BGRedLouse", "Louse B", 5, 0, vec![]),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGCleave", 1, "ATTACK", 0, None));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 3); // 5 - 2
        assert_eq!(monsters[1].hp, 3); // 5 - 2
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn damage_all_skips_dead_monsters() {
    let hand = vec![make_hand_card("BGCleave", 1, "ATTACK")];
    let mut dead = make_monster("BGRedLouse", "Dead Louse", 0, 0, vec![]);
    dead.is_gone = true;
    let monsters = vec![
        make_monster("BGGreenLouse", "Louse A", 5, 0, vec![]),
        dead,
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGCleave", 1, "ATTACK", 0, None));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 3);
        assert_eq!(monsters[1].hp, 0); // stayed dead
    } else {
        panic!("Expected Combat screen");
    }
}

// ── Block ──

#[test]
fn defend_gains_block() {
    let hand = vec![make_hand_card("BGDefend_R", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGDefend_R", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, .. } = state.current_screen() {
        assert_eq!(*player_block, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn block_stacks() {
    let hand = vec![
        make_hand_card("BGDefend_R", 1, "SKILL"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 2, vec![]);

    state.apply(&play_action("BGDefend_R", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, .. } = state.current_screen() {
        assert_eq!(*player_block, 3); // 2 existing + 1
    } else {
        panic!("Expected Combat screen");
    }
}

// ── ApplyPower ──

#[test]
fn bash_applies_vulnerable() {
    let hand = vec![make_hand_card("BGBash", 2, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGBash", 2, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 6); // 8 - 2
        assert_eq!(monsters[0].powers.len(), 1);
        assert_eq!(monsters[0].powers[0].id, "BGVulnerable");
        assert_eq!(monsters[0].powers[0].amount, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn shockwave_applies_to_all_enemies() {
    let hand = vec![make_hand_card("BGShockwave", 2, "SKILL")];
    let monsters = vec![
        make_monster("BGGreenLouse", "Louse A", 5, 0, vec![]),
        make_monster("BGRedLouse", "Louse B", 5, 0, vec![]),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGShockwave", 2, "SKILL", 0, None));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        for m in monsters.iter() {
            let weak = m.powers.iter().find(|p| p.id == "BGWeakened").unwrap();
            let vuln = m.powers.iter().find(|p| p.id == "BGVulnerable").unwrap();
            assert_eq!(weak.amount, 1);
            assert_eq!(vuln.amount, 1);
        }
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn inflame_applies_strength_to_player() {
    let hand = vec![make_hand_card("BGInflame", 2, "POWER")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGInflame", 2, "POWER", 0, None));

    if let Screen::Combat { player_powers, discard_pile, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(player_powers.len(), 1);
        assert_eq!(player_powers[0].id, "Strength");
        assert_eq!(player_powers[0].amount, 1);
        // Power card consumed — not in discard or exhaust
        assert!(discard_pile.is_empty());
        assert!(exhaust_pile.is_empty());
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn power_stacks() {
    let hand = vec![
        make_hand_card("BGInflame", 2, "POWER"),
        make_hand_card("BGInflame", 2, "POWER"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 6, 0, vec![]);

    state.apply(&play_action("BGInflame", 2, "POWER", 0, None));
    state.apply(&play_action("BGInflame", 2, "POWER", 0, None));

    if let Screen::Combat { player_powers, .. } = state.current_screen() {
        assert_eq!(player_powers.len(), 1);
        assert_eq!(player_powers[0].id, "Strength");
        assert_eq!(player_powers[0].amount, 2);
    } else {
        panic!("Expected Combat screen");
    }
}

// ── Draw ──

#[test]
fn pommel_strike_deals_damage_and_draws() {
    let hand = vec![make_hand_card("BGPommel Strike", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [make_card("BGDefend_R", 1, "SKILL")],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGPommel Strike", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, hand, draw_pile, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 6); // 8 - 2
        assert_eq!(hand.len(), 1); // drew 1
        assert_eq!(hand[0].card.id, "BGDefend_R");
        assert!(draw_pile.is_empty());
    } else {
        panic!("Expected Combat screen");
    }
}

// ── GainEnergy + LoseHP ──

#[test]
fn offering_loses_hp_gains_energy_draws() {
    let hand = vec![make_hand_card("BGOffering", 0, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [
                make_card("BGStrike_R", 1, "ATTACK"),
                make_card("BGStrike_R", 1, "ATTACK"),
                make_card("BGDefend_R", 1, "SKILL")
            ],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGOffering", 0, "SKILL", 0, None));

    assert_eq!(state.hp, 9); // lost 1 HP

    if let Screen::Combat { hand, player_energy, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(*player_energy, 5); // 3 + 2
        assert_eq!(hand.len(), 3); // drew 3
        assert_eq!(exhaust_pile.len(), 1); // Offering exhausted
        assert_eq!(exhaust_pile[0].id, "BGOffering");
    } else {
        panic!("Expected Combat screen");
    }
}

// ── Composite: Uppercut ──

#[test]
fn uppercut_damages_and_debuffs() {
    let hand = vec![make_hand_card("BGUppercut", 2, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGUppercut", 2, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 5); // 8 - 3
        let weak = monsters[0].powers.iter().find(|p| p.id == "BGWeakened").unwrap();
        let vuln = monsters[0].powers.iter().find(|p| p.id == "BGVulnerable").unwrap();
        assert_eq!(weak.amount, 1);
        assert_eq!(vuln.amount, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

// ── AddCardToPile ──

#[test]
fn wild_strike_adds_dazed_to_draw() {
    let hand = vec![make_hand_card("BGWild Strike", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGWild Strike", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, draw_pile, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 5); // 8 - 3
        assert_eq!(draw_pile.len(), 1);
        assert_eq!(draw_pile[0].id, "Dazed");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn power_through_blocks_and_adds_dazed() {
    let hand = vec![make_hand_card("BGPower Through", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGPower Through", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, draw_pile, .. } = state.current_screen() {
        assert_eq!(*player_block, 3);
        assert_eq!(draw_pile.len(), 1);
        assert_eq!(draw_pile[0].id, "Dazed");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn immolate_damages_all_and_adds_two_dazed() {
    let hand = vec![make_hand_card("BGImmolate", 2, "ATTACK")];
    let monsters = vec![
        make_monster("BGGreenLouse", "Louse A", 5, 0, vec![]),
        make_monster("BGRedLouse", "Louse B", 5, 0, vec![]),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGImmolate", 2, "ATTACK", 0, None));

    if let Screen::Combat { monsters, draw_pile, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 0); // 5 - 5, dead
        assert_eq!(monsters[1].hp, 0); // 5 - 5, dead
        assert_eq!(draw_pile.len(), 2);
        assert!(draw_pile.iter().all(|c| c.id == "Dazed"));
    } else {
        // Both monsters dead — should transition to rewards
        assert!(matches!(state.current_screen(), Screen::CombatRewards { .. }));
    }
}

// ── PlayCondition ──

#[test]
fn clash_not_in_actions_with_non_attack_in_hand() {
    let hand = vec![
        make_hand_card("BGClash", 0, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    let actions = state.available_actions();
    let has_clash = actions.iter().any(|a| matches!(a, Action::PlayCard { card, .. } if card.id == "BGClash"));
    assert!(!has_clash, "Clash should not be playable with a non-attack in hand");
}

#[test]
fn clash_in_actions_when_hand_all_attacks() {
    let hand = vec![
        make_hand_card("BGClash", 0, "ATTACK"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    let actions = state.available_actions();
    let has_clash = actions.iter().any(|a| matches!(a, Action::PlayCard { card, .. } if card.id == "BGClash"));
    assert!(has_clash, "Clash should be playable with only attacks in hand");
}

#[test]
fn clash_becomes_playable_after_playing_non_attack() {
    let hand = vec![
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGClash", 0, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    // With Defend in hand, Clash should not be playable
    let actions = state.available_actions();
    assert!(!actions.iter().any(|a| matches!(a, Action::PlayCard { card, .. } if card.id == "BGClash")));

    // Play Defend — now only attacks remain
    state.apply(&play_action("BGDefend_R", 1, "SKILL", 2, None));

    let actions = state.available_actions();
    assert!(actions.iter().any(|a| matches!(a, Action::PlayCard { card, .. } if card.id == "BGClash")),
        "Clash should be playable after removing non-attack from hand");
}

#[test]
fn clash_deals_damage() {
    let hand = vec![
        make_hand_card("BGClash", 0, "ATTACK"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGClash", 0, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, player_energy, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 5); // 8 - 3
        assert_eq!(*player_energy, 3); // cost 0
    } else {
        panic!("Expected Combat screen");
    }
}

// ── DamageBasedOn ──

#[test]
fn body_slam_deals_damage_equal_to_block() {
    let hand = vec![make_hand_card("BGBody Slam", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 4, vec![]);

    state.apply(&play_action("BGBody Slam", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, player_block, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 4); // 8 - 4 block = 4
        assert_eq!(*player_block, 4); // block not consumed
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn body_slam_zero_block_deals_no_damage() {
    let hand = vec![make_hand_card("BGBody Slam", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGBody Slam", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 8); // no damage
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn rampage_deals_damage_equal_to_exhaust_pile_size() {
    let hand = vec![make_hand_card("BGRampage", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [
                make_card("BGStrike_R", 1, "ATTACK"),
                make_card("BGDefend_R", 1, "SKILL"),
                make_card("BGDefend_R", 1, "SKILL"),
            ],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGRampage", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 17); // 20 - 3 (exhaust pile had 3 cards)
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn rampage_empty_exhaust_deals_no_damage() {
    let hand = vec![make_hand_card("BGRampage", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGRampage", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 8); // no damage
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn rampage_upgraded_exhausts_then_deals_damage() {
    let upgraded_rampage = HandCard {
        card: Card {
            id: "BGRampage".to_string(),
            name: "BGRampage".to_string(),
            cost: 1,
            card_type: "ATTACK".to_string(),
            upgraded: true,
        },
    };
    let hand = vec![
        upgraded_rampage.clone(),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [
                make_card("BGDefend_R", 1, "SKILL"),
                make_card("BGDefend_R", 1, "SKILL"),
            ],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&Action::PlayCard {
        card: upgraded_rampage.card,
        hand_index: 0,
        target_index: Some(0),
        target_name: Some("Jaw Worm".to_string()),
    });

    // Should pause for hand select (exhaust a card)
    assert!(matches!(state.current_screen(), Screen::HandSelect { .. }));

    state.apply(&Action::PickHandCard {
        card: make_card("BGStrike_R", 1, "ATTACK"),
        choice_index: 0,
    });

    // Exhaust pile: 2 original + 1 exhausted = 3, so damage = 3
    if let Screen::Combat { monsters, exhaust_pile, hand, .. } = state.current_screen() {
        assert_eq!(exhaust_pile.len(), 3);
        assert_eq!(monsters[0].hp, 17); // 20 - 3
        assert_eq!(hand.len(), 1); // Defend remains
        assert_eq!(hand[0].card.id, "BGDefend_R");
    } else {
        panic!("Expected Combat screen");
    }
}

// ── Rebound ──

#[test]
fn anger_deals_damage_and_rebounds_to_draw() {
    let hand = vec![make_hand_card("BGAnger", 0, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGAnger", 0, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, draw_pile, discard_pile, player_energy, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 7); // 8 - 1
        assert_eq!(*player_energy, 3); // cost 0
        assert!(discard_pile.is_empty(), "Anger should not go to discard");
        assert_eq!(draw_pile.len(), 1);
        assert_eq!(draw_pile[0].id, "BGAnger");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn anger_rebound_goes_on_top_of_draw() {
    let hand = vec![make_hand_card("BGAnger", 0, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [
                make_card("BGStrike_R", 1, "ATTACK"),
                make_card("BGDefend_R", 1, "SKILL"),
            ],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGAnger", 0, "ATTACK", 0, Some(0)));

    if let Screen::Combat { draw_pile, .. } = state.current_screen() {
        assert_eq!(draw_pile.len(), 3); // 2 existing + 1 rebounded
        // Anger should be on top (last element = top of draw pile)
        assert_eq!(draw_pile.last().unwrap().id, "BGAnger");
    } else {
        panic!("Expected Combat screen");
    }
}

// ── DoubleBlock / DoubleStrength ──

#[test]
fn entrench_doubles_block() {
    let hand = vec![make_hand_card("BGEntrench", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 5, vec![]);

    state.apply(&play_action("BGEntrench", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(*player_block, 10); // 5 * 2
        assert_eq!(exhaust_pile.len(), 1); // exhausts
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn entrench_zero_block_stays_zero() {
    let hand = vec![make_hand_card("BGEntrench", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGEntrench", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, .. } = state.current_screen() {
        assert_eq!(*player_block, 0);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn limit_break_doubles_strength() {
    let hand = vec![make_hand_card("BGLimit Break", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [{"id": "Strength", "amount": 3}],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGLimit Break", 1, "SKILL", 0, None));

    if let Screen::Combat { player_powers, exhaust_pile, .. } = state.current_screen() {
        let strength = player_powers.iter().find(|p| p.id == "Strength").unwrap();
        assert_eq!(strength.amount, 6); // 3 * 2
        assert_eq!(exhaust_pile.len(), 1); // exhausts
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn limit_break_no_strength_does_nothing() {
    let hand = vec![make_hand_card("BGLimit Break", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGLimit Break", 1, "SKILL", 0, None));

    if let Screen::Combat { player_powers, .. } = state.current_screen() {
        assert!(player_powers.iter().find(|p| p.id == "Strength").is_none());
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn limit_break_capped_at_max_strength() {
    let hand = vec![make_hand_card("BGLimit Break", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [{"id": "Strength", "amount": 5}],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGLimit Break", 1, "SKILL", 0, None));

    if let Screen::Combat { player_powers, .. } = state.current_screen() {
        let strength = player_powers.iter().find(|p| p.id == "Strength").unwrap();
        assert_eq!(strength.amount, 8); // 5 * 2 = 10, capped at 8
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn strength_cap_via_apply_power() {
    // Stack Inflame (Strength +1) past the cap
    let hand = vec![
        make_hand_card("BGInflame", 2, "POWER"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [{"id": "Strength", "amount": 8}],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGInflame", 2, "POWER", 0, None));

    if let Screen::Combat { player_powers, .. } = state.current_screen() {
        let strength = player_powers.iter().find(|p| p.id == "Strength").unwrap();
        assert_eq!(strength.amount, 8); // still capped
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn warcry_draws_then_puts_on_top() {
    let hand = vec![make_hand_card("BGWarcry", 0, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [
                make_card("BGStrike_R", 1, "ATTACK"),
                make_card("BGDefend_R", 1, "SKILL"),
                make_card("BGBash", 2, "ATTACK"),
            ],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGWarcry", 0, "SKILL", 0, None));

    // Drew 2 cards, now should be on HandSelect to put 1 on top of draw
    assert!(matches!(state.current_screen(), Screen::HandSelect { .. }),
        "Expected HandSelect, got {:?}", state.current_screen());

    // Pick the first card to put on top of draw
    state.apply(&Action::PickHandCard {
        card: make_card("BGBash", 2, "ATTACK"),
        choice_index: 0,
    });

    if let Screen::Combat { hand, draw_pile, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(hand.len(), 1); // 2 drawn - 1 put back
        assert_eq!(draw_pile.len(), 2); // 3 - 2 drawn + 1 put back
        assert_eq!(exhaust_pile.len(), 1); // Warcry exhausted
        assert_eq!(exhaust_pile[0].id, "BGWarcry");
    } else {
        panic!("Expected Combat screen");
    }
}

// ── GainTemporaryStrength / Strength cap ──

#[test]
fn battle_trance_draws_and_applies_no_draw() {
    let hand = vec![make_hand_card("BGBattle Trance", 0, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [
                make_card("BGStrike_R", 1, "ATTACK"),
                make_card("BGStrike_R", 1, "ATTACK"),
                make_card("BGDefend_R", 1, "SKILL"),
            ],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGBattle Trance", 0, "SKILL", 0, None));

    if let Screen::Combat { hand, player_powers, .. } = state.current_screen() {
        assert_eq!(hand.len(), 3); // drew 3
        let no_draw = player_powers.iter().find(|p| p.id == "NoDrawPower");
        assert!(no_draw.is_some(), "Should have NoDrawPower");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn double_tap_applies_power() {
    let hand = vec![make_hand_card("BGDouble Tap", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGDouble Tap", 1, "SKILL", 0, None));

    if let Screen::Combat { player_powers, player_energy, .. } = state.current_screen() {
        assert_eq!(*player_energy, 2); // cost 1
        let power = player_powers.iter().find(|p| p.id == "BGDoubleAttack");
        assert!(power.is_some(), "Should have BGDoubleAttack power");
        assert_eq!(power.unwrap().amount, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn sever_soul_damages_and_exhausts_from_hand() {
    let hand = vec![
        make_hand_card("BGSever Soul", 2, "ATTACK"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGSever Soul", 2, "ATTACK", 0, Some(0)));

    // Should pause on HandSelect to exhaust 1 card
    assert!(matches!(state.current_screen(), Screen::HandSelect { .. }),
        "Expected HandSelect, got {:?}", state.current_screen());

    state.apply(&Action::PickHandCard {
        card: make_card("BGDefend_R", 1, "SKILL"),
        choice_index: 1,
    });

    if let Screen::Combat { monsters, hand, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 17); // 20 - 3
        assert_eq!(hand.len(), 1); // Strike remains
        assert_eq!(exhaust_pile.len(), 1);
        assert_eq!(exhaust_pile[0].id, "BGDefend_R");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn sever_soul_upgraded_exhausts_up_to_two() {
    let upgraded = HandCard {
        card: Card {
            id: "BGSever Soul".to_string(),
            name: "BGSever Soul".to_string(),
            cost: 2,
            card_type: "ATTACK".to_string(),
            upgraded: true,
        },
    };
    let hand = vec![
        upgraded.clone(),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
        make_hand_card("BGBash", 2, "ATTACK"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&Action::PlayCard {
        card: upgraded.card,
        hand_index: 0,
        target_index: Some(0),
        target_name: Some("Jaw Worm".to_string()),
    });

    // Should pause on HandSelect (min 1, max 2)
    if let Screen::HandSelect { min_cards, max_cards, .. } = state.current_screen() {
        assert_eq!(*min_cards, 1);
        assert_eq!(*max_cards, 2);
    } else {
        panic!("Expected HandSelect, got {:?}", state.current_screen());
    }

    // Pick first card
    state.apply(&Action::PickHandCard {
        card: make_card("BGStrike_R", 1, "ATTACK"),
        choice_index: 0,
    });
    // Pick second card
    state.apply(&Action::PickHandCard {
        card: make_card("BGDefend_R", 1, "SKILL"),
        choice_index: 0,
    });

    if let Screen::Combat { monsters, hand, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 16); // 20 - 4 (upgraded damage)
        assert_eq!(hand.len(), 1); // Bash remains
        assert_eq!(exhaust_pile.len(), 2);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn battle_trance_upgraded_draws_four() {
    let upgraded = HandCard {
        card: Card {
            id: "BGBattle Trance".to_string(),
            name: "BGBattle Trance".to_string(),
            cost: 0,
            card_type: "SKILL".to_string(),
            upgraded: true,
        },
    };
    let hand = vec![upgraded.clone()];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [
                make_card("BGStrike_R", 1, "ATTACK"),
                make_card("BGStrike_R", 1, "ATTACK"),
                make_card("BGDefend_R", 1, "SKILL"),
                make_card("BGDefend_R", 1, "SKILL"),
            ],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&Action::PlayCard {
        card: upgraded.card,
        hand_index: 0,
        target_index: None,
        target_name: None,
    });

    if let Screen::Combat { hand, draw_pile, .. } = state.current_screen() {
        assert_eq!(hand.len(), 4); // drew 4
        assert_eq!(draw_pile.len(), 0);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn flex_gains_temporary_strength_and_exhausts() {
    let hand = vec![make_hand_card("BGFlex", 0, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGFlex", 0, "SKILL", 0, None));

    if let Screen::Combat { player_powers, exhaust_pile, player_energy, .. } = state.current_screen() {
        assert_eq!(*player_energy, 3); // cost 0
        let strength = player_powers.iter().find(|p| p.id == "Strength").unwrap();
        assert_eq!(strength.amount, 1);
        let lose = player_powers.iter().find(|p| p.id == "LoseStrength").unwrap();
        assert_eq!(lose.amount, 1);
        assert_eq!(exhaust_pile.len(), 1); // Flex exhausts
        assert_eq!(exhaust_pile[0].id, "BGFlex");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn flex_capped_at_max_strength() {
    let hand = vec![make_hand_card("BGFlex", 0, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [{"id": "Strength", "amount": 8}],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGFlex", 0, "SKILL", 0, None));

    if let Screen::Combat { player_powers, .. } = state.current_screen() {
        let strength = player_powers.iter().find(|p| p.id == "Strength").unwrap();
        assert_eq!(strength.amount, 8); // still capped
        assert!(player_powers.iter().find(|p| p.id == "LoseStrength").is_none(),
            "Should not have LoseStrength when at cap");
    } else {
        panic!("Expected Combat screen");
    }
}

// ── OnExhaust ──

#[test]
fn sentinel_gains_energy_when_exhausted_by_true_grit() {
    let hand = vec![
        make_hand_card("BGTrue Grit", 1, "SKILL"),
        make_hand_card("BGSentinel", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    // Play True Grit — only Sentinel left, auto-exhausts it
    state.apply(&play_action("BGTrue Grit", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, player_energy, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(*player_block, 1); // True Grit block
        assert_eq!(exhaust_pile.len(), 1);
        assert_eq!(exhaust_pile[0].id, "BGSentinel");
        assert_eq!(*player_energy, 4); // 3 - 1 (True Grit cost) + 2 (Sentinel on_exhaust)
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn sentinel_upgraded_gains_more_energy_on_exhaust() {
    let upgraded_sentinel = HandCard {
        card: Card {
            id: "BGSentinel".to_string(),
            name: "BGSentinel".to_string(),
            cost: 1,
            card_type: "SKILL".to_string(),
            upgraded: true,
        },
    };
    let hand = vec![
        make_hand_card("BGTrue Grit", 1, "SKILL"),
        upgraded_sentinel,
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGTrue Grit", 1, "SKILL", 0, None));

    if let Screen::Combat { player_energy, .. } = state.current_screen() {
        assert_eq!(*player_energy, 5); // 3 - 1 + 3 (upgraded on_exhaust)
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn sentinel_no_trigger_when_played_normally() {
    let hand = vec![make_hand_card("BGSentinel", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    // Play Sentinel normally — it goes to discard, not exhaust
    state.apply(&play_action("BGSentinel", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, player_energy, discard_pile, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(*player_block, 2);
        assert_eq!(*player_energy, 2); // 3 - 1, no on_exhaust bonus
        assert_eq!(discard_pile.len(), 1);
        assert!(exhaust_pile.is_empty());
    } else {
        panic!("Expected Combat screen");
    }
}

// ── ForEachInHand ──

#[test]
fn rage_blocks_per_attack_in_hand() {
    let hand = vec![
        make_hand_card("BGRage", 1, "SKILL"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGRage", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, hand, .. } = state.current_screen() {
        assert_eq!(*player_block, 2); // 2 attacks in hand
        assert_eq!(hand.len(), 3); // cards not exhausted
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn rage_zero_attacks_no_block() {
    let hand = vec![
        make_hand_card("BGRage", 1, "SKILL"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGRage", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, .. } = state.current_screen() {
        assert_eq!(*player_block, 0);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn second_wind_blocks_and_exhausts_non_attacks() {
    let hand = vec![
        make_hand_card("BGSecond Wind", 1, "SKILL"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGSecond Wind", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, hand, exhaust_pile, .. } = state.current_screen() {
        // 2 non-attack cards (the 2 Defends) — Second Wind itself was already played
        assert_eq!(*player_block, 2); // 1 block per non-attack
        assert_eq!(exhaust_pile.len(), 2); // 2 Defends exhausted
        assert_eq!(hand.len(), 1); // only Strike remains
        assert_eq!(hand[0].card.id, "BGStrike_R");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn second_wind_no_non_attacks_does_nothing() {
    let hand = vec![
        make_hand_card("BGSecond Wind", 1, "SKILL"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGSecond Wind", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, hand, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(*player_block, 0);
        assert_eq!(hand.len(), 1);
        assert_eq!(exhaust_pile.len(), 0);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn fiend_fire_exhausts_hand_and_damages_per_card() {
    let hand = vec![
        make_hand_card("BGFiend Fire", 2, "ATTACK"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
        make_hand_card("BGBash", 2, "ATTACK"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGFiend Fire", 2, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, hand, exhaust_pile, .. } = state.current_screen() {
        // 3 cards were in hand when ForEachInHand ran (Fiend Fire already played)
        assert_eq!(monsters[0].hp, 17); // 20 - 3 (1 damage per card)
        assert!(hand.is_empty()); // all exhausted
        // exhaust pile: 3 from hand + Fiend Fire itself (it has exhaust flag)
        assert_eq!(exhaust_pile.len(), 4);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn fiend_fire_empty_hand_deals_no_damage() {
    let hand = vec![
        make_hand_card("BGFiend Fire", 2, "ATTACK"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGFiend Fire", 2, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 8); // no damage
        assert_eq!(exhaust_pile.len(), 1); // just Fiend Fire itself
    } else {
        panic!("Expected Combat screen");
    }
}

// ── SelectFromDiscardToDrawTop ──

#[test]
fn headbutt_damages_and_selects_from_discard() {
    let hand = vec![make_hand_card("BGHeadbutt", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [
                make_card("BGStrike_R", 1, "ATTACK"),
                make_card("BGDefend_R", 1, "SKILL"),
            ],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGHeadbutt", 1, "ATTACK", 0, Some(0)));

    // Should pause on DiscardSelect
    assert!(matches!(state.current_screen(), Screen::DiscardSelect { .. }),
        "Expected DiscardSelect, got {:?}", state.current_screen());

    let actions = state.available_actions();
    assert_eq!(actions.len(), 2); // two cards in discard (Headbutt not yet disposed)

    state.apply(&Action::PickDiscard {
        card: make_card("BGDefend_R", 1, "SKILL"),
        choice_index: 1,
    });

    if let Screen::Combat { monsters, draw_pile, discard_pile, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 18); // 20 - 2
        assert_eq!(draw_pile.len(), 1);
        assert_eq!(draw_pile[0].id, "BGDefend_R");
        // Strike remains in discard + Headbutt disposed after effects
        assert_eq!(discard_pile.len(), 2);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn headbutt_empty_discard_skips_selection() {
    let hand = vec![make_hand_card("BGHeadbutt", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];
    // Empty discard — disposition happens after effects drain, so discard is
    // empty when SelectFromDiscardToDrawTop runs. Selection is skipped entirely.
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGHeadbutt", 1, "ATTACK", 0, Some(0)));

    // Nothing to select, Headbutt just goes to discard after effects
    if let Screen::Combat { draw_pile, discard_pile, .. } = state.current_screen() {
        assert!(draw_pile.is_empty());
        assert_eq!(discard_pile.len(), 1);
        assert_eq!(discard_pile[0].id, "BGHeadbutt");
    } else {
        panic!("Expected Combat screen");
    }
}

// ── FlameBarrier ──

#[test]
fn flame_barrier_blocks_and_damages_attacking_monsters() {
    let hand = vec![make_hand_card("BGFlame Barrier", 2, "SKILL")];
    // Jaw Worm attacks (1 hit), Louse attacks (2 hits), Cultist buffs (no attack)
    let mut cultist = make_monster("BGCultist", "Cultist", 10, 0, vec![]);
    cultist.intent = "BUFF".to_string();
    cultist.damage = None;
    let mut louse = make_monster("BGGreenLouse", "Louse", 10, 0, vec![]);
    louse.hits = 2;
    let monsters = vec![
        make_monster("BGJawWorm", "Jaw Worm", 10, 0, vec![]),
        louse,
        cultist,
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGFlame Barrier", 2, "SKILL", 0, None));

    if let Screen::Combat { player_block, monsters, .. } = state.current_screen() {
        assert_eq!(*player_block, 3);
        // Jaw Worm: 1 hit → 1 damage
        assert_eq!(monsters[0].hp, 9); // 10 - 1
        // Louse: 2 hits → 2 damage
        assert_eq!(monsters[1].hp, 8); // 10 - 2
        // Cultist: no attack → no damage
        assert_eq!(monsters[2].hp, 10);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn flame_barrier_skips_dead_monsters() {
    let hand = vec![make_hand_card("BGFlame Barrier", 2, "SKILL")];
    let mut dead = make_monster("BGJawWorm", "Jaw Worm", 0, 0, vec![]);
    dead.is_gone = true;
    let monsters = vec![
        dead,
        make_monster("BGGreenLouse", "Louse", 10, 0, vec![]),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGFlame Barrier", 2, "SKILL", 0, None));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 0); // dead, untouched
        assert_eq!(monsters[1].hp, 9); // 10 - 1
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn flame_barrier_no_attacking_monsters() {
    let hand = vec![make_hand_card("BGFlame Barrier", 2, "SKILL")];
    let mut cultist = make_monster("BGCultist", "Cultist", 10, 0, vec![]);
    cultist.intent = "BUFF".to_string();
    cultist.damage = None;
    let monsters = vec![cultist];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGFlame Barrier", 2, "SKILL", 0, None));

    if let Screen::Combat { player_block, monsters, .. } = state.current_screen() {
        assert_eq!(*player_block, 3); // still get block
        assert_eq!(monsters[0].hp, 10); // no damage
    } else {
        panic!("Expected Combat screen");
    }
}

// ── PlayTopOfDraw ──

#[test]
fn havoc_plays_untargeted_card_from_draw() {
    let hand = vec![make_hand_card("BGHavoc", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [make_card("BGDefend_R", 1, "SKILL")],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGHavoc", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, draw_pile, exhaust_pile, discard_pile, .. } = state.current_screen() {
        assert_eq!(*player_block, 1); // Defend gave 1 block
        assert!(draw_pile.is_empty());
        // Defend exhausted (Havoc forces exhaust), Havoc discarded
        assert_eq!(exhaust_pile.len(), 1);
        assert_eq!(exhaust_pile[0].id, "BGDefend_R");
        assert_eq!(discard_pile.len(), 1);
        assert_eq!(discard_pile[0].id, "BGHavoc");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn havoc_plays_targeted_card_needs_target_select() {
    let hand = vec![make_hand_card("BGHavoc", 1, "SKILL")];
    let monsters = vec![
        make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![]),
        make_monster("BGGreenLouse", "Louse", 5, 0, vec![]),
    ];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [make_card("BGStrike_R", 1, "ATTACK")],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGHavoc", 1, "SKILL", 0, None));

    // Should push TargetSelect for the Strike
    assert!(matches!(state.current_screen(), Screen::TargetSelect { .. }),
        "Expected TargetSelect, got {:?}", state.current_screen());

    let actions = state.available_actions();
    assert_eq!(actions.len(), 2); // two live monsters

    // Pick Jaw Worm
    state.apply(&Action::PickTarget {
        reason: TargetReason::Card(make_card("BGStrike_R", 1, "ATTACK")),
        target_index: 0,
        target_name: "Jaw Worm".to_string(),
    });

    if let Screen::Combat { monsters, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 7); // 8 - 1
        assert_eq!(exhaust_pile.len(), 1);
        assert_eq!(exhaust_pile[0].id, "BGStrike_R");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn havoc_empty_draw_does_nothing() {
    let hand = vec![make_hand_card("BGHavoc", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGHavoc", 1, "SKILL", 0, None));

    if let Screen::Combat { draw_pile, discard_pile, .. } = state.current_screen() {
        assert!(draw_pile.is_empty());
        assert_eq!(discard_pile.len(), 1); // just Havoc
        assert_eq!(discard_pile[0].id, "BGHavoc");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn havoc_plays_power_card_without_exhaust() {
    let hand = vec![make_hand_card("BGHavoc", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [make_card("BGInflame", 2, "POWER")],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGHavoc", 1, "SKILL", 0, None));

    if let Screen::Combat { player_powers, exhaust_pile, .. } = state.current_screen() {
        let strength = player_powers.iter().find(|p| p.id == "Strength");
        assert!(strength.is_some()); // Inflame applied
        assert!(exhaust_pile.is_empty()); // Power not exhausted
    } else {
        panic!("Expected Combat screen");
    }
}

// ── SelectFromExhaustToHand ──

#[test]
fn exhume_selects_from_exhaust_to_hand() {
    let hand = vec![make_hand_card("BGExhume", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [
                make_card("BGOffering", 0, "SKILL"),
                make_card("BGImpervious", 2, "SKILL"),
            ],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGExhume", 1, "SKILL", 0, None));

    // Should pause on ExhaustSelect
    assert!(matches!(state.current_screen(), Screen::ExhaustSelect { .. }),
        "Expected ExhaustSelect, got {:?}", state.current_screen());

    let actions = state.available_actions();
    assert_eq!(actions.len(), 2); // two cards in exhaust pile

    state.apply(&Action::PickExhaust {
        card: make_card("BGOffering", 0, "SKILL"),
        choice_index: 0,
    });

    if let Screen::Combat { hand, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(hand.len(), 1);
        assert_eq!(hand[0].card.id, "BGOffering");
        // Impervious remains in exhaust + Exhume disposed there after
        assert_eq!(exhaust_pile.len(), 2);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn exhume_auto_resolves_with_one_card() {
    let hand = vec![make_hand_card("BGExhume", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [
                make_card("BGOffering", 0, "SKILL"),
            ],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGExhume", 1, "SKILL", 0, None));

    // Auto-resolves with 1 card
    if let Screen::Combat { hand, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(hand.len(), 1);
        assert_eq!(hand[0].card.id, "BGOffering");
        assert_eq!(exhaust_pile.len(), 1); // just Exhume itself
        assert_eq!(exhaust_pile[0].id, "BGExhume");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn exhume_empty_exhaust_does_nothing() {
    let hand = vec![make_hand_card("BGExhume", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGExhume", 1, "SKILL", 0, None));

    // Empty exhaust pile (Exhume hasn't been disposed yet), nothing to select
    if let Screen::Combat { hand, exhaust_pile, .. } = state.current_screen() {
        assert!(hand.is_empty());
        assert_eq!(exhaust_pile.len(), 1); // just Exhume itself
    } else {
        panic!("Expected Combat screen");
    }
}

// ── DamageSource::StrikesInHand / StrengthMultiplier ──

#[test]
fn perfected_strike_bonus_per_strike() {
    let hand = vec![
        make_hand_card("BGPerfected Strike", 2, "ATTACK"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGTwin Strike", 1, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGPerfected Strike", 2, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        // 2 other Strikes in hand (BGStrike_R, BGTwin Strike)
        // Damage = 3 + 1*2 = 5
        assert_eq!(monsters[0].hp, 15); // 20 - 5
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn perfected_strike_no_other_strikes() {
    let hand = vec![
        make_hand_card("BGPerfected Strike", 2, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGPerfected Strike", 2, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 17); // 20 - 3 (base only)
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn heavy_blade_scales_with_strength() {
    let hand = vec![make_hand_card("BGHeavy Blade", 2, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [{"id": "Strength", "amount": 2}],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGHeavy Blade", 2, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        // Damage = 3 + 3*2 = 9
        assert_eq!(monsters[0].hp, 11); // 20 - 9
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn heavy_blade_no_strength() {
    let hand = vec![make_hand_card("BGHeavy Blade", 2, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGHeavy Blade", 2, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 17); // 20 - 3 (base only)
    } else {
        panic!("Expected Combat screen");
    }
}

// ── ConditionalOnDieRoll ──

#[test]
fn spot_weakness_gains_strength_on_low_roll() {
    let hand = vec![make_hand_card("BGSpot Weakness", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "die_roll": 2,
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGSpot Weakness", 1, "SKILL", 0, None));

    if let Screen::Combat { player_powers, .. } = state.current_screen() {
        let strength = player_powers.iter().find(|p| p.id == "Strength").unwrap();
        assert_eq!(strength.amount, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn spot_weakness_no_strength_on_high_roll() {
    let hand = vec![make_hand_card("BGSpot Weakness", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "die_roll": 5,
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGSpot Weakness", 1, "SKILL", 0, None));

    if let Screen::Combat { player_powers, .. } = state.current_screen() {
        assert!(player_powers.iter().find(|p| p.id == "Strength").is_none());
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn spot_weakness_upgraded_succeeds_on_four() {
    let upgraded = HandCard {
        card: Card {
            id: "BGSpot Weakness".to_string(),
            name: "BGSpot Weakness".to_string(),
            cost: 1,
            card_type: "SKILL".to_string(),
            upgraded: true,
        },
    };
    let hand = vec![upgraded.clone()];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "die_roll": 4,
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&Action::PlayCard {
        card: upgraded.card,
        hand_index: 0,
        target_index: None,
        target_name: None,
    });

    if let Screen::Combat { player_powers, .. } = state.current_screen() {
        let strength = player_powers.iter().find(|p| p.id == "Strength").unwrap();
        assert_eq!(strength.amount, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

// ── StrengthIfTargetDead ──

#[test]
fn feed_gains_strength_on_kill() {
    let hand = vec![make_hand_card("BGFeed", 1, "ATTACK")];
    let monsters = vec![
        make_monster("BGJawWorm", "Jaw Worm", 3, 0, vec![]),
        make_monster("BGGreenLouse", "Louse", 5, 0, vec![]),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGFeed", 1, "ATTACK", 0, Some(0)));

    // Jaw Worm had 3 HP, Feed deals 3 → dead → gain strength
    if let Screen::Combat { monsters, player_powers, exhaust_pile, .. } = state.current_screen() {
        assert!(monsters[0].is_gone);
        let strength = player_powers.iter().find(|p| p.id == "Strength").unwrap();
        assert_eq!(strength.amount, 1);
        assert_eq!(exhaust_pile.len(), 1); // Feed exhausts
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn feed_no_strength_if_target_survives() {
    let hand = vec![make_hand_card("BGFeed", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 10, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGFeed", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, player_powers, .. } = state.current_screen() {
        assert!(!monsters[0].is_gone);
        assert_eq!(monsters[0].hp, 7); // 10 - 3
        assert!(player_powers.iter().find(|p| p.id == "Strength").is_none());
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn feed_upgraded_gains_two_strength_on_kill() {
    let upgraded = HandCard {
        card: Card {
            id: "BGFeed".to_string(),
            name: "BGFeed".to_string(),
            cost: 1,
            card_type: "ATTACK".to_string(),
            upgraded: true,
        },
    };
    let hand = vec![upgraded.clone()];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 2, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&Action::PlayCard {
        card: upgraded.card,
        hand_index: 0,
        target_index: Some(0),
        target_name: Some("Jaw Worm".to_string()),
    });

    if let Screen::Combat { player_powers, .. } = state.current_screen() {
        let strength = player_powers.iter().find(|p| p.id == "Strength").unwrap();
        assert_eq!(strength.amount, 2);
    } else {
        // Monster died, might transition to rewards
        assert!(matches!(state.current_screen(), Screen::CombatRewards { .. }));
    }
}

// ── XCost ──

#[test]
fn whirlwind_presents_energy_choices() {
    let hand = vec![make_hand_card("BGWhirlwind", -1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGWhirlwind", -1, "ATTACK", 0, None));

    // Should present 4 choices: Spend 0, 1, 2, 3
    assert!(matches!(state.current_screen(), Screen::XCostSelect { .. }));
    let actions = state.available_actions();
    assert_eq!(actions.len(), 4);
    assert!(matches!(&actions[0], Action::PickChoice { label, choice_index: 0 } if label == "Spend 0"));
    assert!(matches!(&actions[3], Action::PickChoice { label, choice_index: 3 } if label == "Spend 3"));
}

#[test]
fn whirlwind_spend_two_damages_all_twice() {
    let hand = vec![make_hand_card("BGWhirlwind", -1, "ATTACK")];
    let monsters = vec![
        make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![]),
        make_monster("BGGreenLouse", "Louse", 10, 0, vec![]),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGWhirlwind", -1, "ATTACK", 0, None));
    state.apply(&Action::PickChoice { label: "Spend 2".to_string(), choice_index: 2 });

    if let Screen::Combat { monsters, player_energy, .. } = state.current_screen() {
        assert_eq!(*player_energy, 1); // 3 - 2
        assert_eq!(monsters[0].hp, 18); // 20 - 2 (1 damage x 2 hits)
        assert_eq!(monsters[1].hp, 8);  // 10 - 2
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn whirlwind_spend_zero_deals_no_damage() {
    let hand = vec![make_hand_card("BGWhirlwind", -1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGWhirlwind", -1, "ATTACK", 0, None));
    state.apply(&Action::PickChoice { label: "Spend 0".to_string(), choice_index: 0 });

    if let Screen::Combat { monsters, player_energy, .. } = state.current_screen() {
        assert_eq!(*player_energy, 3); // no energy spent
        assert_eq!(monsters[0].hp, 8); // no damage
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn whirlwind_upgraded_gets_bonus_hit() {
    let upgraded = HandCard {
        card: Card {
            id: "BGWhirlwind".to_string(),
            name: "BGWhirlwind".to_string(),
            cost: -1,
            card_type: "ATTACK".to_string(),
            upgraded: true,
        },
    };
    let hand = vec![upgraded.clone()];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 2, 0, vec![]);

    state.apply(&Action::PlayCard {
        card: upgraded.card,
        hand_index: 0,
        target_index: None,
        target_name: None,
    });
    // Spend 2 energy → 2 + 1 bonus = 3 hits
    state.apply(&Action::PickChoice { label: "Spend 2".to_string(), choice_index: 2 });

    if let Screen::Combat { monsters, player_energy, .. } = state.current_screen() {
        assert_eq!(*player_energy, 0); // 2 - 2
        assert_eq!(monsters[0].hp, 17); // 20 - 3 (1 damage x 3 hits)
    } else {
        panic!("Expected Combat screen");
    }
}

// ── ChooseOne ──

#[test]
fn iron_wave_base_deals_damage_and_blocks() {
    let hand = vec![make_hand_card("BGIron Wave", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGIron Wave", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, player_block, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 7); // 8 - 1
        assert_eq!(*player_block, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn iron_wave_upgraded_presents_choice() {
    let upgraded = HandCard {
        card: Card {
            id: "BGIron Wave".to_string(),
            name: "BGIron Wave".to_string(),
            cost: 1,
            card_type: "ATTACK".to_string(),
            upgraded: true,
        },
    };
    let hand = vec![upgraded.clone()];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&Action::PlayCard {
        card: upgraded.card,
        hand_index: 0,
        target_index: Some(0),
        target_name: Some("Jaw Worm".to_string()),
    });

    // Should pause on ChoiceSelect
    assert!(matches!(state.current_screen(), Screen::ChoiceSelect { .. }),
        "Expected ChoiceSelect, got {:?}", state.current_screen());

    let actions = state.available_actions();
    assert_eq!(actions.len(), 2);
    assert!(matches!(&actions[0], Action::PickChoice { label, choice_index: 0 } if label == "Spear"));
    assert!(matches!(&actions[1], Action::PickChoice { label, choice_index: 1 } if label == "Shield"));
}

#[test]
fn iron_wave_upgraded_spear_choice() {
    let upgraded = HandCard {
        card: Card {
            id: "BGIron Wave".to_string(),
            name: "BGIron Wave".to_string(),
            cost: 1,
            card_type: "ATTACK".to_string(),
            upgraded: true,
        },
    };
    let hand = vec![upgraded.clone()];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&Action::PlayCard {
        card: upgraded.card,
        hand_index: 0,
        target_index: Some(0),
        target_name: Some("Jaw Worm".to_string()),
    });
    state.apply(&Action::PickChoice { label: "Spear".to_string(), choice_index: 0 });

    if let Screen::Combat { monsters, player_block, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 6); // 8 - 2 (Spear)
        assert_eq!(*player_block, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn iron_wave_upgraded_shield_choice() {
    let upgraded = HandCard {
        card: Card {
            id: "BGIron Wave".to_string(),
            name: "BGIron Wave".to_string(),
            cost: 1,
            card_type: "ATTACK".to_string(),
            upgraded: true,
        },
    };
    let hand = vec![upgraded.clone()];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&Action::PlayCard {
        card: upgraded.card,
        hand_index: 0,
        target_index: Some(0),
        target_name: Some("Jaw Worm".to_string()),
    });
    state.apply(&Action::PickChoice { label: "Shield".to_string(), choice_index: 1 });

    if let Screen::Combat { monsters, player_block, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 7); // 8 - 1 (Shield)
        assert_eq!(*player_block, 2);
    } else {
        panic!("Expected Combat screen");
    }
}

// ── ExhaustFromHand (effect queue + sub-decision) ──

#[test]
fn true_grit_blocks_then_pushes_hand_select() {
    let hand = vec![
        make_hand_card("BGTrue Grit", 1, "SKILL"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGTrue Grit", 1, "SKILL", 0, None));

    // Should push HandSelect for exhaust choice
    if let Screen::HandSelect { cards, min_cards, max_cards, action, .. } = state.current_screen() {
        assert_eq!(*action, sts_simulator::effects::HandSelectAction::Exhaust);
        assert_eq!(*min_cards, 1);
        assert_eq!(*max_cards, 1);
        assert_eq!(cards.len(), 2); // Strike and Defend remain
    } else {
        panic!("Expected HandSelect, got {:?}", state.current_screen());
    }
}

#[test]
fn true_grit_exhaust_pick_resolves() {
    let hand = vec![
        make_hand_card("BGTrue Grit", 1, "SKILL"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGTrue Grit", 1, "SKILL", 0, None));
    state.apply(&Action::PickHandCard {
        card: make_card("BGStrike_R", 1, "ATTACK"),
        choice_index: 0,
    });

    if let Screen::Combat { hand, exhaust_pile, player_block, .. } = state.current_screen() {
        assert_eq!(*player_block, 1);
        assert_eq!(hand.len(), 1);
        assert_eq!(hand[0].card.id, "BGDefend_R");
        assert_eq!(exhaust_pile.len(), 1);
        assert_eq!(exhaust_pile[0].id, "BGStrike_R");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn burning_pact_exhausts_then_draws() {
    let hand = vec![
        make_hand_card("BGBurning Pact", 1, "SKILL"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [
                make_card("BGBash", 2, "ATTACK"),
                make_card("BGDefend_R", 1, "SKILL"),
                make_card("BGStrike_R", 1, "ATTACK"),
            ],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGBurning Pact", 1, "SKILL", 0, None));
    assert!(matches!(state.current_screen(), Screen::HandSelect { .. }));

    // Exhaust Strike
    state.apply(&Action::PickHandCard {
        card: make_card("BGStrike_R", 1, "ATTACK"),
        choice_index: 0,
    });

    // Back in combat — Defend remains + drew 2 from queue continuation
    if let Screen::Combat { hand, exhaust_pile, draw_pile, .. } = state.current_screen() {
        assert_eq!(exhaust_pile.len(), 1);
        assert_eq!(exhaust_pile[0].id, "BGStrike_R");
        assert_eq!(hand.len(), 3); // Defend + 2 drawn
        assert_eq!(draw_pile.len(), 1); // 3 - 2 = 1
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn exhaust_auto_resolves_with_empty_hand() {
    let hand = vec![
        make_hand_card("BGBurning Pact", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [
                make_card("BGDefend_R", 1, "SKILL"),
                make_card("BGBash", 2, "ATTACK"),
            ],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGBurning Pact", 1, "SKILL", 0, None));

    // Empty hand → auto-resolve, no HandSelect, drew 2 directly
    if let Screen::Combat { hand, draw_pile, .. } = state.current_screen() {
        assert_eq!(hand.len(), 2);
        assert_eq!(draw_pile.len(), 0);
    } else {
        panic!("Expected Combat screen, got {:?}", state.current_screen());
    }
}

// ── calculate_damage: Strength, Vulnerable, Weakened ──

fn make_power(id: &str, amount: i32) -> Power {
    Power { id: id.to_string(), amount }
}

#[test]
fn strength_increases_player_damage() {
    let hand = vec![make_hand_card("BGStrike_R", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 10, 0, vec![])];
    let player_powers = vec![make_power("Strength", 2)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    state.apply(&play_action("BGStrike_R", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        // Strike base 1 + 2 Strength = 3 damage, 10 - 3 = 7
        assert_eq!(monsters[0].hp, 7);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn vulnerable_doubles_damage_and_ticks_down() {
    let hand = vec![make_hand_card("BGStrike_R", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 10, 0, vec![make_power("BGVulnerable", 2)])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGStrike_R", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        // Strike base 1 × 2 Vulnerable = 2 damage, 10 - 2 = 8
        assert_eq!(monsters[0].hp, 8);
        // Vulnerable ticked from 2 to 1
        assert_eq!(monsters[0].powers.iter().find(|p| p.id == "BGVulnerable").unwrap().amount, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn strength_plus_vulnerable() {
    let hand = vec![make_hand_card("BGStrike_R", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![make_power("BGVulnerable", 1)])];
    let player_powers = vec![make_power("Strength", 3)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    state.apply(&play_action("BGStrike_R", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        // (1 base + 3 Strength) × 2 Vulnerable = 8 damage, 20 - 8 = 12
        assert_eq!(monsters[0].hp, 12);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn weakened_reduces_damage_and_ticks_down() {
    let hand = vec![make_hand_card("BGStrike_R", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 10, 0, vec![])];
    let player_powers = vec![make_power("BGWeakened", 2)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    state.apply(&play_action("BGStrike_R", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, player_powers, .. } = state.current_screen() {
        // Strike base 1 - 1 Weak = 0 damage, hp unchanged
        assert_eq!(monsters[0].hp, 10);
        // Weakened ticked from 2 to 1
        assert_eq!(player_powers.iter().find(|p| p.id == "BGWeakened").unwrap().amount, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn weakened_and_vulnerable_cancel_and_both_tick_down() {
    let hand = vec![make_hand_card("BGStrike_R", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 10, 0, vec![make_power("BGVulnerable", 1)])];
    let player_powers = vec![make_power("BGWeakened", 1)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    state.apply(&play_action("BGStrike_R", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, player_powers, .. } = state.current_screen() {
        // Weak and Vulnerable cancel, base damage only: 10 - 1 = 9
        assert_eq!(monsters[0].hp, 9);
        // Both ticked from 1 to 0 and removed
        assert!(monsters[0].powers.iter().find(|p| p.id == "BGVulnerable").is_none());
        assert!(player_powers.iter().find(|p| p.id == "BGWeakened").is_none());
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn damage_fixed_ignores_strength_and_weak_and_vulnerable() {
    // Flame Barrier queues DamageFixed based on monster intent
    let hand = vec![make_hand_card("BGFlame Barrier", 2, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 10, 0, vec![make_power("BGVulnerable", 2)])];
    let player_powers = vec![make_power("Strength", 3), make_power("BGWeakened", 1)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    state.apply(&play_action("BGFlame Barrier", 2, "SKILL", 0, None));

    if let Screen::Combat { monsters, player_powers, .. } = state.current_screen() {
        // FlameBarrier does 1 DamageFixed per hit — ignores Strength, Weak, Vulnerable
        assert_eq!(monsters[0].hp, 9);
        // Vulnerable should not be ticked down
        assert_eq!(monsters[0].powers.iter().find(|p| p.id == "BGVulnerable").unwrap().amount, 2);
        // Weakened should not be ticked down
        assert_eq!(player_powers.iter().find(|p| p.id == "BGWeakened").unwrap().amount, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn aoe_attack_ticks_down_vulnerable_on_all_monsters() {
    // Cleave is DamageAll — should tick Vulnerable on every monster that had it
    let hand = vec![make_hand_card("BGCleave", 1, "ATTACK")];
    let monsters = vec![
        make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![make_power("BGVulnerable", 2)]),
        make_monster("BGGreenLouse", "Louse", 20, 0, vec![make_power("BGVulnerable", 1)]),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, vec![]);

    state.apply(&play_action("BGCleave", 1, "ATTACK", 0, None));

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        // Cleave base 2 × 2 Vulnerable = 4 damage each
        assert_eq!(monsters[0].hp, 16);
        assert_eq!(monsters[1].hp, 16);
        // Jaw Worm: 2 → 1
        assert_eq!(monsters[0].powers.iter().find(|p| p.id == "BGVulnerable").unwrap().amount, 1);
        // Louse: 1 → 0, removed
        assert!(monsters[1].powers.iter().find(|p| p.id == "BGVulnerable").is_none());
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn havoc_attack_with_vulnerable_and_weakened() {
    let hand = vec![make_hand_card("BGHavoc", 1, "SKILL")];
    let monsters = vec![
        make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![make_power("BGVulnerable", 2)]),
        make_monster("BGGreenLouse", "Louse", 20, 0, vec![make_power("BGVulnerable", 1)]),
    ];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [make_card("BGStrike_R", 1, "ATTACK")],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [{"id": "BGWeakened", "amount": 2}],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&play_action("BGHavoc", 1, "SKILL", 0, None));

    // Should push TargetSelect for the Strike
    assert!(matches!(state.current_screen(), Screen::TargetSelect { .. }));

    // Pick Jaw Worm (index 0)
    state.apply(&Action::PickTarget {
        reason: TargetReason::Card(make_card("BGStrike_R", 1, "ATTACK")),
        target_index: 0,
        target_name: "Jaw Worm".to_string(),
    });

    if let Screen::Combat { monsters, player_powers, .. } = state.current_screen() {
        // Weak + Vulnerable cancel: base damage 1, 20 - 1 = 19
        assert_eq!(monsters[0].hp, 19);
        // Jaw Worm Vulnerable ticked from 2 → 1 (targeted)
        assert_eq!(monsters[0].powers.iter().find(|p| p.id == "BGVulnerable").unwrap().amount, 1);
        // Louse Vulnerable unchanged at 1 (not targeted)
        assert_eq!(monsters[1].powers.iter().find(|p| p.id == "BGVulnerable").unwrap().amount, 1);
        // Player Weakened ticked from 2 → 1
        assert_eq!(player_powers.iter().find(|p| p.id == "BGWeakened").unwrap().amount, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

// ── Block cap and OnGainBlock triggers ──

#[test]
fn block_capped_at_20() {
    // BGDefend_R grants Block(1). Starting at 20, should stay at 20.
    let hand = vec![
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 10, 0, vec![])];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 20, vec![]);

    state.apply(&play_action("BGDefend_R", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, .. } = state.current_screen() {
        assert_eq!(*player_block, 20, "Block should cap at 20");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn juggernaut_deals_damage_on_block_gain() {
    let hand = vec![
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![
        make_monster("BGJawWorm", "Jaw Worm", 10, 0, vec![]),
        make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![]),
    ];
    let player_powers = vec![make_power("BGJuggernaut", 2)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    state.apply(&play_action("BGDefend_R", 1, "SKILL", 0, None));

    assert!(matches!(state.current_screen(), Screen::TargetSelect { .. }),
        "Expected TargetSelect screen for Juggernaut");

    state.apply(&Action::PickTarget {
        reason: TargetReason::Power(make_power("BGJuggernaut", 2)),
        target_index: 0,
        target_name: "Jaw Worm".to_string(),
    });

    if let Screen::Combat { monsters, player_block, .. } = state.current_screen() {
        assert_eq!(*player_block, 1, "Should have 1 block from Defend");
        assert_eq!(monsters[0].hp, 8, "Target should take 2 damage from Juggernaut");
        assert_eq!(monsters[1].hp, 8, "Non-target should be untouched");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn juggernaut_does_not_trigger_at_block_cap() {
    let hand = vec![
        make_hand_card("BGDefend_R", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 10, 0, vec![])];
    let player_powers = vec![make_power("BGJuggernaut", 2)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 20, player_powers);

    state.apply(&play_action("BGDefend_R", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, monsters, .. } = state.current_screen() {
        assert_eq!(*player_block, 20, "Block stays at cap");
        assert_eq!(monsters[0].hp, 10, "No Juggernaut damage");
    } else {
        panic!("Expected Combat screen, not TargetSelect");
    }
}

// ── Power modifiers ──

#[test]
fn barricade_prevents_block_decay() {
    let monsters = vec![make_monster("TestMonster", "Test", 10, 0, vec![])];
    let player_powers = vec![make_power("Barricade", 1)];
    let mut state = combat_state_with_monsters(vec![], monsters, 3, 5, player_powers);

    state.apply(&Action::EndTurn);

    if let Screen::Combat { player_block, .. } = state.current_screen() {
        assert_eq!(*player_block, 5, "Block should not decay with Barricade");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn no_draw_power_prevents_draws() {
    let hand = vec![
        make_hand_card("BGShrug It Off", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 10, 0, vec![])];
    let player_powers = vec![make_power("NoDrawPower", 1)];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [
                make_card("BGStrike_R", 1, "ATTACK"),
            ],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": player_powers,
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    // Shrug It Off: Block(2), Draw(1) — but NoDrawPower should prevent the draw
    state.apply(&play_action("BGShrug It Off", 1, "SKILL", 0, None));

    if let Screen::Combat { hand, draw_pile, .. } = state.current_screen() {
        assert!(hand.is_empty(), "Draw should be prevented by NoDrawPower");
        assert_eq!(draw_pile.len(), 1, "Card should remain in draw pile");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn no_draw_power_expires_at_end_of_turn() {
    let monsters = vec![make_monster("TestMonster", "Test", 10, 0, vec![])];
    let player_powers = vec![make_power("NoDrawPower", 1)];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": [],
            "draw_pile": [
                make_card("BGStrike_R", 1, "ATTACK"),
            ],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": player_powers,
            "die_roll": 1,
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&Action::EndTurn);

    if let Screen::Combat { player_powers, hand, .. } = state.current_screen() {
        // NoDrawPower should be removed at end of turn
        assert!(!player_powers.iter().any(|p| p.id == "NoDrawPower"),
            "NoDrawPower should be removed at end of turn");
        // Draw 5 happens after NoDrawPower removal — but NoDrawPower fires
        // at EndOfTurn (before discard), and draw happens at start of next turn.
        // Since NoDrawPower is removed during EndOfTurn triggers, the draw should work.
        assert_eq!(hand.len(), 1, "Should draw after NoDrawPower expires");
    } else {
        panic!("Expected Combat screen");
    }
}

// ── On-exhaust power triggers (BGBerserk) ──

#[test]
fn berserk_deals_damage_to_all_on_exhaust() {
    let hand = vec![
        make_hand_card("BGTrue Grit", 1, "SKILL"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
    ];
    let monsters = vec![
        make_monster("BGJawWorm", "Jaw Worm", 10, 0, vec![]),
        make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![]),
    ];
    let player_powers = vec![make_power("BGBerserk", 3)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    state.apply(&play_action("BGTrue Grit", 1, "SKILL", 0, None));

    if let Screen::Combat { monsters, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(exhaust_pile.len(), 1);
        assert_eq!(monsters[0].hp, 7, "First monster takes 3 damage from BGBerserk");
        assert_eq!(monsters[1].hp, 5, "Second monster takes 3 damage from BGBerserk");
    } else {
        panic!("Expected Combat screen");
    }
}

// ── Start-of-turn power triggers ──

#[test]
fn demon_form_gains_strength_at_start_of_turn() {
    let monsters = vec![make_monster("TestMonster", "Test", 20, 0, vec![])];
    let player_powers = vec![make_power("DemonForm", 1)];
    let mut state = combat_state_with_monsters(vec![], monsters, 3, 0, player_powers);

    state.apply(&Action::EndTurn);

    if let Screen::Combat { player_powers, .. } = state.current_screen() {
        let strength = player_powers.iter().find(|p| p.id == "Strength");
        assert!(strength.is_some(), "Should have Strength power");
        assert_eq!(strength.unwrap().amount, 1, "Should have 1 Strength from DemonForm");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn demon_form_stacks_strength_over_turns() {
    let monsters = vec![make_monster("TestMonster", "Test", 20, 0, vec![])];
    let player_powers = vec![make_power("DemonForm", 1)];
    let mut state = combat_state_with_monsters(vec![], monsters, 3, 0, player_powers);

    state.apply(&Action::EndTurn);
    state.apply(&Action::EndTurn);

    if let Screen::Combat { player_powers, .. } = state.current_screen() {
        let strength = player_powers.iter().find(|p| p.id == "Strength").unwrap();
        assert_eq!(strength.amount, 2, "Should have 2 Strength after 2 turns of DemonForm");
    } else {
        panic!("Expected Combat screen");
    }
}

// ── End-of-turn power triggers ──

#[test]
fn metallicize_grants_block_at_end_of_turn() {
    let monsters = vec![make_monster("TestMonster", "Test", 20, 0, vec![])];
    let player_powers = vec![
        make_power("Metallicize", 2),
        make_power("Barricade", 1),
    ];
    let mut state = combat_state_with_monsters(vec![], monsters, 3, 0, player_powers);

    state.apply(&Action::EndTurn);

    if let Screen::Combat { player_block, .. } = state.current_screen() {
        // Metallicize grants 2 block at end of turn. Barricade prevents
        // block from decaying at start of next turn, so we can verify it.
        assert_eq!(*player_block, 2, "Metallicize should grant 2 block");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn combust_deals_damage_at_end_of_turn() {
    let monsters = vec![
        make_monster("TestMonster", "Test", 10, 0, vec![]),
        make_monster("TestMonster", "Test", 8, 0, vec![]),
    ];
    let player_powers = vec![make_power("BGCombust", 3)];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": [],
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": player_powers,
            "die_roll": 1,
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&Action::EndTurn);

    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 7, "First monster takes 3 damage from BGCombust");
        assert_eq!(monsters[1].hp, 5, "Second monster takes 3 damage from BGCombust");
    } else {
        panic!("Expected Combat screen");
    }
}

// ── On-draw power triggers ──

#[test]
fn evolve_draws_on_status_draw() {
    let hand = vec![
        make_hand_card("BGShrug It Off", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let player_powers = vec![make_power("Evolve", 1)];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [
                make_card("BGDefend_R", 1, "SKILL"),
                make_card("BGDefend_R", 1, "SKILL"),
                make_card("Dazed", -2, "STATUS"),
            ],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": player_powers,
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    // Play Shrug It Off: Block(2), Draw(1)
    // Draw(1) → DrawOneCard → draws Dazed (Status) → Evolve triggers Draw(1) → draws BGDefend_R
    state.apply(&play_action("BGShrug It Off", 1, "SKILL", 0, None));

    if let Screen::Combat { hand, draw_pile, .. } = state.current_screen() {
        // Hand should have: Dazed + BGDefend_R (from Evolve trigger)
        assert_eq!(hand.len(), 2, "Expected 2 cards in hand: Dazed + Evolve draw");
        assert_eq!(hand[0].card.id, "Dazed");
        assert_eq!(hand[1].card.id, "BGDefend_R");
        assert_eq!(draw_pile.len(), 1, "One card should remain in draw pile");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn fire_breathing_damages_on_status_draw() {
    let hand = vec![
        make_hand_card("BGShrug It Off", 1, "SKILL"),
    ];
    let monsters = vec![
        make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![]),
        make_monster("BGJawWorm", "Jaw Worm", 6, 0, vec![]),
    ];
    let player_powers = vec![make_power("FireBreathing", 2)];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [
                make_card("Dazed", -2, "STATUS"),
            ],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": player_powers,
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    // Play Shrug It Off: Block(2), Draw(1) → draws Dazed → FireBreathing deals 2 to all
    state.apply(&play_action("BGShrug It Off", 1, "SKILL", 0, None));

    if let Screen::Combat { hand, monsters, .. } = state.current_screen() {
        assert_eq!(hand.len(), 1);
        assert_eq!(hand[0].card.id, "Dazed");
        assert_eq!(monsters[0].hp, 6, "First monster should take 2 damage");
        assert_eq!(monsters[1].hp, 4, "Second monster should take 2 damage");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn draw_triggers_shuffle_when_draw_pile_empty() {
    let hand = vec![
        make_hand_card("BGShrug It Off", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [],
            "discard_pile": [
                make_card("BGStrike_R", 1, "ATTACK"),
            ],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    // Play Shrug It Off: Block(2), Draw(1) → draw pile empty → shuffle discard → draw BGStrike_R
    state.apply(&play_action("BGShrug It Off", 1, "SKILL", 0, None));

    if let Screen::Combat { hand, draw_pile, discard_pile, .. } = state.current_screen() {
        assert_eq!(hand.len(), 1, "Should have drawn 1 card after shuffle");
        assert_eq!(hand[0].card.id, "BGStrike_R");
        assert!(draw_pile.is_empty());
        // Discard has the played Shrug It Off
        assert_eq!(discard_pile.len(), 1);
        assert_eq!(discard_pile[0].id, "BGShrug It Off");
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn evolve_and_fire_breathing_both_trigger_on_status() {
    let hand = vec![
        make_hand_card("BGShrug It Off", 1, "SKILL"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 10, 0, vec![])];
    let player_powers = vec![
        make_power("Evolve", 1),
        make_power("FireBreathing", 1),
    ];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [
                make_card("BGDefend_R", 1, "SKILL"),
                make_card("Dazed", -2, "STATUS"),
            ],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": player_powers,
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    // Draw(1) → Dazed → Evolve draws 1 + FireBreathing deals 1 to all
    state.apply(&play_action("BGShrug It Off", 1, "SKILL", 0, None));

    if let Screen::Combat { hand, monsters, .. } = state.current_screen() {
        // Hand: Dazed + BGDefend_R (from Evolve)
        assert_eq!(hand.len(), 2);
        // FireBreathing dealt 1 damage
        assert_eq!(monsters[0].hp, 9);
    } else {
        panic!("Expected Combat screen");
    }
}

// ── On-exhaust power triggers ──

#[test]
fn feel_no_pain_gains_block_on_exhaust() {
    // True Grit exhausts a card from hand
    let hand = vec![
        make_hand_card("BGTrue Grit", 1, "SKILL"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let player_powers = vec![make_power("FeelNoPain", 2)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    // Play True Grit (Block 3 + exhaust a card from hand)
    state.apply(&play_action("BGTrue Grit", 1, "SKILL", 0, None));

    // Auto-resolves since only 1 card left in hand
    if let Screen::Combat { player_block, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(exhaust_pile.len(), 1);
        // Block = 1 (True Grit base) + 2 (FeelNoPain)
        assert_eq!(*player_block, 3);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn dark_embrace_draws_on_exhaust() {
    let hand = vec![
        make_hand_card("BGTrue Grit", 1, "SKILL"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let player_powers = vec![make_power("BGDarkEmbrace", 1)];

    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": hand,
            "draw_pile": [
                make_card("BGDefend_R", 1, "SKILL"),
                make_card("BGDefend_R", 1, "SKILL"),
            ],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": player_powers,
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    // Play True Grit (auto-exhausts the only other card)
    state.apply(&play_action("BGTrue Grit", 1, "SKILL", 0, None));

    if let Screen::Combat { hand, exhaust_pile, draw_pile, .. } = state.current_screen() {
        assert_eq!(exhaust_pile.len(), 1);
        // Drew 1 card from DarkEmbrace
        assert_eq!(hand.len(), 1);
        assert_eq!(draw_pile.len(), 1);
    } else {
        panic!("Expected Combat screen");
    }
}

// ── RepeatAttack (BGDoubleAttack) ──

#[test]
fn double_attack_repeats_strike_damage() {
    let hand = vec![make_hand_card("BGStrike_R", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let player_powers = vec![make_power("BGDoubleAttack", 1)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    state.apply(&play_action("BGStrike_R", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, player_powers, .. } = state.current_screen() {
        // Strike deals 1 damage, doubled = 2 total damage: 8 - 2 = 6
        assert_eq!(monsters[0].hp, 6);
        // Power should be consumed
        assert!(player_powers.iter().all(|p| p.id != "BGDoubleAttack" || p.amount == 0));
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn double_attack_does_not_repeat_skills() {
    let hand = vec![make_hand_card("BGDefend_R", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let player_powers = vec![make_power("BGDoubleAttack", 1)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    state.apply(&play_action("BGDefend_R", 1, "SKILL", 0, None));

    if let Screen::Combat { player_block, player_powers, .. } = state.current_screen() {
        // Defend gives 1 block, should NOT be doubled
        assert_eq!(*player_block, 1);
        // Power should still be active
        assert!(player_powers.iter().any(|p| p.id == "BGDoubleAttack" && p.amount == 1));
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn double_attack_stacks_repeat_multiple_attacks() {
    let hand = vec![
        make_hand_card("BGStrike_R", 1, "ATTACK"),
        make_hand_card("BGStrike_R", 1, "ATTACK"),
    ];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let player_powers = vec![make_power("BGDoubleAttack", 2)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    // Play first strike: doubled, 2 damage total
    state.apply(&play_action("BGStrike_R", 1, "ATTACK", 0, Some(0)));
    // Play second strike: doubled, 2 damage total
    state.apply(&play_action("BGStrike_R", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, player_powers, .. } = state.current_screen() {
        // 8 - 2 - 2 = 4
        assert_eq!(monsters[0].hp, 4);
        assert!(player_powers.iter().all(|p| p.id != "BGDoubleAttack" || p.amount == 0));
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn double_attack_expires_at_end_of_turn() {
    let hand = vec![make_hand_card("BGDefend_R", 1, "SKILL")];
    let monsters = vec![make_monster("TestMonster", "Test", 8, 0, vec![])];
    let player_powers = vec![make_power("BGDoubleAttack", 1)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    // Don't play any attacks, just end the turn
    state.apply(&Action::EndTurn);

    if let Screen::Combat { player_powers, .. } = state.current_screen() {
        assert!(
            player_powers.iter().all(|p| p.id != "BGDoubleAttack"),
            "BGDoubleAttack should be removed at end of turn"
        );
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn double_attack_with_whirlwind_doubles_xcost_effects() {
    let hand = vec![make_hand_card("BGWhirlwind", -1, "ATTACK")];
    let monsters = vec![
        make_monster("BGJawWorm", "Jaw Worm", 20, 0, vec![]),
        make_monster("BGGreenLouse", "Louse", 10, 0, vec![]),
    ];
    let player_powers = vec![make_power("BGDoubleAttack", 1)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    state.apply(&play_action("BGWhirlwind", -1, "ATTACK", 0, None));
    // Spend 2 energy: 2 hits of DamageAll(1), doubled by RepeatAttack = 4 hits
    state.apply(&Action::PickChoice { label: "Spend 2".to_string(), choice_index: 2 });

    if let Screen::Combat { monsters, player_energy, player_powers, .. } = state.current_screen() {
        // Energy deducted once: 3 - 2 = 1
        assert_eq!(*player_energy, 1);
        // Each monster takes 4 damage (2 hits x 2 from RepeatAttack)
        assert_eq!(monsters[0].hp, 16); // 20 - 4
        assert_eq!(monsters[1].hp, 6);  // 10 - 4
        // Power consumed
        assert!(player_powers.iter().all(|p| p.id != "BGDoubleAttack" || p.amount == 0));
    } else {
        panic!("Expected Combat screen");
    }
}

// ── Corruption (SkillsCostZero + ForceExhaust) ──

#[test]
fn corruption_makes_skill_cost_zero() {
    // Shrug It Off costs 1, but with Corruption it should cost 0
    let hand = vec![make_hand_card("BGShrug It Off", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let player_powers = vec![make_power("BGCorruption", 1)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    state.apply(&play_action("BGShrug It Off", 1, "SKILL", 0, None));

    if let Screen::Combat { player_energy, .. } = state.current_screen() {
        // No energy deducted
        assert_eq!(*player_energy, 3);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn corruption_exhausts_skill() {
    let hand = vec![make_hand_card("BGShrug It Off", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let player_powers = vec![make_power("BGCorruption", 1)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    state.apply(&play_action("BGShrug It Off", 1, "SKILL", 0, None));

    if let Screen::Combat { exhaust_pile, discard_pile, .. } = state.current_screen() {
        assert_eq!(exhaust_pile.len(), 1);
        assert_eq!(discard_pile.len(), 0);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn corruption_does_not_affect_attacks() {
    let hand = vec![make_hand_card("BGStrike_R", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let player_powers = vec![make_power("BGCorruption", 1)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0, player_powers);

    state.apply(&play_action("BGStrike_R", 1, "ATTACK", 0, Some(0)));

    if let Screen::Combat { player_energy, discard_pile, exhaust_pile, .. } = state.current_screen() {
        // Attack still costs 1 energy
        assert_eq!(*player_energy, 2);
        // Attack goes to discard, not exhaust
        assert_eq!(discard_pile.len(), 1);
        assert_eq!(exhaust_pile.len(), 0);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn corruption_makes_expensive_skill_playable() {
    // Shrug It Off costs 1, player has 0 energy — with Corruption it's playable
    let hand = vec![make_hand_card("BGShrug It Off", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    let player_powers = vec![make_power("BGCorruption", 1)];
    let state = combat_state_with_monsters(hand, monsters, 0, 0, player_powers);

    let actions = state.available_actions();
    let has_play = actions.iter().any(|a| matches!(a, Action::PlayCard { .. }));
    assert!(has_play, "Skill should be playable with 0 energy under Corruption");
}

// ── Monster turns ──

#[test]
fn monster_deals_damage_to_player() {
    // Spike Slime S: Tackle = 1 damage
    let monsters = vec![make_monster("BGSpikeSlime_S", "Spike Slime", 3, 0, vec![])];
    let mut state = combat_state_with_monsters(vec![], monsters, 3, 0, vec![]);

    state.apply(&Action::EndTurn);

    assert_eq!(state.hp, 9); // 10 - 1
}

#[test]
fn player_block_absorbs_monster_damage() {
    let monsters = vec![make_monster("BGSpikeSlime_S", "Spike Slime", 3, 0, vec![])];
    let mut state = combat_state_with_monsters(vec![], monsters, 3, 5, vec![]);

    state.apply(&Action::EndTurn);

    // Block absorbs the 1 damage, HP unchanged
    assert_eq!(state.hp, 10);
    if let Screen::Combat { player_block, .. } = state.current_screen() {
        // Block decayed to 0 at start of next turn (no Barricade)
        assert_eq!(*player_block, 0);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn player_defeat_on_zero_hp() {
    // Jaw Worm Chomp = 3 damage, player has 2 HP
    let mut monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0, vec![])];
    monsters[0].move_index = 0; // Chomp
    let json = serde_json::json!({
        "hp": 2, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": monsters,
            "hand": [],
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "die_roll": 1,
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&Action::EndTurn);

    assert!(matches!(state.current_screen(), Screen::GameOver { victory: false }));
}

#[test]
fn jaw_worm_die_roll_selects_bellow() {
    // Die roll 1-2 → Bellow (move index 2): block + Strength
    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": [{
                "id": "BGJawWorm", "name": "Jaw Worm", "hp": 8, "max_hp": 8,
                "block": 0, "intent": "DEFEND_BUFF", "damage": null, "hits": 1,
                "powers": [], "is_gone": false, "move_index": 2
            }],
            "hand": [],
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "die_roll": 2,
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    state.apply(&Action::EndTurn);

    // Bellow: no damage to player, monster gained Strength
    assert_eq!(state.hp, 10);
    if let Screen::Combat { monsters, .. } = state.current_screen() {
        let str_amount = monsters[0].powers.iter()
            .find(|p| p.id == "Strength").map(|p| p.amount).unwrap_or(0);
        assert_eq!(str_amount, 1);
    } else {
        panic!("Expected Combat screen");
    }
}

#[test]
fn cultist_first_turn_incantation_then_dark_strike() {
    let json = serde_json::json!({
        "hp": 10, "max_hp": 10, "gold": 0, "floor": 1, "act": 1, "ascension": 0,
        "deck": [],
        "relics": [{"id": "BoardGame:BurningBlood", "name": "Burning Blood"}],
        "potions": [null, null, null],
        "screen": {
            "type": "combat",
            "encounter": "test",
            "monsters": [{
                "id": "BGCultist", "name": "Cultist", "hp": 9, "max_hp": 9,
                "block": 0, "intent": "ATTACK_BUFF", "damage": 1, "hits": 1,
                "powers": [],
                "is_gone": false, "move_index": 0
            }],
            "hand": [],
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "player_block": 0,
            "player_energy": 3,
            "player_powers": [],
            "die_roll": 1,
            "turn": 1
        }
    });
    let mut state = GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap();

    // Apply pre-battle effects (Ritual)
    state.apply_monster_starting_effects();
    if let Screen::Combat { monsters, .. } = state.current_screen() {
        assert!(monsters[0].powers.iter().any(|p| p.id == "Ritual" && p.amount == 1));
    }

    // Turn 1: Incantation (1 dmg). Ritual fires at MonsterEndOfTurn → +1 Str.
    state.apply(&Action::EndTurn);
    assert_eq!(state.hp, 9); // 10 - 1
    if let Screen::Combat { monsters, .. } = state.current_screen() {
        let str_amount = monsters[0].powers.iter()
            .find(|p| p.id == "Strength").map(|p| p.amount).unwrap_or(0);
        assert_eq!(str_amount, 1);
        assert_eq!(monsters[0].move_index, 1);
    } else {
        panic!("Expected Combat screen");
    }

    // Turn 2: Dark Strike (1 base + 1 Str = 2 dmg). Ritual fires → +1 Str.
    state.apply(&Action::EndTurn);
    assert_eq!(state.hp, 7); // 9 - 2
    if let Screen::Combat { monsters, .. } = state.current_screen() {
        let str_amount = monsters[0].powers.iter()
            .find(|p| p.id == "Strength").map(|p| p.amount).unwrap_or(0);
        assert_eq!(str_amount, 2);
    } else {
        panic!("Expected Combat screen");
    }
}
