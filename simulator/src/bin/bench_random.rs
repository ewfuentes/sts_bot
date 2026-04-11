use std::time::Instant;

use rayon::prelude::*;
use sts_simulator::{GameState, Rng, Screen};

fn make_combat(_seed: u64, encounter_id: &str) -> GameState {
    let json = serde_json::json!({
        "hp": 8, "max_hp": 8, "gold": 5, "floor": 1, "act": 1, "ascension": 0,
        "deck": [
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGStrike_R", "name": "Strike", "cost": 1, "type": "ATTACK", "upgraded": false},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": false},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": false},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": false},
            {"id": "BGDefend_R", "name": "Defend", "cost": 1, "type": "SKILL", "upgraded": false},
            {"id": "BGBash", "name": "Bash", "cost": 2, "type": "ATTACK", "upgraded": false},
        ],
        "relics": [
            {"id": "BoardGame:BurningBlood", "name": "Burning Blood", "counter": -1},
        ],
        "potions": [null, null],
        "actions": [],
        "screen": {
            "type": "combat",
            "encounter": encounter_id,
        }
    });

    let mut state = GameState::from_json(&json.to_string()).unwrap();

    // Populate monsters from encounter_db
    if let Some(enc) = sts_simulator::encounter_db::lookup(encounter_id) {
        if let Screen::Combat { monsters, .. } = state.current_screen_mut() {
            for em in enc.monsters {
                monsters.push(sts_simulator::Monster {
                    id: em.id.to_string(),
                    name: em.id.to_string(),
                    hp: em.hp,
                    max_hp: em.hp,
                    block: 0,
                    intent: "UNKNOWN".to_string(),
                    damage: None,
                    hits: 1,
                    powers: vec![],
                    state: sts_simulator::MonsterState::Alive,
                    move_index: em.move_index,
                    pattern: sts_simulator::monster_db::MovePattern::default(),
                });
            }
        }
    }

    // start_combat handles shuffling, die roll, intent resolution, and drawing
    state.start_combat();
    state
}

struct GameResult {
    victory: bool,
    steps: u32,
    floor: u8,
    hp: u16,
    max_screen: String,
}

fn play_random_game(seed: u64) -> GameResult {
    let mut state = GameState::new_ironclad_game(seed);
    let mut rng = Rng::from_seed(seed.wrapping_add(1000000));
    let mut steps: u32 = 0;
    let mut max_screen = String::new();

    loop {
        match state.current_screen() {
            Screen::GameOver { victory } => return GameResult {
                victory: *victory, steps, floor: state.floor, hp: state.hp, max_screen,
            },
            Screen::Complete => return GameResult {
                victory: false, steps, floor: state.floor, hp: state.hp, max_screen,
            },
            _ => {}
        }

        let actions = state.available_actions();
        if actions.is_empty() {
            return GameResult {
                victory: false, steps, floor: state.floor, hp: state.hp, max_screen,
            };
        }

        // Track what screens we visit
        let screen_name = match state.current_screen() {
            Screen::Combat { encounter, .. } => format!("Combat({})", encounter),
            Screen::Map { .. } => "Map".into(),
            Screen::CardReward { .. } => "CardReward".into(),
            Screen::CombatRewards { .. } => "CombatRewards".into(),
            Screen::Shop { .. } => "Shop".into(),
            Screen::Rest { .. } => "Rest".into(),
            Screen::Event { event_id, .. } => format!("Event({})", event_id),
            Screen::Treasure => "Treasure".into(),
            Screen::BossRelic { .. } => "BossRelic".into(),
            Screen::Grid { .. } => "Grid".into(),
            Screen::HandSelect { .. } => "HandSelect".into(),
            Screen::TargetSelect { .. } => "TargetSelect".into(),
            _ => String::new(),
        };
        if !screen_name.is_empty() {
            max_screen = screen_name;
        }

        let idx = rng.roll_die(actions.len() as u8) as usize - 1;
        state.apply(&actions[idx]);
        steps += 1;

        // Safety valve
        if steps > 50_000 {
            return GameResult {
                victory: false, steps, floor: state.floor, hp: state.hp, max_screen,
            };
        }
    }
}

