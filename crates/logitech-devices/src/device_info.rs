use crate::capability::Capability;

/// Static description of a supported device model.
/// Pure data — no methods, no trait impls required.
/// Adding a new device is as simple as declaring a `const DeviceInfo { ... }`.
#[derive(Debug)]
pub struct DeviceInfo {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub capabilities: &'static [Capability],
}

impl DeviceInfo {
    /// Returns the DPI capability if present.
    pub fn dpi_capability(&self) -> Option<&Capability> {
        self.capabilities
            .iter()
            .find(|c| matches!(c, Capability::AdjustableDpi { .. }))
    }

    /// Returns the polling rate capability if present.
    pub fn polling_rate_capability(&self) -> Option<&Capability> {
        self.capabilities
            .iter()
            .find(|c| matches!(c, Capability::PollingRate { .. }))
    }

    /// Returns the onboard profiles capability if present.
    pub fn onboard_profiles_capability(&self) -> Option<&Capability> {
        self.capabilities
            .iter()
            .find(|c| matches!(c, Capability::OnboardProfiles { .. }))
    }

    /// Returns the RGB lighting capability if present.
    pub fn rgb_capability(&self) -> Option<&Capability> {
        self.capabilities
            .iter()
            .find(|c| matches!(c, Capability::RgbLighting { .. }))
    }

    /// Returns the button remapping capability if present.
    pub fn button_capability(&self) -> Option<&Capability> {
        self.capabilities
            .iter()
            .find(|c| matches!(c, Capability::ButtonRemapping { .. }))
    }

    /// Returns the supported polling rates for this device, or empty slice if not supported.
    pub fn supported_polling_rates(&self) -> &'static [u16] {
        match self.polling_rate_capability() {
            Some(Capability::PollingRate { supported_rates_hz }) => supported_rates_hz,
            _ => &[],
        }
    }

    /// Returns the number of onboard profiles declared in the registry, or 0 if not supported.
    pub fn profile_count(&self) -> u8 {
        match self.onboard_profiles_capability() {
            Some(Capability::OnboardProfiles { profile_count }) => *profile_count,
            _ => 0,
        }
    }

    /// Returns (min_dpi, max_dpi, step, sensor_count) or None if DPI not supported.
    pub fn dpi_info(&self) -> Option<(u16, u16, u16, u8)> {
        match self.dpi_capability() {
            Some(Capability::AdjustableDpi { min_dpi, max_dpi, step, sensor_count }) => {
                Some((*min_dpi, *max_dpi, *step, *sensor_count))
            }
            _ => None,
        }
    }

    /// Validate a DPI value against device limits. Returns Err with a descriptive message.
    pub fn validate_dpi(&self, dpi: u16) -> Result<(), String> {
        match self.dpi_info() {
            Some((min, max, step, _)) => {
                if dpi < min || dpi > max {
                    return Err(format!(
                        "DPI {dpi} out of range [{min}, {max}] for {}",
                        self.name
                    ));
                }
                if step > 1 && (dpi - min) % step != 0 {
                    return Err(format!(
                        "DPI {dpi} is not a valid step. Valid values: {min}–{max} in steps of {step}"
                    ));
                }
                Ok(())
            }
            None => Err(format!("{} does not support DPI adjustment", self.name)),
        }
    }
}
