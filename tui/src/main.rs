use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use sts_simulator::{Action, GameState, MapChoice, MapNodeKind, Screen};

mod model_eval;

struct App {
    state: GameState,
    actions: Vec<Action>,
    selected: usize,
    log: Vec<String>,
    /// Which screen in the stack to display (0 = bottom, len-1 = top)
    view_screen: usize,
    evaluator: Option<model_eval::ModelEvaluator>,
    action_values: Vec<(f64, f64)>,
}

impl App {
    fn new(model_path: Option<&str>) -> Self {
        let state = make_initial_state();
        let actions = state.available_actions();
        let view_screen = state.screen.len().saturating_sub(1);

        let evaluator = model_path.map(|path| {
            eprintln!("Loading model from {}...", path);
            model_eval::ModelEvaluator::load(path).expect("Failed to load model")
        });

        let mut app = App {
            state,
            actions,
            selected: 0,
            log: vec!["Run started. Determinized with seed 42.".into()],
            view_screen,
            evaluator,
            action_values: vec![],
        };
        app.update_action_values();
        app
    }

    fn update_action_values(&mut self) {
        self.action_values = if let Some(eval) = &self.evaluator {
            let mut states: Vec<GameState> = Vec::with_capacity(self.actions.len());
            for action in &self.actions {
                let mut s = self.state.clone();
                s.apply(action);
                states.push(s);
            }
            if states.is_empty() {
                vec![]
            } else {
                eval.evaluate_batch(&states)
            }
        } else {
            vec![]
        };
    }

    fn select_action(&mut self) {
        if self.actions.is_empty() {
            return;
        }
        let action = self.actions[self.selected].clone();
        self.log
            .push(format!("> {}", format_action(&action, &self.state)));
        self.state.apply(&action);

        // If we've popped back to Complete with only the map underneath,
        // pop Complete to reveal the Map screen (with updated available_nodes)
        if matches!(self.state.current_screen(), Screen::Complete) && self.state.screen.len() > 1 {
            self.state.pop_screen();
        }

        self.actions = self.state.available_actions();
        self.selected = 0;
        self.view_screen = self.state.screen.len().saturating_sub(1);
        self.update_action_values();
    }
}

fn make_initial_state() -> GameState {
    let mut rng = sts_simulator::Rng::from_seed(42);
    let (map, start_node) = sts_simulator::dungeon::generate_act1_map(&mut rng);

    // Build the initial map choices — the starting node is the only choice
    let start = &map.nodes[start_node];
    let initial_choices = vec![MapChoice {
        label: format!("{:?} ({},{})", start.kind, start.x, start.y),
        kind: start.kind,
        node_index: start_node,
    }];

    let json = serde_json::json!({
        "hp": 8, "max_hp": 8, "gold": 5, "floor": 0, "act": 1, "ascension": 0,
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
            "type": "map",
            "available_nodes": initial_choices.iter().map(|c| serde_json::json!({
                "label": c.label,
                "kind": c.kind,
                "node_index": c.node_index,
            })).collect::<Vec<_>>(),
        }
    });
    let mut state = GameState::from_json(&json.to_string()).unwrap();
    state.map = Some(map);
    state.determinize(42);
    state
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let model_path = args.iter()
        .position(|a| a == "--model")
        .and_then(|i| args.get(i + 1).cloned());

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, model_path.as_deref());
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, model_path: Option<&str>) -> io::Result<()> {
    let mut app = App::new(model_path);

    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.selected > 0 {
                        app.selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.selected + 1 < app.actions.len() {
                        app.selected += 1;
                    }
                }
                KeyCode::Tab => {
                    if !app.state.screen.is_empty() {
                        app.view_screen = (app.view_screen + 1) % app.state.screen.len();
                    }
                }
                KeyCode::BackTab => {
                    if !app.state.screen.is_empty() {
                        app.view_screen = if app.view_screen == 0 {
                            app.state.screen.len() - 1
                        } else {
                            app.view_screen - 1
                        };
                    }
                }
                KeyCode::Enter => app.select_action(),
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw(frame: &mut Frame, app: &App) {
    let outer =
        Layout::vertical([Constraint::Min(5), Constraint::Length(12)]).split(frame.area());

    let top = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[0]);

    // Left panel: game state
    let status = build_status(&app.state, app.view_screen);
    let status_block = Paragraph::new(status)
        .wrap(ratatui::widgets::Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title("Game State (Tab to switch)"));
    frame.render_widget(status_block, top[0]);

    // Right panel: actions
    let items: Vec<ListItem> = app
        .actions
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let style = if i == app.selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if i == app.selected { "▸ " } else { "  " };
            let action_str = format_action(action, &app.state);

            let label = if let Some(&(mean, log_var)) = app.action_values.get(i) {
                let std = (log_var / 2.0).exp();
                format!("{}{} [{:.2} ± {:.2}]", prefix, action_str, mean, std)
            } else {
                format!("{}{}", prefix, action_str)
            };

            ListItem::new(label).style(style)
        })
        .collect();

    let actions_list =
        List::new(items).block(Block::default().borders(Borders::ALL).title("Actions"));
    frame.render_widget(actions_list, top[1]);

    // Bottom panel: log
    let log_items: Vec<ListItem> = app
        .log
        .iter()
        .rev()
        .take(10)
        .rev()
        .map(|s| ListItem::new(s.as_str()))
        .collect();
    let log_list = List::new(log_items).block(Block::default().borders(Borders::ALL).title("Log"));
    frame.render_widget(log_list, outer[1]);
}

