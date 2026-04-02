use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use hidpp_core::features::OnboardProfile;

use crate::tui::{AMBER, CYAN, DIM};

pub struct ProfilesPanel<'a> {
    pub focused: bool,
    pub active_profile: u8,
    pub pending_profile: usize,
    pub profiles: &'a [Option<OnboardProfile>],
}

impl Widget for ProfilesPanel<'_> {
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
                " Profiles ",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 {
            return;
        }

        let total = self.profiles.len();
        let pending = self.pending_profile;
        let is_pending = pending != self.active_profile as usize;
        let mut lines: Vec<Line> = Vec::new();

        // Header: navigation indicator
        let num_style = if is_pending {
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
        };
        let header = Line::from(vec![
            Span::styled("Active  ", Style::default().fg(DIM)),
            Span::styled("← ", Style::default().fg(DIM)),
            Span::styled(format!("{}", pending + 1), num_style),
            Span::styled(format!(" / {total}"), Style::default().fg(DIM)),
            Span::styled(" →", Style::default().fg(DIM)),
            if is_pending {
                Span::styled("  ↵ to switch", Style::default().fg(AMBER))
            } else {
                Span::raw("")
            },
        ]);
        lines.push(header);

        // Show info for the pending profile
        if let Some(Some(profile)) = self.profiles.get(pending) {
            // DPI slots
            let mut dpi_spans: Vec<Span> = vec![Span::styled("Slots  ", Style::default().fg(DIM))];
            for (i, &dpi) in profile.dpi_slots.iter().enumerate() {
                if i > 0 {
                    dpi_spans.push(Span::styled("  ·  ", Style::default().fg(DIM)));
                }
                if i == profile.active_dpi_slot as usize {
                    dpi_spans.push(Span::styled(
                        format!("[{}]", dpi),
                        Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
                    ));
                } else {
                    dpi_spans.push(Span::styled(
                        format!("{}", dpi),
                        Style::default().fg(DIM),
                    ));
                }
            }
            lines.push(Line::from(dpi_spans));

            // Polling rate stored in profile
            lines.push(Line::from(vec![
                Span::styled("Rate   ", Style::default().fg(DIM)),
                Span::styled(
                    format!("{} Hz", profile.polling_rate_hz),
                    Style::default().fg(DIM),
                ),
            ]));
        }

        Paragraph::new(lines).render(inner, buf);
    }
}
