use hidpp_core::features::dpi::DpiList;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::tui::{AMBER, CYAN, DIM, GREEN};

pub struct DpiPanel<'a> {
    pub focused: bool,
    pub current_dpi: u16,
    pub dpi_values: &'a [u16],
    pub pending_idx: usize,
    /// Some = user is typing a custom DPI value.
    pub dpi_input: Option<&'a str>,
    pub dpi_list: &'a DpiList,
}

impl Widget for DpiPanel<'_> {
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
                " DPI ",
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
                format!("{}", self.current_dpi),
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" dpi", Style::default().fg(DIM)),
        ]);

        if let Some(typed) = self.dpi_input {
            // Text input mode: show what the user is typing
            let range_hint = match self.dpi_list {
                DpiList::Range { min, step, max } => {
                    format!("  {min}–{max}, step {step}")
                }
                DpiList::Discrete(_) => String::new(),
            };

            let input_line = Line::from(vec![
                Span::styled("Enter DPI  ", Style::default().fg(DIM)),
                Span::styled(
                    format!("{typed}_"),
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                ),
            ]);

            let hint_line = Line::from(Span::styled(range_hint, Style::default().fg(DIM)));
            let confirm_line = Line::from(Span::styled(
                "  ↵ Apply  Esc Cancel",
                Style::default().fg(GREEN),
            ));

            let lines = vec![current_line, Line::raw(""), input_line, hint_line];
            Paragraph::new(lines).render(inner, buf);

            if inner.height >= 5 {
                let hint_area = Rect {
                    y: inner.y + inner.height - 1,
                    height: 1,
                    ..inner
                };
                Paragraph::new(confirm_line).render(hint_area, buf);
            }
        } else {
            // Normal mode: sliding value strip
            let pending_dpi = self.dpi_values.get(self.pending_idx).copied().unwrap_or(0);
            let is_changed = pending_dpi != self.current_dpi;

            let window = build_window(self.dpi_values, self.pending_idx, inner.width as usize);
            let mut spans: Vec<Span> = Vec::new();
            for (i, val) in window.iter() {
                if *i == self.pending_idx {
                    let style = if is_changed {
                        Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
                    };
                    spans.push(Span::styled(format!("[{}]", val), style));
                } else {
                    spans.push(Span::styled(format!(" {} ", val), Style::default().fg(DIM)));
                }
            }
            let value_line = Line::from(spans);

            let lines = vec![current_line, Line::raw(""), value_line];
            Paragraph::new(lines).render(inner, buf);

            if self.focused && inner.height >= 5 {
                let hint_text = if is_changed {
                    "  ↵ Apply  or type a number"
                } else {
                    "  ← → Change  or type a number"
                };
                let hint = Line::from(Span::styled(
                    hint_text,
                    if is_changed {
                        Style::default().fg(AMBER)
                    } else {
                        Style::default().fg(DIM)
                    },
                ));
                let hint_area = Rect {
                    y: inner.y + inner.height - 1,
                    height: 1,
                    ..inner
                };
                Paragraph::new(hint).render(hint_area, buf);
            } else if inner.height >= 4 {
                // Compact hint when not focused or small area
                if is_changed {
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
    }
}

/// Returns a window of (original_index, value) pairs around `center`.
fn build_window(values: &[u16], center: usize, width: usize) -> Vec<(usize, u16)> {
    if values.is_empty() {
        return Vec::new();
    }

    // Estimate chars per item: "1234 " = 5 chars average
    let max_items = (width / 6).max(3);
    let half = max_items / 2;

    let start = center.saturating_sub(half);
    let end = (start + max_items).min(values.len());
    let start = end.saturating_sub(max_items);

    values[start..end]
        .iter()
        .enumerate()
        .map(|(i, &v)| (start + i, v))
        .collect()
}