fn build_status(state: &GameState, view_screen: usize) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(vec![
            Span::raw("HP: "),
            Span::styled(
                format!("{}/{}", state.hp, state.max_hp),
                Style::default().fg(Color::Red),
            ),
            Span::raw(format!(
                "  Gold: {}  Floor: {}  Act: {}",
                state.gold, state.floor, state.act
            )),
        ]),
        Line::from(""),
    ];

    // Screen stack indicator
    let stack_line: Vec<Span> = state.screen.iter().enumerate().map(|(i, s)| {
        let name = format_screen_name(s);
        if i == view_screen {
            Span::styled(format!("[{}]", name), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(format!(" {} ", name), Style::default().fg(Color::DarkGray))
        }
    }).collect();
    lines.push(Line::from(stack_line));
    lines.push(Line::from(""));

    // Show details for the viewed screen
    let viewed = state.screen.get(view_screen);
    if let Some(Screen::Map { current_node, .. }) = viewed {
        // Map visualization
        if let Some(map) = &state.map {
            let current = *current_node;
            for row in (0..13).rev() {
                let mut row_str = format!("{:>2} ", row);
                for col in 0..7 {
                    let idx = row * 7 + col;
                    let node = &map.nodes[idx];
                    let ch = match node.kind {
                        MapNodeKind::Monster => 'M',
                        MapNodeKind::Elite => 'E',
                        MapNodeKind::Boss => 'B',
                        MapNodeKind::Rest => 'R',
                        MapNodeKind::Shop => '$',
                        MapNodeKind::Event => '?',
                        MapNodeKind::Treasure => 'T',
                        MapNodeKind::Unknown => '.',
                    };
                    if idx == current {
                        row_str.push_str(&format!("[{}]", ch));
                    } else {
                        row_str.push_str(&format!(" {} ", ch));
                    }
                }
                lines.push(Line::from(row_str));
            }
        }
    } else if let Some(combat) = find_combat(&state.screen) {
        if let Screen::Combat {
            monsters,
            hand,
            draw_pile,
            discard_pile,
            exhaust_pile,
            player_block,
            player_energy,
            player_powers,
            turn,
            die_roll,
            ..
        } = combat
        {
            lines.push(Line::from(vec![
                Span::raw("Energy: "),
                Span::styled(
                    format!("{}", player_energy),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(format!("  Block: {}", player_block)),
                Span::raw(format!("  Turn: {}", turn)),
                Span::raw(format!(
                    "  Die: {}",
                    die_roll
                        .map(|d| d.to_string())
                        .unwrap_or_else(|| "-".into())
                )),
            ]));

            if !player_powers.is_empty() {
                let powers_str: Vec<String> = player_powers
                    .iter()
                    .map(|p| format!("{}({})", p.id, p.amount))
                    .collect();
                lines.push(Line::from(format!("Powers: {}", powers_str.join(", "))));
            }

            lines.push(Line::from(""));

            // Monsters
            for m in monsters {
                if m.state != sts_simulator::MonsterState::Alive {
                    continue;
                }
                let mut parts: Vec<Span> = vec![
                    Span::styled(
                        format!("{}", m.name),
                        Style::default().fg(Color::Magenta),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("{}/{}", m.hp, m.max_hp),
                        Style::default().fg(Color::Red),
                    ),
                ];
                if m.block > 0 {
                    parts.push(Span::styled(
                        format!(" [{}]", m.block),
                        Style::default().fg(Color::Blue),
                    ));
                }
                parts.push(Span::raw(format!("  {}", format_intent(m))));
                if !m.powers.is_empty() {
                    let mp: Vec<String> = m
                        .powers
                        .iter()
                        .map(|p| format!("{}({})", p.id, p.amount))
                        .collect();
                    parts.push(Span::raw(format!("  [{}]", mp.join(", "))));
                }
                lines.push(Line::from(parts));
            }

            lines.push(Line::from(""));

            // Hand
            let hand_str: Vec<String> = hand
                .iter()
                .map(|hc| {
                    let c = &hc.card;
                    let cost = if c.cost >= 0 {
                        format!("({})", c.cost)
                    } else {
                        "".into()
                    };
                    format!(
                        "{}{}{}",
                        c.name,
                        if c.upgraded { "+" } else { "" },
                        cost
                    )
                })
                .collect();
            lines.push(Line::from(format!("Hand: {}", hand_str.join(", "))));

            // Pile counts
            lines.push(Line::from(format!(
                "Draw: {}  Discard: {}  Exhaust: {}",
                draw_pile.len(),
                discard_pile.len(),
                exhaust_pile.len()
            )));
        }
    } else {
        // Non-combat: show deck/relics/potions
        let deck: Vec<String> = state
            .deck
            .iter()
            .map(|c| {
                if c.upgraded {
                    format!("{}+", c.id)
                } else {
                    c.id.clone()
                }
            })
            .collect();

        let relics: Vec<&str> = state.relics.iter().map(|r| r.id.as_str()).collect();

        let potions: Vec<String> = state
            .potions
            .iter()
            .map(|p| match p {
                Some(pot) => pot.id.clone(),
                None => "empty".into(),
            })
            .collect();

        lines.push(Line::from(format!(
            "Deck ({}): {}",
            deck.len(),
            deck.join(", ")
        )));
        lines.push(Line::from(format!("Relics: {}", relics.join(", "))));
        lines.push(Line::from(format!("Potions: [{}]", potions.join(", "))));
    }

    lines
}

fn find_combat(stack: &[Screen]) -> Option<&Screen> {
    stack
        .iter()
        .rev()
        .find(|s| matches!(s, Screen::Combat { .. }))
}

fn format_screen_name(screen: &Screen) -> String {
    match screen {
        Screen::Neow { .. } => "Neow's Blessing".into(),
        Screen::Map { .. } => "Map".into(),
        Screen::Combat { encounter, .. } => format!("Combat ({})", encounter),
        Screen::Event { event_name, .. } => format!("Event: {}", event_name),
        Screen::Rest { .. } => "Rest Site".into(),
        Screen::Shop { .. } => "Shop".into(),
        Screen::ShopRoom => "Shop Room".into(),
        Screen::Treasure => "Treasure".into(),
        Screen::CardReward { cards } => format!("Card Reward ({} cards)", cards.len()),
        Screen::CombatRewards { rewards } => format!("Combat Rewards ({} items)", rewards.len()),
        Screen::BossRelic { relics, cards } => {
            format!("Boss Relic ({} relics, {} cards)", relics.len(), cards.len())
        }
        Screen::Grid { purpose, cards } => {
            format!("Select card to {} ({} cards)", purpose, cards.len())
        }
        Screen::HandSelect { cards, .. } => format!("Select from hand ({} cards)", cards.len()),
        Screen::DiscardSelect { cards, .. } => {
            format!("Select from discard ({} cards)", cards.len())
        }
        Screen::ExhaustSelect { cards, .. } => {
            format!("Select from exhaust ({} cards)", cards.len())
        }
        Screen::TargetSelect { reason, .. } => {
            let reason_str = match reason {
                sts_simulator::TargetReason::Card(c) => c.name.clone(),
                sts_simulator::TargetReason::Power(p) => p.id.clone(),
                sts_simulator::TargetReason::Pending => "...".into(),
            };
            format!("Select target ({})", reason_str)
        }
        Screen::ChoiceSelect { choices, .. } => format!("Choose ({} options)", choices.len()),
        Screen::XCostSelect { max_energy, .. } => format!("Choose X (max {})", max_energy),
        Screen::AutoPlaySelect { cards } => format!("Auto-play ({} cards remaining)", cards.len()),
        Screen::CustomScreen { screen_enum, .. } => format!("Custom: {}", screen_enum),
        Screen::GameOver { victory: true } => "Victory!".into(),
        Screen::GameOver { victory: false } => "Defeated".into(),
        Screen::Complete => "Complete".into(),
        Screen::MainMenu => "Main Menu".into(),
        Screen::Error { message } => format!("Error: {}", message),
        Screen::Unknown { raw_screen_type } => format!("Unknown: {}", raw_screen_type),
    }
}

fn format_intent(m: &sts_simulator::Monster) -> String {
    match m.intent.as_str() {
        "ATTACK" => {
            let dmg = m.damage.unwrap_or(0);
            if m.hits > 1 {
                format!("ATK {}x{}", dmg, m.hits)
            } else {
                format!("ATK {}", dmg)
            }
        }
        "ATTACK_DEFEND" => {
            let dmg = m.damage.unwrap_or(0);
            format!("ATK {} + BLK", dmg)
        }
        "DEFEND" => "BLK".into(),
        "BUFF" => "BUFF".into(),
        "DEBUFF" => "DEBUFF".into(),
        "ATTACK_BUFF" => {
            let dmg = m.damage.unwrap_or(0);
            format!("ATK {} + BUFF", dmg)
        }
        "ATTACK_DEBUFF" => {
            let dmg = m.damage.unwrap_or(0);
            format!("ATK {} + DEBUFF", dmg)
        }
        "DEFEND_BUFF" => "BLK + BUFF".into(),
        "UNKNOWN" => "???".into(),
        other => other.into(),
    }
}

fn format_action(action: &Action, state: &GameState) -> String {
    match action {
        Action::TravelTo { kind, label, .. } => format!("Travel to {:?} ({})", kind, label),
        Action::PickNeowBlessing { label, .. } => format!("Neow: {}", label),
        Action::PickEventOption { label, .. } => format!("Event: {}", label),
        Action::TakeCard { card, .. } => {
            format!(
                "Take {}{}",
                card.name,
                if card.upgraded { "+" } else { "" }
            )
        }
        Action::SkipCardReward => "Skip card reward".into(),
        Action::TakeReward { choice_index, .. } => {
            if let Screen::CombatRewards { rewards } = state.current_screen() {
                let idx = *choice_index as usize;
                if idx < rewards.len() {
                    let r = &rewards[idx];
                    return match r.reward_type.as_str() {
                        "GOLD" => format!("Take {} gold", r.gold.unwrap_or(0)),
                        "POTION" => format!(
                            "Take potion: {}",
                            r.potion.as_ref().map_or("?", |p| &p.name)
                        ),
                        "RELIC" => format!(
                            "Take relic: {}",
                            r.relic.as_ref().map_or("?", |r| &r.name)
                        ),
                        "CARD" => "Open card reward".into(),
                        "UPGRADED_CARD" => "Open upgraded card reward".into(),
                        "RARE_CARD" => "Open rare card reward".into(),
                        other => format!("Take {}", other),
                    };
                }
            }
            format!("Take reward {}", choice_index)
        }
        Action::PickBossRelic { choice_index, .. } => {
            if let Screen::BossRelic { relics, .. } = state.current_screen() {
                let idx = *choice_index as usize;
                if idx < relics.len() {
                    return format!("Pick {}", relics[idx].name);
                }
            }
            format!("Pick boss relic {}", choice_index)
        }
        Action::SkipBossRelic => "Skip boss relic".into(),
        Action::BuyCard { card, price, .. } => format!("Buy {} ({}g)", card.name, price),
        Action::BuyRelic { relic, price, .. } => format!("Buy {} ({}g)", relic, price),
        Action::BuyPotion { potion, price, .. } => format!("Buy {} ({}g)", potion, price),
        Action::Purge { price, .. } => format!("Purge ({}g)", price),
        Action::LeaveShop => "Leave shop".into(),
        Action::Rest { .. } => "Rest (heal)".into(),
        Action::Smith { .. } => "Smith (upgrade)".into(),
        Action::OpenChest { .. } => "Open chest".into(),
        Action::PickGridCard { card, .. } => {
            format!(
                "Select {}{}",
                card.name,
                if card.upgraded { "+" } else { "" }
            )
        }
        Action::PickHandCard { card, .. } => format!("Pick {}", card.name),
        Action::PickChoice { label, .. } => label.clone(),
        Action::PickDiscard { card, .. } => format!("Discard {}", card.name),
        Action::PickExhaust { card, .. } => format!("Exhaust {}", card.name),
        Action::PickTarget {
            target_name,
            reason,
            ..
        } => {
            let reason_str = match reason {
                sts_simulator::TargetReason::Card(c) => c.name.clone(),
                sts_simulator::TargetReason::Power(p) => p.id.clone(),
                sts_simulator::TargetReason::Pending => "?".into(),
            };
            format!("Target {} ({})", target_name, reason_str)
        }
        Action::PickAutoPlay { card, .. } => format!("Auto-play {}", card.name),
        Action::PickCustomScreenOption { label, .. } => label.clone(),
        Action::PlayCard {
            card, target_name, ..
        } => {
            let suffix = if card.upgraded { "+" } else { "" };
            match target_name {
                Some(name) => format!("Play {}{} → {}", card.name, suffix, name),
                None => format!("Play {}{}", card.name, suffix),
            }
        }
        Action::EndTurn => "End Turn".into(),
        Action::DiscardPotion { slot } => {
            let idx = *slot as usize;
            if idx < state.potions.len() {
                if let Some(p) = &state.potions[idx] {
                    return format!("Discard {}", p.name);
                }
            }
            format!("Discard potion slot {}", slot)
        }
        Action::UsePotion { label, .. } => format!("Use {}", label),
        Action::Proceed => "Proceed".into(),
        Action::Skip => "Skip".into(),
    }
}
