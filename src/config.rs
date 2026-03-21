use config::{Config, ConfigError, File};
use serde::{Deserialize, Deserializer, Serialize};
use std::path::{Path, PathBuf};

use crate::ipc::Mode;

struct MiriDefaults;

impl MiriDefaults {
    const DEFAULT_WORKSPACE_MODE: Mode = Mode::Master;
    const MASTER_WIDTH_PERCENTAGE: f64 = 50.0;
    const MASTER_MAXIMIZE_SINGLE_WINDOW: bool = true;
    const SCROLL_MAINTAIN_FOCUS_ON_NEW_WINDOW: bool = false;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    #[serde(deserialize_with = "deserialize_mode")]
    pub default_workspace_mode: Mode,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            default_workspace_mode: MiriDefaults::DEFAULT_WORKSPACE_MODE,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MasterConfig {
    pub column_width_percentage: f64,
    pub maximize_single_window: bool,
}

impl Default for MasterConfig {
    fn default() -> Self {
        Self {
            column_width_percentage: MiriDefaults::MASTER_WIDTH_PERCENTAGE,
            maximize_single_window: MiriDefaults::MASTER_MAXIMIZE_SINGLE_WINDOW,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScrollConfig {
    pub maintain_focus_on_new_window: bool,
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            maintain_focus_on_new_window: MiriDefaults::SCROLL_MAINTAIN_FOCUS_ON_NEW_WINDOW,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MiriConfig {
    pub global: GlobalConfig,
    pub master: MasterConfig,
    pub scroll: ScrollConfig,
}

impl Default for MiriConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            master: MasterConfig::default(),
            scroll: ScrollConfig::default(),
        }
    }
}

impl MiriConfig {
    pub fn load() -> Self {
        let config_path = Self::default_config_path();
        match Self::from_file(&config_path) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Warning: failed to load {}: {}", config_path.display(), e);
                eprintln!("Falling back to default configuration.");
                Self::default()
            }
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let config = Config::builder().add_source(File::from(path.to_path_buf())).build()?;
        config.try_deserialize()
    }

    pub fn default_config_path() -> PathBuf {
        match std::env::var("HOME") {
            Ok(home) => PathBuf::from(home).join(".config").join("miri").join("config.toml"),
            Err(_) => PathBuf::from(".config/miri/config.toml"),
        }
    }
}

fn deserialize_mode<'deserialize, D>(deserializer: D) -> Result<Mode, D::Error>
where
    D: Deserializer<'deserialize>,
{
    match Option::<String>::deserialize(deserializer)? {
        Some(s) => match s.to_lowercase().as_str() {
            "scroll" => Ok(Mode::Scroll),
            "master" => Ok(Mode::Master),
            _ => Ok(MiriDefaults::DEFAULT_WORKSPACE_MODE),
        },
        None => Ok(MiriDefaults::DEFAULT_WORKSPACE_MODE),
    }
}
