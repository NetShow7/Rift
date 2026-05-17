use serde::{Deserialize, Deserializer, Serialize};

/// A colour represented as a 24-bit hex string (#rrggbb) or a named ANSI colour.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Color {
    Hex(String),
    Named(String),
}

impl Default for Color {
    fn default() -> Self {
        Color::Named("reset".into())
    }
}

impl Color {
    /// Convert to ratatui Color.
    pub fn to_ratatui(&self) -> ratatui::style::Color {
        match self {
            Color::Hex(s) | Color::Named(s) => parse_color(s),
        }
    }
}

fn parse_color(s: &str) -> ratatui::style::Color {
    use ratatui::style::Color as RC;
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return RC::Rgb(r, g, b);
            }
        }
    }
    match s.to_lowercase().as_str() {
        "black" => RC::Black,
        "red" => RC::Red,
        "green" => RC::Green,
        "yellow" => RC::Yellow,
        "blue" => RC::Blue,
        "magenta" => RC::Magenta,
        "cyan" => RC::Cyan,
        "white" => RC::White,
        "gray" | "grey" => RC::Gray,
        "reset" => RC::Reset,
        _ => RC::Reset,
    }
}

/// Border style for panes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BorderStyle {
    Plain,
    Rounded,
    Double,
    Thick,
    None,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self::Rounded
    }
}

/// Built-in theme preset selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreset {
    Custom,
    TokyoNight,
    CatppuccinMocha,
    CatppuccinLatte,
    Dracula,
    EverforestDark,
    EverforestLight,
    Nord,
    SolarizedDark,
    SolarizedLight,
    GruvboxDark,
    GruvboxLight,
}

impl Default for ThemePreset {
    fn default() -> Self {
        Self::TokyoNight
    }
}

impl ThemePreset {
    /// Serde default — unknown/missing values become Custom.
    pub fn custom() -> Self {
        Self::Custom
    }

