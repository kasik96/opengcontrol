/// HID++ 2.0 feature codes (wire protocol values).
/// Non-exhaustive so adding new features does not break existing match arms.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum FeatureCode {
    IRoot = 0x0000,
    IFeatureSet = 0x0001,
    IFirmwareInfo = 0x0003,
    DeviceName = 0x0005,
    AdjustableDpi = 0x2201,
    PollingRate = 0x8060,
    RgbEffects = 0x8070,
    PerKeyLighting = 0x8071,
    OnboardProfiles = 0x8100,
    ReprogControls = 0x1B00,
    ReprogControlsV2 = 0x1B01,
    ReprogControlsV2_2 = 0x1B02,
    ReprogControlsV3 = 0x1B03,
    ReprogControlsV4 = 0x1B04,
    PersistentRemappableAction = 0x1C00,
}

impl FeatureCode {
    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

/// Runtime feature index returned by IRoot::GetFeature.
/// Newtype prevents accidentally passing a FeatureCode where an index is expected.
/// A value of 0x00 for a non-IRoot query means "feature not present on this device".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeatureIndex(pub u8);

impl FeatureIndex {
    /// IRoot is always at index 0.
    pub const IROOT: Self = Self(0x00);

    /// Returns true if this index indicates the feature is absent.
    /// (Only meaningful for non-IRoot features — index 0 is reserved for IRoot.)
    pub fn is_absent(self) -> bool {
        self.0 == 0x00
    }
}

impl std::fmt::Display for FeatureIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#04X}", self.0)
    }
}
