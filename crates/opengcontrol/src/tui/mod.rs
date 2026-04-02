mod app;
mod device_state;
mod event;
mod panels;
mod poller;
mod selector;
mod ui;

use std::io;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, style::Color, Terminal};

use crate::context::{DeviceContext, PhysicalDevice};
use crate::output::OutputFormat;
use app::{AppState, Focus, StatusKind};
use event::AppEvent;
use poller::PollerCommand;

// Logitech G color palette
pub const CYAN: Color = Color::Rgb(0, 180, 255);
pub const GREEN: Color = Color::Rgb(0, 220, 80);
pub const AMBER: Color = Color::Rgb(255, 184, 0);
pub const RED: Color = Color::Rgb(255, 59, 59);
pub const DIM: Color = Color::Rgb(90, 90, 90);

pub fn run(ctx: DeviceContext) -> Result<(), io::Error> {
    with_terminal(|terminal| run_main(terminal, ctx))
}

/// Entry point when multiple devices are connected and no `--device` flag was given.
/// Shows the device selector screen first, then opens the chosen device.
pub fn run_with_selection(
    devices: Vec<PhysicalDevice>,
    output: OutputFormat,
) -> Result<(), io::Error> {
    with_terminal(|terminal| {
        let Some(path) = selector::run(terminal, &devices)? else {
            return Ok(());
        };
        let ctx = DeviceContext::open(Some(&path), output)
            .map_err(|e| io::Error::other(e.to_string()))?;
        run_main(terminal, ctx)
    })
}

/// Set up the terminal, run `f`, then always restore.
fn with_terminal<F>(f: F) -> Result<(), io::Error>
where
    F: FnOnce(&mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), io::Error>,
{
    // Panic hook: restore terminal on panic so the shell isn't left in raw mode
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = f(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_main(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ctx: DeviceContext,
) -> Result<(), io::Error> {
    let info = ctx.device_info();
    let mut app = AppState::new(info.name, info.vid, info.pid);

    let (event_tx, event_rx) = mpsc::sync_channel::<AppEvent>(32);
    let cmd_tx = poller::spawn_poller(ctx, event_tx);

    event_loop(terminal, &mut app, &event_rx, &cmd_tx)
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
    event_rx: &mpsc::Receiver<AppEvent>,
    cmd_tx: &mpsc::SyncSender<PollerCommand>,
) -> Result<(), io::Error> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        // Drain device events (non-blocking)
        while let Ok(ev) = event_rx.try_recv() {
            handle_device_event(app, ev);
        }

        // Block up to 16ms for a keypress (~60 Hz)
        if crossterm::event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = crossterm::event::read()? {
                handle_key(app, key, cmd_tx);
            }
        }

        // Expire status message after 3 seconds
        app.tick_status();

        if app.should_quit {
            cmd_tx.send(PollerCommand::Quit).ok();
            break;
        }
    }
    Ok(())
}

fn handle_device_event(app: &mut AppState, ev: AppEvent) {
    match ev {
        AppEvent::DeviceData(Ok(snap)) => {
            app.apply_snapshot(snap);
        }
        AppEvent::DeviceData(Err(e)) => {
            app.set_status(format!("Read error: {e}"), StatusKind::Error);
            app.refreshing = false;
        }
        AppEvent::WriteResult(Ok(())) => {
            let msg = match app.focus {
                Focus::Dpi => {
                    let dpi = app.pending_dpi().unwrap_or(0);
                    format!("✓  DPI set to {dpi}")
                }
                Focus::Polling => {
                    let hz = app.pending_polling().unwrap_or(0);
                    format!("✓  Polling rate set to {hz} Hz")
                }
                Focus::Profiles => {
                    format!("✓  Profile {} activated", app.pending_profile_idx + 1)
                }
                Focus::Buttons => {
                    format!("✓  Button {} reassigned", app.selected_button + 1)
                }
            };
            app.set_status(msg, StatusKind::Ok);
        }
        AppEvent::WriteResult(Err(e)) => {
            app.set_status(format!("✗  {e}"), StatusKind::Error);
        }
    }
}

