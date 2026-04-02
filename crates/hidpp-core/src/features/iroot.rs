use crate::device::HidppDevice;
use crate::protocol::{error::HidppError, feature::FeatureCode};
use crate::transport::HidTransport;

/// Feature 0x0000 — IRoot
/// Always present at runtime index 0. Used to look up other features.
pub struct IRoot;

impl IRoot {
    /// Return the protocol version supported by the device (major, minor).
    pub fn get_protocol_version<T: HidTransport>(
        device: &HidppDevice<T>,
    ) -> Result<(u8, u8), HidppError> {
        // Function 1 = GetProtocolVersion, ping byte = 0x00
        let resp = device.call_feature_short(FeatureCode::IRoot, 1, [0x00, 0x00, 0x00, 0x00])?;
        let params = resp.params();
        Ok((params[0], params[1]))
    }
}
