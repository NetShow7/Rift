pub mod keymap;
pub mod theme;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use keymap::{Action, KeyBinding, Keymap};
pub use theme::Theme;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutMode {
    Single,
    Dual,
    Miller,
}

impl Default for LayoutMode {
    fn default() -> Self {
        Self::Miller
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellMode {
    Takeover,
    Capture,
}

impl Default for ShellMode {
    fn default() -> Self {
        Self::Capture
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub layout: LayoutMode,
    pub show_hidden: bool,
    pub follow_symlinks: bool,
    pub scroll_threshold: usize,
    pub shell: String,
    pub shell_mode: ShellMode,
    pub confirm_delete: bool,
    pub trash_dir: Option<PathBuf>,
    pub show_shortcut_hints: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            layout: LayoutMode::default(),
            show_hidden: false,
            follow_symlinks: true,
            scroll_threshold: 500,
            shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()),
            shell_mode: ShellMode::default(),
            confirm_delete: true,
            show_shortcut_hints: true,
            trash_dir: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub theme: Theme,
    pub keymap: Keymap,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        toml::from_str(&raw)
            .with_context(|| format!("Failed to parse config at {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(&self)?;
        std::fs::write(&path, raw)?;
        Ok(())
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
        .join("rift")
        .join("config.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_mode_default() {
        assert_eq!(LayoutMode::default(), LayoutMode::Miller);
    }

    #[test]
    fn shell_mode_default() {
        assert_eq!(ShellMode::default(), ShellMode::Capture);
    }

    #[test]
    fn general_config_defaults() {
        let c = GeneralConfig::default();
        assert_eq!(c.layout, LayoutMode::Miller);
        assert!(!c.show_hidden);
        assert!(c.follow_symlinks);
        assert_eq!(c.scroll_threshold, 500);
        assert_eq!(c.shell_mode, ShellMode::Capture);
        assert!(c.confirm_delete);
        assert!(c.show_shortcut_hints);
        assert_eq!(c.trash_dir, None);
    }

    #[test]
    fn config_path_ends_correctly() {
        let p = Config::config_path();
        assert!(p.ends_with("rift/config.toml"));
    }

    #[test]
    fn config_default_is_ok() {
        let c = Config::default();
        assert_eq!(c.general.layout, LayoutMode::Miller);
    }

    #[test]
    fn config_toml_roundtrip() {
        let c = Config::default();
        let s = toml::to_string_pretty(&c).unwrap();
        let c2: Config = toml::from_str(&s).unwrap();
        let s2 = toml::to_string_pretty(&c2).unwrap();
        // Compare parsed values (HashMap ordering differs between serializations)
        assert_eq!(c.general.layout, c2.general.layout);
        assert_eq!(c.general.show_hidden, c2.general.show_hidden);
        assert_eq!(c.general.follow_symlinks, c2.general.follow_symlinks);
        assert_eq!(c.general.scroll_threshold, c2.general.scroll_threshold);
        assert_eq!(c.general.shell_mode, c2.general.shell_mode);
        assert_eq!(c.general.confirm_delete, c2.general.confirm_delete);
        assert_eq!(c.theme.border_style, c2.theme.border_style);
        assert_eq!(s.len(), s2.len());
    }
}
