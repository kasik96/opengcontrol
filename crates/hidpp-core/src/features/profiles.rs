use serde::{Deserialize, Serialize};

use crate::device::HidppDevice;
use crate::protocol::{error::HidppError, feature::FeatureCode};
use crate::transport::HidTransport;

/// Feature 0x8110 — Onboard Profiles
///
/// Manages profiles stored in the mouse's onboard flash memory.
/// Settings saved here persist across USB reconnects and across computers.
///
/// Profile binary layout (96 bytes per profile, Logitech format):
/// - Bytes 0-1:  Report rate index
/// - Bytes 2-3:  Default DPI slot index
/// - Bytes 4-13: DPI slots (5x 2 bytes, big-endian u16)
/// - Bytes 14+:  Button assignments (variable)
///
/// Note: the exact layout varies by device. We use the G403 format.
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
}

impl OnboardProfiles {
    /// Function 0 — getProfileDescriptors.
    /// Returns (active_profile_index, profile_count).
    pub fn get_info<T: HidTransport>(device: &HidppDevice<T>) -> Result<ProfileInfo, HidppError> {
        let resp = device.call_feature_short(FeatureCode::OnboardProfiles, 0, [0; 4])?;
        let params = resp.params();
        Ok(ProfileInfo {
            memory_model: params[0],
            profile_format: params[1],
            macro_format: params[2],
            profile_count: params[3],
        })
    }

    /// Function 1 — setCurrentProfile.
    /// Activates a profile by index (0-based).
    pub fn set_active_profile<T: HidTransport>(
        device: &HidppDevice<T>,
        index: u8,
    ) -> Result<(), HidppError> {
        device.call_feature_short(FeatureCode::OnboardProfiles, 1, [index, 0, 0, 0])?;
        Ok(())
    }

    /// Function 2 — getCurrentProfile.
    /// Returns the index of the currently active profile.
    pub fn get_active_profile<T: HidTransport>(device: &HidppDevice<T>) -> Result<u8, HidppError> {
        let resp = device.call_feature_short(FeatureCode::OnboardProfiles, 2, [0; 4])?;
        Ok(resp.params()[0])
    }

    /// Read a profile from onboard flash.
    /// This uses function 3 (readFromFlash) with sector/page addressing.
    ///
    /// For the G403, profiles are stored at specific memory addresses.
    /// Profile 0 starts at page 0, each profile occupies 3 pages of 16 bytes each.
    pub fn read_profile<T: HidTransport>(
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
        })
    }

    /// Write a profile to onboard flash.
    pub fn write_profile<T: HidTransport>(
        device: &HidppDevice<T>,
        profile: &OnboardProfile,
    ) -> Result<(), HidppError> {
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
}

#[derive(Debug)]
pub struct ProfileInfo {
    pub memory_model: u8,
    pub profile_format: u8,
    pub macro_format: u8,
    pub profile_count: u8,
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
        }
    }
}
