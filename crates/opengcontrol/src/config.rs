#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Optional host-side configuration file at `~/.config/opengcontrol/config.toml`.
/// Settings here serve as defaults; the mouse's onboard flash is the primary store.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    /// Per-device configuration, keyed by "VID:PID" hex string (e.g. "046D:C083").
    #[serde(default)]
    pub devices: HashMap<String, DeviceConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// If true, apply the default_profile settings when the device is detected.
    #[serde(default)]
    pub auto_apply: bool,
    /// Which profile index to activate by default.
    pub default_profile: Option<u8>,
}

impl Config {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("opengcontrol")
            .join("config.toml")
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents =
            std::fs::read_to_string(&path).map_err(|e| format!("Cannot read config: {e}"))?;
        toml::from_str(&contents).map_err(|e| format!("Config parse error: {e}"))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create config directory: {e}"))?;
        }
        let contents =
            toml::to_string_pretty(self).map_err(|e| format!("Config serialize error: {e}"))?;
        std::fs::write(&path, contents).map_err(|e| format!("Cannot write config: {e}"))?;
        Ok(())
    }

    pub fn device_key(vid: u16, pid: u16) -> String {
        format!("{vid:04X}:{pid:04X}")
    }
}
