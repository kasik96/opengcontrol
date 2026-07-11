use std::path::PathBuf;

use clap::{Args, Subcommand};
use hidpp_core::features::{
    disabled_record, key_record, mouse_button_record, CloneOptions, OnboardProfile, OnboardProfiles,
};
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
        #[arg(value_name = "OUTPUT")]
        output_file: PathBuf,
    },
    /// Import and write a profile from a TOML file to the mouse
    Import {
        /// TOML file previously exported with 'profile export'
        file: PathBuf,
    },
    /// Remap one button on a profile (writes onboard flash; sector-addressed devices).
    ///
    /// Backs up the affected sector to a file, writes via read-modify-write, and verifies
    /// by read-back. Reversible: re-run with the previous action, or restore the backup.
    SetButton {
        /// Button index (0-based). The G-Shift bank is button_count..2*button_count.
        button: u8,
        /// left | right | middle | back | forward | dpi | disabled |
        /// key:<F13..F24|0xNN> (optionally ctrl+/shift+/alt+/win+ prefixed)
        action: String,
        /// Profile index (0-based). Defaults to the active profile.
        #[arg(long)]
        profile: Option<u8>,
    },
    /// Clone a profile from another mouse onto this one (sector-addressed devices).
    ///
    /// Reads the source read-only, remaps buttons by physical position (derived from each
    /// model's factory default — no hardcoded tables), and provisions this mouse's user
    /// flash if it is still ROM-only. Always writes destination profile 0 and activates it.
    /// Use --dry-run to preview the plan without writing.
    Clone {
        /// Source device HID path (see 'opengcontrol list'). Opened read-only.
        #[arg(long)]
        from: String,
        /// Source profile index to copy (0-based).
        #[arg(long, default_value_t = 0)]
        profile: u8,
        /// Override the cloned profile's DPI (slot 0).
        #[arg(long)]
        dpi: Option<u16>,
        /// Compute and print the plan without writing.
        #[arg(long)]
        dry_run: bool,
    },
}

/// Parse a button action string into its 4-byte sector-layout record.
fn parse_button_action(s: &str) -> Result<[u8; 4], String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "left" => return Ok(mouse_button_record(0x0001)),
        "right" => return Ok(mouse_button_record(0x0002)),
        "middle" => return Ok(mouse_button_record(0x0004)),
        "back" => return Ok(mouse_button_record(0x0008)),
        "forward" => return Ok(mouse_button_record(0x0010)),
        "dpi" => return Ok(mouse_button_record(0x2000)),
        "disabled" | "none" => return Ok(disabled_record()),
        _ => {}
    }
    if let Some(spec) = s.trim().strip_prefix("func:") {
        let code = match spec.trim().to_ascii_lowercase().as_str() {
            "tilt-left" => 0x01,
            "tilt-right" => 0x02,
            "next-dpi" => 0x03,
            "prev-dpi" => 0x04,
            "cycle-dpi" => 0x05,
            "default-dpi" => 0x06,
            "dpi-shift" | "sniper" => 0x07,
            "next-profile" => 0x08,
            "prev-profile" => 0x09,
            "cycle-profile" => 0x0A,
            "gshift" | "g-shift" => 0x0B,
            "battery" => 0x0C,
            hex => u8::from_str_radix(hex.strip_prefix("0x").unwrap_or(hex), 16)
                .map_err(|_| format!("unknown function '{spec}'"))?,
        };
        // Sector function record: 0x90, <code>, 0x00, <data=0>.
        return Ok([0x90, code, 0x00, 0x00]);
    }
    if let Some(spec) = s.trim().strip_prefix("key:") {
        let tokens: Vec<&str> = spec.split('+').collect();
        let (mods, key_tok) = tokens.split_at(tokens.len() - 1);
        let mut modifiers = 0u8;
        for m in mods {
            modifiers |= match m.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => 0x01,
                "shift" => 0x02,
                "alt" | "option" => 0x04,
                "win" | "gui" | "cmd" | "super" => 0x08,
                other => return Err(format!("unknown modifier '{other}'")),
            };
        }
        return Ok(key_record(modifiers, parse_hid_key(key_tok[0])?));
    }
    Err(format!(
        "unknown action '{s}' (use left|right|middle|back|forward|dpi|disabled|\
         key:<F13-F24|0xNN>|func:<gshift|dpi-shift|cycle-profile|tilt-left|…|0xNN>)"
    ))
}

