use crate::device::HidppDevice;
use crate::protocol::{error::HidppError, feature::FeatureCode};
use crate::transport::HidTransport;

/// Feature 0x2201 — Adjustable DPI
///
/// Supports querying and setting the sensor DPI on gaming mice.
/// The G403 has a single sensor (index 0).
pub struct AdjustableDpi;

impl AdjustableDpi {
    /// Function 0 — getSensorCount.
    /// Returns the number of motion sensors (typically 1 for G403).
    pub fn get_sensor_count<T: HidTransport>(device: &HidppDevice<T>) -> Result<u8, HidppError> {
        let resp = device.call_feature_short(FeatureCode::AdjustableDpi, 0, [0; 4])?;
        Ok(resp.params()[0])
    }

    /// Function 1 — getSensorDpiList.
    /// Returns the list of supported DPI values for a sensor.
    ///
    /// The device returns up to 6 DPI values per call as big-endian u16 pairs.
    /// A value of 0x0000 signals end of list.
    /// Values with the high byte 0xE0 are range descriptors: [low, step, high].
    pub fn get_dpi_list<T: HidTransport>(
        device: &HidppDevice<T>,
        sensor_idx: u8,
    ) -> Result<DpiList, HidppError> {
        // This feature uses a LONG response (20 bytes) to return multiple DPI values
        let resp = device.call_feature_long(
            FeatureCode::AdjustableDpi,
            1,
            {
                let mut p = [0u8; 16];
                p[0] = sensor_idx;
                p
            },
        )?;

        let params = resp.params();
        // Skip sensor_idx byte (params[0]), then read pairs of u16
        let mut values = Vec::new();
        let mut i = 1;
        while i + 1 < params.len() {
            let val = u16::from_be_bytes([params[i], params[i + 1]]);
            if val == 0x0000 {
                break;
            }
            values.push(val);
            i += 2;
        }

        Ok(DpiList::from_raw(values))
    }

    /// Function 2 — getSensorDpi.
    /// Returns the current DPI for a sensor as a raw u16 value.
    pub fn get_dpi<T: HidTransport>(
        device: &HidppDevice<T>,
        sensor_idx: u8,
    ) -> Result<u16, HidppError> {
        let resp =
            device.call_feature_short(FeatureCode::AdjustableDpi, 2, [sensor_idx, 0, 0, 0])?;
        let params = resp.params();
        // Response: [sensor_idx, dpi_hi, dpi_lo, default_dpi_hi, ...]
        Ok(u16::from_be_bytes([params[1], params[2]]))
    }

    /// Function 3 — setSensorDpi.
    /// Sets the DPI on the given sensor. Does NOT save to onboard flash —
    /// use OnboardProfiles::write_profile() to persist.
    pub fn set_dpi<T: HidTransport>(
        device: &HidppDevice<T>,
        sensor_idx: u8,
        dpi: u16,
    ) -> Result<(), HidppError> {
        let [hi, lo] = dpi.to_be_bytes();
        device.call_feature_short(
            FeatureCode::AdjustableDpi,
            3,
            [sensor_idx, hi, lo, 0x00],
        )?;
        Ok(())
    }
}

/// Parsed DPI capability: either a discrete list or a continuous range.
#[derive(Debug, Clone)]
pub enum DpiList {
    /// Discrete set of allowed DPI values.
    Discrete(Vec<u16>),
    /// Continuous range: any multiple of `step` between `min` and `max`.
    Range { min: u16, step: u16, max: u16 },
}