    /// Human-readable label for UI display.
    pub fn display_name(&self) -> &'static str {
        use ThemePreset::*;
        match self {
            Custom => "Custom",
            TokyoNight => "Tokyo Night",
            CatppuccinMocha => "Catppuccin Mocha",
            CatppuccinLatte => "Catppuccin Latte",
            Dracula => "Dracula",
            EverforestDark => "Everforest Dark",
            EverforestLight => "Everforest Light",
            Nord => "Nord",
            SolarizedDark => "Solarized Dark",
            SolarizedLight => "Solarized Light",
            GruvboxDark => "Gruvbox Dark",
            GruvboxLight => "Gruvbox Light",
        }
    }

    /// All built-in presets use rounded borders.
    pub fn border_style(&self) -> BorderStyle {
        BorderStyle::Rounded
    }

    fn theme_colors(&self) -> Option<ThemeColors> {
        use ThemePreset::*;
        match self {
            Custom => None,
            TokyoNight => Some(ThemeColors {
                background: Color::Named("reset".into()),
                foreground: Color::Hex("#c0caf5".into()),
                selection_bg: Color::Hex("#364a82".into()),
                selection_fg: Color::Hex("#c0caf5".into()),
                directory: Color::Hex("#7aa2f7".into()),
                symlink: Color::Hex("#b4f9f8".into()),
                executable: Color::Hex("#9ece6a".into()),
                archive: Color::Hex("#f7768e".into()),
                media: Color::Hex("#ff9e64".into()),
                hidden: Color::Hex("#565f89".into()),
                border_active: Color::Hex("#7aa2f7".into()),
                border_inactive: Color::Hex("#414868".into()),
                statusbar_bg: Color::Hex("#1f2335".into()),
                statusbar_fg: Color::Hex("#a9b1d6".into()),
                error: Color::Hex("#f7768e".into()),
                warning: Color::Hex("#e0af68".into()),
                success: Color::Hex("#9ece6a".into()),
                modal_bg: Color::Hex("#1a1b26".into()),
                modal_border: Color::Hex("#bb9af7".into()),
            }),
            CatppuccinMocha => Some(ThemeColors {
                background: Color::Hex("#1e1e2e".into()),
                foreground: Color::Hex("#cdd6f4".into()),
                selection_bg: Color::Hex("#45475a".into()),
                selection_fg: Color::Hex("#cdd6f4".into()),
                directory: Color::Hex("#89b4fa".into()),
                symlink: Color::Hex("#94e2d5".into()),
                executable: Color::Hex("#a6e3a1".into()),
                archive: Color::Hex("#f38ba8".into()),
                media: Color::Hex("#fab387".into()),
                hidden: Color::Hex("#585b70".into()),
                border_active: Color::Hex("#89b4fa".into()),
                border_inactive: Color::Hex("#45475a".into()),
                statusbar_bg: Color::Hex("#181825".into()),
                statusbar_fg: Color::Hex("#a6adc8".into()),
                error: Color::Hex("#f38ba8".into()),
                warning: Color::Hex("#f9e2af".into()),
                success: Color::Hex("#a6e3a1".into()),
                modal_bg: Color::Hex("#11111b".into()),
                modal_border: Color::Hex("#cba6f7".into()),
            }),
            CatppuccinLatte => Some(ThemeColors {
                background: Color::Hex("#eff1f5".into()),
                foreground: Color::Hex("#4c4f69".into()),
                selection_bg: Color::Hex("#ccd0da".into()),
                selection_fg: Color::Hex("#4c4f69".into()),
                directory: Color::Hex("#1e66f5".into()),
                symlink: Color::Hex("#04a5e5".into()),
                executable: Color::Hex("#40a02b".into()),
                archive: Color::Hex("#d20f39".into()),
                media: Color::Hex("#fe640b".into()),
                hidden: Color::Hex("#9ca0b0".into()),
                border_active: Color::Hex("#1e66f5".into()),
                border_inactive: Color::Hex("#ccd0da".into()),
                statusbar_bg: Color::Hex("#e6e9ef".into()),
                statusbar_fg: Color::Hex("#5c5f77".into()),
                error: Color::Hex("#d20f39".into()),
                warning: Color::Hex("#df8e1d".into()),
                success: Color::Hex("#40a02b".into()),
                modal_bg: Color::Hex("#dce0e8".into()),
                modal_border: Color::Hex("#7287fd".into()),
            }),
            Dracula => Some(ThemeColors {
                background: Color::Hex("#282a36".into()),
                foreground: Color::Hex("#f8f8f2".into()),
                selection_bg: Color::Hex("#44475a".into()),
                selection_fg: Color::Hex("#f8f8f2".into()),
                directory: Color::Hex("#8be9fd".into()),
                symlink: Color::Hex("#bd93f9".into()),
                executable: Color::Hex("#50fa7b".into()),
                archive: Color::Hex("#ff5555".into()),
                media: Color::Hex("#ffb86c".into()),
                hidden: Color::Hex("#6272a4".into()),
                border_active: Color::Hex("#bd93f9".into()),
                border_inactive: Color::Hex("#44475a".into()),
                statusbar_bg: Color::Hex("#21222c".into()),
                statusbar_fg: Color::Hex("#6272a4".into()),
                error: Color::Hex("#ff5555".into()),
                warning: Color::Hex("#f1fa8c".into()),
                success: Color::Hex("#50fa7b".into()),
                modal_bg: Color::Hex("#1c1d26".into()),
                modal_border: Color::Hex("#ff79c6".into()),
            }),
            EverforestDark => Some(ThemeColors {
                background: Color::Hex("#2d353b".into()),
                foreground: Color::Hex("#d3c6aa".into()),
                selection_bg: Color::Hex("#475258".into()),
                selection_fg: Color::Hex("#d3c6aa".into()),
                directory: Color::Hex("#7fbbb3".into()),
                symlink: Color::Hex("#e69875".into()),
                executable: Color::Hex("#a7c080".into()),
                archive: Color::Hex("#e67e80".into()),
                media: Color::Hex("#e69875".into()),
                hidden: Color::Hex("#5c6a6f".into()),
                border_active: Color::Hex("#7fbbb3".into()),
                border_inactive: Color::Hex("#475258".into()),
                statusbar_bg: Color::Hex("#232a2e".into()),
                statusbar_fg: Color::Hex("#859289".into()),
                error: Color::Hex("#e67e80".into()),
                warning: Color::Hex("#dbbc7f".into()),
                success: Color::Hex("#a7c080".into()),
                modal_bg: Color::Hex("#1e2428".into()),
                modal_border: Color::Hex("#d699b6".into()),
            }),
            EverforestLight => Some(ThemeColors {
                background: Color::Hex("#fdf6e9".into()),
                foreground: Color::Hex("#5c6a72".into()),
                selection_bg: Color::Hex("#dfd8c8".into()),
                selection_fg: Color::Hex("#5c6a72".into()),
                directory: Color::Hex("#609f92".into()),
                symlink: Color::Hex("#e37e5c".into()),
                executable: Color::Hex("#85981c".into()),
                archive: Color::Hex("#e65e72".into()),
                media: Color::Hex("#e37e5c".into()),
                hidden: Color::Hex("#b3b09e".into()),
                border_active: Color::Hex("#609f92".into()),
                border_inactive: Color::Hex("#dfd8c8".into()),
                statusbar_bg: Color::Hex("#f1ebda".into()),
                statusbar_fg: Color::Hex("#9ca8a0".into()),
                error: Color::Hex("#e65e72".into()),
                warning: Color::Hex("#d7a95a".into()),
                success: Color::Hex("#85981c".into()),
                modal_bg: Color::Hex("#f1ebda".into()),
                modal_border: Color::Hex("#d699b6".into()),
            }),
            Nord => Some(ThemeColors {
                background: Color::Hex("#2e3440".into()),
                foreground: Color::Hex("#d8dee9".into()),
                selection_bg: Color::Hex("#434c5e".into()),
                selection_fg: Color::Hex("#d8dee9".into()),
                directory: Color::Hex("#88c0d0".into()),
                symlink: Color::Hex("#b48ead".into()),
                executable: Color::Hex("#a3be8c".into()),
                archive: Color::Hex("#bf616a".into()),
                media: Color::Hex("#d08770".into()),
                hidden: Color::Hex("#4c566a".into()),
                border_active: Color::Hex("#81a1c1".into()),
                border_inactive: Color::Hex("#434c5e".into()),
                statusbar_bg: Color::Hex("#242933".into()),
                statusbar_fg: Color::Hex("#616e88".into()),
                error: Color::Hex("#bf616a".into()),
                warning: Color::Hex("#ebcb8b".into()),
                success: Color::Hex("#a3be8c".into()),
                modal_bg: Color::Hex("#1e232e".into()),
                modal_border: Color::Hex("#b48ead".into()),
            }),
            SolarizedDark => Some(ThemeColors {
                background: Color::Hex("#002b36".into()),
                foreground: Color::Hex("#839496".into()),
                selection_bg: Color::Hex("#073642".into()),
                selection_fg: Color::Hex("#839496".into()),
                directory: Color::Hex("#268bd2".into()),
                symlink: Color::Hex("#6c71c4".into()),
                executable: Color::Hex("#859900".into()),
                archive: Color::Hex("#dc322f".into()),
                media: Color::Hex("#cb4b16".into()),
                hidden: Color::Hex("#586e75".into()),
                border_active: Color::Hex("#268bd2".into()),
                border_inactive: Color::Hex("#073642".into()),
                statusbar_bg: Color::Hex("#00151d".into()),
                statusbar_fg: Color::Hex("#657b83".into()),
                error: Color::Hex("#dc322f".into()),
                warning: Color::Hex("#b58900".into()),
                success: Color::Hex("#859900".into()),
                modal_bg: Color::Hex("#00212b".into()),
                modal_border: Color::Hex("#d33682".into()),
            }),
            SolarizedLight => Some(ThemeColors {
                background: Color::Hex("#fdf6e3".into()),
                foreground: Color::Hex("#657b83".into()),
                selection_bg: Color::Hex("#eee8d5".into()),
                selection_fg: Color::Hex("#657b83".into()),
                directory: Color::Hex("#268bd2".into()),
                symlink: Color::Hex("#6c71c4".into()),
                executable: Color::Hex("#859900".into()),
                archive: Color::Hex("#dc322f".into()),
                media: Color::Hex("#cb4b16".into()),
                hidden: Color::Hex("#93a1a1".into()),
                border_active: Color::Hex("#268bd2".into()),
                border_inactive: Color::Hex("#eee8d5".into()),
                statusbar_bg: Color::Hex("#f5efdc".into()),
                statusbar_fg: Color::Hex("#586e75".into()),
                error: Color::Hex("#dc322f".into()),
                warning: Color::Hex("#b58900".into()),
                success: Color::Hex("#859900".into()),
                modal_bg: Color::Hex("#fcf4dc".into()),
                modal_border: Color::Hex("#d33682".into()),
            }),
            GruvboxDark => Some(ThemeColors {
                background: Color::Hex("#282828".into()),
                foreground: Color::Hex("#ebdbb2".into()),
                selection_bg: Color::Hex("#504945".into()),
                selection_fg: Color::Hex("#ebdbb2".into()),
                directory: Color::Hex("#458588".into()),
                symlink: Color::Hex("#b16286".into()),
                executable: Color::Hex("#98971a".into()),
                archive: Color::Hex("#cc241d".into()),
                media: Color::Hex("#d65d0e".into()),
                hidden: Color::Hex("#665c54".into()),
                border_active: Color::Hex("#689d6a".into()),
                border_inactive: Color::Hex("#504945".into()),
                statusbar_bg: Color::Hex("#1d2021".into()),
                statusbar_fg: Color::Hex("#a89984".into()),
                error: Color::Hex("#cc241d".into()),
                warning: Color::Hex("#d79921".into()),
                success: Color::Hex("#98971a".into()),
                modal_bg: Color::Hex("#141617".into()),
                modal_border: Color::Hex("#b16286".into()),
            }),
            GruvboxLight => Some(ThemeColors {
                background: Color::Hex("#fbf1c7".into()),
                foreground: Color::Hex("#3c3836".into()),
                selection_bg: Color::Hex("#d5c4a1".into()),
                selection_fg: Color::Hex("#3c3836".into()),
                directory: Color::Hex("#458588".into()),
                symlink: Color::Hex("#b16286".into()),
                executable: Color::Hex("#98971a".into()),
                archive: Color::Hex("#cc241d".into()),
                media: Color::Hex("#d65d0e".into()),
                hidden: Color::Hex("#bdae93".into()),
                border_active: Color::Hex("#689d6a".into()),
                border_inactive: Color::Hex("#d5c4a1".into()),
                statusbar_bg: Color::Hex("#ebdbb2".into()),
                statusbar_fg: Color::Hex("#7c6f64".into()),
                error: Color::Hex("#cc241d".into()),
                warning: Color::Hex("#d79921".into()),
                success: Color::Hex("#98971a".into()),
                modal_bg: Color::Hex("#f2e5bc".into()),
                modal_border: Color::Hex("#b16286".into()),
            }),
        }
    }
}

