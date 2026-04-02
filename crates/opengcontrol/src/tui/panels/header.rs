use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};

use crate::tui::{CYAN, DIM};

pub struct Header<'a> {
    pub name: &'a str,
    pub vid: u16,
    pub pid: u16,
    pub refreshing: bool,
}

impl Widget for Header<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let status = if self.refreshing {
            Span::styled(" refreshing… ", Style::default().fg(CYAN))
        } else {
            Span::styled(" live ", Style::default().fg(CYAN).add_modifier(Modifier::DIM))
        };

        let id = format!(" {:04X}:{:04X} ", self.vid, self.pid);

        let title_line = Line::from(vec![
            Span::raw(" ◈  "),
            Span::styled(self.name, Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  {id}"), Style::default().fg(DIM)),
        ]);

        let block = Block::bordered()
            .border_style(Style::default().fg(CYAN))
            .title_bottom(Line::from(vec![status]).right_aligned());

        let inner = block.inner(area);
        block.render(area, buf);
        Paragraph::new(title_line).render(inner, buf);
    }
}
