use clap::Args;
use hidapi::HidApi;
use logitech_devices::enumerate_supported_devices;

#[cfg(target_os = "macos")]
use crate::permissions::open_input_monitoring_settings;
use crate::style::{self, bold, dim, g_cyan_bold, CheckState};

#[derive(Args)]
pub struct DoctorArgs {
    /// Open System Settings to the Input Monitoring pane (macOS only)
    #[cfg(target_os = "macos")]
    #[arg(long)]
    pub open_settings: bool,
}

pub fn handle_doctor(args: &DoctorArgs) -> Result<(), String> {
    println!();
    println!(
        "  {}  {}",
        g_cyan_bold("opengcontrol"),
        dim("doctor — system diagnostics")
    );
    style::print_rule();
    println!();

    // ── Check 1: HidApi init ───────────────────────────────────────────────
    let api = match HidApi::new() {
        Ok(api) => {
            style::print_check(CheckState::Ok, "HID API initialized", "");
            api
        }
        Err(e) => {
            style::print_check(
                CheckState::Fail,
                "HID API initialization failed",
                &e.to_string(),
            );
            println!();
            #[cfg(target_os = "macos")]
            println!("  {}  Try: {}", dim("hint"), bold("brew install hidapi"));
            #[cfg(target_os = "windows")]
            println!(
                "  {}  HID API init failed. Ensure the device is connected.",
                dim("hint")
            );
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            println!(
                "  {}  Try installing hidapi: {}",
                dim("hint"),
                bold("sudo apt install libhidapi-dev")
            );
            println!();
            return Ok(());
        }
    };

    // ── Check 2: Device enumeration ────────────────────────────────────────
    let all_count = api.device_list().count();
    if all_count == 0 {
        style::print_check(
            CheckState::Warn,
            "No HID devices visible",
            "may be a permission issue",
        );
    } else {
        style::print_check(
            CheckState::Ok,
            "HID devices visible",
            &format!("{all_count} total"),
        );
    }

    // ── Check 3: Supported Logitech devices ────────────────────────────────
    let supported = enumerate_supported_devices(&api);

    // Deduplicate by VID:PID
    let mut seen = std::collections::HashSet::new();
    let unique_devices: Vec<_> = supported
        .iter()
        .filter(|(info, _)| seen.insert((info.vid, info.pid)))
        .collect();

    if unique_devices.is_empty() {
        style::print_check(
            CheckState::Warn,
            "No supported Logitech mice found",
            "ensure USB is connected",
        );
        println!();
        println!("  {}  Supported devices:", dim("info"));
        println!("       {} G403 Prodigy  (046D:C083)", dim("·"));
        println!("       {} G403 HERO     (046D:C08F)", dim("·"));
        println!("       {} G403 Wireless (046D:C082)", dim("·"));
    } else {
        style::print_check(
            CheckState::Ok,
            "Supported Logitech device found",
            &format!("{} device(s)", unique_devices.len()),
        );
        for (info, hid_info) in &unique_devices {
            println!(
                "       {} {}  {}",
                dim("·"),
                style::g_cyan(info.name),
                dim(&format!(
                    "({:04X}:{:04X})",
                    hid_info.vendor_id(),
                    hid_info.product_id()
                ))
            );
        }
    }

    // ── Check 4: Device open ───────────────────────────────────────────────
    if let Some((_info, hid_info)) = supported.first() {
        match api.open_path(hid_info.path()) {
            Ok(_) => {
                style::print_check(CheckState::Ok, "Device opened successfully", "");
            }
            Err(e) => {
                let msg = e.to_string().to_lowercase();
                if msg.contains("permission")
                    || msg.contains("operation not permitted")
                    || msg.contains("access denied")
                {
                    style::print_check(CheckState::Fail, "Permission denied", "device is locked");
                    println!();

                    #[cfg(target_os = "macos")]
                    {
                        println!("  {}  To fix:", dim("fix"));
                        println!(
                            "       1. Open {} > {} > {}",
                            bold("System Settings"),
                            bold("Privacy & Security"),
                            bold("Input Monitoring")
                        );
                        println!("       2. Add {} to the list", bold("opengcontrol"));
                        println!("       3. Toggle off then back on if already listed");

                        if args.open_settings {
                            println!();
                            println!("  {}  Opening System Settings…", dim("›"));
                            open_input_monitoring_settings();
                        } else {
                            println!();
                            println!(
                                "  {}  Run {} to open System Settings automatically.",
                                dim("tip"),
                                bold("opengcontrol doctor --open-settings")
                            );
                        }
                    }

                    #[cfg(target_os = "windows")]
                    println!(
                        "  {}  Close Logitech G HUB or Logi Options+ and try again.",
                        dim("fix")
                    );

                    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                    println!("  {}  Check udev rules or run with sudo.", dim("fix"));
                } else {
                    style::print_check(CheckState::Fail, "Could not open device", &e.to_string());
                    println!();
                    println!(
                        "  {}  Close Logitech G HUB or Logi Options+ and try again.",
                        dim("hint")
                    );
                }
            }
        }
    } else {
        style::print_check(CheckState::Skip, "Device open test", "skipped (no device)");
    }

    // ── Check 5: Conflicting software ──────────────────────────────────────
    let conflicts: Vec<&str> = ["lghub", "logioptionsplus", "LogiMgr"]
        .iter()
        .copied()
        .filter(|name| is_process_running(name))
        .collect();

    if conflicts.is_empty() {
        style::print_check(CheckState::Ok, "No conflicting software running", "");
    } else {
        style::print_check(
            CheckState::Warn,
            "Conflicting software detected",
            &conflicts.join(", "),
        );
        println!(
            "  {}  Quit {} before using opengcontrol.",
            dim("fix"),
            bold(&conflicts.join(" / "))
        );
    }

    println!();
    style::print_rule();
    println!();

    Ok(())
}

#[cfg(unix)]
fn is_process_running(name: &str) -> bool {
    std::process::Command::new("pgrep")
        .args(["-ix", name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_process_running(name: &str) -> bool {
    let output = std::process::Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {name}.exe"), "/NH"])
        .output();
    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.to_lowercase().contains(&name.to_lowercase())
        }
        Err(_) => false,
    }
}
