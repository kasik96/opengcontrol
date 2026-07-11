use clap::{Parser, Subcommand};

use crate::output::OutputFormat;

pub mod device;
pub mod doctor;
pub mod dpi;
pub mod polling;
pub mod profile;
pub mod selector;
pub mod tui;

#[derive(Parser)]
#[command(
    name = "opengcontrol",
    about = "Open-source CLI alternative to Logitech G software for macOS",
    version,
    propagate_version = true,
    after_help = "Run 'opengcontrol doctor' if you encounter permission or connection issues."
)]
pub struct Cli {
    /// Output format
    #[arg(long, global = true, value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Target a specific device by HID path (useful when multiple mice are connected).
    /// Use 'opengcontrol list' to find device paths.
    #[arg(long, global = true, value_name = "PATH")]
    pub device: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all connected Logitech devices supported by opengcontrol
    #[command(alias = "ls")]
    List,

    /// Show current device settings (DPI, polling rate, active profile)
    Info,

    /// Show battery level and charging status
    Battery,

    /// DPI configuration
    Dpi(dpi::DpiArgs),

    /// Polling rate configuration
    #[command(name = "poll")]
    Polling(polling::PollingArgs),

    /// Onboard profile management
    Profile(profile::ProfileArgs),

    /// Diagnose connectivity and permission issues
    Doctor(doctor::DoctorArgs),

    /// Interactive TUI dashboard — shows all device settings at once
    Tui(tui::TuiArgs),
}