fn play_random_combat(seed: u64, encounter_id: &str) -> GameResult {
    let mut state = make_combat(seed, encounter_id);
    let mut rng = Rng::from_seed(seed.wrapping_add(1000000));
    let mut steps: u32 = 0;

    loop {
        match state.current_screen() {
            Screen::GameOver { victory } => return GameResult {
                victory: *victory, steps, floor: state.floor, hp: state.hp, max_screen: String::new(),
            },
            Screen::Complete => return GameResult {
                // Combat won — we didn't die
                victory: true, steps, floor: state.floor, hp: state.hp, max_screen: String::new(),
            },
            _ => {}
        }

        let actions = state.available_actions();
        if actions.is_empty() {
            return GameResult {
                victory: false, steps, floor: state.floor, hp: state.hp, max_screen: String::new(),
            };
        }

        let idx = rng.roll_die(actions.len() as u8) as usize - 1;
        state.apply(&actions[idx]);
        steps += 1;

        if steps > 50_000 {
            return GameResult {
                victory: false, steps, floor: state.floor, hp: state.hp, max_screen: "timeout".into(),
            };
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("run");

    match mode {
        "combat" => {
            let encounter = args.get(2).map(|s| s.as_str()).unwrap_or("BoardGame:Jaw Worm (Easy)");
            let num_games: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(10000);
            bench_combat(encounter, num_games);
        }
        _ => {
            let num_games: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1000);
            bench_run(num_games);
        }
    }
}

fn bench_combat(encounter: &str, num_games: u64) {
    println!("Playing {} random combats vs {}...", num_games, encounter);

    let start = Instant::now();
    let results: Vec<_> = (0..num_games)
        .into_par_iter()
        .map(|seed| {
            let enc = encounter.to_string();
            std::panic::catch_unwind(move || play_random_combat(seed, &enc))
        })
        .collect();

    let mut wins = 0u64;
    let mut total_steps = 0u64;
    let mut errors = 0u64;
    let mut hp_counts: std::collections::HashMap<u16, u64> = std::collections::HashMap::new();
    for result in &results {
        match result {
            Ok(gr) => {
                if gr.victory { wins += 1; }
                total_steps += gr.steps as u64;
                *hp_counts.entry(gr.hp).or_default() += 1;
            }
            Err(_) => { errors += 1; }
        }
    }

    let completed = num_games - errors;
    let elapsed = start.elapsed();
    let games_per_sec = num_games as f64 / elapsed.as_secs_f64();
    let steps_per_sec = total_steps as f64 / elapsed.as_secs_f64();

    println!("Completed in {:.2?}", elapsed);
    println!("{:.1} combats/sec", games_per_sec);
    println!("{:.0} steps/sec", steps_per_sec);
    println!(
        "Wins: {}/{} ({:.1}%)",
        wins, completed,
        if completed > 0 { wins as f64 / completed as f64 * 100.0 } else { 0.0 }
    );
    println!("Avg steps/combat: {:.0}", total_steps as f64 / completed.max(1) as f64);
    if errors > 0 {
        println!("Panics: {} ({:.1}%)", errors, errors as f64 / num_games as f64 * 100.0);
    }

    println!("\n--- HP at end of combat ---");
    let mut hps: Vec<_> = hp_counts.into_iter().collect();
    hps.sort_by_key(|(hp, _)| *hp);
    for (hp, count) in &hps {
        println!("  HP {:>2}: {:>5} ({:>5.1}%)", hp, count, *count as f64 / completed as f64 * 100.0);
    }
}

fn bench_run(num_games: u64) {
    println!("Playing {} random games...", num_games);

    let start = Instant::now();
    let results: Vec<_> = (0..num_games)
        .into_par_iter()
        .map(|seed| std::panic::catch_unwind(|| play_random_game(seed)))
        .collect();

    let mut wins = 0u64;
    let mut total_steps = 0u64;
    let mut errors = 0u64;
    let mut floor_counts: std::collections::HashMap<u8, u64> = std::collections::HashMap::new();
    let mut last_screen_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for result in results {
        match result {
            Ok(gr) => {
                if gr.victory { wins += 1; }
                total_steps += gr.steps as u64;
                *floor_counts.entry(gr.floor).or_default() += 1;
                *last_screen_counts.entry(gr.max_screen).or_default() += 1;
            }
            Err(_) => { errors += 1; }
        }
    }

    let completed = num_games - errors;
    let elapsed = start.elapsed();
    let games_per_sec = num_games as f64 / elapsed.as_secs_f64();
    let steps_per_sec = total_steps as f64 / elapsed.as_secs_f64();

    println!("Completed in {:.2?}", elapsed);
    println!("{:.1} games/sec", games_per_sec);
    println!("{:.0} steps/sec", steps_per_sec);
    println!(
        "Wins: {}/{} ({:.1}%)",
        wins, completed,
        if completed > 0 { wins as f64 / completed as f64 * 100.0 } else { 0.0 }
    );
    println!("Avg steps/game: {:.0}", total_steps as f64 / completed.max(1) as f64);
    if errors > 0 {
        println!("Panics: {} ({:.1}%)", errors, errors as f64 / num_games as f64 * 100.0);
    }

    println!("\n--- Floor distribution ---");
    let mut floors: Vec<_> = floor_counts.into_iter().collect();
    floors.sort_by_key(|(f, _)| *f);
    for (floor, count) in &floors {
        println!("  Floor {:>2}: {:>5} ({:>5.1}%)", floor, count, *count as f64 / completed as f64 * 100.0);
    }

    println!("\n--- Last screen before death/end ---");
    let mut screens: Vec<_> = last_screen_counts.into_iter().collect();
    screens.sort_by(|a, b| b.1.cmp(&a.1));
    for (screen, count) in screens.iter().take(20) {
        println!("  {:>5} ({:>5.1}%)  {}", count, *count as f64 / completed as f64 * 100.0, screen);
    }
}
