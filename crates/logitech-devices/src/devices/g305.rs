use crate::capability::Capability;
use crate::device_info::DeviceInfo;

static G305_POLLING_RATES: &[u16] = &[125, 250, 500, 1000];

// G305 has no RGB — single-color logo LED only, not controllable via HID++ RgbLighting
static G305_CAPABILITIES: &[Capability] = &[
    Capability::AdjustableDpi {
        min_dpi: 200,
        max_dpi: 12000,
        step: 50,
        sensor_count: 1,
    },
    Capability::PollingRate {
        supported_rates_hz: G305_POLLING_RATES,
    },
    Capability::OnboardProfiles { profile_count: 3 },
    Capability::ButtonRemapping { button_count: 6 },
];

/// Logitech G305 LIGHTSPEED (wireless receiver) — PID 0xC092
pub static G305_WIRELESS: DeviceInfo = DeviceInfo {
    name: "Logitech G305 LIGHTSPEED",
    vid: 0x046D,
    pid: 0xC092,
    capabilities: G305_CAPABILITIES,
};

/// Logitech G305 SE (wireless receiver) — PID 0xC53F
pub static G305_SE_WIRELESS: DeviceInfo = DeviceInfo {
    name: "Logitech G305 SE",
    vid: 0x046D,
    pid: 0xC53F,
    capabilities: G305_CAPABILITIES,
};
