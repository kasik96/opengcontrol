use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::{
    app::{AppState, Focus},
    panels::{
        buttons::ButtonsPanel, dpi::DpiPanel, header::Header, polling::PollingPanel,
        profiles::ProfilesPanel, statusbar::StatusBar,
    },
    DIM,
};

pub fn render(f: &mut Frame, app: &AppState) {
    let area = f.area();

    // Guard: terminal too small
    if area.width < 60 || area.height < 18 {
        let msg = Paragraph::new(Line::from(Span::styled(
            "Terminal too small (min 60×18)",
            Style::default().fg(DIM),
        )));
        f.render_widget(msg, area);
        return;
    }

    // Outer vertical split: header | body | statusbar
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(0),     // body
            Constraint::Length(1),  // statusbar
        ])
        .split(area);

    // Header — always use the known device identity
    f.render_widget(
        Header {
            name: app.device_name,
            vid: app.device_vid,
            pid: app.device_pid,
            refreshing: app.refreshing,
        },
        outer[0],
    );

    // Body: vertical split into top row + profiles row + buttons row
    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // DPI + Polling side-by-side
            Constraint::Length(6),  // Profiles
            Constraint::Min(0),     // Buttons (takes remaining)
        ])
        .split(outer[1]);

    // Top row: DPI left, Polling right
    let top_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(body[0]);

    if let Some(snap) = &app.snapshot {
        f.render_widget(
            DpiPanel {
                focused: app.focus == Focus::Dpi,
                current_dpi: snap.current_dpi,
                dpi_values: &app.dpi_values,
                pending_idx: app.pending_dpi_idx,
                dpi_input: app.dpi_input.as_deref(),
                dpi_list: &snap.dpi_list,
            },
            top_row[0],
        );

        f.render_widget(
            PollingPanel {
                focused: app.focus == Focus::Polling,
                current_hz: snap.current_polling_hz,
                supported_hz: &snap.supported_polling_hz,
                pending_idx: app.pending_polling_idx,
            },
            top_row[1],
        );

        f.render_widget(
            ProfilesPanel {
                focused: app.focus == Focus::Profiles,
                active_profile: snap.active_profile,
                pending_profile: app.pending_profile_idx,
                profiles: &snap.profiles,
            },
            body[1],
        );

        let assignments = snap
            .profiles
            .get(snap.active_profile as usize)
            .and_then(|p| p.as_ref())
            .map(|p| p.button_assignments.as_slice())
            .unwrap_or(&[]);

        f.render_widget(
            ButtonsPanel {
                focused: app.focus == Focus::Buttons,
                assignments,
                selected: app.selected_button,
                pending_action: app.pending_button_action.as_ref(),
            },
            body[2],
        );
    } else {
        // Show bordered loading placeholders so the layout looks complete
        f.render_widget(loading_panel("DPI"), top_row[0]);
        f.render_widget(loading_panel("Polling Rate"), top_row[1]);
        f.render_widget(loading_panel("Profiles"), body[1]);
        f.render_widget(loading_panel("Buttons"), body[2]);
    }

    // Status bar
    f.render_widget(
        StatusBar {
            msg: app.status_msg.as_ref(),
            focus: app.focus,
            button_editing: app.pending_button_action.is_some(),
        },
        outer[2],
    );
}

fn loading_panel(label: &'static str) -> impl ratatui::widgets::Widget {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(
            format!(" {label} "),
            Style::default().fg(DIM).add_modifier(Modifier::BOLD),
        ));
    Paragraph::new(Line::from(Span::styled(
        "loading…",
        Style::default().fg(DIM),
    )))
    .block(block)
}
