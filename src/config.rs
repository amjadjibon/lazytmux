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

use crate::ui::ThemePreset;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default)]
    pub preset: ThemePreset,
    pub accent_color: String,
    pub border_style: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            preset: ThemePreset::Default,
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

/// Directories searched for lazytmux data, most preferred first.
///
/// The XDG location is listed first on every platform: it is what the README
/// documents and what terminal users expect, including on macOS where
/// `ProjectDirs` would otherwise resolve to `~/Library/Application Support`.
/// The platform directory is still honoured so existing installs keep working.
fn config_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        dirs.push(xdg.join("lazytmux"));
    }
    if let Some(home) = dirs_fallback() {
        let dot_config = home.join(".config").join("lazytmux");
        if !dirs.contains(&dot_config) {
            dirs.push(dot_config);
        }
    }
    if let Some(project) = ProjectDirs::from("com", "lazytmux", "lazytmux") {
        let platform = project.config_dir().to_path_buf();
        if !dirs.contains(&platform) {
            dirs.push(platform);
        }
    }

    dirs
}

/// The first existing candidate for `file_name`, else the preferred location.
pub fn resolve_path(file_name: &str) -> Option<PathBuf> {
    let dirs = config_dirs();
    dirs.iter()
        .map(|d| d.join(file_name))
        .find(|p| p.exists())
        .or_else(|| dirs.first().map(|d| d.join(file_name)))
}

impl Config {
    pub fn config_path() -> Option<PathBuf> {
        resolve_path("config.toml")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.refresh_interval_ms, 750);
        assert_eq!(config.pane_preview_lines, 30);
        assert!(config.confirm_on_kill);
        assert!(config.enable_mouse);
        assert_eq!(config.theme.accent_color, "cyan");
        assert_eq!(config.theme.border_style, "rounded");
    }

    #[test]
    fn test_custom_toml_deserialization() {
        let toml_str = r#"
            refresh_interval_ms = 500
            pane_preview_lines = 50
            confirm_on_kill = false
            enable_mouse = true

            [theme]
            accent_color = "green"
            border_style = "double"
        "#;

        let config: Config = toml::from_str(toml_str).expect("Valid TOML");
        assert_eq!(config.refresh_interval_ms, 500);
        assert_eq!(config.pane_preview_lines, 50);
        assert!(!config.confirm_on_kill);
        assert!(config.enable_mouse);
        assert_eq!(config.theme.accent_color, "green");
        assert_eq!(config.theme.border_style, "double");
    }

    #[test]
    fn test_config_path_prefers_xdg_location() {
        let path = Config::config_path().expect("a config path is always resolvable");
        assert!(path.ends_with("lazytmux/config.toml"), "got {path:?}");
        // The README documents ~/.config/lazytmux; unless the user already has a
        // config elsewhere, that is where we look.
        if std::env::var_os("XDG_CONFIG_HOME").is_none()
            && let Some(home) = dirs_fallback()
        {
            let documented = home.join(".config").join("lazytmux").join("config.toml");
            let platform_exists = ProjectDirs::from("com", "lazytmux", "lazytmux")
                .map(|d| d.config_dir().join("config.toml").exists())
                .unwrap_or(false);
            if !platform_exists {
                assert_eq!(path, documented);
            }
        }
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let original = Config::default();
        let serialized = toml::to_string(&original).expect("Serialization should succeed");
        let deserialized: Config =
            toml::from_str(&serialized).expect("Deserialization should succeed");
        assert_eq!(
            deserialized.refresh_interval_ms,
            original.refresh_interval_ms
        );
        assert_eq!(deserialized.theme.accent_color, original.theme.accent_color);
    }
}
