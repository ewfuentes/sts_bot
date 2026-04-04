use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use sts_simulator::{Action, ActMap, GameState, MapChoice, MapNode, MapNodeKind, Screen};

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
        self.log
            .push(format!("> {}", format_action(&action, &self.state)));
        self.state.apply(&action);

        // If we've popped back to an empty stack, show the map again
        if matches!(self.state.current_screen(), Screen::Complete) && self.state.screen.len() == 1 {
            let (map, choices) = make_map();
            self.state.map = Some(map);
            self.state.set_screen(Screen::Map {
                current_node: 0,
                available_nodes: choices,
            });
        }

        self.actions = self.state.available_actions();
        self.selected = 0;
    }
}

/// Encounters that only use implemented monsters.
const PLAYABLE_ENCOUNTERS: &[(&str, &str, MapNodeKind)] = &[
    // Weak encounters
    ("Jaw Worm", "BoardGame:Jaw Worm (Easy)", MapNodeKind::Monster),
    ("Cultist", "BoardGame:Cultist", MapNodeKind::Monster),
    ("Small Slimes", "BoardGame:Easy Small Slimes", MapNodeKind::Monster),
    ("2 Louse", "BoardGame:2 Louse", MapNodeKind::Monster),
    // Strong encounters
    ("Cultist+Slime", "BoardGame:Cultist and SpikeSlime", MapNodeKind::Monster),
    ("Cultist+Louse", "BoardGame:Cultist and Louse", MapNodeKind::Monster),
    ("Fungi Beasts", "BoardGame:Fungi Beasts", MapNodeKind::Monster),
    ("Slime Trio", "BoardGame:Slime Trio", MapNodeKind::Monster),
    ("3 Louse", "BoardGame:3 Louse (Hard)", MapNodeKind::Monster),
    ("Blue Slaver", "BoardGame:Blue Slaver", MapNodeKind::Monster),
    ("Red Slaver", "BoardGame:Red Slaver", MapNodeKind::Monster),
    ("Jaw Worm (M)", "BoardGame:Jaw Worm (Medium)", MapNodeKind::Monster),
    ("Sneaky Gremlin", "BoardGame:Sneaky Gremlin Team", MapNodeKind::Monster),
    ("Angry Gremlin", "BoardGame:Angry Gremlin Team", MapNodeKind::Monster),
    ("Looter", "BoardGame:Looter", MapNodeKind::Monster),
    // Elites
    ("Gremlin Nob", "BoardGame:Gremlin Nob", MapNodeKind::Elite),
    ("Lagavulin", "BoardGame:Lagavulin", MapNodeKind::Elite),
    ("3 Sentries", "BoardGame:3 Sentries", MapNodeKind::Elite),
    // Rest
    ("Rest", "", MapNodeKind::Rest),
];

fn make_map() -> (ActMap, Vec<MapChoice>) {
    let nodes: Vec<MapNode> = PLAYABLE_ENCOUNTERS
        .iter()
        .enumerate()
        .map(|(i, (_, encounter_id, kind))| MapNode {
            x: i as u8,
            y: 0,
            kind: *kind,
            edges: vec![],
            encounter: if encounter_id.is_empty() {
                None
            } else {
                Some(encounter_id.to_string())
            },
        })
        .collect();

    let choices: Vec<MapChoice> = PLAYABLE_ENCOUNTERS
        .iter()
        .enumerate()
        .map(|(i, (label, _, kind))| MapChoice {
            label: label.to_string(),
            kind: *kind,
            node_index: i,
        })
        .collect();

    (ActMap { nodes }, choices)
}

fn make_initial_state() -> GameState {
    let (map, choices) = make_map();
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
            "available_nodes": choices.iter().map(|c| serde_json::json!({
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
    let outer =
        Layout::vertical([Constraint::Min(5), Constraint::Length(12)]).split(frame.area());

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

fn build_status(state: &GameState) -> Vec<Line<'static>> {
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

    let screen_name = format_screen_name(state.current_screen());
    lines.push(Line::from(vec![
        Span::raw("Screen: "),
        Span::styled(screen_name, Style::default().fg(Color::Cyan)),
        Span::raw(format!("  (stack: {})", state.screen.len())),
    ]));
    lines.push(Line::from(""));

    // Show combat-specific details when in combat
    if let Some(combat) = find_combat(&state.screen) {
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
                if m.is_gone {
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
        Action::Proceed => "Proceed".into(),
        Action::Skip => "Skip".into(),
    }
}
