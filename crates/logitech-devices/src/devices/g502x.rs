use crate::capability::Capability;
use crate::device_info::DeviceInfo;

static G502X_POLLING_RATES: &[u16] = &[125, 250, 500, 1000];

// G502 X LIGHTSPEED has no RGB lighting (unlike the G502 X PLUS).
static G502X_LIGHTSPEED_CAPABILITIES: &[Capability] = &[
    Capability::AdjustableDpi {
        min_dpi: 100,
        max_dpi: 25600,
        step: 50,
        sensor_count: 1,
    },
    Capability::PollingRate {
        supported_rates_hz: G502X_POLLING_RATES,
    },
    Capability::OnboardProfiles { profile_count: 5 },
    Capability::ButtonRemapping { button_count: 11 },
];

/// Logitech G502 X LIGHTSPEED (wired via USB cable) — PID 0xC098
pub static G502X_LIGHTSPEED_WIRED: DeviceInfo = DeviceInfo {
    name: "Logitech G502 X LIGHTSPEED",
    vid: 0x046D,
    pid: 0xC098,
    capabilities: G502X_LIGHTSPEED_CAPABILITIES,
};

/// Logitech G502 X LIGHTSPEED (wireless receiver) — PID 0xC547
pub static G502X_LIGHTSPEED_WIRELESS: DeviceInfo = DeviceInfo {
    name: "Logitech G502 X LIGHTSPEED (Wireless)",
    vid: 0x046D,
    pid: 0xC547,
    capabilities: G502X_LIGHTSPEED_CAPABILITIES,
};
