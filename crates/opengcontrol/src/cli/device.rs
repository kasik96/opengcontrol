use hidapi::HidApi;
use logitech_devices::enumerate_supported_devices;
use serde::Serialize;

use crate::output::{json, OutputFormat};
use crate::style::{self, dim, g_cyan, g_cyan_bold};

#[derive(Serialize)]
struct DeviceEntry {
    name: &'static str,
    vid: String,
    pid: String,
    path: String,
}

pub fn handle_list(output: OutputFormat) -> Result<(), String> {
    let api = HidApi::new().map_err(|e| format!("Failed to initialize HID API: {e}"))?;
    let devices = enumerate_supported_devices(&api);

    // Deduplicate: a mouse exposes multiple HID interfaces; show each device once
    let mut seen = std::collections::HashSet::new();
    let devices: Vec<_> = devices
        .into_iter()
        .filter(|(_info, hid)| seen.insert((hid.vendor_id(), hid.product_id())))
        .collect();

    if devices.is_empty() {
        match output {
            OutputFormat::Human => {
                println!();
                println!("  {}  No supported Logitech devices found.", style::g_amber(style::SYM_WARN));
                println!();
                println!("  {}  Ensure the mouse is connected via USB.", dim("hint"));
                println!("  {}  Close Logitech G HUB if it is running.", dim("hint"));
                println!();
            }
            OutputFormat::Json => json::print_json(&serde_json::json!([])),
        }
        return Ok(());
    }

    let entries: Vec<DeviceEntry> = devices
        .iter()
        .map(|(info, hid_info)| DeviceEntry {
            name: info.name,
            vid: format!("{:04X}", hid_info.vendor_id()),
            pid: format!("{:04X}", hid_info.product_id()),
            path: hid_info.path().to_string_lossy().to_string(),
        })
        .collect();

    match output {
        OutputFormat::Human => {
            println!();
            for entry in &entries {
                // ◈  Logitech G403 HERO
                println!(
                    "  {}  {}",
                    g_cyan(style::SYM_DEVICE),
                    g_cyan_bold(entry.name)
                );
                // path  ·  VID:PID
                println!(
                    "     {}  {}  {}",
                    dim(&entry.path),
                    dim(style::SYM_DOT),
                    dim(&format!("{}:{}", entry.vid, entry.pid))
                );
                println!();
            }
        }
        OutputFormat::Json => json::print_json(&entries),
    }

    Ok(())
}
