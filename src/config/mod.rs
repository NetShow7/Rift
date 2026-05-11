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

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rift")
            .join("config.toml")
    }
}
