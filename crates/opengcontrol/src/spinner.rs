use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

use crate::output::OutputFormat;

// Braille spinner frames — smooth rotation
const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// An optionally-animated spinner.
/// In JSON mode (`OutputFormat::Json`) all methods are no-ops so machine output
/// is never polluted with control codes or partial lines.
pub struct Spinner {
    inner: Option<ProgressBar>,
}

impl Spinner {
    /// Create a spinner visible only in human output mode.
    pub fn new(msg: impl Into<String>, output: OutputFormat) -> Self {
        match output {
            OutputFormat::Human => {
                let pb = ProgressBar::new_spinner();
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .tick_strings(FRAMES)
                        // {spinner:.cyan} renders the frame in Logitech G cyan via indicatif's
                        // built-in ANSI colour support.
                        .template("{spinner:.cyan}  {msg}")
                        .unwrap(),
                );
                pb.set_message(msg.into());
                pb.enable_steady_tick(Duration::from_millis(80));
                Self { inner: Some(pb) }
            }
            OutputFormat::Json => Self { inner: None },
        }
    }

    /// Finish with a green ✓ success line.
    pub fn finish_ok(self, msg: impl Into<String>) {
        if let Some(pb) = self.inner {
            // Clear the spinner line then print styled success beneath it
            pb.finish_and_clear();
            crate::style::print_ok(&msg.into());
        }
    }

    /// Finish with a red ✗ error line (written to stderr).
    pub fn finish_err(self, msg: impl Into<String>) {
        if let Some(pb) = self.inner {
            pb.finish_and_clear();
            crate::style::print_err(&msg.into());
        }
    }

    /// Silently remove the spinner (used when a later step handles output).
    pub fn clear(self) {
        if let Some(pb) = self.inner {
            pb.finish_and_clear();
        }
    }
}
