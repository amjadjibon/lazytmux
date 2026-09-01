use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub refresh_interval_ms: u64,
    pub pane_preview_lines: usize,
    pub confirm_on_kill: bool,
    pub enable_mouse: bool,
    pub theme: ThemeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub accent_color: String,
    pub border_style: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            accent_color: "cyan".to_string(),
            border_style: "rounded".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            refresh_interval_ms: 750,
            pane_preview_lines: 30,
            confirm_on_kill: true,
            enable_mouse: true,
            theme: ThemeConfig::default(),
        }
    }
}

impl Config {
    pub fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "lazytmux", "lazytmux")
            .map(|dirs| dirs.config_dir().join("config.toml"))
            .or_else(|| {
                dirs_fallback().map(|home| home.join(".config").join("lazytmux").join("config.toml"))
            })
    }

    pub fn load_or_default() -> Self {
        Self::config_path()
            .filter(|p| p.exists())
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|c| toml::from_str::<Config>(&c).ok())
            .unwrap_or_default()
    }
}

fn dirs_fallback() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