/// Custom deserializer: unknown strings fall back to [`ThemePreset::Custom`].
impl<'de> Deserialize<'de> for ThemePreset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "tokyo_night" => ThemePreset::TokyoNight,
            "catppuccin_mocha" => ThemePreset::CatppuccinMocha,
            "catppuccin_latte" => ThemePreset::CatppuccinLatte,
            "dracula" => ThemePreset::Dracula,
            "everforest_dark" => ThemePreset::EverforestDark,
            "everforest_light" => ThemePreset::EverforestLight,
            "nord" => ThemePreset::Nord,
            "solarized_dark" => ThemePreset::SolarizedDark,
            "solarized_light" => ThemePreset::SolarizedLight,
            "gruvbox_dark" => ThemePreset::GruvboxDark,
            "gruvbox_light" => ThemePreset::GruvboxLight,
            _ => ThemePreset::Custom,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeColors {
    pub background: Color,
    pub foreground: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub directory: Color,
    pub symlink: Color,
    pub executable: Color,
    pub archive: Color,
    pub media: Color,
    pub hidden: Color,
    pub border_active: Color,
    pub border_inactive: Color,
    pub statusbar_bg: Color,
    pub statusbar_fg: Color,
    pub error: Color,
    pub warning: Color,
    pub success: Color,
    pub modal_bg: Color,
    pub modal_border: Color,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            background: Color::Named("reset".into()),
            foreground: Color::Hex("#c0caf5".into()),
            selection_bg: Color::Hex("#364a82".into()),
            selection_fg: Color::Hex("#c0caf5".into()),
            directory: Color::Hex("#7aa2f7".into()),
            symlink: Color::Hex("#b4f9f8".into()),
            executable: Color::Hex("#9ece6a".into()),
            archive: Color::Hex("#f7768e".into()),
            media: Color::Hex("#ff9e64".into()),
            hidden: Color::Hex("#565f89".into()),
            border_active: Color::Hex("#7aa2f7".into()),
            border_inactive: Color::Hex("#414868".into()),
            statusbar_bg: Color::Hex("#1f2335".into()),
            statusbar_fg: Color::Hex("#a9b1d6".into()),
            error: Color::Hex("#f7768e".into()),
            warning: Color::Hex("#e0af68".into()),
            success: Color::Hex("#9ece6a".into()),
            modal_bg: Color::Hex("#1a1b26".into()),
            modal_border: Color::Hex("#bb9af7".into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeSymbols {
    pub dir_open: String,
    pub dir_closed: String,
    pub symlink: String,
    pub file: String,
    pub selected: String,
    pub unselected: String,
    pub copy_marker: String,
    pub cut_marker: String,
    pub error_marker: String,
}

impl Default for ThemeSymbols {
    fn default() -> Self {
        Self {
            dir_open: " ".into(),
            dir_closed: " ".into(),
            symlink: " ".into(),
            file: " ".into(),
            selected: "󰄬 ".into(),
            unselected: "  ".into(),
            copy_marker: " ".into(),
            cut_marker: "󰆐 ".into(),
            error_marker: " ".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    #[serde(default = "ThemePreset::custom")]
    pub preset: ThemePreset,
    pub colors: ThemeColors,
    pub symbols: ThemeSymbols,
    pub border_style: BorderStyle,
}

impl Default for Theme {
    fn default() -> Self {
        let mut theme = Self {
            preset: ThemePreset::Custom,
            colors: ThemeColors::default(),
            symbols: ThemeSymbols::default(),
            border_style: BorderStyle::default(),
        };
        theme.apply_preset(ThemePreset::TokyoNight);
        theme.preset = ThemePreset::TokyoNight;
        theme
    }
}

impl Theme {
    /// Overwrite all colour fields and border style to match the given preset.
    /// Does nothing for [`ThemePreset::Custom`].
    pub fn apply_preset(&mut self, preset: ThemePreset) {
        if let Some(colors) = preset.theme_colors() {
            self.colors = colors;
            self.border_style = preset.border_style();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_default_is_reset() {
        assert!(matches!(Color::default(), Color::Named(s) if s == "reset"));
    }

    #[test]
    fn hex_color_to_ratatui() {
        let c = Color::Hex("#c0caf5".into());
        assert_eq!(c.to_ratatui(), ratatui::style::Color::Rgb(192, 202, 245));
    }

    #[test]
    fn named_color_to_ratatui() {
        assert_eq!(
            Color::Named("red".into()).to_ratatui(),
            ratatui::style::Color::Red
        );
        assert_eq!(
            Color::Named("green".into()).to_ratatui(),
            ratatui::style::Color::Green
        );
        assert_eq!(
            Color::Named("blue".into()).to_ratatui(),
            ratatui::style::Color::Blue
        );
        assert_eq!(
            Color::Named("reset".into()).to_ratatui(),
            ratatui::style::Color::Reset
        );
        assert_eq!(
            Color::Named("black".into()).to_ratatui(),
            ratatui::style::Color::Black
        );
        assert_eq!(
            Color::Named("white".into()).to_ratatui(),
            ratatui::style::Color::White
        );
    }

    #[test]
    fn unknown_named_color_falls_back_to_reset() {
        assert_eq!(
            Color::Named("burgundy".into()).to_ratatui(),
            ratatui::style::Color::Reset
        );
    }

    #[test]
    fn border_style_default() {
        assert_eq!(BorderStyle::default(), BorderStyle::Rounded);
    }

    #[test]
    fn theme_colors_default_has_tokyo_night_palette() {
        let c = ThemeColors::default();
        assert!(matches!(c.background, Color::Named(s) if s == "reset"));
        assert!(matches!(c.foreground, Color::Hex(s) if s == "#c0caf5"));
        assert!(matches!(c.directory, Color::Hex(s) if s == "#7aa2f7"));
        assert!(matches!(c.error, Color::Hex(s) if s == "#f7768e"));
    }

    #[test]
    fn theme_symbols_default() {
        let s = ThemeSymbols::default();
        assert_eq!(s.selected, "󰄬 ");
        assert_eq!(s.unselected, "  ");
    }

    #[test]
    fn theme_default_has_rounded_border() {
        let t = Theme::default();
        assert_eq!(t.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn named_color_gray_and_grey() {
        assert_eq!(
            Color::Named("gray".into()).to_ratatui(),
            ratatui::style::Color::Gray
        );
        assert_eq!(
            Color::Named("grey".into()).to_ratatui(),
            ratatui::style::Color::Gray
        );
    }

    #[test]
    fn color_case_insensitive() {
        assert_eq!(
            Color::Named("RED".into()).to_ratatui(),
            ratatui::style::Color::Red
        );
    }

    // --- ThemePreset tests ---

    #[test]
    fn theme_preset_default_is_tokyo_night() {
        assert_eq!(ThemePreset::default(), ThemePreset::TokyoNight);
    }

    #[test]
    fn theme_preset_custom_returns_custom() {
        assert_eq!(ThemePreset::custom(), ThemePreset::Custom);
    }

    #[test]
    fn theme_preset_display_names() {
        assert_eq!(ThemePreset::Custom.display_name(), "Custom");
        assert_eq!(ThemePreset::TokyoNight.display_name(), "Tokyo Night");
        assert_eq!(ThemePreset::CatppuccinMocha.display_name(), "Catppuccin Mocha");
        assert_eq!(ThemePreset::CatppuccinLatte.display_name(), "Catppuccin Latte");
        assert_eq!(ThemePreset::Dracula.display_name(), "Dracula");
        assert_eq!(ThemePreset::EverforestDark.display_name(), "Everforest Dark");
        assert_eq!(ThemePreset::EverforestLight.display_name(), "Everforest Light");
        assert_eq!(ThemePreset::Nord.display_name(), "Nord");
        assert_eq!(ThemePreset::SolarizedDark.display_name(), "Solarized Dark");
        assert_eq!(ThemePreset::SolarizedLight.display_name(), "Solarized Light");
        assert_eq!(ThemePreset::GruvboxDark.display_name(), "Gruvbox Dark");
        assert_eq!(ThemePreset::GruvboxLight.display_name(), "Gruvbox Light");
    }

    #[test]
    fn theme_preset_border_style_rounded_for_all() {
        let presets = [
            ThemePreset::Custom,
            ThemePreset::TokyoNight,
            ThemePreset::CatppuccinMocha,
            ThemePreset::CatppuccinLatte,
            ThemePreset::Dracula,
            ThemePreset::EverforestDark,
            ThemePreset::EverforestLight,
            ThemePreset::Nord,
            ThemePreset::SolarizedDark,
            ThemePreset::SolarizedLight,
            ThemePreset::GruvboxDark,
            ThemePreset::GruvboxLight,
        ];
        for p in presets {
            assert_eq!(p.border_style(), BorderStyle::Rounded, "{}", p.display_name());
        }
    }

    #[test]
    fn theme_default_uses_tokyo_night_preset() {
        let t = Theme::default();
        assert_eq!(t.preset, ThemePreset::TokyoNight);
    }

    #[test]
    fn apply_preset_changes_colors() {
        let mut theme = Theme::default();
        theme.apply_preset(ThemePreset::Nord);

        assert_eq!(theme.preset, ThemePreset::TokyoNight, "apply_preset should not change the preset field");
        assert!(matches!(theme.colors.background, Color::Hex(s) if s == "#2e3440"));
        assert!(matches!(theme.colors.foreground, Color::Hex(s) if s == "#d8dee9"));
        assert!(matches!(theme.colors.directory, Color::Hex(s) if s == "#88c0d0"));
        assert_eq!(theme.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn apply_preset_custom_does_nothing() {
        let mut theme = Theme::default();
        theme.colors.foreground = Color::Hex("#ff0000".into());
        theme.apply_preset(ThemePreset::Custom);

        assert!(matches!(theme.colors.foreground, Color::Hex(s) if s == "#ff0000"));
    }

    #[test]
    fn preset_serde_roundtrip() {
        let variants = [
            (ThemePreset::TokyoNight, "tokyo_night"),
            (ThemePreset::CatppuccinMocha, "catppuccin_mocha"),
            (ThemePreset::Dracula, "dracula"),
            (ThemePreset::Nord, "nord"),
            (ThemePreset::Custom, "custom"),
        ];
        for (preset, expected) in &variants {
            let toml_str = format!("preset = \"{}\"", expected);
            let theme: Theme = toml::from_str(&toml_str).unwrap();
            assert_eq!(theme.preset, *preset);
        }
    }

    #[test]
    fn preset_deserialize_unknown_falls_back_to_custom() {
        let toml_str = "preset = \"unknown_preset_name\"".to_string();
        let theme: Theme = toml::from_str(&toml_str).unwrap();
        assert_eq!(theme.preset, ThemePreset::Custom);
    }

    #[test]
    fn apply_preset_tokyo_night_matches_default_colors() {
        let default_colors = ThemeColors::default();
        let mut theme = Theme::default();
        theme.apply_preset(ThemePreset::TokyoNight);

        assert_eq!(
            format!("{:?}", theme.colors.background),
            format!("{:?}", default_colors.background)
        );
        assert_eq!(
            format!("{:?}", theme.colors.foreground),
            format!("{:?}", default_colors.foreground)
        );
        assert_eq!(
            format!("{:?}", theme.colors.directory),
            format!("{:?}", default_colors.directory)
        );
    }

    // --- Preset color accuracy tests ---

    #[test]
    fn preset_tokyo_night_values() {
        let mut theme = Theme::default();
        theme.apply_preset(ThemePreset::TokyoNight);
        assert!(matches!(theme.colors.background, Color::Named(s) if s == "reset"));
        assert!(matches!(theme.colors.foreground, Color::Hex(s) if s == "#c0caf5"));
        assert!(matches!(theme.colors.directory, Color::Hex(s) if s == "#7aa2f7"));
        assert_eq!(theme.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn preset_catppuccin_mocha_values() {
        let mut theme = Theme::default();
        theme.apply_preset(ThemePreset::CatppuccinMocha);
        assert!(matches!(theme.colors.foreground, Color::Hex(s) if s == "#cdd6f4"));
        assert!(matches!(theme.colors.directory, Color::Hex(s) if s == "#89b4fa"));
        assert!(matches!(theme.colors.error, Color::Hex(s) if s == "#f38ba8"));
        assert_eq!(theme.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn preset_catppuccin_latte_values() {
        let mut theme = Theme::default();
        theme.apply_preset(ThemePreset::CatppuccinLatte);
        assert!(matches!(theme.colors.foreground, Color::Hex(s) if s == "#4c4f69"));
        assert!(matches!(theme.colors.directory, Color::Hex(s) if s == "#1e66f5"));
        assert!(matches!(theme.colors.error, Color::Hex(s) if s == "#d20f39"));
        assert_eq!(theme.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn preset_dracula_values() {
        let mut theme = Theme::default();
        theme.apply_preset(ThemePreset::Dracula);
        assert!(matches!(theme.colors.foreground, Color::Hex(s) if s == "#f8f8f2"));
        assert!(matches!(theme.colors.background, Color::Hex(s) if s == "#282a36"));
        assert!(matches!(theme.colors.directory, Color::Hex(s) if s == "#8be9fd"));
        assert_eq!(theme.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn preset_everforest_dark_values() {
        let mut theme = Theme::default();
        theme.apply_preset(ThemePreset::EverforestDark);
        assert!(matches!(theme.colors.foreground, Color::Hex(s) if s == "#d3c6aa"));
        assert!(matches!(theme.colors.background, Color::Hex(s) if s == "#2d353b"));
        assert!(matches!(theme.colors.directory, Color::Hex(s) if s == "#7fbbb3"));
        assert_eq!(theme.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn preset_everforest_light_values() {
        let mut theme = Theme::default();
        theme.apply_preset(ThemePreset::EverforestLight);
        assert!(matches!(theme.colors.foreground, Color::Hex(s) if s == "#5c6a72"));
        assert!(matches!(theme.colors.background, Color::Hex(s) if s == "#fdf6e9"));
        assert!(matches!(theme.colors.directory, Color::Hex(s) if s == "#609f92"));
        assert_eq!(theme.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn preset_nord_values() {
        let mut theme = Theme::default();
        theme.apply_preset(ThemePreset::Nord);
        assert!(matches!(theme.colors.foreground, Color::Hex(s) if s == "#d8dee9"));
        assert!(matches!(theme.colors.background, Color::Hex(s) if s == "#2e3440"));
        assert!(matches!(theme.colors.directory, Color::Hex(s) if s == "#88c0d0"));
        assert_eq!(theme.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn preset_solarized_dark_values() {
        let mut theme = Theme::default();
        theme.apply_preset(ThemePreset::SolarizedDark);
        assert!(matches!(theme.colors.foreground, Color::Hex(s) if s == "#839496"));
        assert!(matches!(theme.colors.background, Color::Hex(s) if s == "#002b36"));
        assert!(matches!(theme.colors.directory, Color::Hex(s) if s == "#268bd2"));
        assert_eq!(theme.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn preset_solarized_light_values() {
        let mut theme = Theme::default();
        theme.apply_preset(ThemePreset::SolarizedLight);
        assert!(matches!(theme.colors.foreground, Color::Hex(s) if s == "#657b83"));
        assert!(matches!(theme.colors.background, Color::Hex(s) if s == "#fdf6e3"));
        assert!(matches!(theme.colors.directory, Color::Hex(s) if s == "#268bd2"));
        assert_eq!(theme.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn preset_gruvbox_dark_values() {
        let mut theme = Theme::default();
        theme.apply_preset(ThemePreset::GruvboxDark);
        assert!(matches!(theme.colors.foreground, Color::Hex(s) if s == "#ebdbb2"));
        assert!(matches!(theme.colors.background, Color::Hex(s) if s == "#282828"));
        assert!(matches!(theme.colors.directory, Color::Hex(s) if s == "#458588"));
        assert_eq!(theme.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn preset_gruvbox_light_values() {
        let mut theme = Theme::default();
        theme.apply_preset(ThemePreset::GruvboxLight);
        assert!(matches!(theme.colors.foreground, Color::Hex(s) if s == "#3c3836"));
        assert!(matches!(theme.colors.background, Color::Hex(s) if s == "#fbf1c7"));
        assert!(matches!(theme.colors.directory, Color::Hex(s) if s == "#458588"));
        assert_eq!(theme.border_style, BorderStyle::Rounded);
    }

    // --- Application tests ---

    #[test]
    fn preset_apply_overwrites_all_fields() {
        let mut theme = Theme::default();
        theme.colors.foreground = Color::Hex("#111111".into());
        theme.colors.background = Color::Hex("#222222".into());
        theme.colors.selection_bg = Color::Hex("#333333".into());
        theme.colors.selection_fg = Color::Hex("#444444".into());
        theme.colors.directory = Color::Hex("#555555".into());
        theme.colors.symlink = Color::Hex("#666666".into());
        theme.colors.executable = Color::Hex("#777777".into());
        theme.colors.archive = Color::Hex("#888888".into());
        theme.colors.media = Color::Hex("#999999".into());
        theme.colors.hidden = Color::Hex("#aaaaaa".into());
        theme.colors.border_active = Color::Hex("#bbbbbb".into());
        theme.colors.border_inactive = Color::Hex("#cccccc".into());
        theme.colors.statusbar_bg = Color::Hex("#dddddd".into());
        theme.colors.statusbar_fg = Color::Hex("#eeeeee".into());
        theme.colors.error = Color::Hex("#ffffff".into());
        theme.colors.warning = Color::Hex("#000000".into());
        theme.colors.success = Color::Hex("#abcdef".into());
        theme.colors.modal_bg = Color::Hex("#fedcba".into());
        theme.colors.modal_border = Color::Hex("#123456".into());
        theme.border_style = BorderStyle::None;

        theme.apply_preset(ThemePreset::Nord);

        let nord_colors = ThemePreset::Nord.theme_colors().unwrap();
        assert_eq!(
            format!("{:?}", theme.colors),
            format!("{:?}", nord_colors),
            "All 18 color fields should be overwritten by Nord preset",
        );
        assert_eq!(theme.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn theme_default_matches_tokyo_night() {
        let default = Theme::default();
        let tokyo_colors = ThemePreset::TokyoNight.theme_colors().unwrap();
        assert_eq!(
            format!("{:?}", default.colors),
            format!("{:?}", tokyo_colors),
            "Default theme colors should exactly match TokyoNight preset",
        );
        assert_eq!(default.border_style, BorderStyle::Rounded);
        assert_eq!(default.preset, ThemePreset::TokyoNight);
    }

    #[test]
    fn preset_cycle_full() {
        let presets = [
            ThemePreset::TokyoNight,
            ThemePreset::CatppuccinMocha,
            ThemePreset::CatppuccinLatte,
            ThemePreset::Dracula,
            ThemePreset::EverforestDark,
            ThemePreset::EverforestLight,
            ThemePreset::Nord,
            ThemePreset::SolarizedDark,
            ThemePreset::SolarizedLight,
            ThemePreset::GruvboxDark,
            ThemePreset::GruvboxLight,
            ThemePreset::Custom,
        ];
        assert_eq!(presets.len(), 12, "Must have exactly 12 preset variants");

        let mut last_dbg: Option<String> = None;
        for p in &presets {
            if *p == ThemePreset::Custom {
                assert!(p.theme_colors().is_none(), "Custom preset should return None");
                continue;
            }
            let colors = p.theme_colors().unwrap();
            let dbg = format!("{:?}", colors);
            if let Some(ref prev) = last_dbg {
                assert_ne!(
                    dbg, *prev,
                    "Each preset must produce unique colors — {} repeats previous",
                    p.display_name(),
                );
            }
            last_dbg = Some(dbg);
        }
    }

    // --- Serialization tests ---

    #[test]
    fn preset_serde_snake_case() {
        let cases = [
            (ThemePreset::Custom, "custom"),
            (ThemePreset::TokyoNight, "tokyo_night"),
            (ThemePreset::CatppuccinMocha, "catppuccin_mocha"),
            (ThemePreset::CatppuccinLatte, "catppuccin_latte"),
            (ThemePreset::Dracula, "dracula"),
            (ThemePreset::EverforestDark, "everforest_dark"),
            (ThemePreset::EverforestLight, "everforest_light"),
            (ThemePreset::Nord, "nord"),
            (ThemePreset::SolarizedDark, "solarized_dark"),
            (ThemePreset::SolarizedLight, "solarized_light"),
            (ThemePreset::GruvboxDark, "gruvbox_dark"),
            (ThemePreset::GruvboxLight, "gruvbox_light"),
        ];
        for (preset, expected) in &cases {
            let theme = Theme {
                preset: *preset,
                ..Theme::default()
            };
            let s = toml::to_string(&theme).unwrap();
            assert!(
                s.contains(&format!("preset = \"{}\"", expected)),
                "preset {:?} should serialize as 'preset = \"{}\"'",
                preset,
                expected,
            );
        }
    }

    #[test]
    fn preset_deserialize_valid() {
        let cases = [
            ("tokyo_night", ThemePreset::TokyoNight),
            ("catppuccin_mocha", ThemePreset::CatppuccinMocha),
            ("catppuccin_latte", ThemePreset::CatppuccinLatte),
            ("dracula", ThemePreset::Dracula),
            ("everforest_dark", ThemePreset::EverforestDark),
            ("everforest_light", ThemePreset::EverforestLight),
            ("nord", ThemePreset::Nord),
            ("solarized_dark", ThemePreset::SolarizedDark),
            ("solarized_light", ThemePreset::SolarizedLight),
            ("gruvbox_dark", ThemePreset::GruvboxDark),
            ("gruvbox_light", ThemePreset::GruvboxLight),
        ];
        for (toml_name, expected) in &cases {
            let toml_str = format!("preset = \"{}\"", toml_name);
            let theme: Theme = toml::from_str(&toml_str).unwrap();
            assert_eq!(
                theme.preset, *expected,
                "Deserializing '{}' should give {:?}",
                toml_name, expected,
            );
        }
    }

    #[test]
    fn preset_serde_roundtrip_all_variants() {
        let variants = [
            ThemePreset::Custom,
            ThemePreset::TokyoNight,
            ThemePreset::CatppuccinMocha,
            ThemePreset::CatppuccinLatte,
            ThemePreset::Dracula,
            ThemePreset::EverforestDark,
            ThemePreset::EverforestLight,
            ThemePreset::Nord,
            ThemePreset::SolarizedDark,
            ThemePreset::SolarizedLight,
            ThemePreset::GruvboxDark,
            ThemePreset::GruvboxLight,
        ];
        for preset in &variants {
            let theme = Theme {
                preset: *preset,
                ..Theme::default()
            };
            let toml_str = toml::to_string(&theme).unwrap();
            let theme2: Theme = toml::from_str(&toml_str).unwrap();
            assert_eq!(
                theme2.preset, *preset,
                "Roundtrip failed for {:?}",
                preset,
            );
        }
    }
}
