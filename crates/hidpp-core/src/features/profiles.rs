use serde::{Deserialize, Serialize};

use crate::device::HidppDevice;
use crate::features::key_names::{
    consumer_key_name, hid_key_name, modifier_prefix, mouse_button_name,
};
use crate::protocol::{error::HidppError, feature::FeatureCode};
use crate::transport::HidTransport;

/// Feature 0x8100 — Onboard Profiles
///
/// Manages profiles stored in the mouse's onboard flash memory.
/// Settings saved here persist across USB reconnects and across computers.
///
/// Devices report a `memory_model` (see [`ProfileInfo`]) that selects the addressing
/// scheme used for profile data:
/// - `0`: flat page-addressed layout (older devices, e.g. G403). 96 bytes per profile:
///   bytes 0-1 report rate index, bytes 2-3 default DPI slot index, bytes 4-13 DPI
///   slots (5x 2 bytes, big-endian u16), bytes 14+ button assignments.
/// - `1`: sector-addressed filesystem layout (newer devices, e.g. G502 X). A profile
///   directory sector maps profile index -> data sector; only DPI slots and report
///   rate are parsed for this layout today (see `read_profile_sector`).
pub struct OnboardProfiles;

/// A profile as read from / to be written to the mouse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardProfile {
    /// 0-based profile slot index on the device.
    pub index: u8,
    /// DPI values for each slot (up to 5). Value 0 means slot is disabled.
    pub dpi_slots: Vec<u16>,
    /// Which DPI slot is currently active (0-based).
    pub active_dpi_slot: u8,
    /// Report rate in Hz (125, 250, 500, or 1000).
    pub polling_rate_hz: u16,
    /// Button assignments (6 buttons on G403).
    pub button_assignments: Vec<ButtonAssignment>,
    /// Profile name as stored on the device (sector-addressed devices only). A name of
    /// `PROFILE_NAME_DEFAULT` (or similar) indicates the slot was never customized —
    /// e.g. via G HUB — and still holds the factory placeholder.
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonAssignment {
    /// Button index (0-based).
    pub button_index: u8,
    pub action: ButtonAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ButtonAction {
    /// Standard mouse button (1=left, 2=right, 3=middle, 4=back, 5=forward)
    MouseButton(u8),
    /// Cycle DPI upward through slots
    DpiCycleUp,
    /// Cycle DPI downward through slots
    DpiCycleDown,
    /// Cycle to next profile
    ProfileCycle,
    /// Key combination (modifier bitmask + HID key code)
    KeyCombo { modifiers: u8, key: u8 },
    /// Button disabled
    Disabled,
    /// A button mapping decoded from the sector-addressed layout (`memory_model == 1`),
    /// as a human-readable description. This layout's action encoding is richer than the
    /// flat layout's (macros, consumer keys, mouse-specific functions like G-Shift/DPI
    /// cycle) and read-only, so it's rendered directly rather than modeled as separate
    /// round-trippable variants.
    Described(String),
}

impl ButtonAction {
    /// Human-readable description of the assignment, uniform across both memory models.
    pub fn describe(&self) -> String {
        match self {
            ButtonAction::MouseButton(b) => format!("mouse: {}", mouse_button_name(*b as u16)),
            ButtonAction::DpiCycleUp => "DPI cycle up".to_string(),
            ButtonAction::DpiCycleDown => "DPI cycle down".to_string(),
            ButtonAction::ProfileCycle => "profile cycle".to_string(),
            ButtonAction::KeyCombo { modifiers, key } => {
                format!("key: {}{}", modifier_prefix(*modifiers), hid_key_name(*key))
            }
            ButtonAction::Disabled => "disabled".to_string(),
            ButtonAction::Described(s) => s.clone(),
        }
    }

    /// True when the button has no meaningful assignment (empty slot). Used to hide
    /// noise (e.g. unused G-Shift banks) from list output.
    pub fn is_unassigned(&self) -> bool {
        match self {
            ButtonAction::Disabled => true,
            ButtonAction::Described(s) => {
                matches!(s.as_str(), "unassigned" | "none")
                    || s.strip_prefix("[G-Shift] ")
                        .is_some_and(|inner| matches!(inner, "unassigned" | "none"))
            }
            _ => false,
        }
    }
}

impl OnboardProfiles {
    /// Function 0 — getInfo.
    /// Long response: memory model, profile/macro format, profile count, then
    /// (oob, button count, sector count, profile size, shift) used for sector-addressed
    /// devices (`memory_model == 1`).
    pub fn get_info<T: HidTransport>(device: &HidppDevice<T>) -> Result<ProfileInfo, HidppError> {
        let resp = device.call_feature_long(FeatureCode::OnboardProfiles, 0, [0; 16])?;
        let params = resp.params();
        let get = |i: usize| params.get(i).copied().unwrap_or(0);
        Ok(ProfileInfo {
            memory_model: get(0),
            profile_format: get(1),
            macro_format: get(2),
            profile_count: get(3),
            button_count: get(5),
            sector_count: get(6),
            profile_size: u16::from_be_bytes([get(7), get(8)]),
            shift: get(9),
        })
    }

    /// Function 3 — setCurrentProfile (`CMD 0x30`).
    /// Activates a profile by 0-based `index`.
    ///
    /// The device identifies the profile by its **data-sector address**, not a bare index:
    /// params[0..1] hold the sector big-endian. The sector is looked up in the profile
    /// directory, so this works for both user-flash profiles (`0x0001`, …) and factory
    /// ROM profiles (`0x0101`, …). Sending a bare `index + 1` fails with `INVALID_ARGUMENT`
    /// on a device whose active profiles live in ROM. Note: this is function 3, not 1 —
    /// function 1 is `setOnboardMode`.
    pub fn set_active_profile<T: HidTransport>(
        device: &HidppDevice<T>,
        index: u8,
    ) -> Result<(), HidppError> {
        let sector = Self::profile_data_sector(device, index)?;
        let [sec_hi, sec_lo] = sector.to_be_bytes();
        device.call_feature_short(FeatureCode::OnboardProfiles, 3, [sec_hi, sec_lo, 0, 0])?;
        Ok(())
    }

    /// Function 4 — getCurrentProfile (`CMD 0x40`).
    /// Returns the 0-based index of the currently active profile.
    ///
    /// The response carries the active profile's **data-sector address** (params[0..1],
    /// big-endian), which is mapped back to its position in the profile directory. Note:
    /// this is function 4, not 2 — function 2 is `getOnboardMode`, which merely returns 1
    /// (onboard) / 2 (host) and only coincided with the right index when it was 0.
    pub fn get_active_profile<T: HidTransport>(device: &HidppDevice<T>) -> Result<u8, HidppError> {
        let resp = device.call_feature_short(FeatureCode::OnboardProfiles, 4, [0; 4])?;
        let p = resp.params();
        let sector = u16::from_be_bytes([p[0], p[1]]);
        let headers = Self::get_profile_headers(device)?;
        headers
            .iter()
            .position(|(s, _)| *s == sector)
            .map(|i| i as u8)
            .ok_or(HidppError::Transport(format!(
                "active profile sector {sector:#06X} not found in directory"
            )))
    }

    /// Read a profile from onboard flash.
    ///
    /// Devices report a `memory_model` via [`Self::get_info`]: `0` uses the flat
    /// page-addressed layout (older devices, e.g. G403); `1` uses the newer
    /// sector-addressed filesystem layout (e.g. G502 X). Dispatches accordingly.
    pub fn read_profile<T: HidTransport>(
        device: &HidppDevice<T>,
        index: u8,
    ) -> Result<OnboardProfile, HidppError> {
        let info = Self::get_info(device)?;
        if info.memory_model == 0x01 {
            Self::read_profile_sector(device, index, &info)
        } else {
            Self::read_profile_flat(device, index)
        }
    }

    /// Flat page-addressed layout (function 3/4, `readFromFlash`/`writeToFlash`).
    /// Profile 0 starts at page 0, each profile occupies 3 pages of 16 bytes each.
    fn read_profile_flat<T: HidTransport>(
        device: &HidppDevice<T>,
        index: u8,
    ) -> Result<OnboardProfile, HidppError> {
        // Page address = index * 3 (each profile = 3 pages × 16 bytes = 48 bytes)
        let page = index * 3;

        // Read first page (polling rate + DPI slots)
        let page0 = Self::read_page(device, page)?;
        // Read second page (button assignments)
        let page1 = Self::read_page(device, page + 1)?;

        let polling_rate_hz = polling_rate_index_to_hz(page0[0]);
        let active_dpi_slot = page0[1];

        // DPI slots: 5 × 2 bytes starting at offset 2
        let mut dpi_slots = Vec::new();
        for i in 0..5 {
            let offset = 2 + i * 2;
            let dpi = u16::from_be_bytes([page0[offset], page0[offset + 1]]);
            if dpi == 0 {
                break;
            }
            dpi_slots.push(dpi);
        }

        // Button assignments: 6 buttons × 4 bytes in page1
        let button_assignments = parse_button_assignments(&page1);

        Ok(OnboardProfile {
            index,
            dpi_slots,
            active_dpi_slot,
            polling_rate_hz,
            button_assignments,
            name: None,
        })
    }

    /// Sector-addressed filesystem layout used by newer devices (`memory_model == 1`).
    ///
    /// Profile data lives in a sector whose number is looked up in a directory table
    /// (itself a sector) via [`Self::get_profile_headers`]. Lighting fields are not
    /// parsed (not needed by any current command).
    fn read_profile_sector<T: HidTransport>(
        device: &HidppDevice<T>,
        index: u8,
        info: &ProfileInfo,
    ) -> Result<OnboardProfile, HidppError> {
        let headers = Self::get_profile_headers(device)?;
        let (sector, _enabled) =
            *headers
                .get(index as usize)
                .ok_or(HidppError::ProfileIndexOutOfRange {
                    index,
                    count: headers.len() as u8,
                })?;

        let bytes = Self::read_sector(device, sector, info.profile_size)?;

        let polling_rate_hz = polling_rate_index_to_hz(bytes[0]);
        let active_dpi_slot = bytes[1];

        // 5 × 2 bytes, little-endian, starting at offset 3 (byte 2 is the shift-slot index).
        let mut dpi_slots = Vec::new();
        for i in 0..5 {
            let offset = 3 + i * 2;
            let dpi = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if dpi == 0 {
                break;
            }
            dpi_slots.push(dpi);
        }

        // Buttons: `button_count` entries of 4 bytes each, starting at offset 32.
        // If `shift` indicates a G-Shift bank, a second set follows at offset 96.
        let has_gshift = info.shift & 0x3 == 0x2;
        let mut button_assignments = Vec::new();
        for i in 0..info.button_count as usize {
            let offset = 32 + i * 4;
            let Some(chunk) = bytes.get(offset..offset + 4) else {
                break;
            };
            button_assignments.push(ButtonAssignment {
                button_index: i as u8,
                action: ButtonAction::Described(describe_sector_button(chunk)),
            });
        }
        if has_gshift {
            for i in 0..info.button_count as usize {
                let offset = 96 + i * 4;
                let Some(chunk) = bytes.get(offset..offset + 4) else {
                    break;
                };
                button_assignments.push(ButtonAssignment {
                    button_index: (info.button_count as usize + i) as u8,
                    action: ButtonAction::Described(format!(
                        "[G-Shift] {}",
                        describe_sector_button(chunk)
                    )),
                });
            }
        }

        // Name: 48 bytes, UTF-16LE, starting at offset 160. Blank slots are either
        // all-0x00 or all-0xFF.
        let name = bytes.get(160..208).and_then(|name_bytes| {
            let units: Vec<u16> = name_bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let s: String = char::decode_utf16(units)
                .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
                .collect::<String>()
                .trim_end_matches(['\0', '\u{ffff}'])
                .to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        });

        Ok(OnboardProfile {
            index,
            dpi_slots,
            active_dpi_slot,
            polling_rate_hz,
            button_assignments,
            name,
        })
    }

    /// Reads the profile directory: a list of (sector, enabled) entries terminated by
    /// a 0xFFFF marker. The user directory lives in sector `0x0000`; when it reads back
    /// blank/erased (a factory-fresh device that has never had onboard profiles written,
    /// e.g. a G502 X out of the box), we fall back to the read-only ROM directory at base
    /// `0x0100`. The ROM directory's entries point at ROM data sectors (`0x0101`, …), so
    /// the returned sector values are already ROM-addressed and must not be written.
    fn get_profile_headers<T: HidTransport>(
        device: &HidppDevice<T>,
    ) -> Result<Vec<(u16, u8)>, HidppError> {
        let mut sector_base: u16 = 0x0000;
        let mut chunk = Self::read_sector_chunk(device, sector_base, 0)?;
        if chunk[0..4] == [0, 0, 0, 0] || chunk[0..4] == [0xFF, 0xFF, 0xFF, 0xFF] {
            sector_base = 0x0100;
            chunk = Self::read_sector_chunk(device, sector_base, 0)?;
        }

        let mut headers = Vec::new();
        let mut i: u16 = 0;
        while chunk[0] != 0xFF || chunk[1] != 0xFF {
            let sector = u16::from_be_bytes([chunk[0], chunk[1]]);
            let enabled = chunk[2];
            headers.push((sector, enabled));
            i += 1;
            chunk = Self::read_sector_chunk(device, sector_base, i * 4)?;
        }
        Ok(headers)
    }

    /// Reads `size` bytes from `sector`, 16 bytes at a time. The final chunk is read
    /// back-aligned to the sector's end (matching the device's own addressing), so it
    /// may overlap with the previous chunk — the overlap is trimmed.
    fn read_sector<T: HidTransport>(
        device: &HidppDevice<T>,
        sector: u16,
        size: u16,
    ) -> Result<Vec<u8>, HidppError> {
        let size = size as i32;
        let mut bytes = Vec::new();
        let mut o: i32 = 0;
        while o < size - 15 {
            let chunk = Self::read_sector_chunk(device, sector, o as u16)?;
            bytes.extend_from_slice(&chunk);
            o += 16;
        }
        let chunk = Self::read_sector_chunk(device, sector, (size - 16) as u16)?;
        let start = (16 + o - size).clamp(0, 16) as usize;
        bytes.extend_from_slice(&chunk[start..]);
        Ok(bytes)
    }

    /// Function 5 — memoryRead. Reads 16 bytes at `offset` within `sector`.
    fn read_sector_chunk<T: HidTransport>(
        device: &HidppDevice<T>,
        sector: u16,
        offset: u16,
    ) -> Result<[u8; 16], HidppError> {
        let mut params = [0u8; 16];
        let [sector_hi, sector_lo] = sector.to_be_bytes();
        let [offset_hi, offset_lo] = offset.to_be_bytes();
        params[0] = sector_hi;
        params[1] = sector_lo;
        params[2] = offset_hi;
        params[3] = offset_lo;
        let resp = device.call_feature_long(FeatureCode::OnboardProfiles, 5, params)?;
        let mut result = [0u8; 16];
        result.copy_from_slice(resp.params());
        Ok(result)
    }

    /// Write a profile to onboard flash.
    ///
    /// Only supports the flat page-addressed layout (`memory_model == 0`). The newer
    /// sector-addressed filesystem layout used by `memory_model == 1` devices requires
    /// a start/write-chunk/commit sequence with a CRC16 trailer that isn't implemented
    /// yet — writing a guessed layout risks corrupting the device's onboard profiles.
    pub fn write_profile<T: HidTransport>(
        device: &HidppDevice<T>,
        profile: &OnboardProfile,
    ) -> Result<(), HidppError> {
        let info = Self::get_info(device)?;
        if info.memory_model != 0x00 {
            return Err(HidppError::UnsupportedProfileMemoryModel {
                memory_model: info.memory_model,
            });
        }

        let page = profile.index * 3;

        // Build page 0: [poll_rate_index, active_dpi_slot, dpi0_hi, dpi0_lo, ...]
        let mut page0 = [0u8; 16];
        page0[0] = hz_to_polling_rate_index(profile.polling_rate_hz);
        page0[1] = profile.active_dpi_slot;
        for (i, &dpi) in profile.dpi_slots.iter().enumerate().take(5) {
            let offset = 2 + i * 2;
            let [hi, lo] = dpi.to_be_bytes();
            page0[offset] = hi;
            page0[offset + 1] = lo;
        }

        // Build page 1: button assignments
        let mut page1 = [0u8; 16];
        encode_button_assignments(&profile.button_assignments, &mut page1);

        Self::write_page(device, page, &page0)?;
        Self::write_page(device, page + 1, &page1)?;

        Ok(())
    }

    /// Function 3 — readFromFlash. Reads 16 bytes from a given page.
    fn read_page<T: HidTransport>(
        device: &HidppDevice<T>,
        page: u8,
    ) -> Result<[u8; 16], HidppError> {
        let mut params = [0u8; 16];
        params[0] = page;
        let resp = device.call_feature_long(FeatureCode::OnboardProfiles, 3, params)?;
        let mut result = [0u8; 16];
        result.copy_from_slice(resp.params());
        Ok(result)
    }

    /// Function 4 — writeToFlash. Writes 16 bytes to a given page.
    fn write_page<T: HidTransport>(
        device: &HidppDevice<T>,
        page: u8,
        data: &[u8; 16],
    ) -> Result<(), HidppError> {
        let mut params = [0u8; 16];
        params[0] = page;
        params[1..].copy_from_slice(&data[..15]);
        device.call_feature_long(FeatureCode::OnboardProfiles, 4, params)?;
        Ok(())
    }

    /// Read a raw sector (sector-addressed devices, `memory_model == 1`). Returns
    /// `sector_size` bytes, including the trailing CRC. Read-only and safe.
    pub fn read_raw_sector<T: HidTransport>(
        device: &HidppDevice<T>,
        sector: u16,
    ) -> Result<Vec<u8>, HidppError> {
        let info = Self::get_info(device)?;
        Self::read_sector(device, sector, info.profile_size)
    }

    /// Number of profiles actually present in the on-device directory. May be fewer than
    /// the `profile_count` capacity from [`Self::get_info`] — e.g. a factory-fresh device
    /// with blank user flash exposes only its ROM profiles (2 on the G502 X).
    pub fn provisioned_profile_count<T: HidTransport>(
        device: &HidppDevice<T>,
    ) -> Result<u8, HidppError> {
        Ok(Self::get_profile_headers(device)?.len() as u8)
    }

    /// The data sector backing profile `index` (0-based), read from the profile
    /// directory. On a factory-fresh device this may be a read-only ROM sector
    /// (`>= 0x0100`); [`Self::write_raw_sector`] refuses to write those.
    pub fn profile_data_sector<T: HidTransport>(
        device: &HidppDevice<T>,
        index: u8,
    ) -> Result<u16, HidppError> {
        let headers = Self::get_profile_headers(device)?;
        headers
            .get(index as usize)
            .map(|(sector, _enabled)| *sector)
            .ok_or(HidppError::ProfileIndexOutOfRange {
                index,
                count: headers.len() as u8,
            })
    }

    /// Write `data` to a user-flash `sector` using the sector-addressed write sequence:
    /// fn 6 (begin: sector + byte count), fn 7 (16-byte data chunks), fn 8 (end/commit).
    ///
    /// The last two bytes of `data` are overwritten with the CRC-16 computed over the
    /// preceding bytes (the on-device validation scheme). `data.len()` must equal the
    /// device's `sector_size`.
    ///
    /// Refuses to write ROM sectors (base `0x0100` and above), which are the read-only
    /// factory fallback — writing there is never correct and removes the safety net.
    pub fn write_raw_sector<T: HidTransport>(
        device: &HidppDevice<T>,
        sector: u16,
        data: &mut [u8],
    ) -> Result<(), HidppError> {
        if sector >= 0x0100 {
            return Err(HidppError::Transport(format!(
                "refusing to write ROM/reserved sector {sector:#06X}"
            )));
        }
        let len = data.len();
        if len < 2 {
            return Err(HidppError::Transport("sector data too small".into()));
        }

        // Stamp the CRC-16 trailer (big-endian) over all but the last two bytes.
        let crc = crc_ccitt(&data[..len - 2]);
        let [crc_hi, crc_lo] = crc.to_be_bytes();
        data[len - 2] = crc_hi;
        data[len - 1] = crc_lo;

        // fn 6 — begin: sector (BE), sub-address 0, byte count (BE).
        let [sec_hi, sec_lo] = sector.to_be_bytes();
        let [cnt_hi, cnt_lo] = (len as u16).to_be_bytes();
        let mut begin = [0u8; 16];
        begin[0] = sec_hi;
        begin[1] = sec_lo;
        begin[4] = cnt_hi;
        begin[5] = cnt_lo;
        device.call_feature_long(FeatureCode::OnboardProfiles, 6, begin)?;

        // fn 7 — data: sequential 16-byte chunks, zero-padded past the final byte
        // (the device stops at the byte count set above, so padding is ignored).
        let mut offset = 0;
        while offset < len {
            let mut chunk = [0u8; 16];
            let end = (offset + 16).min(len);
            chunk[..end - offset].copy_from_slice(&data[offset..end]);
            device.call_feature_long(FeatureCode::OnboardProfiles, 7, chunk)?;
            offset += 16;
        }

        // fn 8 — end/commit.
        device.call_feature_short(FeatureCode::OnboardProfiles, 8, [0; 4])?;
        Ok(())
    }

    /// Byte offset of button `index`'s 4-byte record within a profile sector
    /// (sector-addressed layout). The primary bank starts at 32; the G-Shift bank
    /// (indices `>= button_count`) starts at 96.
    fn button_record_offset(index: u8, button_count: u8) -> usize {
        if index < button_count {
            32 + index as usize * 4
        } else {
            96 + (index - button_count) as usize * 4
        }
    }

    /// Remap one button on a profile via read-modify-write: read the profile's sector,
    /// overwrite just that button's 4-byte record, recompute the CRC, write it back, and
    /// verify by read-back. Every other byte is preserved exactly.
    ///
    /// `profile_index` and `button_index` are 0-based; `button_index` may address the
    /// G-Shift bank (`>= button_count`). Only for sector-addressed devices
    /// (`memory_model == 1`). Parametric over the device's `sector_size`, so it works
    /// across G502-family devices without hardcoded sizes.
    ///
    /// Returns the original 4-byte record (so callers can back it up / undo).
    pub fn set_button<T: HidTransport>(
        device: &HidppDevice<T>,
        profile_index: u8,
        button_index: u8,
        record: [u8; 4],
    ) -> Result<[u8; 4], HidppError> {
        let info = Self::get_info(device)?;
        if info.memory_model != 0x01 {
            return Err(HidppError::UnsupportedProfileMemoryModel {
                memory_model: info.memory_model,
            });
        }
        let max_index = info.button_count.saturating_mul(2);
        if button_index >= max_index {
            return Err(HidppError::Transport(format!(
                "button index {button_index} out of range (0..{max_index})"
            )));
        }

        let sector = Self::profile_data_sector(device, profile_index)?;
        let mut data = Self::read_sector(device, sector, info.profile_size)?;

        let off = Self::button_record_offset(button_index, info.button_count);
        if off + 4 > data.len() {
            return Err(HidppError::Transport(
                "button record offset past end of sector".into(),
            ));
        }
        let original: [u8; 4] = data[off..off + 4].try_into().unwrap();

        // No-op guard: skip the write (and flash wear) if nothing changes.
        if original == record {
            return Ok(original);
        }

        data[off..off + 4].copy_from_slice(&record);
        Self::write_raw_sector(device, sector, &mut data)?;

        // Verify by read-back — the written record must be present and the CRC valid.
        let after = Self::read_sector(device, sector, info.profile_size)?;
        if after[off..off + 4] != record {
            return Err(HidppError::Transport(
                "read-back verification failed: button record did not persist".into(),
            ));
        }
        let n = after.len();
        if crc_ccitt(&after[..n - 2]) != u16::from_be_bytes([after[n - 2], after[n - 1]]) {
            return Err(HidppError::Transport(
                "read-back verification failed: CRC invalid after write".into(),
            ));
        }
        Ok(original)
    }
}

/// Build a sector-layout button record binding a mouse button by its bitmask `code`
/// (0x0001 left, 0x0002 right, 0x0004 middle, 0x0008 back, 0x0010 forward, ...).
///
/// Note the sector layout uses a *bitmask*, unlike the flat layout's sequential
/// [`ButtonAction::MouseButton`] codes — don't feed one into the other.
pub fn mouse_button_record(code: u16) -> [u8; 4] {
    let [hi, lo] = code.to_be_bytes();
    [0x80, 0x01, hi, lo]
}

/// Build a sector-layout button record binding a keyboard keystroke: `modifiers` is the
/// HID modifier bitmask, `hid_key` the HID keyboard usage (e.g. 0x71 = F22).
pub fn key_record(modifiers: u8, hid_key: u8) -> [u8; 4] {
    [0x80, 0x02, modifiers, hid_key]
}

/// A record that disables the button ("none").
pub fn disabled_record() -> [u8; 4] {
    [0x80, 0x00, 0x00, 0x00]
}

/// CRC-16-CCITT (poly `0x1021`, init `0xFFFF`, no input/output reflection, no final XOR)
/// as used by Logitech onboard-profile sectors. Bit-serial form transcribed from
/// libratbag's `hidpp_crc_ccitt` (itself "provided by Logitech"); verified byte-for-byte
/// against real on-device sector data (see tests).
pub fn crc_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        let temp = (crc >> 8) ^ byte as u16;
        crc <<= 8;
        let mut quick = temp ^ (temp >> 4);
        crc ^= quick;
        quick <<= 5;
        crc ^= quick;
        quick <<= 7;
        crc ^= quick;
    }
    crc
}

