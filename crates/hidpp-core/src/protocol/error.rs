use thiserror::Error;

#[derive(Debug, Error)]
pub enum HidppError {
    // Transport
    #[error("HID device not found (VID: {vid:#06X}, PID: {pid:#06X})")]
    DeviceNotFound { vid: u16, pid: u16 },

    #[error("No supported device found. Run 'opengcontrol list' to see connected devices.")]
    NoDeviceFound,

    #[error("Cannot open HID device: {0}")]
    Transport(String),

    #[error("Permission denied accessing HID device. Run 'opengcontrol doctor' for help.")]
    PermissionDenied,

    // Protocol
    #[error("Feature {feature_code:#06X} is not supported by this device")]
    FeatureNotSupported { feature_code: u16 },

    #[error("Invalid response from device: expected {expected} bytes, got {actual}")]
    InvalidResponse { expected: usize, actual: usize },

    #[error("Device returned error {error_code:#04X} for feature index {feature_index:#04X}")]
    DeviceError { feature_index: u8, error_code: u8 },

    #[error("Response timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: i32 },

    #[error("Unexpected response: feature index {got:#04X}, expected {expected:#04X}")]
    UnexpectedResponse { expected: u8, got: u8 },

    // Validation
    #[error("DPI {requested} is out of range [{min}, {max}]")]
    DpiOutOfRange { requested: u16, min: u16, max: u16 },

    #[error("DPI {requested} is not a multiple of the step size {step}")]
    DpiStepMismatch { requested: u16, step: u16 },

    #[error("Polling rate {requested}Hz not supported. Valid rates: {valid:?}")]
    UnsupportedPollingRate { requested: u16, valid: Vec<u16> },

    #[error("Profile index {index} is out of range (device has {count} profiles, 0-indexed)")]
    ProfileIndexOutOfRange { index: u8, count: u8 },

    #[error(
        "Writing onboard profiles is not supported for memory model {memory_model} (only the flat memory_model 0 is supported)"
    )]
    UnsupportedProfileMemoryModel { memory_model: u8 },

    // I/O
    #[error("Config file I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config parse error: {0}")]
    TomlDe(String),

    #[error("Config serialize error: {0}")]
    TomlSer(String),
}
