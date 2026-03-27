use sts_simulator::{Action, Card, GameState, HandCard, Monster, Screen};

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

fn make_monster(id: &str, name: &str, hp: u16, block: u16) -> Monster {
    Monster {
        id: id.to_string(),
        name: name.to_string(),
        hp,
        max_hp: hp,
        block,
        intent: "ATTACK".to_string(),
        damage: Some(1),
        hits: 1,
        powers: vec![],
        is_gone: false,
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
            "player_powers": [],
            "turn": 1
        }
    });
    GameState::from_json(&serde_json::to_string(&json).unwrap()).unwrap()
}

// ── Damage ──

#[test]
fn strike_deals_damage() {
    let hand = vec![make_hand_card("BGStrike_R", 1, "ATTACK")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 3)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 5, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

    state.apply(&play_action("BGBludgeon", 3, "ATTACK", 0, Some(0)));

    // Killing the last monster should transition to combat rewards
    assert!(matches!(state.current_screen(), Screen::CombatRewards { .. }),
        "Expected CombatRewards, got {:?}", state.current_screen());
}

#[test]
fn damage_kills_one_of_two_monsters_stays_in_combat() {
    let hand = vec![make_hand_card("BGBludgeon", 3, "ATTACK")];
    let monsters = vec![
        make_monster("BGJawWorm", "Jaw Worm", 5, 0),
        make_monster("BGGreenLouse", "Louse", 5, 0),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
        make_monster("BGGreenLouse", "Louse A", 5, 0),
        make_monster("BGRedLouse", "Louse B", 5, 0),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let mut dead = make_monster("BGRedLouse", "Dead Louse", 0, 0);
    dead.is_gone = true;
    let monsters = vec![
        make_monster("BGGreenLouse", "Louse A", 5, 0),
        dead,
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 2);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
        make_monster("BGGreenLouse", "Louse A", 5, 0),
        make_monster("BGRedLouse", "Louse B", 5, 0),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 6, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
        make_monster("BGGreenLouse", "Louse A", 5, 0),
        make_monster("BGRedLouse", "Louse B", 5, 0),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 4);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 5);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

    state.apply(&play_action("BGFiend Fire", 2, "ATTACK", 0, Some(0)));

    if let Screen::Combat { monsters, exhaust_pile, .. } = state.current_screen() {
        assert_eq!(monsters[0].hp, 8); // no damage
        assert_eq!(exhaust_pile.len(), 1); // just Fiend Fire itself
    } else {
        panic!("Expected Combat screen");
    }
}

// ── ConditionalOnDieRoll ──

#[test]
fn spot_weakness_gains_strength_on_low_roll() {
    let hand = vec![make_hand_card("BGSpot Weakness", 1, "SKILL")];
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
        make_monster("BGJawWorm", "Jaw Worm", 3, 0),
        make_monster("BGGreenLouse", "Louse", 5, 0),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 10, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 2, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

    state.apply(&play_action("BGWhirlwind", -1, "ATTACK", 0, None));

    // Should present 4 choices: Spend 0, 1, 2, 3
    if let Screen::ChoiceSelect { choices, .. } = state.current_screen() {
        assert_eq!(choices.len(), 4);
        assert_eq!(choices[0].0, "Spend 0");
        assert_eq!(choices[3].0, "Spend 3");
    } else {
        panic!("Expected ChoiceSelect, got {:?}", state.current_screen());
    }
}

#[test]
fn whirlwind_spend_two_damages_all_twice() {
    let hand = vec![make_hand_card("BGWhirlwind", -1, "ATTACK")];
    let monsters = vec![
        make_monster("BGJawWorm", "Jaw Worm", 20, 0),
        make_monster("BGGreenLouse", "Louse", 10, 0),
    ];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 20, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 2, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];
    let mut state = combat_state_with_monsters(hand, monsters, 3, 0);

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
    let monsters = vec![make_monster("BGJawWorm", "Jaw Worm", 8, 0)];

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
