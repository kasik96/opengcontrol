use std::path::PathBuf;

use clap::{Args, Subcommand};
use hidpp_core::features::{OnboardProfile, OnboardProfiles};
use serde::Serialize;

use crate::context::DeviceContext;
use crate::output::{json, OutputFormat};
use crate::spinner::Spinner;
use crate::style::{self, dim, g_cyan, g_cyan_bold};

#[derive(Args)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub command: ProfileCommand,
}

#[derive(Subcommand)]
pub enum ProfileCommand {
    /// List all onboard profiles with their DPI slots and polling rate
    List,
    /// Show the currently active profile index
    Active,
    /// Activate a profile by index
    Switch {
        /// Profile index (0-based)
        index: u8,
    },
    /// Export a profile to a TOML file
    Export {
        /// Profile index to export
        index: u8,
        /// Output file path
        output: PathBuf,
    },
    /// Import and write a profile from a TOML file to the mouse
    Import {
        /// TOML file previously exported with 'profile export'
        file: PathBuf,
    },
}

#[derive(Serialize)]
struct ActiveProfileResult {
    active_profile: u8,
}

pub fn handle_profile(
    ctx: &DeviceContext,
    cmd: &ProfileCommand,
    output: OutputFormat,
) -> Result<(), String> {
    match cmd {
        ProfileCommand::List => {
            let sp = Spinner::new("Reading onboard profiles…", output);

            let info = match OnboardProfiles::get_info(ctx.device()) {
                Ok(v) => v,
                Err(e) => {
                    sp.finish_err("Failed to read profile info");
                    return Err(e.to_string());
                }
            };
            let active = match OnboardProfiles::get_active_profile(ctx.device()) {
                Ok(v) => v,
                Err(e) => {
                    sp.finish_err("Failed to read active profile");
                    return Err(e.to_string());
                }
            };

            let mut profiles = Vec::new();
            for i in 0..info.profile_count {
                profiles.push((i, OnboardProfiles::read_profile(ctx.device(), i).ok()));
            }
            sp.clear();

            match output {
                OutputFormat::Human => {
                    println!();
                    for (i, maybe_profile) in &profiles {
                        let is_active = *i == active;
                        let header = if is_active {
                            format!(
                                "  {}  {}",
                                g_cyan(style::SYM_DEVICE),
                                g_cyan_bold(&format!("Profile {i}  (active)"))
                            )
                        } else {
                            format!("     {}", dim(&format!("Profile {i}")))
                        };
                        println!("{header}");

                        if let Some(p) = maybe_profile {
                            let dpi_str = p
                                .dpi_slots
                                .iter()
                                .enumerate()
                                .map(|(slot, dpi)| {
                                    if slot == p.active_dpi_slot as usize {
                                        g_cyan_bold(&dpi.to_string())
                                    } else {
                                        dim(&dpi.to_string())
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("  ");

                            println!("     {:<14}{}", dim("DPI slots"), dpi_str);
                            println!(
                                "     {:<14}{}",
                                dim("Poll rate"),
                                g_cyan(&format!("{} Hz", p.polling_rate_hz))
                            );
                        } else {
                            println!("     {}", dim("(unreadable)"));
                        }
                        println!();
                    }
                }
                OutputFormat::Json => {
                    let data: Vec<_> = profiles
                        .iter()
                        .map(|(i, p)| {
                            serde_json::json!({
                                "index": i,
                                "active": *i == active,
                                "profile": p.as_ref().map(|p| serde_json::json!({
                                    "dpi_slots": p.dpi_slots,
                                    "active_dpi_slot": p.active_dpi_slot,
                                    "polling_rate_hz": p.polling_rate_hz,
                                }))
                            })
                        })
                        .collect();
                    json::print_json(&serde_json::json!({ "active": active, "profiles": data }));
                }
            }
        }

        ProfileCommand::Active => {
            let sp = Spinner::new("Querying active profile…", output);
            let active = match OnboardProfiles::get_active_profile(ctx.device()) {
                Ok(v) => {
                    sp.clear();
                    v
                }
                Err(e) => {
                    sp.finish_err("Failed to read active profile");
                    return Err(e.to_string());
                }
            };
            match output {
                OutputFormat::Human => println!(
                    "  {}  Active profile  {}",
                    style::g_green(style::SYM_OK),
                    g_cyan_bold(&active.to_string())
                ),
                OutputFormat::Json => json::print_json(&ActiveProfileResult {
                    active_profile: active,
                }),
            }
        }

        ProfileCommand::Switch { index } => {
            let sp = Spinner::new(format!("Switching to profile {index}…"), output);
            match OnboardProfiles::set_active_profile(ctx.device(), *index) {
                Ok(()) => {}
                Err(e) => {
                    sp.finish_err(format!("Failed to switch to profile {index}"));
                    return Err(e.to_string());
                }
            }
            match output {
                OutputFormat::Human => sp.finish_ok(format!(
                    "Switched to profile  {}",
                    g_cyan_bold(&index.to_string())
                )),
                OutputFormat::Json => {
                    sp.clear();
                    json::print_json(&ActiveProfileResult {
                        active_profile: *index,
                    });
                }
            }
        }

        ProfileCommand::Export {
            index,
            output: out_path,
        } => {
            let sp = Spinner::new(format!("Reading profile {index} from flash…"), output);
            let profile = match OnboardProfiles::read_profile(ctx.device(), *index) {
                Ok(p) => {
                    sp.clear();
                    p
                }
                Err(e) => {
                    sp.finish_err(format!("Failed to read profile {index}"));
                    return Err(e.to_string());
                }
            };

            let toml_str =
                toml::to_string_pretty(&profile).map_err(|e| format!("Serialize error: {e}"))?;
            std::fs::write(out_path, toml_str).map_err(|e| format!("Write error: {e}"))?;

            match output {
                OutputFormat::Human => style::print_ok(&format!(
                    "Profile {} exported → {}",
                    g_cyan_bold(&index.to_string()),
                    out_path.display()
                )),
                OutputFormat::Json => json::print_json(&serde_json::json!({
                    "exported": index,
                    "file": out_path.display().to_string()
                })),
            }
        }

        ProfileCommand::Import { file } => {
            let contents = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
            let profile: OnboardProfile =
                toml::from_str(&contents).map_err(|e| format!("Parse error: {e}"))?;
            let index = profile.index;

            let sp = Spinner::new(format!("Writing profile {index} to flash…"), output);
            match OnboardProfiles::write_profile(ctx.device(), &profile) {
                Ok(()) => {}
                Err(e) => {
                    sp.finish_err(format!("Failed to write profile {index}"));
                    return Err(e.to_string());
                }
            }
            match output {
                OutputFormat::Human => sp.finish_ok(format!(
                    "Profile {} imported from {}",
                    g_cyan_bold(&index.to_string()),
                    file.display()
                )),
                OutputFormat::Json => {
                    sp.clear();
                    json::print_json(&serde_json::json!({
                        "imported": index,
                        "file": file.display().to_string()
                    }));
                }
            }
        }
    }

    Ok(())
}
