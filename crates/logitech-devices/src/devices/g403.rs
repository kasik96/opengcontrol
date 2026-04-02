use crate::capability::{Capability, RgbZone};
use crate::device_info::DeviceInfo;

static G403_POLLING_RATES: &[u16] = &[125, 250, 500, 1000];

static G403_RGB_ZONES: &[RgbZone] = &[
    RgbZone { name: "logo",         zone_id: 0x00 },
    RgbZone { name: "scroll_wheel", zone_id: 0x01 },
];

static G403_CAPABILITIES: &[Capability] = &[
    Capability::AdjustableDpi {
        min_dpi: 200,
        max_dpi: 12000,
        step: 50,
        sensor_count: 1,
    },
    Capability::PollingRate {
        supported_rates_hz: G403_POLLING_RATES,
    },
    Capability::RgbLighting {
        zone_count: 2,
        zones: G403_RGB_ZONES,
    },
    Capability::OnboardProfiles {
        profile_count: 3,
    },
    Capability::ButtonRemapping {
        button_count: 6,
    },
];

/// Logitech G403 Prodigy (wired) — PID 0xC083
pub static G403_WIRED: DeviceInfo = DeviceInfo {
    name: "Logitech G403 Prodigy",
    vid: 0x046D,
    pid: 0xC083,
    capabilities: G403_CAPABILITIES,
};

/// Logitech G403 HERO (wired) — PID 0xC08F
pub static G403_HERO_WIRED: DeviceInfo = DeviceInfo {
    name: "Logitech G403 HERO",
    vid: 0x046D,
    pid: 0xC08F,
    capabilities: G403_CAPABILITIES,
};

/// Logitech G403 (wireless receiver) — PID 0xC082
pub static G403_WIRELESS: DeviceInfo = DeviceInfo {
    name: "Logitech G403 (Wireless)",
    vid: 0x046D,
    pid: 0xC082,
    capabilities: G403_CAPABILITIES,
};
