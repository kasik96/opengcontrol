//! # logitech-devices
//!
//! Static device registry for Logitech G-series mice.
//! Contains VID/PID and capability definitions for each supported device.
//!
//! ## Adding a new device
//!
//! See `docs/ADDING_DEVICES.md` in the workspace root.

pub mod capability;
pub mod device_info;
pub mod devices;
pub mod registry;

pub use capability::{Capability, RgbZone};
pub use device_info::DeviceInfo;
pub use registry::{all_devices, enumerate_supported_devices, find_device};
