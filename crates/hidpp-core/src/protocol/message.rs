use crate::protocol::error::HidppError;

/// HID report IDs for HID++ messages.
pub const SHORT_REPORT_ID: u8 = 0x10;
pub const LONG_REPORT_ID: u8 = 0x11;

/// Device ID used for directly-connected (wired) devices.
pub const DEVICE_ID_WIRED: u8 = 0xFF;

/// HID++ 2.0 error report feature index.
pub const ERROR_FEATURE_INDEX: u8 = 0xFF;

/// A HID++ 2.0 message — either SHORT (7 bytes) or LONG (20 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HidppMessage {
    Short(ShortMessage),
    Long(LongMessage),
}

impl HidppMessage {
    pub fn device_id(&self) -> u8 {
        match self {
            Self::Short(m) => m.device_id,
            Self::Long(m) => m.device_id,
        }
    }

    pub fn feature_index(&self) -> u8 {
        match self {
            Self::Short(m) => m.feature_index,
            Self::Long(m) => m.feature_index,
        }
    }

    pub fn function_id(&self) -> u8 {
        match self {
            Self::Short(m) => m.function_id,
            Self::Long(m) => m.function_id,
        }
    }

    /// Returns true if the device returned a HID++ error response.
    pub fn is_error(&self) -> bool {
        self.feature_index() == ERROR_FEATURE_INDEX
    }

    pub fn params(&self) -> &[u8] {
        match self {
            Self::Short(m) => &m.params,
            Self::Long(m) => &m.params,
        }
    }

    /// Parse a raw HID report buffer into a HidppMessage.
    pub fn from_bytes(buf: &[u8]) -> Result<Self, HidppError> {
        match buf.first() {
            Some(&SHORT_REPORT_ID) => Ok(Self::Short(ShortMessage::from_bytes(buf)?)),
            Some(&LONG_REPORT_ID) => Ok(Self::Long(LongMessage::from_bytes(buf)?)),
            Some(&id) => Err(HidppError::InvalidResponse {
                expected: SHORT_REPORT_ID as usize,
                actual: id as usize,
            }),
            None => Err(HidppError::InvalidResponse {
                expected: 7,
                actual: 0,
            }),
        }
    }
}

/// A 7-byte SHORT HID++ message.
///
/// Wire format: `[0x10, device_id, feature_index, function_id, p0, p1, p2, p3]`
/// Note: hidapi requires a 0x00 report ID byte prefix for devices not using numbered
/// reports at the USB layer. The HID++ report ID *is* the first payload byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortMessage {
    pub device_id: u8,
    pub feature_index: u8,
    /// High nibble = function index (0-15), low nibble = software ID (use 0x0).
    pub function_id: u8,
    pub params: [u8; 4],
}

impl ShortMessage {
    pub fn new(device_id: u8, feature_index: u8, function: u8, params: [u8; 4]) -> Self {
        Self {
            device_id,
            feature_index,
            function_id: function << 4,
            params,
        }
    }

    /// Encode to 7-byte wire format (without leading 0x00 report ID padding).
    /// Wire layout: [report_id, device_id, feature_index, function_id, p0, p1, p2]
    /// Note: SHORT carries 3 param bytes on wire; params[3] is unused at this layer.
    pub fn to_bytes(&self) -> [u8; 7] {
        [
            SHORT_REPORT_ID,
            self.device_id,
            self.feature_index,
            self.function_id,
            self.params[0],
            self.params[1],
            self.params[2],
        ]
    }

    /// Buffer to send via hidapi write() — prepend 0x00 report ID (required by hidapi on all platforms).
    pub fn to_write_buf(&self) -> [u8; 8] {
        let b = self.to_bytes();
        [0x00, b[0], b[1], b[2], b[3], b[4], b[5], b[6]]
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self, HidppError> {
        if buf.len() < 7 {
            return Err(HidppError::InvalidResponse {
                expected: 7,
                actual: buf.len(),
            });
        }
        // buf[0] is report ID (0x10), skip it
        Ok(Self {
            device_id: buf[1],
            feature_index: buf[2],
            function_id: buf[3],
            params: [buf[4], buf[5], buf[6], 0x00],
        })
    }
}

/// A 20-byte LONG HID++ message.
///
/// Wire format: `[0x11, device_id, feature_index, function_id, p0..p15]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongMessage {
    pub device_id: u8,
    pub feature_index: u8,
    pub function_id: u8,
    pub params: [u8; 16],
}

impl LongMessage {
    pub fn new(device_id: u8, feature_index: u8, function: u8, params: [u8; 16]) -> Self {
        Self {
            device_id,
            feature_index,
            function_id: function << 4,
            params,
        }
    }

    pub fn to_bytes(&self) -> [u8; 20] {
        let mut buf = [0u8; 20];
        buf[0] = LONG_REPORT_ID;
        buf[1] = self.device_id;
        buf[2] = self.feature_index;
        buf[3] = self.function_id;
        buf[4..20].copy_from_slice(&self.params);
        buf
    }

    /// Buffer to send via hidapi write() — prepend 0x00 report ID (required by hidapi on all platforms).
    pub fn to_write_buf(&self) -> [u8; 21] {
        let b = self.to_bytes();
        let mut buf = [0u8; 21];
        buf[0] = 0x00;
        buf[1..21].copy_from_slice(&b);
        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self, HidppError> {
        if buf.len() < 20 {
            return Err(HidppError::InvalidResponse {
                expected: 20,
                actual: buf.len(),
            });
        }
        let mut params = [0u8; 16];
        params.copy_from_slice(&buf[4..20]);
        Ok(Self {
            device_id: buf[1],
            feature_index: buf[2],
            function_id: buf[3],
            params,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_message_roundtrip() {
        let msg = ShortMessage::new(0xFF, 0x01, 0x02, [0xAA, 0xBB, 0xCC, 0x00]);
        let bytes = msg.to_bytes();
        assert_eq!(bytes[0], SHORT_REPORT_ID);
        assert_eq!(bytes[1], 0xFF); // device_id
        assert_eq!(bytes[2], 0x01); // feature_index
        assert_eq!(bytes[3], 0x20); // function 2 << 4
        assert_eq!(bytes[4], 0xAA);
        assert_eq!(bytes[5], 0xBB);
        assert_eq!(bytes[6], 0xCC);

        let parsed = ShortMessage::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.device_id, msg.device_id);
        assert_eq!(parsed.feature_index, msg.feature_index);
        assert_eq!(parsed.function_id, msg.function_id);
        assert_eq!(parsed.params[0], 0xAA);
    }

    #[test]
    fn long_message_roundtrip() {
        let mut params = [0u8; 16];
        params[0] = 0x01;
        params[15] = 0xFF;
        let msg = LongMessage::new(0xFF, 0x03, 0x01, params);
        let bytes = msg.to_bytes();
        assert_eq!(bytes[0], LONG_REPORT_ID);
        assert_eq!(bytes[1], 0xFF);
        assert_eq!(bytes[3], 0x10); // function 1 << 4

        let parsed = LongMessage::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.params[0], 0x01);
        assert_eq!(parsed.params[15], 0xFF);
    }

    #[test]
    fn dpi_encoding() {
        // 1600 DPI = 0x0640
        let dpi: u16 = 1600;
        let [hi, lo] = dpi.to_be_bytes();
        assert_eq!(hi, 0x06);
        assert_eq!(lo, 0x40);
        assert_eq!(u16::from_be_bytes([hi, lo]), 1600);
    }
}
