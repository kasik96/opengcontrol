use hidpp_core::HidppError;

/// Attempt to open System Settings to the Input Monitoring privacy pane.
pub fn open_input_monitoring_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
        .spawn();
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
pub fn print_permission_guidance() {
    eprintln!("\nlogictl requires Input Monitoring permission to access HID devices.");
    eprintln!("\nTo grant permission:");
    eprintln!("  1. Open System Settings > Privacy & Security > Input Monitoring");
    eprintln!("  2. Click the + button and add opengcontrol");
    eprintln!("  3. If already listed, toggle off and back on");
    eprintln!("\nOr run:  opengcontrol doctor --open-settings");
}
