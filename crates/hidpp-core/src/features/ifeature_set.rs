use crate::device::HidppDevice;
use crate::protocol::{error::HidppError, feature::FeatureCode};
use crate::transport::HidTransport;

/// Feature 0x0001 — IFeatureSet
/// Enumerates all features supported by the device.
pub struct IFeatureSet;

impl IFeatureSet {
    /// Return the total count of features (excluding IRoot).
    pub fn get_count<T: HidTransport>(device: &HidppDevice<T>) -> Result<u8, HidppError> {
        let resp = device.call_feature_short(FeatureCode::IFeatureSet, 0, [0; 4])?;
        Ok(resp.params()[0])
    }

    /// Return the feature code at a given index (1-based, as device counts them).
    /// Returns (feature_code_u16, is_obsolete, is_hidden).
    pub fn get_feature<T: HidTransport>(
        device: &HidppDevice<T>,
        index: u8,
    ) -> Result<(u16, bool, bool), HidppError> {
        let resp = device.call_feature_short(FeatureCode::IFeatureSet, 1, [index, 0, 0, 0])?;
        let params = resp.params();
        let code = u16::from_be_bytes([params[0], params[1]]);
        let flags = params[2];
        let is_obsolete = (flags & 0x80) != 0;
        let is_hidden = (flags & 0x40) != 0;
        Ok((code, is_obsolete, is_hidden))
    }

    /// Enumerate all features on the device.
    /// Returns a list of (feature_index, feature_code_u16, is_obsolete, is_hidden).
    pub fn enumerate<T: HidTransport>(
        device: &HidppDevice<T>,
    ) -> Result<Vec<(u8, u16, bool, bool)>, HidppError> {
        let count = Self::get_count(device)?;
        let mut features = Vec::with_capacity(count as usize);
        for i in 1..=count {
            let (code, obsolete, hidden) = Self::get_feature(device, i)?;
            features.push((i, code, obsolete, hidden));
        }
        Ok(features)
    }
}