#[derive(Debug)]
pub struct ProfileInfo {
    pub memory_model: u8,
    pub profile_format: u8,
    pub macro_format: u8,
    pub profile_count: u8,
    /// Number of onboard-remappable buttons per profile (sector-addressed devices only).
    pub button_count: u8,
    /// Number of flash sectors reserved for profile storage (sector-addressed devices only).
    pub sector_count: u8,
    /// Size in bytes of one profile's sector (sector-addressed devices only).
    pub profile_size: u16,
    /// Bitfield; bits 0-1 == 2 means the device has a second, "shifted" (G-Shift) set
    /// of button assignments per profile (sector-addressed devices only).
    pub shift: u8,
}

fn polling_rate_index_to_hz(index: u8) -> u16 {
    match index {
        0x01 => 1000,
        0x02 => 500,
        0x03 => 250,
        0x04 => 125,
        _ => 1000,
    }
}

fn hz_to_polling_rate_index(hz: u16) -> u8 {
    match hz {
        1000 => 0x01,
        500 => 0x02,
        250 => 0x03,
        125 => 0x04,
        _ => 0x01,
    }
}

/// Parse 6 button assignments from a 16-byte page.
/// Each button uses 4 bytes: [cid_hi, cid_lo, action_type, action_param].
fn parse_button_assignments(page: &[u8; 16]) -> Vec<ButtonAssignment> {
    let mut assignments = Vec::new();
    for i in 0..4 {
        // We fit 4 buttons in 16 bytes (4 bytes each); rest are in next page
        let offset = i * 4;
        if offset + 3 >= page.len() {
            break;
        }
        let action_type = page[offset + 2];
        let action_param = page[offset + 3];
        let action = match action_type {
            0x01 => ButtonAction::MouseButton(action_param),
            0x31 => ButtonAction::DpiCycleUp,
            0x32 => ButtonAction::DpiCycleDown,
            0x20 => ButtonAction::ProfileCycle,
            0x11 => ButtonAction::KeyCombo {
                modifiers: page[offset + 2],
                key: page[offset + 3],
            },
            0xFF => ButtonAction::Disabled,
            _ => ButtonAction::Disabled,
        };
        assignments.push(ButtonAssignment {
            button_index: i as u8,
            action,
        });
    }
    assignments
}

