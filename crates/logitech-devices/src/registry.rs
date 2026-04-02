use crate::device_info::DeviceInfo;
use crate::devices::g305::{G305_SE_WIRELESS, G305_WIRELESS};
use crate::devices::g403::{G403_HERO_WIRED, G403_WIRED, G403_WIRELESS};

/// All supported devices. To add a new device:
/// 1. Create a file in `src/devices/<name>.rs` with `static DEVICE: DeviceInfo = ...`
/// 2. Add it to the `mod` declarations in `src/devices/mod.rs`
/// 3. Import and add it to `DEVICE_REGISTRY` below
static DEVICE_REGISTRY: &[&DeviceInfo] = &[
    &G403_WIRED,
    &G403_HERO_WIRED,
    &G403_WIRELESS,
    &G305_WIRELESS,
    &G305_SE_WIRELESS,
    // Future additions:
    // &g502::G502_WIRED,
    // &g502::G502_HERO_WIRED,
    // &g_pro::G_PRO_WIRED,
    // &mx_master::MX_MASTER_3,
];

/// Look up a device by USB Vendor ID and Product ID.
/// Returns `None` if the device is not in the registry.
pub fn find_device(vid: u16, pid: u16) -> Option<&'static DeviceInfo> {
    DEVICE_REGISTRY
        .iter()
        .find(|d| d.vid == vid && d.pid == pid)
        .copied()
}

/// Returns all registered device definitions.
pub fn all_devices() -> &'static [&'static DeviceInfo] {
    DEVICE_REGISTRY
}

/// Enumerate all HID devices and return those recognized by the registry.
/// Returns (DeviceInfo, hidapi::DeviceInfo) pairs, one per matching HID interface.
///
/// Results are sorted so that vendor-specific HID++ interfaces (usage_page=0xFF00,
/// usage=0x01) appear before standard HID interfaces. This ensures that wireless
/// mice using a Unifying receiver are always addressed through the correct interface.
pub fn enumerate_supported_devices(
    hid_api: &hidapi::HidApi,
) -> Vec<(&'static DeviceInfo, hidapi::DeviceInfo)> {
    let mut matches: Vec<_> = hid_api
        .device_list()
        .filter_map(|hid_info| {
            find_device(hid_info.vendor_id(), hid_info.product_id())
                .map(|dev| (dev, hid_info.clone()))
        })
        .collect();

    // Prefer the HID++ receiver interface over standard mouse/keyboard interfaces.
    // usage_page=0xFF00, usage=0x01 is the Unifying receiver's short-message channel.
    matches.sort_by_key(|(_, h)| {
        if h.usage_page() == 0xFF00 && h.usage() == 0x01 {
            0u8
        } else {
            1u8
        }
    });

    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_g403_wired() {
        let dev = find_device(0x046D, 0xC083).unwrap();
        assert_eq!(dev.name, "Logitech G403 Prodigy");
    }

    #[test]
    fn find_g403_hero() {
        let dev = find_device(0x046D, 0xC08F).unwrap();
        assert_eq!(dev.name, "Logitech G403 HERO");
    }

    #[test]
    fn find_g305_wireless() {
        let dev = find_device(0x046D, 0xC092).unwrap();
        assert_eq!(dev.name, "Logitech G305 LIGHTSPEED");
    }

    #[test]
    fn find_g305_se_wireless() {
        let dev = find_device(0x046D, 0xC53F).unwrap();
        assert_eq!(dev.name, "Logitech G305 SE");
    }

    #[test]
    fn unknown_device_returns_none() {
        assert!(find_device(0x046D, 0xDEAD).is_none());
    }

    #[test]
    fn g403_dpi_validation() {
        let dev = find_device(0x046D, 0xC083).unwrap();
        assert!(dev.validate_dpi(800).is_ok());
        assert!(dev.validate_dpi(12000).is_ok());
        assert!(dev.validate_dpi(12001).is_err()); // above max
        assert!(dev.validate_dpi(100).is_err()); // below min
        assert!(dev.validate_dpi(801).is_err()); // not on 50-step grid
    }

    #[test]
    fn g403_polling_rates() {
        let dev = find_device(0x046D, 0xC083).unwrap();
        let rates = dev.supported_polling_rates();
        assert!(rates.contains(&1000));
        assert!(rates.contains(&500));
        assert!(rates.contains(&250));
        assert!(rates.contains(&125));
    }
}
