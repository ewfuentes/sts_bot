use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use sts_simulator::{Action, Card, GameState, HandCard, MapChoice, MapNodeKind, Monster, Screen};

struct App {
    state: GameState,
    actions: Vec<Action>,
    selected: usize,
    log: Vec<String>,
}

impl App {
    fn new() -> Self {
        let state = make_initial_state();
        let actions = state.available_actions();
        App {
            state,
            actions,
            selected: 0,
            log: vec!["Run started. Determinized with seed 42.".into()],
        }
    }

    fn select_action(&mut self) {
        if self.actions.is_empty() {
            return;
        }
        let action = self.actions[self.selected].clone();
        self.log.push(format!("> {}", format_action(&action, &self.state)));
        self.state.apply(&action);

        // If we entered combat, set up the encounter
        if let Screen::Combat { monsters, hand, turn, .. } = self.state.current_screen() {
            if monsters.is_empty() && hand.is_empty() && *turn == 0 {
                setup_combat(&mut self.state);
            }
        }

        // If we've popped back to an empty stack, show the map again
        if matches!(self.state.current_screen(), Screen::Complete) && self.state.screen.len() == 1 {
            self.state.set_screen(Screen::Map {
                current_node: 0,
                available_nodes: vec![
                    MapChoice { label: "x=0".into(), kind: MapNodeKind::Monster, node_index: 0 },
                    MapChoice { label: "x=1".into(), kind: MapNodeKind::Elite, node_index: 1 },
                    MapChoice { label: "x=2".into(), kind: MapNodeKind::Rest, node_index: 2 },
                    MapChoice { label: "x=3".into(), kind: MapNodeKind::Shop, node_index: 3 },
                    MapChoice { label: "x=4".into(), kind: MapNodeKind::Treasure, node_index: 4 },
                    MapChoice { label: "x=5".into(), kind: MapNodeKind::Boss, node_index: 5 },
                ],
            });
        }

        self.actions = self.state.available_actions();
        self.selected = 0;
    }
}

fn make_initial_state() -> GameState {
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
            {"id": "BGAscendersBane", "name": "Ascender's Bane", "cost": -2, "type": "CURSE", "upgraded": false},
        ],
        "relics": [
            {"id": "BoardGame:BurningBlood", "name": "Burning Blood", "counter": -1},
        ],
        "potions": [null, null],
        "actions": [],
        "screen": {
            "type": "map",
            "available_nodes": [
                {"label": "x=1", "kind": "monster"},
                {"label": "x=3", "kind": "event"},
                {"label": "x=5", "kind": "monster"},
            ]
        }
    });
    let mut state = GameState::from_json(&json.to_string()).unwrap();
    state.determinize(42);
    state
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App::new();

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
                KeyCode::Enter => app.select_action(),
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw(frame: &mut Frame, app: &App) {
    let outer = Layout::vertical([Constraint::Min(5), Constraint::Length(12)]).split(frame.area());

    let top = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[0]);

    // Left panel: game state
    let status = build_status(&app.state);
    let status_block = Paragraph::new(status)
        .wrap(ratatui::widgets::Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title("Game State"));
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
            ListItem::new(format!("{}{}", prefix, format_action(action, &app.state))).style(style)
        })
        .collect();

    let actions_list =
        List::new(items).block(Block::default().borders(Borders::ALL).title("Actions"));
    frame.render_widget(actions_list, top[1]);

    // Bottom panel: log
    let log_items: Vec<ListItem> = app.log.iter().rev().take(10).rev().map(|s| ListItem::new(s.as_str())).collect();
    let log_list = List::new(log_items).block(Block::default().borders(Borders::ALL).title("Log"));
    frame.render_widget(log_list, outer[1]);
}

