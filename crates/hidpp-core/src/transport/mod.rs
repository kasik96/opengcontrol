pub mod hidapi_transport;
pub mod mock;

pub use hidapi_transport::HidapiTransport;
pub use mock::MockTransport;

use crate::protocol::error::HidppError;

/// Abstraction over the HID I/O channel.
/// Decouples HID++ protocol logic from the underlying HID library,
/// enabling deterministic unit tests via MockTransport.
pub trait HidTransport: Send {
    /// Write a raw HID report to the device.
    /// The buffer should include the 0x00 report ID prefix required by macOS IOKit.
    fn write(&self, data: &[u8]) -> Result<(), HidppError>;

    /// Read a raw HID report from the device.
    /// `timeout_ms` of -1 means block indefinitely.
    fn read(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, HidppError>;
}
