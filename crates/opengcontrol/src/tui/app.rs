use std::time::Instant;

use hidpp_core::features::dpi::DpiList;

use super::device_state::DeviceSnapshot;

#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    Dpi,
    Polling,
    Profiles,
    Buttons,
}

impl Focus {
    pub fn next(self) -> Self {
        match self {
            Self::Dpi => Self::Polling,
            Self::Polling => Self::Profiles,
            Self::Profiles => Self::Buttons,
            Self::Buttons => Self::Dpi,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Dpi => Self::Buttons,
            Self::Polling => Self::Dpi,
            Self::Profiles => Self::Polling,
            Self::Buttons => Self::Profiles,
        }
    }
}

pub enum StatusKind {
    Ok,
    Error,
}

pub struct AppState {
    pub focus: Focus,
    pub snapshot: Option<DeviceSnapshot>,

    /// Device identity known from registry before the first snapshot arrives.
    pub device_name: &'static str,
    pub device_vid: u16,
    pub device_pid: u16,

    /// Precomputed list of valid DPI values (for Range devices, expanded from min/step/max).
    pub dpi_values: Vec<u16>,
    /// Index into dpi_values for the pending/selected DPI.
    pub pending_dpi_idx: usize,

    /// Index into supported_polling_hz for the pending/selected polling rate.
    pub pending_polling_idx: usize,

    pub status_msg: Option<(String, StatusKind, Instant)>,
    pub refreshing: bool,
    pub should_quit: bool,

    /// When Some, the user is typing a custom DPI value (text input mode).
    pub dpi_input: Option<String>,
}

impl AppState {
    pub fn new(name: &'static str, vid: u16, pid: u16) -> Self {
        Self {
            focus: Focus::Dpi,
            snapshot: None,
            device_name: name,
            device_vid: vid,
            device_pid: pid,
            dpi_values: Vec::new(),
            pending_dpi_idx: 0,
            pending_polling_idx: 0,
            status_msg: None,
            refreshing: true,
            should_quit: false,
            dpi_input: None,
        }
    }

    /// Update state from a fresh device snapshot.
    pub fn apply_snapshot(&mut self, snap: DeviceSnapshot) {
        // Recompute DPI values list
        self.dpi_values = match &snap.dpi_list {
            DpiList::Discrete(list) => list.clone(),
            DpiList::Range { min, step, max } => {
                let step = (*step).max(1) as u32;
                let mut v = Vec::new();
                let mut val = *min as u32;
                while val <= *max as u32 {
                    v.push(val as u16);
                    val += step;
                }
                v
            }
        };

        // Set pending DPI index to current value
        self.pending_dpi_idx = self
            .dpi_values
            .iter()
            .position(|&v| v == snap.current_dpi)
            .unwrap_or(0);

        // Set pending polling index to current value
        self.pending_polling_idx = snap
            .supported_polling_hz
            .iter()
            .position(|&v| v == snap.current_polling_hz)
            .unwrap_or(0);

        self.snapshot = Some(snap);
        self.refreshing = false;
    }

    pub fn pending_dpi(&self) -> Option<u16> {
        self.dpi_values.get(self.pending_dpi_idx).copied()
    }

    pub fn pending_polling(&self) -> Option<u16> {
        self.snapshot
            .as_ref()
            .and_then(|s| s.supported_polling_hz.get(self.pending_polling_idx).copied())
    }

    pub fn dpi_step_right(&mut self) {
        if !self.dpi_values.is_empty() {
            self.pending_dpi_idx =
                (self.pending_dpi_idx + 1).min(self.dpi_values.len() - 1);
        }
    }

    pub fn dpi_step_left(&mut self) {
        if self.pending_dpi_idx > 0 {
            self.pending_dpi_idx -= 1;
        }
    }

    pub fn polling_step_right(&mut self) {
        if let Some(snap) = &self.snapshot {
            let len = snap.supported_polling_hz.len();
            if len > 0 {
                self.pending_polling_idx = (self.pending_polling_idx + 1).min(len - 1);
            }
        }
    }

    pub fn polling_step_left(&mut self) {
        if self.pending_polling_idx > 0 {
            self.pending_polling_idx -= 1;
        }
    }

    pub fn set_status(&mut self, msg: String, kind: StatusKind) {
        self.status_msg = Some((msg, kind, Instant::now()));
    }

    pub fn dpi_input_append(&mut self, c: char) {
        let s = self.dpi_input.get_or_insert_with(String::new);
        if s.len() < 5 {
            s.push(c);
        }
    }

    pub fn dpi_input_backspace(&mut self) {
        if let Some(s) = &mut self.dpi_input {
            s.pop();
            if s.is_empty() {
                self.dpi_input = None;
            }
        }
    }

    pub fn dpi_input_cancel(&mut self) {
        self.dpi_input = None;
    }

    /// Consume the typed string and return it, if any.
    pub fn take_dpi_input(&mut self) -> Option<String> {
        self.dpi_input.take()
    }

    pub fn tick_status(&mut self) {
        if let Some((_, _, ts)) = &self.status_msg {
            if ts.elapsed().as_secs() >= 3 {
                self.status_msg = None;
            }
        }
    }
}
