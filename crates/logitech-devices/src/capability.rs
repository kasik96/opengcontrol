use serde::Serialize;

/// What a specific device model can do.
/// Each variant maps to one or more HID++ 2.0 feature codes.
/// Adding a new capability here and registering it on a DeviceInfo
/// is all that's needed to unlock that feature for the device in the CLI.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Capability {
    /// Feature 0x2201 — Adjustable DPI
    AdjustableDpi {
        min_dpi: u16,
        max_dpi: u16,
        /// DPI step size (e.g. 50 means valid values are min, min+50, min+100, ...)
        step: u16,
        sensor_count: u8,
    },
    /// Feature 0x8060 — Report Rate
    PollingRate {
        /// Supported rates in Hz, e.g. [125, 250, 500, 1000]
        supported_rates_hz: &'static [u16],
    },
    /// Feature 0x8070 / 0x8071 — RGB Lighting
    RgbLighting {
        zone_count: u8,
        zones: &'static [RgbZone],
    },
    /// Feature 0x8110 — Onboard Profiles
    OnboardProfiles { profile_count: u8 },
    /// Button remapping via onboard profiles
    ButtonRemapping { button_count: u8 },
}

/// A named RGB lighting zone on a device.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RgbZone {
    pub name: &'static str,
    /// Zone ID used in HID++ feature messages.
    pub zone_id: u8,
}
