//! Visual design for opengcontrol.
//!
//! Color palette inspired by Logitech G branding:
//!   Primary:  #00B4FF  — Logitech G cyan/blue
//!   Success:  #00DC50  — neon green
//!   Warning:  #FFB800  — amber
//!   Error:    #FF3B3B  — red

use owo_colors::{DynColors, OwoColorize, Style};
use owo_colors::Stream::Stdout;

// ─── Symbols ────────────────────────────────────────────────────────────────

pub const SYM_DEVICE: &str = "◈";
pub const SYM_OK:     &str = "✓";
pub const SYM_FAIL:   &str = "✗";
pub const SYM_WARN:   &str = "⚠";
pub const SYM_ARROW:  &str = "›";
pub const SYM_DOT:    &str = "·";
pub const SYM_DASH:   &str = "─";

// ─── Pre-built styles ────────────────────────────────────────────────────────

fn rgb(r: u8, g: u8, b: u8) -> Style {
    Style::new().color(DynColors::Rgb(r, g, b))
}

fn rgb_bold(r: u8, g: u8, b: u8) -> Style {
    Style::new().bold().color(DynColors::Rgb(r, g, b))
}

fn dimmed_style() -> Style {
    Style::new().dimmed()
}

fn bold_style() -> Style {
    Style::new().bold()
}

// ─── Color helpers ──────────────────────────────────────────────────────────

/// Logitech G primary cyan #00B4FF
pub fn g_cyan(s: &str) -> String {
    s.if_supports_color(Stdout, |t| t.style(rgb(0, 180, 255))).to_string()
}

/// Logitech G cyan, bold — device names, headers, key values
pub fn g_cyan_bold(s: &str) -> String {
    s.if_supports_color(Stdout, |t| t.style(rgb_bold(0, 180, 255))).to_string()
}

/// Neon green #00DC50 — success values and checkmarks
pub fn g_green(s: &str) -> String {
    s.if_supports_color(Stdout, |t| t.style(rgb(0, 220, 80))).to_string()
}

/// Amber #FFB800 — warnings
pub fn g_amber(s: &str) -> String {
    s.if_supports_color(Stdout, |t| t.style(rgb(255, 184, 0))).to_string()
}

/// Red #FF3B3B — errors
pub fn g_red(s: &str) -> String {
    s.if_supports_color(Stdout, |t| t.style(rgb(255, 59, 59))).to_string()
}

/// Dimmed — secondary info (paths, VID:PID)
pub fn dim(s: &str) -> String {
    s.if_supports_color(Stdout, |t| t.style(dimmed_style())).to_string()
}

/// Bold white — labels and hints
pub fn bold(s: &str) -> String {
    s.if_supports_color(Stdout, |t| t.style(bold_style())).to_string()
}

// ─── Formatted output lines ──────────────────────────────────────────────────

/// ✓  message
pub fn print_ok(msg: &str) {
    println!("  {}  {}", g_green(SYM_OK), msg);
}

/// ✗  message  (stderr)
pub fn print_err(msg: &str) {
    eprintln!("  {}  {}", g_red(SYM_FAIL), msg);
}

/// Two-column key / value row
pub fn print_kv(key: &str, value: &str) {
    println!("  {:<18}{}", dim(key), value);
}

/// Horizontal rule in dim
pub fn print_rule() {
    println!("  {}", dim(&SYM_DASH.repeat(46)));
}

// ─── Doctor check lines ──────────────────────────────────────────────────────

pub enum CheckState { Ok, Fail, Warn, Skip }

pub fn print_check(state: CheckState, label: &str, detail: &str) {
    let sym = match state {
        CheckState::Ok   => g_green(SYM_OK),
        CheckState::Fail => g_red(SYM_FAIL),
        CheckState::Warn => g_amber(SYM_WARN),
        CheckState::Skip => dim(SYM_DOT),
    };
    if detail.is_empty() {
        println!("  {sym}  {label}");
    } else {
        println!("  {sym}  {label:<38}{}", dim(detail));
    }
}
