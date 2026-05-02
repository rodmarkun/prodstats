use crate::agents::{AgentHarnessConfig, default_harnesses};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub input: InputConfig,
    pub agents: AgentsConfig,
    pub git: GitConfig,
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub sample_interval_ms: u64,
    pub csv_flush_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub backend: String,
    pub count_keys: bool,
    pub count_mouse_buttons: bool,
    pub count_wheel: bool,
    pub count_pointer_motion: bool,
    pub count_key_repeats: bool,
    #[serde(default = "default_active_input_window_seconds")]
    pub active_window_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    pub activity_window_seconds: u64,
    pub cpu_active_threshold_ms_per_second: u64,
    pub io_active_threshold_bytes_per_second: u64,
    pub harnesses: Vec<AgentHarnessConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    pub enabled: bool,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub format: String,
    pub corner: String,
}

fn default_active_input_window_seconds() -> u64 {
    60
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                sample_interval_ms: 1000,
                csv_flush_interval_seconds: 10,
            },
            input: InputConfig {
                backend: "evdev".into(),
                count_keys: true,
                count_mouse_buttons: true,
                count_wheel: true,
                count_pointer_motion: false,
                count_key_repeats: false,
                active_window_seconds: default_active_input_window_seconds(),
            },
            agents: AgentsConfig {
                activity_window_seconds: 5,
                cpu_active_threshold_ms_per_second: 20,
                io_active_threshold_bytes_per_second: 65536,
                harnesses: default_harnesses(),
            },
            git: GitConfig {
                enabled: true,
                source: "shell-wrapper".into(),
            },
            display: DisplayConfig {
                format: "compact".into(),
                corner: "top-right".into(),
            },
        }
    }
}

impl Config {
    pub fn load_or_default(path: &std::path::Path) -> anyhow::Result<Self> {
        if path.exists() {
            Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn write_default(path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            std::fs::write(path, toml::to_string_pretty(&Self::default())?)?;
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(rest)
    } else {
        PathBuf::from(path)
    }
}
