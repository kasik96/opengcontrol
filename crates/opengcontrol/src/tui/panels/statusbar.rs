use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::tui::{DIM, GREEN, RED};
use crate::tui::app::StatusKind;

pub struct StatusBar<'a> {
    pub msg: Option<&'a (String, StatusKind, std::time::Instant)>,
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let line = if let Some((text, kind, _)) = self.msg {
            let style = match kind {
                StatusKind::Ok => Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                StatusKind::Error => Style::default().fg(RED).add_modifier(Modifier::BOLD),
            };
            Line::from(Span::styled(format!("  {text}"), style))
        } else {
            Line::from(vec![
                Span::styled(" Tab", Style::default().fg(DIM).add_modifier(Modifier::BOLD)),
                Span::styled("/↑↓ Focus   ", Style::default().fg(DIM)),
                Span::styled("←→", Style::default().fg(DIM).add_modifier(Modifier::BOLD)),
                Span::styled(" Change   ", Style::default().fg(DIM)),
                Span::styled("↵", Style::default().fg(DIM).add_modifier(Modifier::BOLD)),
                Span::styled(" Apply   ", Style::default().fg(DIM)),
                Span::styled("r", Style::default().fg(DIM).add_modifier(Modifier::BOLD)),
                Span::styled(" Refresh   ", Style::default().fg(DIM)),
                Span::styled("q", Style::default().fg(DIM).add_modifier(Modifier::BOLD)),
                Span::styled(" Quit", Style::default().fg(DIM)),
            ])
        };

        Paragraph::new(line).render(area, buf);
    }
}
