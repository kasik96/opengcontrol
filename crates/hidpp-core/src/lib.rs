//! # hidpp-core
//!
//! HID++ 2.0 protocol implementation for Logitech G-series devices.
//! Device-agnostic: this crate has no knowledge of specific Logitech products.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use hidpp_core::{HidppDevice, HidapiTransport};
//! use hidpp_core::features::AdjustableDpi;
//!
//! let api = hidapi::HidApi::new().unwrap();
//! let hid = api.open(0x046D, 0xC083).unwrap(); // G403 wired
//! let transport = HidapiTransport::new(hid);
//! let device = HidppDevice::new(transport);
//!
//! let dpi = AdjustableDpi::get_dpi(&device, 0).unwrap();
//! println!("Current DPI: {dpi}");
//! ```

pub mod device;
pub mod features;
pub mod protocol;
pub mod transport;

pub use device::HidppDevice;
pub use protocol::{FeatureCode, FeatureIndex, HidppError};
pub use transport::{HidapiTransport, HidTransport, MockTransport};
