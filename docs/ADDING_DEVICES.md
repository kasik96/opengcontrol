# Adding a New Device

This guide explains how to add support for a new Logitech G-series mouse.

## Step 1: Find the USB IDs

Connect the mouse and run:
```bash
opengcontrol doctor
# or: system_profiler SPUSBDataType | grep -A5 Logitech
```

Note the Vendor ID (always `046D` for Logitech) and Product ID.
Cross-reference with https://devicehunt.com/view/type/usb/vendor/046D

## Step 2: Identify supported features

Connect via USB, close G HUB, then run (once the feature dump tool is added):
```bash
opengcontrol features --device <path>
```

Or cross-reference with:
- https://github.com/libratbag/libratbag/tree/master/data/devices
- https://github.com/PixlOne/logiops (check `/etc/logid.cfg` examples)
- https://pwr-solaar.github.io/Solaar/features/

## Step 3: Create the device file

Create `crates/logitech-devices/src/devices/<model>.rs`:

```rust
use crate::capability::{Capability, RgbZone};
use crate::device_info::DeviceInfo;

static G502_POLLING_RATES: &[u16] = &[125, 250, 500, 1000];

static G502_RGB_ZONES: &[RgbZone] = &[
    RgbZone { name: "logo",         zone_id: 0x00 },
    RgbZone { name: "scroll_wheel", zone_id: 0x01 },
    RgbZone { name: "underglow",    zone_id: 0x02 },
];

static G502_CAPABILITIES: &[Capability] = &[
    Capability::AdjustableDpi {
        min_dpi: 100,
        max_dpi: 25600,
        step: 50,
        sensor_count: 1,
    },
    Capability::PollingRate {
        supported_rates_hz: G502_POLLING_RATES,
    },
    Capability::RgbLighting {
        zone_count: 3,
        zones: G502_RGB_ZONES,
    },
    Capability::OnboardProfiles { profile_count: 5 },
    Capability::ButtonRemapping { button_count: 11 },
];

pub static G502_HERO_WIRED: DeviceInfo = DeviceInfo {
    name: "Logitech G502 HERO",
    vid: 0x046D,
    pid: 0xC08B,
    capabilities: G502_CAPABILITIES,
};
```

## Step 4: Register the device

In `crates/logitech-devices/src/devices/mod.rs`, add:
```rust
pub mod g502;
```

In `crates/logitech-devices/src/registry.rs`, add to `DEVICE_REGISTRY`:
```rust
use crate::devices::g502::G502_HERO_WIRED;

static DEVICE_REGISTRY: &[&DeviceInfo] = &[
    // ...existing devices...
    &G502_HERO_WIRED,
];
```

## Step 5: Test

```bash
# Connect the new mouse, close G HUB
cargo run -- list
cargo run -- info
cargo run -- dpi get
cargo run -- dpi set 800
```

## HID++ feature notes

Some devices have quirks:
- Wireless mice may need `device_id = 0x01` instead of `0xFF` when using a USB receiver
- Some older devices use HID++ 1.0 (not supported yet — open an issue)
- The onboard profile format can differ between models; if `profile list` fails, it may need a device-specific implementation

If you hit issues, check the logiops source for that device model, or run `hidpp-list-features` from the cvuchener/hidpp toolkit on Linux.