/// Decodes a single 4-byte sector-model button entry into a human-readable description.
/// Format (see Logitech HID++ 2.0 Onboard Profiles, sector layout): the top nibble of
/// byte 0 selects a "behavior"; the rest of the 4 bytes are behavior-specific.
fn describe_sector_button(b: &[u8]) -> String {
    if b == [0xFF, 0xFF, 0xFF, 0xFF] {
        return "unassigned".to_string();
    }
    let behavior = b[0] >> 4;
    match behavior {
        0x0 | 0x1 => {
            let sector = (((b[0] & 0x0F) as u16) << 8) | b[1] as u16;
            let address = ((b[2] as u16) << 8) | b[3] as u16;
            let kind = if behavior == 0x0 {
                "macro"
            } else {
                "macro-stop"
            };
            format!("{kind} (sector {sector:#06X} @ {address:#06X})")
        }
        0x2 => "macro-stop-all".to_string(),
        0x8 => match b[1] {
            0x0 => "none".to_string(),
            0x1 => {
                let code = ((b[2] as u16) << 8) | b[3] as u16;
                format!("mouse: {}", mouse_button_name(code))
            }
            0x2 => format!("key: {}{}", modifier_prefix(b[2]), hid_key_name(b[3])),
            0x3 => {
                let code = ((b[2] as u16) << 8) | b[3] as u16;
                format!("consumer: {}", consumer_key_name(code))
            }
            other => format!("unknown-send(type={other:#04X})"),
        },
        0x9 => format!("function: {} (data={:#04X})", function_name(b[1]), b[3]),
        _ => format!("unknown({b:02X?})"),
    }
}

