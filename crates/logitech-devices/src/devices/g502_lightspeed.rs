use crate::capability::{Capability, RgbZone};
use crate::device_info::DeviceInfo;

static G502_LIGHTSPEED_POLLING_RATES: &[u16] = &[125, 250, 500, 1000];

static G502_LIGHTSPEED_RGB_ZONES: &[RgbZone] = &[
    RgbZone {
        name: "logo",
        zone_id: 0x00,
    },
    RgbZone {
        name: "dpi_indicator",
        zone_id: 0x01,
    },
];

static G502_LIGHTSPEED_CAPABILITIES: &[Capability] = &[
    Capability::AdjustableDpi {
        min_dpi: 100,
        max_dpi: 25600,
        step: 50,
        sensor_count: 1,
    },
    Capability::PollingRate {
        supported_rates_hz: G502_LIGHTSPEED_POLLING_RATES,
    },
    Capability::RgbLighting {
        zone_count: 2,
        zones: G502_LIGHTSPEED_RGB_ZONES,
    },
    Capability::OnboardProfiles { profile_count: 5 },
    Capability::ButtonRemapping { button_count: 11 },
];

/// Logitech G502 LIGHTSPEED (wired via USB cable) — PID 0xC08D
pub static G502_LIGHTSPEED_WIRED: DeviceInfo = DeviceInfo {
    name: "Logitech G502 LIGHTSPEED",
    vid: 0x046D,
    pid: 0xC08D,
    capabilities: G502_LIGHTSPEED_CAPABILITIES,
};

/// Logitech G502 LIGHTSPEED (wireless receiver) — PID 0xC539
pub static G502_LIGHTSPEED_WIRELESS: DeviceInfo = DeviceInfo {
    name: "Logitech G502 LIGHTSPEED (Wireless)",
    vid: 0x046D,
    pid: 0xC539,
    capabilities: G502_LIGHTSPEED_CAPABILITIES,
};
