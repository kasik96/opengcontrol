use clap::{Args, Subcommand};
use hidpp_core::features::AdjustableDpi;
use serde::Serialize;

use crate::context::DeviceContext;
use crate::output::{json, OutputFormat};
use crate::spinner::Spinner;
use crate::style;

#[derive(Args)]
pub struct DpiArgs {
    #[command(subcommand)]
    pub command: DpiCommand,
}

#[derive(Subcommand)]
pub enum DpiCommand {
    /// Print the current DPI setting
    Get,
    /// Set the DPI value (takes effect immediately; use 'profile' to persist to flash)
    Set {
        /// DPI value — must be a multiple of 50 (e.g. 400, 800, 1600)
        value: u16,
        /// Sensor index (default 0; most mice have a single sensor)
        #[arg(long, default_value = "0")]
        sensor: u8,
    },
    /// List all supported DPI values for this device
    List,
}

#[derive(Serialize)]
struct DpiResult {
    dpi: u16,
    sensor: u8,
}

#[derive(Serialize)]
struct DpiListResult {
    available: String,
}

pub fn handle_dpi(
    ctx: &DeviceContext,
    cmd: &DpiCommand,
    output: OutputFormat,
) -> Result<(), String> {
    match cmd {
        DpiCommand::Get => {
            let sp = Spinner::new("Reading DPI…", output);
            let dpi = match AdjustableDpi::get_dpi(ctx.device(), 0) {
                Ok(v) => { sp.clear(); v }
                Err(e) => { sp.finish_err("Failed to read DPI"); return Err(e.to_string()); }
            };
            match output {
                OutputFormat::Human => println!(
                    "  {}  DPI  {}",
                    style::g_green(style::SYM_OK),
                    style::g_cyan_bold(&dpi.to_string())
                ),
                OutputFormat::Json => json::print_json(&DpiResult { dpi, sensor: 0 }),
            }
        }

        DpiCommand::Set { value, sensor } => {
            ctx.device_info().validate_dpi(*value)?;

            let sp = Spinner::new(format!("Setting DPI to {value}…"), output);
            match AdjustableDpi::set_dpi(ctx.device(), *sensor, *value) {
                Ok(()) => {}
                Err(e) => {
                    sp.finish_err(format!("Failed to set DPI to {value}"));
                    return Err(e.to_string());
                }
            }
            match output {
                OutputFormat::Human => sp.finish_ok(format!(
                    "DPI set to  {}",
                    style::g_cyan_bold(&value.to_string())
                )),
                OutputFormat::Json => {
                    sp.clear();
                    json::print_json(&DpiResult { dpi: *value, sensor: *sensor });
                }
            }
        }

        DpiCommand::List => {
            let sp = Spinner::new("Querying DPI range…", output);
            let list = match AdjustableDpi::get_dpi_list(ctx.device(), 0) {
                Ok(l) => { sp.clear(); l }
                Err(e) => { sp.finish_err("Failed to read DPI list"); return Err(e.to_string()); }
            };
            let desc = list.describe();
            match output {
                OutputFormat::Human => {
                    println!();
                    style::print_kv("Supported DPI", &style::g_cyan(&desc));
                    println!();
                }
                OutputFormat::Json => json::print_json(&DpiListResult { available: desc }),
            }
        }
    }

    Ok(())
}