fn function_name(code: u8) -> String {
    match code {
        0x0 => "No Action".into(),
        0x1 => "Tilt Left".into(),
        0x2 => "Tilt Right".into(),
        0x3 => "Next DPI".into(),
        0x4 => "Previous DPI".into(),
        0x5 => "Cycle DPI".into(),
        0x6 => "Default DPI".into(),
        0x7 => "DPI Shift".into(),
        0x8 => "Next Profile".into(),
        0x9 => "Previous Profile".into(),
        0xA => "Cycle Profile".into(),
        0xB => "G-Shift".into(),
        0xC => "Battery Status".into(),
        0xD => "Profile Select".into(),
        0xE => "Mode Switch".into(),
        0xF => "Host Button".into(),
        0x10 => "Scroll Down".into(),
        0x11 => "Scroll Up".into(),
        _ => format!("function(0x{code:02X})"),
    }
}

fn encode_button_assignments(assignments: &[ButtonAssignment], buf: &mut [u8; 16]) {
    for assignment in assignments.iter().take(4) {
        let offset = (assignment.button_index as usize) * 4;
        if offset + 3 >= buf.len() {
            break;
        }
        match &assignment.action {
            ButtonAction::MouseButton(btn) => {
                buf[offset + 2] = 0x01;
                buf[offset + 3] = *btn;
            }
            ButtonAction::DpiCycleUp => {
                buf[offset + 2] = 0x31;
            }
            ButtonAction::DpiCycleDown => {
                buf[offset + 2] = 0x32;
            }
            ButtonAction::ProfileCycle => {
                buf[offset + 2] = 0x20;
            }
            ButtonAction::KeyCombo { modifiers, key } => {
                buf[offset + 2] = *modifiers;
                buf[offset + 3] = *key;
            }
            ButtonAction::Disabled => {
                buf[offset + 2] = 0xFF;
            }
            ButtonAction::Described(_) => {
                // Only produced by the sector-model reader; writing that layout isn't
                // supported (see `write_profile`), so this never round-trips here.
                buf[offset + 2] = 0xFF;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;
    use crate::HidppDevice;

    fn make_short_resp(feature_index: u8, function: u8, params: [u8; 4]) -> Vec<u8> {
        vec![
            0x10,
            0xFF,
            feature_index,
            function << 4,
            params[0],
            params[1],
            params[2],
        ]
    }

    fn make_long_resp(feature_index: u8, function: u8, params: [u8; 16]) -> Vec<u8> {
        let mut v = vec![0x11, 0xFF, feature_index, function << 4];
        v.extend_from_slice(&params);
        v
    }

    // A 2-entry user directory: profile 0 -> sector 0x0001, profile 1 -> sector 0x0002,
    // then the 0xFFFF terminator. get_profile_headers reads a 16-byte window per entry
    // offset (0, 4, 8), using only the leading 4 bytes.
    fn push_directory(transport: &MockTransport, feat: u8) {
        transport.push_response(make_long_resp(
            feat,
            0x05,
            [0, 1, 1, 0xFF, 0, 2, 1, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0],
        ));
        transport.push_response(make_long_resp(
            feat,
            0x05,
            [0, 2, 1, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ));
        transport.push_response(make_long_resp(
            feat,
            0x05,
            [0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ));
    }

    // getCurrentProfile (fn 4) returns the active profile's data sector in params[0..1];
    // it is mapped back to a directory index. Sector 0x0002 -> profile index 1.
    #[test]
    fn get_active_profile_maps_sector_to_index() {
        let transport = MockTransport::new();
        transport.push_response(make_short_resp(0x00, 0x00, [0x08, 0, 0, 0])); // IRoot
        transport.push_response(make_short_resp(0x08, 0x04, [0x00, 0x02, 0, 0])); // sector 0x0002
        push_directory(&transport, 0x08);

        let device = HidppDevice::new(transport);
        assert_eq!(OnboardProfiles::get_active_profile(&device).unwrap(), 1);
    }

    // setCurrentProfile (fn 3) sends the target profile's data sector (big-endian), looked
    // up in the directory — not a bare index. Profile 1 -> sector 0x0002 -> params [00,02].
    #[test]
    fn set_active_profile_sends_data_sector() {
        let transport = MockTransport::new();
        transport.push_response(make_short_resp(0x00, 0x00, [0x08, 0, 0, 0])); // IRoot
        push_directory(&transport, 0x08);
        transport.push_response(make_short_resp(0x08, 0x03, [0, 0, 0, 0])); // set response

        let device = HidppDevice::new(transport);
        OnboardProfiles::set_active_profile(&device, 1).unwrap();

        // Write buffer: [prefix, report_id, device_id, feature_index, fn<<4|sw, p0, p1, p2].
        let writes = device.transport().written_data();
        let w = writes.last().unwrap();
        assert_eq!(
            w[4] >> 4,
            3,
            "function nibble should be setCurrentProfile (3)"
        );
        assert_eq!(
            [w[5], w[6]],
            [0x00, 0x02],
            "params[0..1] = sector 0x0002 BE"
        );
    }

    // Real 255-byte sector read off a G502 LIGHTSPEED (profile 0). The device stores a
    // CRC-16-CCITT over bytes [0..253] big-endian in the last two bytes (0x6094).
    // Reproducing it proves our CRC matches the device's own validation scheme.
    const G502_SECTOR: &str = concat!(
        "02000040060000000000000000ffffffff00ffffffffffffffffffffffffffff",
        "8001000180010002800200718002007380020072800100048002006f80020070",
        "900c00009002000090010000ffffffffffffffffffffffffffffffffffffffff",
        "8001000180010002800200718002007380020072800100048002006f80020070",
        "900c00009002000090010000ffffffffffffffffffffffffffffffffffffffff",
        "500052004f00460049004c0045005f004e0041004d0045005f00440045004600",
        "410055004c00540000000000000000000000000000000000000000000000",
        "0000000000000000ffffffffffffffffffffffffffffffffffffffffffff006094",
    );

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn crc_ccitt_matches_device_sector() {
        let sector = hex(G502_SECTOR);
        assert_eq!(sector.len(), 255);
        let stored = u16::from_be_bytes([sector[253], sector[254]]);
        assert_eq!(stored, 0x6094);
        assert_eq!(crc_ccitt(&sector[..253]), stored);
    }

    #[test]
    fn crc_ccitt_known_vector() {
        // CRC-16/CCITT-FALSE check value for "123456789" is 0x29B1.
        assert_eq!(crc_ccitt(b"123456789"), 0x29B1);
    }

    #[test]
    fn button_records_encode_as_on_device() {
        // Matches the real G502 sector: left=80 01 00 01, right=80 01 00 02, F22 key.
        assert_eq!(mouse_button_record(0x0001), [0x80, 0x01, 0x00, 0x01]);
        assert_eq!(mouse_button_record(0x0002), [0x80, 0x01, 0x00, 0x02]);
        assert_eq!(mouse_button_record(0x0004), [0x80, 0x01, 0x00, 0x04]);
        assert_eq!(key_record(0x00, 0x71), [0x80, 0x02, 0x00, 0x71]); // F22
        assert_eq!(disabled_record(), [0x80, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn button_record_offsets_cover_both_banks() {
        // Primary bank at 32, G-Shift bank at 96 (button_count = 11 on the G502).
        assert_eq!(OnboardProfiles::button_record_offset(0, 11), 32);
        assert_eq!(OnboardProfiles::button_record_offset(2, 11), 40);
        assert_eq!(OnboardProfiles::button_record_offset(10, 11), 72);
        assert_eq!(OnboardProfiles::button_record_offset(11, 11), 96);
        assert_eq!(OnboardProfiles::button_record_offset(13, 11), 104);
    }
}
