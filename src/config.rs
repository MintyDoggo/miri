use config::{Config, ConfigError, File};
use serde::{Deserialize, Deserializer, Serialize};
use std::path::{Path, PathBuf};

use crate::service_state::Mode;

struct MiriDefaults;

impl MiriDefaults {
    const DEFAULT_WORKSPACE_MODE: Mode = Mode::Master;
    const MASTER_WIDTH_PERCENTAGE: f64 = 50.0;
    const MASTER_MAXIMIZE_SINGLE_WINDOW: bool = true;
    const MASTER_SINGLE_WINDOW_MAX_WIDTH: f64 = 0.0;
    const MASTER_SINGLE_WINDOW_ASPECT_RATIO: f64 = 0.0;
    const SCROLL_MAINTAIN_FOCUS_ON_NEW_WINDOW: bool = false;
    const SCROLL_SPREAD_WINDOWS_ON_ENTER: bool = false;
    const SCROLL_COLUMN_WIDTH_PERCENTAGE: f64 = 50.0;
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
    pub single_window_max_width: f64,
    pub single_window_aspect_ratio: f64,
}

impl Default for MasterConfig {
    fn default() -> Self {
        Self {
            column_width_percentage: MiriDefaults::MASTER_WIDTH_PERCENTAGE,
            maximize_single_window: MiriDefaults::MASTER_MAXIMIZE_SINGLE_WINDOW,
            single_window_max_width: MiriDefaults::MASTER_SINGLE_WINDOW_MAX_WIDTH,
            single_window_aspect_ratio: MiriDefaults::MASTER_SINGLE_WINDOW_ASPECT_RATIO,
        }
    }
}

impl MasterConfig {
    pub fn limits_single_window(&self) -> bool {
        self.single_window_max_width > 0.0 || self.single_window_aspect_ratio > 0.0
    }

    pub fn single_window_width(&self, output_width: f64, output_height: f64) -> Option<i32> {
        let mut width = output_width;

        if self.single_window_aspect_ratio > 0.0 {
            width = width.min(output_height * self.single_window_aspect_ratio);
        }
        if self.single_window_max_width > 0.0 {
            width = width.min(self.single_window_max_width);
        }

        if width >= output_width {
            return None;
        }

        Some(width.round() as i32)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScrollConfig {
    pub maintain_focus_on_new_window: bool,
    pub spread_windows_on_enter: bool,
    pub column_width_percentage: f64,
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            maintain_focus_on_new_window: MiriDefaults::SCROLL_MAINTAIN_FOCUS_ON_NEW_WINDOW,
            spread_windows_on_enter: MiriDefaults::SCROLL_SPREAD_WINDOWS_ON_ENTER,
            column_width_percentage: MiriDefaults::SCROLL_COLUMN_WIDTH_PERCENTAGE,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn master_config(max_width: f64, aspect_ratio: f64) -> MasterConfig {
        MasterConfig {
            single_window_max_width: max_width,
            single_window_aspect_ratio: aspect_ratio,
            ..MasterConfig::default()
        }
    }

    #[test]
    fn unlimited_single_window_takes_the_full_width() {
        let config = master_config(0.0, 0.0);

        assert!(!config.limits_single_window());
        assert_eq!(config.single_window_width(3440.0, 1440.0), None);
    }

    #[test]
    fn max_width_caps_a_wider_output_only() {
        let config = master_config(2400.0, 0.0);

        assert_eq!(config.single_window_width(3440.0, 1440.0), Some(2400));
        assert_eq!(config.single_window_width(1920.0, 1080.0), None);
    }

    #[test]
    fn aspect_ratio_scales_with_the_output_height() {
        let config = master_config(0.0, 16.0 / 9.0);

        assert_eq!(config.single_window_width(3440.0, 1440.0), Some(2560));
        assert_eq!(config.single_window_width(1920.0, 1080.0), None);
    }

    #[test]
    fn the_smaller_of_the_two_limits_wins() {
        assert_eq!(
            master_config(2400.0, 16.0 / 9.0).single_window_width(3440.0, 1440.0),
            Some(2400)
        );
        assert_eq!(
            master_config(2800.0, 16.0 / 9.0).single_window_width(3440.0, 1440.0),
            Some(2560)
        );
    }
}