fn handle_key(
    app: &mut AppState,
    key: crossterm::event::KeyEvent,
    cmd_tx: &mpsc::SyncSender<PollerCommand>,
) {
    // --- Button edit mode (must intercept before global Esc handling) ---
    if app.focus == Focus::Buttons && app.pending_button_action.is_some() {
        match key.code {
            KeyCode::Esc => {
                app.button_edit_cancel();
                return;
            }
            KeyCode::Left => {
                app.button_action_prev();
                return;
            }
            KeyCode::Right => {
                app.button_action_next();
                return;
            }
            KeyCode::Enter => {
                if let Some((btn_idx, action)) = app.take_pending_button_action() {
                    if let Some(snap) = &app.snapshot {
                        cmd_tx
                            .send(PollerCommand::SetButtonAction {
                                profile_idx: snap.active_profile,
                                button_idx: btn_idx as u8,
                                action,
                            })
                            .ok();
                        app.refreshing = true;
                    }
                }
                return;
            }
            _ => return,
        }
    }

    // --- DPI text input mode ---
    if app.focus == Focus::Dpi && app.dpi_input.is_some() {
        match key.code {
            KeyCode::Esc => {
                app.dpi_input_cancel();
                return;
            }
            KeyCode::Backspace => {
                app.dpi_input_backspace();
                return;
            }
            KeyCode::Enter => {
                if let Some(s) = app.take_dpi_input() {
                    apply_typed_dpi(app, &s, cmd_tx);
                }
                return;
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                app.dpi_input_append(c);
                return;
            }
            _ => return, // ignore other keys in input mode
        }
    }

    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }

        // Focus navigation
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.focus = app.focus.prev();
            } else {
                app.focus = app.focus.next();
            }
        }

        // Arrow up/down: cycle focus
        KeyCode::Down => {
            app.focus = app.focus.next();
        }
        KeyCode::Up => {
            app.focus = app.focus.prev();
        }

        // Arrow left/right: change value in focused editable panel
        KeyCode::Left => match app.focus {
            Focus::Dpi => app.dpi_step_left(),
            Focus::Polling => app.polling_step_left(),
            Focus::Profiles => app.profile_step_left(),
            Focus::Buttons => {
                let assignments = button_assignments(app);
                app.button_select_prev(&assignments);
            }
        },
        KeyCode::Right => match app.focus {
            Focus::Dpi => app.dpi_step_right(),
            Focus::Polling => app.polling_step_right(),
            Focus::Profiles => app.profile_step_right(),
            Focus::Buttons => {
                let assignments = button_assignments(app);
                app.button_select_next(&assignments);
            }
        },

        // Enter: apply pending value
        KeyCode::Enter => match app.focus {
            Focus::Dpi => {
                if let Some(dpi) = app.pending_dpi() {
                    cmd_tx.send(PollerCommand::SetDpi(dpi)).ok();
                    app.refreshing = true;
                }
            }
            Focus::Polling => {
                if let Some(hz) = app.pending_polling() {
                    cmd_tx.send(PollerCommand::SetPolling(hz)).ok();
                    app.refreshing = true;
                }
            }
            Focus::Profiles => {
                cmd_tx
                    .send(PollerCommand::SwitchProfile(app.pending_profile_idx as u8))
                    .ok();
                app.refreshing = true;
            }
            Focus::Buttons => {
                let assignments = button_assignments(app);
                app.button_edit_start(&assignments);
            }
        },

        // Digit keys on DPI panel: start text input mode
        KeyCode::Char(c) if c.is_ascii_digit() && app.focus == Focus::Dpi => {
            app.dpi_input_append(c);
        }

        // Refresh
        KeyCode::Char('r') => {
            cmd_tx.send(PollerCommand::Refresh).ok();
            app.refreshing = true;
        }

        _ => {}
    }
}

/// Borrow button assignments from the current active profile snapshot.
fn button_assignments(app: &AppState) -> Vec<hidpp_core::features::ButtonAssignment> {
    app.snapshot
        .as_ref()
        .and_then(|s| s.profiles.get(s.active_profile as usize))
        .and_then(|p| p.as_ref())
        .map(|p| p.button_assignments.clone())
        .unwrap_or_default()
}

fn apply_typed_dpi(app: &mut AppState, s: &str, cmd_tx: &mpsc::SyncSender<PollerCommand>) {
    let Ok(dpi) = s.parse::<u16>() else {
        app.set_status(format!("✗  Not a number: {s}"), StatusKind::Error);
        return;
    };

    let valid = app
        .snapshot
        .as_ref()
        .map(|snap| snap.dpi_list.is_valid(dpi))
        .unwrap_or(false);

    if valid {
        cmd_tx.send(PollerCommand::SetDpi(dpi)).ok();
        app.refreshing = true;
    } else if let Some(snap) = &app.snapshot {
        app.set_status(
            format!("✗  {} is not valid — {}", dpi, snap.dpi_list.describe()),
            StatusKind::Error,
        );
    }
}