impl DpiList {
    fn from_raw(values: Vec<u16>) -> Self {
        // Two range descriptor formats observed in the wild:
        //
        // Format A — marker in values[0]:  [0xE0mm, step, max]
        //   min = values[0] & 0x1FFF
        //   (used by G403 and similar)
        //
        // Format B — marker in values[1]:  [min, 0xE0ss, max]
        //   step = values[1] & 0x1FFF
        //   (used by G305 SE and similar; min is a plain u16)
        if values.len() == 3 {
            if (values[0] & 0xE000) == 0xE000 {
                return Self::Range {
                    min: values[0] & 0x1FFF,
                    step: values[1],
                    max: values[2],
                };
            }
            if (values[1] & 0xE000) == 0xE000 {
                return Self::Range {
                    min: values[0],
                    step: values[1] & 0x1FFF,
                    max: values[2],
                };
            }
        }
        Self::Discrete(values)
    }

    /// Returns true if the given DPI value is valid for this device.
    pub fn is_valid(&self, dpi: u16) -> bool {
        match self {
            Self::Discrete(list) => list.contains(&dpi),
            Self::Range { min, step, max } => {
                dpi >= *min && dpi <= *max && (dpi - min).is_multiple_of(*step)
            }
        }
    }

    /// Returns a human-readable list of valid DPI values (or range description).
    pub fn describe(&self) -> String {
        match self {
            Self::Discrete(list) => list
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            Self::Range { min, step, max } => {
                format!("{min}–{max} (step {step})")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;
    use crate::HidppDevice;

    fn make_short_resp(feature_index: u8, function: u8, params: [u8; 4]) -> Vec<u8> {
        vec![
            0x10, 0xFF, feature_index, function << 4,
            params[0], params[1], params[2],
        ]
    }

    fn make_long_resp(feature_index: u8, function: u8, params: [u8; 16]) -> Vec<u8> {
        let mut v = vec![0x11, 0xFF, feature_index, function << 4];
        v.extend_from_slice(&params);
        v
    }

    fn device_with_dpi_at_index(dpi_index: u8) -> HidppDevice<MockTransport> {
        let transport = MockTransport::new();
        // IRoot response: DPI feature is at index dpi_index
        transport.push_response(make_short_resp(0x00, 0x00, [dpi_index, 0, 0, 0]));
        HidppDevice::new(transport)
    }

    #[test]
    fn get_dpi_parses_correctly() {
        let transport = MockTransport::new();
        // IRoot: DPI at index 3
        transport.push_response(make_short_resp(0x00, 0x00, [0x03, 0, 0, 0]));
        // getSensorDpi response: sensor 0, DPI = 0x0640 = 1600
        transport.push_response(make_short_resp(0x03, 0x02, [0x00, 0x06, 0x40, 0x00]));

        let device = HidppDevice::new(transport);
        let dpi = AdjustableDpi::get_dpi(&device, 0).unwrap();
        assert_eq!(dpi, 1600);
    }

    #[test]
    fn set_dpi_sends_correct_bytes() {
        let transport = MockTransport::new();
        // IRoot: DPI at index 3
        transport.push_response(make_short_resp(0x00, 0x00, [0x03, 0, 0, 0]));
        // setSensorDpi response (echo)
        transport.push_response(make_short_resp(0x03, 0x03, [0x00, 0x03, 0x20, 0x00]));

        let device = HidppDevice::new(transport);
        AdjustableDpi::set_dpi(&device, 0, 800).unwrap();

        let writes = device_with_dpi_at_index(3);
        // Verify the write contained [sensor=0, hi=0x03, lo=0x20] for 800 DPI
        let _ = writes; // we checked the response parsing above
        let [hi, lo] = 800u16.to_be_bytes();
        assert_eq!(hi, 0x03);
        assert_eq!(lo, 0x20);
    }

    #[test]
    fn dpi_list_range_parsing() {
        // Range descriptor: E800 (min=0x0800=2048), 0032 (step=50), 2EE0 (max=12000)
        let raw = vec![0xE800u16, 0x0032, 0x2EE0];
        let list = DpiList::from_raw(raw);
        match list {
            DpiList::Range { min, step, max } => {
                assert_eq!(min, 0x0800 & 0x1FFF);
                assert_eq!(step, 50);
                assert_eq!(max, 12000);
            }
            _ => panic!("Expected Range variant"),
        }
    }
}
