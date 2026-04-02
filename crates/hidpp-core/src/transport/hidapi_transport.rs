use hidapi::HidDevice;

use super::HidTransport;
use crate::protocol::error::HidppError;

/// Production HID transport backed by the hidapi library.
/// On macOS this uses Apple's IOHIDManager via the hidapi IOKit backend.
pub struct HidapiTransport {
    device: HidDevice,
    /// True when the HID interface uses numbered reports (e.g. Unifying receiver,
    /// usage_page=0xFF00). For such devices the write buffer must start with the
    /// real report ID (0x10 or 0x11); the leading 0x00 padding used for
    /// report-ID-less devices must be stripped.
    uses_numbered_reports: bool,
}

impl HidapiTransport {
    pub fn new(device: HidDevice) -> Self {
        Self {
            device,
            uses_numbered_reports: false,
        }
    }

    /// Use this constructor for wireless Unifying receiver interfaces
    /// (usage_page=0xFF00). The Unifying receiver exposes numbered HID reports,
    /// so hidapi must NOT receive the extra 0x00 prefix byte.
    pub fn new_wireless(device: HidDevice) -> Self {
        Self {
            device,
            uses_numbered_reports: true,
        }
    }
}

impl HidTransport for HidapiTransport {
    fn write(&self, data: &[u8]) -> Result<(), HidppError> {
        // For numbered-report devices, strip the leading 0x00 placeholder that
        // message::to_write_buf() prepends for report-ID-less (wired) devices.
        let buf = if self.uses_numbered_reports && data.first() == Some(&0x00) {
            &data[1..]
        } else {
            data
        };
        self.device.write(buf).map_err(|e| {
            let msg = e.to_string().to_lowercase();
            if msg.contains("operation not permitted")
                || msg.contains("access denied")
                || msg.contains("unable to open")
                || msg.contains("permission")
            {
                HidppError::PermissionDenied
            } else {
                HidppError::Transport(e.to_string())
            }
        })?;
        Ok(())
    }

    fn read(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, HidppError> {
        let n = self.device.read_timeout(buf, timeout_ms).map_err(|e| {
            let msg = e.to_string().to_lowercase();
            if msg.contains("operation not permitted") || msg.contains("permission") {
                HidppError::PermissionDenied
            } else {
                HidppError::Transport(e.to_string())
            }
        })?;
        if n == 0 {
            return Err(HidppError::Timeout { timeout_ms });
        }
        Ok(n)
    }
}
