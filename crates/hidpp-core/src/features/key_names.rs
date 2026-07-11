//! Human-readable names for the small, mouse-relevant subsets of Logitech's HID++ 2.0
//! naming tables (control IDs, mouse button bitmasks, USB HID keyboard usage IDs,
//! HID consumer-page usage codes). Shared by the onboard-profiles sector decoder and
//! the persistent-remappable-action feature.

/// Bitmask-style mouse button identifiers (as used by Onboard Profiles' sector layout
/// and by `PersistentRemappableAction`'s `Mouse` action type).
pub fn mouse_button_name(code: u16) -> String {
    match code {
        0x0001 => "Left Click".into(),
        0x0002 => "Right Click".into(),
        0x0004 => "Middle Click".into(),
        0x0008 => "Back".into(),
        0x0010 => "Forward".into(),
        0x0020 => "Button 6".into(),
        0x0040 => "Scroll Left".into(),
        0x0080 => "Scroll Right".into(),
        0x0100 => "Button 9".into(),
        0x0200 => "Button 10".into(),
        0x0400 => "Button 11".into(),
        0x0800 => "Button 12".into(),
        0x1000 => "Button 13".into(),
        0x2000 => "DPI Button".into(),
        0x4000 => "Button 15".into(),
        0x8000 => "Button 16".into(),
        _ => format!("mouse-button(0x{code:04X})"),
    }
}

/// Control IDs (CIDs) — physical/logical control identifiers used by
/// `PersistentRemappableAction` and `ReprogControls`. Covers the mouse-button range;
/// falls back to the raw hex value for anything else (mostly keyboard/media controls).
pub fn cid_name(cid: u16) -> String {
    match cid {
        0x0050 => "Left Button".into(),
        0x0051 => "Right Button".into(),
        0x0052 => "Middle Button".into(),
        0x0053 => "Back Button".into(),
        0x0054 => "Back".into(),
        0x0056 => "Forward Button".into(),
        0x0057 => "Forward".into(),
        0x0059 => "Button 6".into(),
        0x005B => "Left Tilt".into(),
        0x005D => "Right Tilt".into(),
        0x005E => "Button 9".into(),
        0x005F => "Button 10".into(),
        0x0060 => "Button 11".into(),
        0x0061 => "Button 12".into(),
        0x0062 => "Button 13".into(),
        0x00C3 => "Gesture Button".into(),
        0x00C4 => "DPI Change".into(),
        0x00ED => "DPI Change".into(),
        0x00FD => "DPI Switch".into(),
        _ => format!("cid(0x{cid:04X})"),
    }
}

pub fn modifier_prefix(bits: u8) -> String {
    let mut s = String::new();
    if bits & 0x01 != 0 {
        s.push_str("Ctrl+");
    }
    if bits & 0x02 != 0 {
        s.push_str("Shift+");
    }
    if bits & 0x04 != 0 {
        s.push_str("Alt+");
    }
    if bits & 0x08 != 0 {
        s.push_str("Win+");
    }
    s
}

/// Standard USB HID keyboard usage IDs (subset covering common keys).
pub fn hid_key_name(code: u8) -> String {
    match code {
        0x00 => "No Output".into(),
        0x04..=0x1D => ((b'A' + (code - 0x04)) as char).to_string(),
        0x1E => "1".into(),
        0x1F => "2".into(),
        0x20 => "3".into(),
        0x21 => "4".into(),
        0x22 => "5".into(),
        0x23 => "6".into(),
        0x24 => "7".into(),
        0x25 => "8".into(),
        0x26 => "9".into(),
        0x27 => "0".into(),
        0x28 => "Enter".into(),
        0x29 => "Esc".into(),
        0x2A => "Backspace".into(),
        0x2B => "Tab".into(),
        0x2C => "Space".into(),
        0x2D => "-".into(),
        0x2E => "=".into(),
        0x2F => "[".into(),
        0x30 => "]".into(),
        0x31 => "\\".into(),
        0x33 => ";".into(),
        0x34 => "'".into(),
        0x35 => "`".into(),
        0x36 => ",".into(),
        0x37 => ".".into(),
        0x38 => "/".into(),
        0x39 => "CapsLock".into(),
        0x3A..=0x45 => format!("F{}", code - 0x3A + 1),
        0x46 => "PrintScreen".into(),
        0x47 => "ScrollLock".into(),
        0x48 => "Pause".into(),
        0x49 => "Insert".into(),
        0x4A => "Home".into(),
        0x4B => "PageUp".into(),
        0x4C => "Delete".into(),
        0x4D => "End".into(),
        0x4E => "PageDown".into(),
        0x4F => "Right".into(),
        0x50 => "Left".into(),
        0x51 => "Down".into(),
        0x52 => "Up".into(),
        0x68..=0x73 => format!("F{}", code - 0x68 + 13),
        0xE0 => "LeftCtrl".into(),
        0xE1 => "LeftShift".into(),
        0xE2 => "LeftAlt".into(),
        0xE3 => "LeftWin".into(),
        0xE4 => "RightCtrl".into(),
        0xE5 => "RightShift".into(),
        0xE6 => "RightAlt".into(),
        0xE7 => "RightWin".into(),
        0xE8 => "Media PlayPause".into(),
        0xEB => "Media Next".into(),
        0xEA => "Media Previous".into(),
        0xED => "Media VolumeUp".into(),
        0xEE => "Media VolumeDown".into(),
        0xEF => "Media Mute".into(),
        _ => format!("key(0x{code:02X})"),
    }
}

/// A subset of the HID consumer-page usage codes used by common media keys.
pub fn consumer_key_name(code: u16) -> String {
    match code {
        0x00E2 => "Mute".into(),
        0x00E9 => "Volume Up".into(),
        0x00EA => "Volume Down".into(),
        0x00CD => "Play/Pause".into(),
        0x00B5 => "Next Track".into(),
        0x00B6 => "Previous Track".into(),
        0x00B7 => "Stop".into(),
        0x0183 => "Media Select".into(),
        0x018A => "Mail".into(),
        0x0192 => "Calculator".into(),
        0x0221 => "Search".into(),
        0x0223 => "Browser Home".into(),
        _ => format!("consumer(0x{code:04X})"),
    }
}
