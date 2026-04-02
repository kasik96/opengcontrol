use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::tui::app::{Focus, StatusKind};
use crate::tui::{DIM, GREEN, RED};

pub struct StatusBar<'a> {
    pub msg: Option<&'a (String, StatusKind, std::time::Instant)>,
    pub focus: Focus,
    pub button_editing: bool,
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
            self.hint_line()
        };

        Paragraph::new(line).render(area, buf);
    }
}

impl StatusBar<'_> {
    fn hint_line(&self) -> Line<'static> {
        let key = |s: &'static str| {
            Span::styled(s, Style::default().fg(DIM).add_modifier(Modifier::BOLD))
        };
        let txt = |s: &'static str| Span::styled(s, Style::default().fg(DIM));

        match (self.focus, self.button_editing) {
            (Focus::Buttons, true) => Line::from(vec![
                key("←→"),
                txt(" Change action   "),
                key("↵"),
                txt(" Apply   "),
                key("Esc"),
                txt(" Cancel   "),
                key("Tab"),
                txt("/↑↓ Focus   "),
                key("r"),
                txt(" Refresh   "),
                key("q"),
                txt(" Quit"),
            ]),
            (Focus::Buttons, false) => Line::from(vec![
                key("←→"),
                txt(" Select button   "),
                key("↵"),
                txt(" Edit   "),
                key("Tab"),
                txt("/↑↓ Focus   "),
                key("r"),
                txt(" Refresh   "),
                key("q"),
                txt(" Quit"),
            ]),
            (Focus::Profiles, _) => Line::from(vec![
                key("←→"),
                txt(" Select profile   "),
                key("↵"),
                txt(" Switch   "),
                key("Tab"),
                txt("/↑↓ Focus   "),
                key("r"),
                txt(" Refresh   "),
                key("q"),
                txt(" Quit"),
            ]),
            _ => Line::from(vec![
                key(" Tab"),
                txt("/↑↓ Focus   "),
                key("←→"),
                txt(" Change   "),
                key("↵"),
                txt(" Apply   "),
                key("r"),
                txt(" Refresh   "),
                key("q"),
                txt(" Quit"),
            ]),
        }
    }
}
