pub mod feature_cache;

use feature_cache::FeatureIndexCache;

use crate::protocol::{
    error::HidppError,
    feature::{FeatureCode, FeatureIndex},
    message::{HidppMessage, LongMessage, ShortMessage, DEVICE_ID_WIRED, ERROR_FEATURE_INDEX},
};
use crate::transport::HidTransport;

const READ_TIMEOUT_MS: i32 = 2000;

/// Central handle for communicating with a HID++ 2.0 device.
/// Generic over `T: HidTransport` so tests can inject `MockTransport`.
pub struct HidppDevice<T: HidTransport> {
    transport: T,
    device_id: u8,
    feature_cache: FeatureIndexCache,
}

impl<T: HidTransport> HidppDevice<T> {
    pub fn new(transport: T) -> Self {
        Self::with_device_id(transport, DEVICE_ID_WIRED)
    }

    pub fn with_device_id(transport: T, device_id: u8) -> Self {
        Self {
            transport,
            device_id,
            feature_cache: FeatureIndexCache::new(),
        }
    }

    /// Resolve the runtime index for a feature code.
    /// Result is cached; subsequent calls for the same code are instant.
    pub fn get_feature_index(&self, code: FeatureCode) -> Result<FeatureIndex, HidppError> {
        // IRoot is always at index 0 — no need to query
        if code == FeatureCode::IRoot {
            return Ok(FeatureIndex::IROOT);
        }

        if let Some(cached) = self.feature_cache.get(code) {
            return Ok(cached);
        }

        // Query IRoot (feature index 0, function 0 = GetFeature)
        let code_u16 = code.as_u16();
        let hi = (code_u16 >> 8) as u8;
        let lo = (code_u16 & 0xFF) as u8;

        let response = self.call_raw_short(
            FeatureIndex::IROOT.0,
            0, // function 0 = GetFeature
            [hi, lo, 0x00, 0x00],
        )?;

        let index = FeatureIndex(response.params()[0]);

        if index.is_absent() {
            return Err(HidppError::FeatureNotSupported {
                feature_code: code_u16,
            });
        }

        self.feature_cache.insert(code, index);
        Ok(index)
    }

    /// Low-level call using a SHORT message, returns parsed response.
    pub fn call_feature_short(
        &self,
        code: FeatureCode,
        function: u8,
        params: [u8; 4],
    ) -> Result<HidppMessage, HidppError> {
        let index = self.get_feature_index(code)?;
        self.call_raw_short(index.0, function, params)
    }

    /// Low-level call using a LONG message, returns parsed response.
    pub fn call_feature_long(
        &self,
        code: FeatureCode,
        function: u8,
        params: [u8; 16],
    ) -> Result<HidppMessage, HidppError> {
        let index = self.get_feature_index(code)?;
        self.call_raw_long(index.0, function, params)
    }

    fn call_raw_short(
        &self,
        feature_index: u8,
        function: u8,
        params: [u8; 4],
    ) -> Result<HidppMessage, HidppError> {
        let msg = ShortMessage::new(self.device_id, feature_index, function, params);
        let write_buf = msg.to_write_buf();
        self.transport.write(&write_buf)?;
        self.read_response(feature_index, function)
    }

    fn call_raw_long(
        &self,
        feature_index: u8,
        function: u8,
        params: [u8; 16],
    ) -> Result<HidppMessage, HidppError> {
        let msg = LongMessage::new(self.device_id, feature_index, function, params);
        let write_buf = msg.to_write_buf();
        self.transport.write(&write_buf)?;
        self.read_response(feature_index, function)
    }

    /// Read a response, skipping unrelated HID reports until we get one
    /// matching our feature_index + function, or a HID++ error response.
    fn read_response(&self, feature_index: u8, function: u8) -> Result<HidppMessage, HidppError> {
        let mut buf = [0u8; 20];

        // Retry up to 10 times to skip unrelated reports (e.g. movement events)
        for _ in 0..10 {
            let n = self.transport.read(&mut buf, READ_TIMEOUT_MS)?;
            if n < 7 {
                continue;
            }

            let msg = match HidppMessage::from_bytes(&buf[..n]) {
                Ok(m) => m,
                Err(_) => continue,
            };

            // HID++ error response: feature_index=0xFF
            if msg.feature_index() == ERROR_FEATURE_INDEX {
                let params = msg.params();
                return Err(HidppError::DeviceError {
                    feature_index: params.first().copied().unwrap_or(0),
                    error_code: params.get(1).copied().unwrap_or(0),
                });
            }

            // Match on feature_index and function (high nibble of function_id)
            if msg.feature_index() == feature_index && (msg.function_id() >> 4) == function {
                return Ok(msg);
            }
            // Otherwise it's an unrelated report; skip and retry
        }

        Err(HidppError::Timeout {
            timeout_ms: READ_TIMEOUT_MS,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;

    /// Build a SHORT response with given feature_index, function, and params
    fn make_short_response(feature_index: u8, function: u8, params: [u8; 4]) -> Vec<u8> {
        vec![
            0x10, // SHORT_REPORT_ID
            0xFF, // device_id
            feature_index,
            function << 4,
            params[0],
            params[1],
            params[2],
        ]
    }

    #[test]
    fn get_feature_index_caches_result() {
        let transport = MockTransport::new();
        // IRoot response: feature index 0x00, fn 0, response says DPI is at index 0x03
        transport.push_response(make_short_response(0x00, 0x00, [0x03, 0x00, 0x00, 0x00]));

        let device = HidppDevice::new(transport);
        let idx = device
            .get_feature_index(FeatureCode::AdjustableDpi)
            .unwrap();
        assert_eq!(idx.0, 0x03);

        // Second call should hit cache — no additional response needed
        let idx2 = device
            .get_feature_index(FeatureCode::AdjustableDpi)
            .unwrap();
        assert_eq!(idx2.0, 0x03);
    }

    #[test]
    fn feature_not_supported_when_index_zero() {
        let transport = MockTransport::new();
        // IRoot returns index 0x00 = not present
        transport.push_response(make_short_response(0x00, 0x00, [0x00, 0x00, 0x00, 0x00]));

        let device = HidppDevice::new(transport);
        let result = device.get_feature_index(FeatureCode::AdjustableDpi);
        assert!(matches!(
            result,
            Err(HidppError::FeatureNotSupported { .. })
        ));
    }
}