fn build_status(state: &GameState) -> Vec<Line<'static>> {
    let screen_name = match state.current_screen() {
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
        Screen::BossRelic { relics, cards } => format!("Boss Relic ({} relics, {} cards)", relics.len(), cards.len()),
        Screen::Grid { purpose, cards } => format!("Select card to {} ({} cards)", purpose, cards.len()),
        Screen::GameOver { victory: true } => "Victory!".into(),
        Screen::GameOver { victory: false } => "Defeated".into(),
        Screen::Complete => "Complete".into(),
        _ => "Unknown".into(),
    };

    let deck: Vec<String> = state.deck.iter().map(|c| {
        if c.upgraded { format!("{}+", c.id) } else { c.id.clone() }
    }).collect();

    let relics: Vec<&str> = state.relics.iter().map(|r| r.id.as_str()).collect();

    let potions: Vec<String> = state.potions.iter().map(|p| {
        match p {
            Some(pot) => pot.id.clone(),
            None => "empty".into(),
        }
    }).collect();

    let stack_depth = state.screen.len();

    let mut lines = vec![
        Line::from(vec![
            Span::raw("HP: "),
            Span::styled(
                format!("{}/{}", state.hp, state.max_hp),
                Style::default().fg(Color::Red),
            ),
            Span::raw(format!("  Gold: {}  Floor: {}  Act: {}", state.gold, state.floor, state.act)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Screen: "),
            Span::styled(screen_name, Style::default().fg(Color::Cyan)),
            Span::raw(format!("  (stack: {})", stack_depth)),
        ]),
        Line::from(""),
    ];

    // Combat-specific display
    if let Screen::Combat {
        monsters, hand, draw_pile, discard_pile, exhaust_pile,
        player_block, player_energy, player_powers, turn, ..
    } = state.current_screen()
    {
        lines.push(Line::from(format!(
            "Energy: {}  Block: {}  Turn: {}", player_energy, player_block, turn
        )));
        if !player_powers.is_empty() {
            let pp: Vec<String> = player_powers.iter().map(|p| format!("{}({})", p.id, p.amount)).collect();
            lines.push(Line::from(format!("Powers: {}", pp.join(", "))));
        }
        lines.push(Line::from(""));
        for m in monsters {
            if m.is_gone { continue; }
            let mp = if m.powers.is_empty() {
                String::new()
            } else {
                format!(" [{}]", m.powers.iter().map(|p| format!("{}({})", p.id, p.amount)).collect::<Vec<_>>().join(", "))
            };
            let dmg = match (m.damage, m.hits) {
                (Some(d), h) if h > 1 => format!(" {}x{}", d, h),
                (Some(d), _) => format!(" {}", d),
                _ => String::new(),
            };
            lines.push(Line::from(format!(
                "  {} {}/{} blk:{} {}{}{}", m.name, m.hp, m.max_hp, m.block, m.intent, dmg, mp
            )));
        }
        lines.push(Line::from(""));
        let hand_strs: Vec<String> = hand.iter().map(|hc| {
            let up = if hc.card.upgraded { "+" } else { "" };
            let unplayable = if !hc.is_playable { " [X]" } else { "" };
            format!("{}{}({}){}", hc.card.name, up, hc.card.cost, unplayable)
        }).collect();
        lines.push(Line::from(format!("Hand: {}", hand_strs.join(", "))));
        lines.push(Line::from(format!(
            "Draw: {}  Discard: {}  Exhaust: {}", draw_pile.len(), discard_pile.len(), exhaust_pile.len()
        )));
    } else {
        lines.push(Line::from(format!("Deck ({}): {}", deck.len(), deck.join(", "))));
    }

    lines.push(Line::from(format!("Relics: {}", relics.join(", "))));
    lines.push(Line::from(format!("Potions: [{}]", potions.join(", "))));

    lines
}

fn format_action(action: &Action, state: &GameState) -> String {
    match action {
        Action::TravelTo { kind, label, .. } => format!("Travel to {:?} ({})", kind, label),
        Action::PickNeowBlessing { label, .. } => format!("Neow: {}", label),
        Action::PickEventOption { label, .. } => format!("Event: {}", label),
        Action::TakeCard { card, .. } => format!("Take {}{}", card.name, if card.upgraded { "+" } else { "" }),
        Action::SkipCardReward => "Skip card reward".into(),
        Action::TakeReward { choice_index, .. } => {
            if let Screen::CombatRewards { rewards } = state.current_screen() {
                let idx = *choice_index as usize;
                if idx < rewards.len() {
                    let r = &rewards[idx];
                    return match r.reward_type.as_str() {
                        "GOLD" => format!("Take {} gold", r.gold.unwrap_or(0)),
                        "POTION" => format!("Take potion: {}", r.potion.as_ref().map_or("?", |p| &p.name)),
                        "RELIC" => format!("Take relic: {}", r.relic.as_ref().map_or("?", |r| &r.name)),
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
        Action::PickGridCard { card, .. } => format!("Select {}{}", card.name, if card.upgraded { "+" } else { "" }),
        Action::PickHandCard { card, .. } => format!("Pick {}", card.name),
        Action::PickCustomScreenOption { label, .. } => label.clone(),
        Action::PlayCard { card, target_name, .. } => {
            let up = if card.upgraded { "+" } else { "" };
            match target_name {
                Some(name) => format!("Play {}{} (cost {}) → {}", card.name, up, card.cost, name),
                None => format!("Play {}{} (cost {})", card.name, up, card.cost),
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
        Action::Proceed => "Proceed".into(),
        Action::Skip => "Skip".into(),
    }
}

/// Set up a combat encounter: populate monsters, shuffle deck into draw pile, draw 5, set energy.
fn setup_combat(state: &mut GameState) {
    // Create a Jaw Worm encounter
    let monsters = vec![
        Monster {
            id: "BGJawWorm".into(),
            name: "Jaw Worm".into(),
            hp: 8,
            max_hp: 8,
            block: 0,
            intent: "ATTACK".into(),
            damage: Some(3),
            hits: 1,
            powers: vec![],
            is_gone: false,
        },
    ];

    // Shuffle deck into draw pile
    let mut draw_pile: Vec<Card> = state.deck.clone();
    // Simple deterministic shuffle: reverse (good enough for demo)
    draw_pile.reverse();

    // Draw 5 cards
    let mut hand = Vec::new();
    for _ in 0..5 {
        if let Some(card) = draw_pile.pop() {
            let info = sts_simulator::card_db::lookup(&card.id);
            let is_playable = info
                .map(|i| {
                    let c = i.effective_cost(card.upgraded);
                    c >= 0 && c <= 3
                })
                .unwrap_or(card.cost >= 0 && card.cost <= 3);
            let has_target = info.map(|i| i.target.has_target()).unwrap_or(false);
            hand.push(HandCard { card, is_playable, has_target });
        }
    }

    if let Some(Screen::Combat {
        monsters: m, hand: h, draw_pile: dp,
        player_energy, player_block, turn, ..
    }) = state.screen.last_mut()
    {
        *m = monsters;
        *h = hand;
        *dp = draw_pile;
        *player_energy = 3;
        *player_block = 0;
        *turn = 1;
        // Keep encounter string (e.g. "UNKNOWN_MONSTER") — it drives reward generation
    }
}
