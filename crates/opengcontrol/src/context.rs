use std::collections::HashSet;

use hidapi::HidApi;
use hidpp_core::{HidapiTransport, HidppDevice, HidppError};
use logitech_devices::{device_info::DeviceInfo, enumerate_supported_devices};

use crate::output::OutputFormat;
use crate::permissions::classify_open_error;
use crate::spinner::Spinner;

/// A deduplicated physical device (one entry per mouse, regardless of HID interfaces).
pub struct PhysicalDevice {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
    /// The preferred HID path for this device (Unifying receiver interface first).
    pub path: String,
}

/// Enumerate all connected supported devices, one entry per physical device.
pub fn enumerate_physical_devices() -> Result<Vec<PhysicalDevice>, String> {
    let api = HidApi::new().map_err(|e| format!("Failed to initialize HID API: {e}"))?;
    let supported = enumerate_supported_devices(&api);
    let mut seen = HashSet::new();
    let devices = supported
        .into_iter()
        .filter(|(_, h)| seen.insert((h.vendor_id(), h.product_id())))
        .map(|(info, h)| PhysicalDevice {
            name: info.name,
            vid: h.vendor_id(),
            pid: h.product_id(),
            path: h.path().to_string_lossy().into_owned(),
        })
        .collect();
    Ok(devices)
}

/// Holds an opened HID++ device and its static registry entry.
pub struct DeviceContext {
    device: HidppDevice<HidapiTransport>,
    device_info: &'static DeviceInfo,
}

impl DeviceContext {
    /// Open the first supported device (or the one at `device_path`).
    /// Shows an animated spinner in human output mode while connecting.
    pub fn open(device_path: Option<&str>, output: OutputFormat) -> Result<Self, HidppError> {
        let api = HidApi::new().map_err(|e| HidppError::Transport(e.to_string()))?;

        let supported = enumerate_supported_devices(&api);
        if supported.is_empty() {
            return Err(HidppError::NoDeviceFound);
        }

        let (device_info, hid_info) = if let Some(path) = device_path {
            supported
                .into_iter()
                .find(|(_, h)| h.path().to_string_lossy() == path)
                .ok_or_else(|| {
                    HidppError::Transport(format!("No device found at path: {path}"))
                })?
        } else {
            supported.into_iter().next().unwrap()
        };

        let spinner = Spinner::new(
            format!("Connecting to {}…", device_info.name),
            output,
        );

        // Detect wireless Unifying receiver interface (usage_page=0xFF00).
        // These require numbered HID reports and device_id=0x01 (first receiver slot).
        let is_wireless_receiver = hid_info.usage_page() == 0xFF00;

        match api.open_path(hid_info.path()) {
            Ok(hid_device) => {
                spinner.clear();
                let transport = if is_wireless_receiver {
                    HidapiTransport::new_wireless(hid_device)
                } else {
                    HidapiTransport::new(hid_device)
                };
                let device = if is_wireless_receiver {
                    HidppDevice::with_device_id(transport, 0x01)
                } else {
                    HidppDevice::new(transport)
                };
                Ok(Self { device, device_info })
            }
            Err(e) => {
                let err = classify_open_error(e);
                let msg = match &err {
                    HidppError::PermissionDenied => "Permission denied".to_string(),
                    _ => format!("Failed to open {}", device_info.name),
                };
                spinner.finish_err(msg);
                Err(err)
            }
        }
    }

    pub fn device(&self) -> &HidppDevice<HidapiTransport> {
        &self.device
    }

    pub fn device_info(&self) -> &'static DeviceInfo {
        self.device_info
    }
}
