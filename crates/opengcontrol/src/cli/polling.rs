use clap::{Args, Subcommand};
use hidpp_core::features::{OnboardProfiles, PollingRate};
use serde::Serialize;

use crate::context::DeviceContext;
use crate::output::{json, OutputFormat};
use crate::spinner::Spinner;
use crate::style;

#[derive(Args)]
pub struct PollingArgs {
    #[command(subcommand)]
    pub command: PollingCommand,
}

#[derive(Subcommand)]
pub enum PollingCommand {
    /// Print the current polling rate in Hz
    Get,
    /// Set the polling rate
    Set {
        /// Rate in Hz — one of: 125, 250, 500, 1000
        hz: u16,
    },
    /// List supported polling rates for this device
    List,
}

#[derive(Serialize)]
struct PollingResult {
    rate_hz: u16,
}

pub fn handle_polling(
    ctx: &DeviceContext,
    cmd: &PollingCommand,
    output: OutputFormat,
) -> Result<(), String> {
    match cmd {
        PollingCommand::Get => {
            let sp = Spinner::new("Reading polling rate…", output);
            let hz = match PollingRate::get_rate_hz(ctx.device()) {
                Ok(v) => {
                    sp.clear();
                    v
                }
                Err(e) => {
                    sp.finish_err("Failed to read polling rate");
                    return Err(e.to_string());
                }
            };
            match output {
                OutputFormat::Human => println!(
                    "  {}  Polling rate  {}",
                    style::g_green(style::SYM_OK),
                    style::g_cyan_bold(&format!("{hz} Hz"))
                ),
                OutputFormat::Json => json::print_json(&PollingResult { rate_hz: hz }),
            }
        }

        PollingCommand::Set { hz } => {
            let sp = Spinner::new(format!("Setting polling rate to {hz} Hz…"), output);
            // On sector-addressed (onboard-profile) mice the rate lives in the active
            // profile; the standalone 0x8060 feature rejects a direct set in onboard mode,
            // so write the profile instead.
            let sector_model = OnboardProfiles::get_info(ctx.device())
                .map(|i| i.memory_model == 0x01)
                .unwrap_or(false);
            let result = if sector_model {
                OnboardProfiles::set_poll_rate(ctx.device(), *hz)
            } else {
                let supported = ctx.device_info().supported_polling_rates();
                PollingRate::set_rate_hz(ctx.device(), *hz, supported)
            };
            match result {
                Ok(()) => {}
                Err(e) => {
                    sp.finish_err(format!("Failed — {e}"));
                    return Err(e.to_string());
                }
            }
            match output {
                OutputFormat::Human => sp.finish_ok(format!(
                    "Polling rate set to  {}{}",
                    style::g_cyan_bold(&format!("{hz} Hz")),
                    if sector_model {
                        style::dim("  (active profile)")
                    } else {
                        String::new()
                    }
                )),
                OutputFormat::Json => {
                    sp.clear();
                    json::print_json(&PollingResult { rate_hz: *hz });
                }
            }
        }

        PollingCommand::List => {
            let rates = ctx.device_info().supported_polling_rates();
            let list_str = rates
                .iter()
                .map(|r| format!("{r} Hz"))
                .collect::<Vec<_>>()
                .join("  ");
            match output {
                OutputFormat::Human => {
                    println!();
                    style::print_kv("Supported rates", &style::g_cyan(&list_str));
                    println!();
                }
                OutputFormat::Json => json::print_json(&serde_json::json!({ "rates_hz": rates })),
            }
        }
    }

    Ok(())
}
