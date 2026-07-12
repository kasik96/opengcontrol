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

/// Feature 0x1001 — Battery Voltage
///
/// Older Logitech wireless devices (e.g. the G502 LIGHTSPEED) report a raw battery
/// voltage rather than a state-of-charge percentage. The percentage is *estimated*
/// from the voltage via a Li-ion discharge curve.
pub struct BatteryVoltage;

impl BatteryVoltage {
    /// Function 0 — getBatteryVoltage. Returns voltage (mV, big-endian) + a flags byte.
    pub fn get_status<T: HidTransport>(
        device: &HidppDevice<T>,
    ) -> Result<(u16, ChargingStatus), HidppError> {
        let resp = device.call_feature_short(FeatureCode::BatteryVoltage, 0, [0; 4])?;
        let p = resp.params();
        let voltage_mv = u16::from_be_bytes([p[0], p[1]]);
        let flags = p[2];
        // Flags byte: bit7 recharging, bit4 charge error (matches Solaar's decode).
        let charging = if flags & 0x10 != 0 {
            ChargingStatus::Error
        } else if flags & 0x80 != 0 {
            ChargingStatus::Charging
        } else {
            ChargingStatus::Discharging
        };
        Ok((voltage_mv, charging))
    }
}

/// Li-ion discharge curve (mV -> %), from Solaar's table, descending. Used to estimate a
/// percentage for voltage-only devices (feature 0x1001). Linearly interpolated, clamped.
const VOLTAGE_CURVE: &[(u16, u8)] = &[
    (4186, 100),
    (4067, 90),
    (3989, 80),
    (3922, 70),
    (3859, 60),
    (3811, 50),
    (3775, 40),
    (3741, 30),
    (3708, 20),
    (3675, 10),
    (3650, 5),
    (3579, 2),
    (3500, 0),
];

/// Estimate a battery percentage (0–100) from a voltage in millivolts.
pub fn estimate_percentage_from_voltage(mv: u16) -> u8 {
    let first = VOLTAGE_CURVE[0];
    if mv >= first.0 {
        return first.1;
    }
    let last = VOLTAGE_CURVE[VOLTAGE_CURVE.len() - 1];
    if mv <= last.0 {
        return last.1;
    }
    for pair in VOLTAGE_CURVE.windows(2) {
        let (hi_mv, hi_pct) = pair[0];
        let (lo_mv, lo_pct) = pair[1];
        if mv <= hi_mv && mv >= lo_mv {
            let span = (hi_mv - lo_mv) as u32;
            let frac = (mv - lo_mv) as u32;
            let pct = lo_pct as u32 + frac * (hi_pct - lo_pct) as u32 / span;
            return pct as u8;
        }
    }
    last.1
}

/// A battery reading normalized across the different battery features.
#[derive(Debug, Clone, Copy)]
pub struct BatteryReading {
    /// Charge percentage (0–100): exact from 0x1004, or estimated from voltage (0x1001).
    pub percentage: Option<u8>,
    /// True when `percentage` was estimated from a voltage curve rather than reported.
    pub estimated: bool,
    /// Coarse discrete level, when the device reports one (0x1004).
    pub level: Option<BatteryLevel>,
    /// Charging state.
    pub charging: ChargingStatus,
    /// Raw voltage in millivolts, when available (0x1001).
    pub voltage_mv: Option<u16>,
}

/// Read the battery, trying the modern UnifiedBattery (0x1004) first and falling back to
/// BatteryVoltage (0x1001) for older devices. Returns `FeatureNotSupported` if neither
/// exists.
pub fn read_battery<T: HidTransport>(
    device: &HidppDevice<T>,
) -> Result<BatteryReading, HidppError> {
    match UnifiedBattery::get_status(device) {
        Ok(s) => {
            return Ok(BatteryReading {
                percentage: s.percentage,
                estimated: false,
                level: Some(s.level),
                charging: s.charging,
                voltage_mv: None,
            })
        }
        Err(HidppError::FeatureNotSupported { .. }) => {}
        Err(e) => return Err(e),
    }
    let (mv, charging) = BatteryVoltage::get_status(device)?;
    Ok(BatteryReading {
        percentage: Some(estimate_percentage_from_voltage(mv)),
        estimated: true,
        level: None,
        charging,
        voltage_mv: Some(mv),
    })
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

    #[test]
    fn voltage_curve_interpolates_and_clamps() {
        assert_eq!(estimate_percentage_from_voltage(4300), 100); // above range
        assert_eq!(estimate_percentage_from_voltage(3400), 0); // below range
        assert_eq!(estimate_percentage_from_voltage(3922), 70); // exact table point
        assert_eq!(estimate_percentage_from_voltage(3888), 64); // interpolated (G502 reading)
    }

    // read_battery falls back to 0x1001 when 0x1004 is absent: IRoot reports 0x1004 at
    // index 0 (absent), 0x1001 at index 6; the voltage read is 0x0F30 = 3888 mV.
    #[test]
    fn read_battery_falls_back_to_voltage() {
        let transport = MockTransport::new();
        transport.push_response(short(0x00, 0x00, [0x00, 0, 0, 0])); // 0x1004 absent
        transport.push_response(short(0x00, 0x00, [0x06, 0, 0, 0])); // 0x1001 at index 6
        transport.push_response(short(0x06, 0x00, [0x0F, 0x30, 0x00, 0])); // 3888 mV, discharging
        let device = HidppDevice::new(transport);

        let r = read_battery(&device).unwrap();
        assert_eq!(r.percentage, Some(64));
        assert!(r.estimated);
        assert_eq!(r.voltage_mv, Some(3888));
        assert_eq!(r.charging, ChargingStatus::Discharging);
        assert!(r.level.is_none());
    }
}
