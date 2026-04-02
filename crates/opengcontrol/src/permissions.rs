use hidpp_core::HidppError;

/// Attempt to open platform-specific permission settings.
#[cfg(target_os = "macos")]
pub fn open_input_monitoring_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
        .spawn();
}

#[cfg(not(target_os = "macos"))]
pub fn open_input_monitoring_settings() {
    // No equivalent on this platform
}

/// Classify a hidapi open error and convert to HidppError with guidance.
pub fn classify_open_error(err: hidapi::HidError) -> HidppError {
    let msg = err.to_string().to_lowercase();
    if msg.contains("operation not permitted")
        || msg.contains("access denied")
        || msg.contains("unable to open")
        || msg.contains("permission")
    {
        HidppError::PermissionDenied
    } else {
        HidppError::Transport(err.to_string())
    }
}

/// Print actionable guidance for a PermissionDenied error.
#[cfg(target_os = "macos")]
pub fn print_permission_guidance() {
    eprintln!("\nopengcontrol requires Input Monitoring permission to access HID devices.");
    eprintln!("\nTo grant permission:");
    eprintln!("  1. Open System Settings > Privacy & Security > Input Monitoring");
    eprintln!("  2. Click the + button and add opengcontrol");
    eprintln!("  3. If already listed, toggle off and back on");
    eprintln!("\nOr run:  opengcontrol doctor --open-settings");
}

#[cfg(target_os = "windows")]
pub fn print_permission_guidance() {
    eprintln!(
        "\nCannot open HID device. Another application (G HUB, Logi Options+) may have exclusive access."
    );
    eprintln!("\nTo fix:");
    eprintln!("  1. Close Logitech G HUB and Logi Options+");
    eprintln!("  2. Try again");
    eprintln!("\nOr run:  opengcontrol doctor");
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn print_permission_guidance() {
    eprintln!("\nCannot open HID device: permission denied.");
    eprintln!("You may need to add a udev rule or run with elevated privileges.");
    eprintln!("\nOr run:  opengcontrol doctor");
}
