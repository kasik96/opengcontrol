use crate::device::HidppDevice;
use crate::protocol::{error::HidppError, feature::FeatureCode};
use crate::transport::HidTransport;

/// Feature 0x8060 — Report Rate (Polling Rate)
///
/// Controls how frequently the mouse reports its position to the host.
/// G403 supports: 125, 250, 500, 1000 Hz.
pub struct PollingRate;

/// The four polling rate indices the protocol uses.
/// These are the values sent on the wire; they map to Hz values as follows:
const RATE_INDEX_TO_HZ: &[(u8, u16)] = &[
    (0x01, 1000),
    (0x02, 500),
    (0x03, 250),
    (0x04, 125),
];

impl PollingRate {
    /// Function 0 — getReportRateList.
    /// Returns a bitmask of supported report rates.
    pub fn get_supported_rates<T: HidTransport>(
        device: &HidppDevice<T>,
    ) -> Result<Vec<u16>, HidppError> {
        let resp = device.call_feature_short(FeatureCode::PollingRate, 0, [0; 4])?;
        let mask = resp.params()[0];
        let mut rates = Vec::new();
        for (idx, hz) in RATE_INDEX_TO_HZ {
            if mask & (1 << (idx - 1)) != 0 {
                rates.push(*hz);
            }
        }
        // Sort descending (1000 first)
        rates.sort_unstable_by(|a, b| b.cmp(a));
        Ok(rates)
    }

    /// Function 1 — getReportRate.
    /// Returns the current polling rate in Hz.
    pub fn get_rate_hz<T: HidTransport>(device: &HidppDevice<T>) -> Result<u16, HidppError> {
        let resp = device.call_feature_short(FeatureCode::PollingRate, 1, [0; 4])?;
        let index = resp.params()[0];
        rate_index_to_hz(index).ok_or(HidppError::InvalidResponse {
            expected: 1,
            actual: index as usize,
        })
    }

    /// Function 2 — setReportRate.
    /// Sets the polling rate. `hz` must be in `supported_rates`.
    pub fn set_rate_hz<T: HidTransport>(
        device: &HidppDevice<T>,
        hz: u16,
        supported: &[u16],
    ) -> Result<(), HidppError> {
        if !supported.contains(&hz) {
            return Err(HidppError::UnsupportedPollingRate {
                requested: hz,
                valid: supported.to_vec(),
            });
        }
        let index = hz_to_rate_index(hz).ok_or_else(|| HidppError::UnsupportedPollingRate {
            requested: hz,
            valid: supported.to_vec(),
        })?;
        device.call_feature_short(FeatureCode::PollingRate, 2, [index, 0, 0, 0])?;
        Ok(())
    }
}

fn rate_index_to_hz(index: u8) -> Option<u16> {
    RATE_INDEX_TO_HZ
        .iter()
        .find(|(i, _)| *i == index)
        .map(|(_, hz)| *hz)
}

fn hz_to_rate_index(hz: u16) -> Option<u8> {
    RATE_INDEX_TO_HZ
        .iter()
        .find(|(_, h)| *h == hz)
        .map(|(i, _)| *i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;
    use crate::HidppDevice;

    fn make_short_resp(feature_index: u8, function: u8, params: [u8; 4]) -> Vec<u8> {
        vec![0x10, 0xFF, feature_index, function << 4, params[0], params[1], params[2]]
    }

    #[test]
    fn get_rate_hz_1000() {
        let transport = MockTransport::new();
        // IRoot: PollingRate at index 4
        transport.push_response(make_short_resp(0x00, 0x00, [0x04, 0, 0, 0]));
        // getReportRate response: index 0x01 = 1000Hz
        transport.push_response(make_short_resp(0x04, 0x01, [0x01, 0, 0, 0]));

        let device = HidppDevice::new(transport);
        let hz = PollingRate::get_rate_hz(&device).unwrap();
        assert_eq!(hz, 1000);
    }

    #[test]
    fn unsupported_rate_error() {
        let transport = MockTransport::new();
        transport.push_response(make_short_resp(0x00, 0x00, [0x04, 0, 0, 0]));
        // No response needed — should error before writing

        let device = HidppDevice::new(transport);
        let result = PollingRate::set_rate_hz(&device, 333, &[125, 250, 500, 1000]);
        assert!(matches!(result, Err(HidppError::UnsupportedPollingRate { .. })));
    }
}
