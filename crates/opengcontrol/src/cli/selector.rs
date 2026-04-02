use std::io::{self, BufRead, Write};

use crate::context::PhysicalDevice;
use crate::style;

/// Print a numbered device list and prompt the user to pick one.
/// Returns the HID path of the chosen device.
pub fn select(devices: &[PhysicalDevice]) -> Result<String, String> {
    println!();
    println!(
        "  {}  Multiple devices found. Select one:\n",
        style::g_amber(style::SYM_WARN)
    );

    for (i, d) in devices.iter().enumerate() {
        println!(
            "  [{}]  {}  {}  {}",
            i + 1,
            style::g_cyan(style::SYM_DEVICE),
            style::g_cyan_bold(d.name),
            style::dim(&format!("{:04X}:{:04X}  {}", d.vid, d.pid, d.path))
        );
    }

    println!();
    print!("  Enter number (1–{}): ", devices.len());
    io::stdout().flush().map_err(|e| e.to_string())?;

    let stdin = io::stdin();
    let line = stdin
        .lock()
        .lines()
        .next()
        .ok_or_else(|| "No input".to_string())?
        .map_err(|e| e.to_string())?;

    let trimmed = line.trim();
    let n: usize = trimmed
        .parse()
        .map_err(|_| format!("Invalid selection: {trimmed}"))?;

    if n < 1 || n > devices.len() {
        return Err(format!(
            "Selection {n} out of range (1–{})",
            devices.len()
        ));
    }

    Ok(devices[n - 1].path.clone())
}
