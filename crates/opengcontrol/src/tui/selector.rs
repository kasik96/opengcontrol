use std::io;
use std::time::Duration;

use crossterm::event::{Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};

use crate::context::PhysicalDevice;

use super::{AMBER, CYAN, DIM};

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    devices: &[PhysicalDevice],
) -> Result<Option<String>, io::Error> {
    let mut state = ListState::default();
    state.select(Some(0));

    loop {
        terminal.draw(|f| render(f, devices, &mut state))?;

        if crossterm::event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                    KeyCode::Up => {
                        let i = state.selected().unwrap_or(0);
                        state.select(Some(i.saturating_sub(1)));
                    }
                    KeyCode::Down => {
                        let i = state.selected().unwrap_or(0);
                        state.select(Some((i + 1).min(devices.len() - 1)));
                    }
                    KeyCode::Enter => {
                        if let Some(i) = state.selected() {
                            return Ok(Some(devices[i].path.clone()));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn render(f: &mut ratatui::Frame, devices: &[PhysicalDevice], state: &mut ListState) {
    let area = f.area();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(devices.len() as u16 + 4),
            Constraint::Fill(1),
        ])
        .split(area);

    let center = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(60),
            Constraint::Fill(1),
        ])
        .split(outer[1]);

    let panel = center[1];

    let items: Vec<ListItem> = devices
        .iter()
        .map(|d| {
            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{:<30}", d.name),
                    Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:04X}:{:04X}", d.vid, d.pid),
                    Style::default().fg(DIM),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(CYAN))
                .title(Span::styled(
                    " Select Device ",
                    Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
                )),
        )
        .highlight_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD))
        .highlight_symbol("► ");

    f.render_stateful_widget(list, panel, state);

    // Footer hint below the panel
    if outer[2].height > 0 {
        let hint = Paragraph::new(Line::from(Span::styled(
            "↑/↓ Navigate   Enter Select   q Quit",
            Style::default().fg(DIM),
        )))
        .alignment(Alignment::Center);
        let hint_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(60),
                Constraint::Fill(1),
            ])
            .split(outer[2])[1];
        f.render_widget(hint, hint_area);
    }
}
