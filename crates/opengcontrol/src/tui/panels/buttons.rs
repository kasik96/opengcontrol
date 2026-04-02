use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use hidpp_core::features::{ButtonAction, ButtonAssignment};

use crate::tui::{AMBER, CYAN, DIM};

pub struct ButtonsPanel<'a> {
    pub focused: bool,
    pub assignments: &'a [ButtonAssignment],
}

impl Widget for ButtonsPanel<'_> {
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
                " Buttons ",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || self.assignments.is_empty() {
            return;
        }

        // Split into rows of 3
        let mut rows: Vec<Line> = Vec::new();
        let mut current_row: Vec<Span> = Vec::new();

        for assignment in self.assignments {
            let label = action_label(&assignment.action);
            let (label_style, special) = action_style(&assignment.action);

            let idx_span = Span::styled(
                format!("[{}]", assignment.button_index + 1),
                Style::default().fg(CYAN).add_modifier(Modifier::DIM),
            );
            let label_span = Span::styled(label, label_style);
            let sep = Span::styled("   ", Style::default());

            current_row.push(idx_span);
            current_row.push(label_span);
            if special {
                current_row.push(Span::styled(" ↑", Style::default().fg(AMBER)));
            }
            current_row.push(sep);

            if current_row.len() >= 12 {
                rows.push(Line::from(current_row.drain(..).collect::<Vec<_>>()));
            }
        }
        if !current_row.is_empty() {
            rows.push(Line::from(current_row));
        }

        Paragraph::new(rows).render(inner, buf);
    }
}

fn action_label(action: &ButtonAction) -> String {
    match action {
        ButtonAction::MouseButton(1) => "Left".to_string(),
        ButtonAction::MouseButton(2) => "Right".to_string(),
        ButtonAction::MouseButton(3) => "Middle".to_string(),
        ButtonAction::MouseButton(4) => "Back".to_string(),
        ButtonAction::MouseButton(5) => "Forward".to_string(),
        ButtonAction::MouseButton(n) => format!("Button {n}"),
        ButtonAction::DpiCycleUp => "DPI ↑".to_string(),
        ButtonAction::DpiCycleDown => "DPI ↓".to_string(),
        ButtonAction::ProfileCycle => "Profile ↻".to_string(),
        ButtonAction::KeyCombo { modifiers, key } => format!("Key {modifiers:02X}+{key:02X}"),
        ButtonAction::Disabled => "—".to_string(),
    }
}

fn action_style(action: &ButtonAction) -> (Style, bool) {
    match action {
        ButtonAction::MouseButton(_) => (Style::default().fg(DIM), false),
        ButtonAction::DpiCycleUp | ButtonAction::DpiCycleDown => {
            (Style::default().fg(AMBER).add_modifier(Modifier::BOLD), true)
        }
        ButtonAction::ProfileCycle => {
            (Style::default().fg(AMBER), false)
        }
        ButtonAction::KeyCombo { .. } => (Style::default().fg(DIM), false),
        ButtonAction::Disabled => (
            Style::default().fg(DIM).add_modifier(Modifier::DIM),
            false,
        ),
    }
}
