pub mod battery;
pub mod dpi;
pub mod ifeature_set;
pub mod iroot;
pub mod key_names;
pub mod persistent_remap;
pub mod polling_rate;
pub mod profiles;

pub use battery::{
    read_battery, BatteryLevel, BatteryReading, BatteryStatus, BatteryVoltage, ChargingStatus,
    UnifiedBattery,
};
pub use dpi::AdjustableDpi;
pub use ifeature_set::IFeatureSet;
pub use iroot::IRoot;
pub use persistent_remap::{PersistentRemap, RemapEntry};
pub use polling_rate::PollingRate;
pub use profiles::{
    crc_ccitt, disabled_record, key_record, mouse_button_record, ButtonAction, ButtonAssignment,
    ButtonRemap, CloneOptions, CloneReport, OnboardProfile, OnboardProfiles, ProfileInfo,
};
