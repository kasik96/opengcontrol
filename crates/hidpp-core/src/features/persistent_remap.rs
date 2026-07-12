use crate::device::HidppDevice;
use crate::features::key_names::{
    cid_name, consumer_key_name, hid_key_name, modifier_prefix, mouse_button_name,
};
use crate::protocol::{error::HidppError, feature::FeatureCode};
use crate::transport::HidTransport;

/// Feature 0x1C00 — Persistent Remappable Action
///
/// Stores button/key remaps directly on the device's own memory, independent of
/// Onboard Profiles (feature 0x8100). This is the mechanism G HUB/Options+ commonly
/// use for per-button remaps on mice normally driven in "host mode" (e.g. remapping
/// a click to a media key or a function key): the remap survives without the
/// software running since it's written to the device itself, but it lives in a
/// completely separate table from the Onboard Profiles sectors.
pub struct PersistentRemap;

/// The current remap for one control (identified by its control ID / CID).
#[derive(Debug, Clone)]
pub struct RemapEntry {
    pub cid: u16,
    /// Raw action-type byte: 0=Empty, 1=Key, 2=Mouse, 3=Xdisp, 4=Ydisp, 5=Vscroll,
    /// 6=Hscroll, 7=Consumer, 8=Internal, 9=Power.
    pub action_id: u8,
    pub remapped: u16,
    pub modifiers: u8,
    pub status: u8,
}

impl RemapEntry {
    /// Name of the control being remapped (e.g. "Middle Button").
    pub fn control_name(&self) -> String {
        cid_name(self.cid)
    }

    /// Human-readable description of what the control currently does.
    pub fn describe(&self) -> String {
        match self.action_id {
            0x00 => "(default action)".to_string(),
            0x01 => format!(
                "key: {}{}",
                modifier_prefix(self.modifiers),
                hid_key_name(self.remapped as u8)
            ),
            0x02 => format!("mouse: {}", mouse_button_name(self.remapped)),
            0x03 => format!("X displacement ({:#06X})", self.remapped),
            0x04 => format!("Y displacement ({:#06X})", self.remapped),
            0x05 => format!("vertical scroll ({:#06X})", self.remapped),
            0x06 => format!("horizontal scroll ({:#06X})", self.remapped),
            0x07 => format!("consumer: {}", consumer_key_name(self.remapped)),
            0x08 => format!("internal action ({:#06X})", self.remapped),
            0x09 => format!("power action ({:#06X})", self.remapped),
            other => format!("unknown action type {other:#04X} ({:#06X})", self.remapped),
        }
    }
}

impl PersistentRemap {
    /// Function 1 — getCount. Number of remappable controls on this device.
    pub fn get_count<T: HidTransport>(device: &HidppDevice<T>) -> Result<u8, HidppError> {
        let resp = device.call_feature_short(FeatureCode::PersistentRemappableAction, 1, [0; 4])?;
        Ok(resp.params()[0])
    }

    /// Function 2 — getCidIndex. Returns the control ID (CID) stored at `index`.
    fn get_cid_at<T: HidTransport>(device: &HidppDevice<T>, index: u8) -> Result<u16, HidppError> {
        let resp = device.call_feature_short(
            FeatureCode::PersistentRemappableAction,
            2,
            [index, 0xFF, 0, 0],
        )?;
        let p = resp.params();
        Ok(u16::from_be_bytes([p[0], p[1]]))
    }

    /// Function 3 — getActionMapping. Returns the current remap for `cid`.
    fn get_mapping<T: HidTransport>(
        device: &HidppDevice<T>,
        cid: u16,
    ) -> Result<RemapEntry, HidppError> {
        let mut params = [0u8; 16];
        let [cid_hi, cid_lo] = cid.to_be_bytes();
        params[0] = cid_hi;
        params[1] = cid_lo;
        params[2] = 0xFF;
        let resp = device.call_feature_long(FeatureCode::PersistentRemappableAction, 3, params)?;
        let p = resp.params();
        Ok(RemapEntry {
            cid,
            action_id: p[3],
            remapped: u16::from_be_bytes([p[4], p[5]]),
            modifiers: p[6],
            status: p[7],
        })
    }

    /// Reads every remappable control on the device and its current mapping.
    pub fn list_remaps<T: HidTransport>(
        device: &HidppDevice<T>,
    ) -> Result<Vec<RemapEntry>, HidppError> {
        let count = Self::get_count(device)?;
        let mut entries = Vec::with_capacity(count as usize);
        for i in 0..count {
            let cid = Self::get_cid_at(device, i)?;
            entries.push(Self::get_mapping(device, cid)?);
        }
        Ok(entries)
    }
}