/// Parse a HID keyboard usage from an `F13`..`F24` name or a `0xNN` hex literal.
fn parse_hid_key(tok: &str) -> Result<u8, String> {
    let t = tok.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return u8::from_str_radix(hex, 16).map_err(|_| format!("bad hex key '{t}'"));
    }
    if let Some(n) = t
        .strip_prefix(['F', 'f'])
        .and_then(|d| d.parse::<u8>().ok())
    {
        if (1..=12).contains(&n) {
            return Ok(0x3A + (n - 1)); // F1..F12
        }
        if (13..=24).contains(&n) {
            return Ok(0x68 + (n - 13)); // F13..F24
        }
        return Err(format!("F{n} out of range (F1..F24)"));
    }
    Err(format!("cannot parse key '{t}' (use F13..F24 or 0xNN)"))
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

            // Iterate the profiles actually present in the directory, not the raw slot
            // capacity — a factory device may expose fewer (e.g. 2 ROM profiles).
            let count = OnboardProfiles::provisioned_profile_count(ctx.device())
                .unwrap_or(info.profile_count);
            let mut profiles = Vec::new();
            for i in 0..count {
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

                            let buttons: Vec<_> = p
                                .button_assignments
                                .iter()
                                .filter(|b| !b.action.is_unassigned())
                                .collect();
                            if !buttons.is_empty() {
                                println!("     {}", dim("Buttons"));
                                for b in buttons {
                                    println!(
                                        "       {:<12}{}",
                                        dim(&format!("Button {}", b.button_index)),
                                        g_cyan(&b.action.describe())
                                    );
                                }
                            }
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
                                    "buttons": p.button_assignments.iter().map(|b| serde_json::json!({
                                        "index": b.button_index,
                                        "action": b.action.describe(),
                                    })).collect::<Vec<_>>(),
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
            output_file: out_path,
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

        ProfileCommand::SetButton {
            button,
            action,
            profile,
        } => {
            let record = parse_button_action(action)?;

            let profile_idx = match profile {
                Some(p) => *p,
                None => OnboardProfiles::get_active_profile(ctx.device())
                    .map_err(|e| format!("Failed to read active profile: {e}"))?,
            };

            // Back up the affected sector before touching it.
            let sector = OnboardProfiles::profile_data_sector(ctx.device(), profile_idx)
                .map_err(|e| e.to_string())?;
            let backup = OnboardProfiles::read_raw_sector(ctx.device(), sector)
                .map_err(|e| e.to_string())?;
            let backup_hex: String = backup.iter().map(|b| format!("{b:02x}")).collect();
            let backup_path = format!("profile{profile_idx}_sector{sector:#06X}_backup.hex");
            std::fs::write(&backup_path, &backup_hex)
                .map_err(|e| format!("Failed to write backup {backup_path}: {e}"))?;

            let sp = Spinner::new(
                format!("Writing button {button} on profile {profile_idx}…"),
                output,
            );
            let original =
                match OnboardProfiles::set_button(ctx.device(), profile_idx, *button, record) {
                    Ok(o) => o,
                    Err(e) => {
                        sp.finish_err(format!("Failed to write button {button}"));
                        return Err(e.to_string());
                    }
                };

            match output {
                OutputFormat::Human => sp.finish_ok(format!(
                    "Button {} on profile {}: {:02X?} → {:02X?}   (backup: {})",
                    g_cyan_bold(&button.to_string()),
                    g_cyan_bold(&profile_idx.to_string()),
                    original,
                    record,
                    backup_path,
                )),
                OutputFormat::Json => {
                    sp.clear();
                    json::print_json(&serde_json::json!({
                        "profile": profile_idx,
                        "button": button,
                        "sector": format!("{sector:#06X}"),
                        "previous": original.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>(),
                        "written": record.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>(),
                        "backup_file": backup_path,
                    }));
                }
            }
        }
        ProfileCommand::Clone {
            from,
            profile,
            dpi,
            dry_run,
        } => {
            let src = DeviceContext::open(Some(from), output).map_err(|e| e.to_string())?;
            let opts = CloneOptions {
                src_profile: *profile,
                dpi_override: *dpi,
                dry_run: *dry_run,
            };
            let sp = Spinner::new(
                if *dry_run {
                    "Computing clone plan…".to_string()
                } else {
                    "Cloning profile…".to_string()
                },
                output,
            );
            let report = match OnboardProfiles::clone_profile(src.device(), ctx.device(), &opts) {
                Ok(r) => r,
                Err(e) => {
                    sp.finish_err("Clone failed");
                    return Err(e.to_string());
                }
            };
            sp.clear();

            match output {
                OutputFormat::Human => {
                    println!();
                    println!(
                        "  {} button remap (source slot → this mouse's slot):",
                        g_cyan(style::SYM_ARROW)
                    );
                    for (s, d) in &report.remap.pairs {
                        println!("     {s:>2} → {d}");
                    }
                    if !report.remap.unmapped_src.is_empty() {
                        println!(
                            "  {} source buttons with no counterpart here (kept factory default): {:?}",
                            dim("!"),
                            report.remap.unmapped_src
                        );
                    }
                    println!();
                    if report.wrote {
                        let how = if report.provisioned {
                            "provisioned user flash and wrote"
                        } else {
                            "overwrote"
                        };
                        println!(
                            "  {} {} profile 0 (sector {:#06X}); activated.",
                            g_cyan_bold("✓"),
                            how,
                            report.target_sector
                        );
                    } else {
                        println!(
                            "  {} dry run — nothing written. Would target sector {:#06X}.",
                            dim("i"),
                            report.target_sector
                        );
                    }
                    println!();
                }
                OutputFormat::Json => json::print_json(&serde_json::json!({
                    "remap": report.remap.pairs,
                    "unmapped_source_slots": report.remap.unmapped_src,
                    "provisioned": report.provisioned,
                    "target_sector": format!("{:#06X}", report.target_sector),
                    "wrote": report.wrote,
                })),
            }
        }
    }

    Ok(())
}
