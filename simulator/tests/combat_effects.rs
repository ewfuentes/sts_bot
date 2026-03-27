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
