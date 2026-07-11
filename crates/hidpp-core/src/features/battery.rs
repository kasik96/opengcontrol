use crate::device::HidppDevice;
use crate::protocol::{error::HidppError, feature::FeatureCode};
use crate::transport::HidTransport;

/// Feature 0x1004 — Unified Battery
///
/// The modern battery feature used by recent Logitech wireless devices (e.g. the
/// Lightspeed G502 family). Reports a state-of-charge percentage when the device
/// supports it, a coarse discrete level, and the charging state.
pub struct UnifiedBattery;

/// What the device is capable of reporting (function 0).
#[derive(Debug, Clone, Copy)]
pub struct BatteryCapabilities {
    /// Bitfield of the discrete levels the device can report
    /// (0x01 critical, 0x02 low, 0x04 good, 0x08 full).
    pub supported_levels: u8,
    /// Whether the device reports a precise state-of-charge percentage.
    pub has_state_of_charge: bool,
}

/// Coarse battery level, for devices that only report discrete steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryLevel {
    Critical,
    Low,
    Good,
    Full,
    Unknown,
}

impl BatteryLevel {
    fn from_bits(bits: u8) -> Self {
        // Reported as a single set bit; prefer the highest if several are set.
        if bits & 0x08 != 0 {
            BatteryLevel::Full
        } else if bits & 0x04 != 0 {
            BatteryLevel::Good
        } else if bits & 0x02 != 0 {
            BatteryLevel::Low
        } else if bits & 0x01 != 0 {
            BatteryLevel::Critical
        } else {
            BatteryLevel::Unknown
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            BatteryLevel::Critical => "critical",
            BatteryLevel::Low => "low",
            BatteryLevel::Good => "good",
            BatteryLevel::Full => "full",
            BatteryLevel::Unknown => "unknown",
        }
    }
}

/// Charging state (function 1, byte 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargingStatus {
    Discharging,
    Charging,
    AlmostFull,
    ChargeComplete,
    Error,
    Unknown(u8),
}

impl ChargingStatus {
    fn from_byte(b: u8) -> Self {
        match b {
            0 => ChargingStatus::Discharging,
            1 => ChargingStatus::Charging,
            2 => ChargingStatus::AlmostFull,
            3 => ChargingStatus::ChargeComplete,
            4 => ChargingStatus::Error,
            other => ChargingStatus::Unknown(other),
        }
    }

    /// True while power is flowing into the battery.
    pub fn is_charging(self) -> bool {
        matches!(self, ChargingStatus::Charging | ChargingStatus::AlmostFull)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ChargingStatus::Discharging => "discharging",
            ChargingStatus::Charging => "charging",
            ChargingStatus::AlmostFull => "charging (almost full)",
            ChargingStatus::ChargeComplete => "charged",
            ChargingStatus::Error => "charging error",
            ChargingStatus::Unknown(_) => "unknown",
        }
    }
}

/// A battery reading.
#[derive(Debug, Clone, Copy)]
pub struct BatteryStatus {
    /// State-of-charge percentage (0–100), when the device supports it.
    pub percentage: Option<u8>,
    /// Coarse discrete level.
    pub level: BatteryLevel,
    /// Charging state.
    pub charging: ChargingStatus,
}

impl UnifiedBattery {
    /// Function 0 — getCapabilities.
    pub fn get_capabilities<T: HidTransport>(
        device: &HidppDevice<T>,
    ) -> Result<BatteryCapabilities, HidppError> {
        let resp = device.call_feature_short(FeatureCode::UnifiedBattery, 0, [0; 4])?;
        let p = resp.params();
        Ok(BatteryCapabilities {
            supported_levels: p[0],
            has_state_of_charge: p[1] & 0x01 != 0,
        })
    }

    /// Function 1 — getStatus. Reads capabilities first to decide whether the
    /// state-of-charge byte is meaningful.
    pub fn get_status<T: HidTransport>(
        device: &HidppDevice<T>,
    ) -> Result<BatteryStatus, HidppError> {
        let caps = Self::get_capabilities(device)?;
        let resp = device.call_feature_short(FeatureCode::UnifiedBattery, 1, [0; 4])?;
        let p = resp.params();
        Ok(BatteryStatus {
            percentage: caps.has_state_of_charge.then_some(p[0]),
            level: BatteryLevel::from_bits(p[1]),
            charging: ChargingStatus::from_byte(p[2]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;
    use crate::HidppDevice;

    fn short(feature_index: u8, function: u8, params: [u8; 4]) -> Vec<u8> {
        vec![
            0x10,
            0xFF,
            feature_index,
            function << 4,
            params[0],
            params[1],
            params[2],
        ]
    }

    // getStatus: capabilities report state-of-charge supported (flag bit 0), status
    // returns 72%, level "good" (0x04), charging (0x01).
    #[test]
    fn get_status_decodes_percentage_level_and_charging() {
        let transport = MockTransport::new();
        transport.push_response(short(0x00, 0x00, [0x06, 0, 0, 0])); // IRoot -> feature index 6
        transport.push_response(short(0x06, 0x00, [0x0F, 0x01, 0, 0])); // capabilities: levels, SoC
        transport.push_response(short(0x06, 0x01, [72, 0x04, 0x01, 0])); // status
        let device = HidppDevice::new(transport);

        let s = UnifiedBattery::get_status(&device).unwrap();
        assert_eq!(s.percentage, Some(72));
        assert_eq!(s.level, BatteryLevel::Good);
        assert_eq!(s.charging, ChargingStatus::Charging);
        assert!(s.charging.is_charging());
    }

    // Without the state-of-charge capability flag, percentage is None but the discrete
    // level is still reported.
    #[test]
    fn get_status_without_state_of_charge_omits_percentage() {
        let transport = MockTransport::new();
        transport.push_response(short(0x00, 0x00, [0x06, 0, 0, 0])); // IRoot
        transport.push_response(short(0x06, 0x00, [0x0F, 0x00, 0, 0])); // no SoC flag
        transport.push_response(short(0x06, 0x01, [0, 0x08, 0x00, 0])); // full, discharging
        let device = HidppDevice::new(transport);

        let s = UnifiedBattery::get_status(&device).unwrap();
        assert_eq!(s.percentage, None);
        assert_eq!(s.level, BatteryLevel::Full);
        assert_eq!(s.charging, ChargingStatus::Discharging);
    }
}
