use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use sts_simulator::{Action, GameState, Screen};

struct App {
    state: GameState,
    actions: Vec<Action>,
    selected: usize,
    log: Vec<String>,
}

impl App {
    fn new() -> Self {
        let state = GameState::new(0);
        let actions = state.available_actions();
        App {
            state,
            actions,
            selected: 0,
            log: vec!["Run started.".into()],
        }
    }

    fn select_action(&mut self) {
        if self.actions.is_empty() {
            return;
        }
        let action = self.actions[self.selected].clone();
        self.log.push(format!("> {:?}", action));
        self.state.apply(action);
        self.actions = self.state.available_actions();
        self.selected = 0;
    }
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
    let outer = Layout::vertical([Constraint::Min(5), Constraint::Length(10)]).split(frame.area());

    let top = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(outer[0]);

    // Left panel: game state
    let status = build_status(&app.state);
    let status_block = Paragraph::new(status)
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
            ListItem::new(format!("{}{}", prefix, format_action(action))).style(style)
        })
        .collect();

    let actions_list =
        List::new(items).block(Block::default().borders(Borders::ALL).title("Actions"));
    frame.render_widget(actions_list, top[1]);

    // Bottom panel: log
    let log_items: Vec<ListItem> = app.log.iter().rev().take(8).rev().map(|s| ListItem::new(s.as_str())).collect();
    let log_list = List::new(log_items).block(Block::default().borders(Borders::ALL).title("Log"));
    frame.render_widget(log_list, outer[1]);
}

fn build_status(state: &GameState) -> Vec<Line<'static>> {
    let screen_name = match &state.screen {
        Screen::Neow => "Neow's Blessing",
        Screen::MapSelect => "Map",
        Screen::Combat => "Combat",
        Screen::Event => "Event",
        Screen::Rest => "Rest Site",
        Screen::Shop => "Shop",
        Screen::Treasure => "Treasure",
        Screen::CardReward => "Card Reward",
        Screen::CombatRewards => "Combat Rewards",
        Screen::BossReward => "Boss Relic",
        Screen::GameOver { victory: true } => "Victory!",
        Screen::GameOver { victory: false } => "Defeated",
    };

    vec![
        Line::from(vec![
            Span::raw("HP: "),
            Span::styled(
                format!("{}/{}", state.hp, state.max_hp),
                Style::default().fg(Color::Red),
            ),
            Span::raw(format!("  Gold: {}  Act: {}  Floor: {}", state.gold, state.act, state.floor)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Screen: "),
            Span::styled(screen_name, Style::default().fg(Color::Cyan)),
        ]),
    ]
}

fn format_action(action: &Action) -> String {
    match action {
        Action::TravelTo { kind, .. } => format!("Travel to {:?}", kind),
        Action::OpenChest => "Open chest".into(),
        Action::Rest => "Rest (heal 30% max HP)".into(),
        Action::Smith => "Smith (upgrade a card)".into(),
        Action::Proceed => "Proceed".into(),
    }
}
