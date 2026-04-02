use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::tui::{AMBER, CYAN, DIM};

pub struct PollingPanel<'a> {
    pub focused: bool,
    pub current_hz: u16,
    pub supported_hz: &'a [u16],
    pub pending_idx: usize,
}

impl Widget for PollingPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_style = if self.focused {
            Style::default().fg(CYAN)
        } else {
            Style::default().fg(DIM)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(
                " Polling Rate ",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 {
            return;
        }

        // Line 1: current value
        let current_line = Line::from(vec![
            Span::styled("Current  ", Style::default().fg(DIM)),
            Span::styled(
                format!("{}", self.current_hz),
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Hz", Style::default().fg(DIM)),
        ]);

        // Line 2: options strip
        let pending_hz = self
            .supported_hz
            .get(self.pending_idx)
            .copied()
            .unwrap_or(0);
        let is_changed = pending_hz != self.current_hz;

        let mut spans: Vec<Span> = Vec::new();
        for (i, &hz) in self.supported_hz.iter().enumerate() {
            if i == self.pending_idx {
                let style = if is_changed {
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
                };
                spans.push(Span::styled(format!("[{}]", hz), style));
            } else {
                spans.push(Span::styled(format!(" {} ", hz), Style::default().fg(DIM)));
            }
        }
        let value_line = Line::from(spans);

        let lines = vec![current_line, Line::raw(""), value_line];
        Paragraph::new(lines).render(inner, buf);

        if is_changed && inner.height >= 4 {
            let hint = Line::from(Span::styled(
                "  ↵ Enter to apply",
                Style::default().fg(AMBER),
            ));
            let hint_area = Rect {
                y: inner.y + inner.height - 1,
                height: 1,
                ..inner
            };
            Paragraph::new(hint).render(hint_area, buf);
        }
    }
}
