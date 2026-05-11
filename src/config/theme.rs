use serde::{Deserialize, Serialize};

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
        "black"   => RC::Black,
        "red"     => RC::Red,
        "green"   => RC::Green,
        "yellow"  => RC::Yellow,
        "blue"    => RC::Blue,
        "magenta" => RC::Magenta,
        "cyan"    => RC::Cyan,
        "white"   => RC::White,
        "gray" | "grey" => RC::Gray,
        "reset"   => RC::Reset,
        _         => RC::Reset,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeColors {
    pub background:         Color,
    pub foreground:         Color,
    pub selection_bg:       Color,
    pub selection_fg:       Color,
    pub directory:          Color,
    pub symlink:            Color,
    pub executable:         Color,
    pub archive:            Color,
    pub media:              Color,
    pub hidden:             Color,
    pub border_active:      Color,
    pub border_inactive:    Color,
    pub statusbar_bg:       Color,
    pub statusbar_fg:       Color,
    pub error:              Color,
    pub warning:            Color,
    pub success:            Color,
    pub modal_bg:           Color,
    pub modal_border:       Color,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            background:      Color::Named("reset".into()),
            foreground:      Color::Hex("#c0caf5".into()),
            selection_bg:    Color::Hex("#364a82".into()),
            selection_fg:    Color::Hex("#c0caf5".into()),
            directory:       Color::Hex("#7aa2f7".into()),
            symlink:         Color::Hex("#b4f9f8".into()),
            executable:      Color::Hex("#9ece6a".into()),
            archive:         Color::Hex("#f7768e".into()),
            media:           Color::Hex("#ff9e64".into()),
            hidden:          Color::Hex("#565f89".into()),
            border_active:   Color::Hex("#7aa2f7".into()),
            border_inactive: Color::Hex("#414868".into()),
            statusbar_bg:    Color::Hex("#1f2335".into()),
            statusbar_fg:    Color::Hex("#a9b1d6".into()),
            error:           Color::Hex("#f7768e".into()),
            warning:         Color::Hex("#e0af68".into()),
            success:         Color::Hex("#9ece6a".into()),
            modal_bg:        Color::Hex("#1a1b26".into()),
            modal_border:    Color::Hex("#bb9af7".into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeSymbols {
    pub dir_open:     String,
    pub dir_closed:   String,
    pub symlink:      String,
    pub file:         String,
    pub selected:     String,
    pub unselected:   String,
    pub copy_marker:  String,
    pub cut_marker:   String,
    pub error_marker: String,
}

impl Default for ThemeSymbols {
    fn default() -> Self {
        Self {
            dir_open:     " ".into(),
            dir_closed:   " ".into(),
            symlink:      " ".into(),
            file:         " ".into(),
            selected:     "󰄬 ".into(),
            unselected:   "  ".into(),
            copy_marker:  " ".into(),
            cut_marker:   "󰆐 ".into(),
            error_marker: " ".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Theme {
    pub colors:       ThemeColors,
    pub symbols:      ThemeSymbols,
    pub border_style: BorderStyle,
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
        assert_eq!(Color::Named("red".into()).to_ratatui(), ratatui::style::Color::Red);
        assert_eq!(Color::Named("green".into()).to_ratatui(), ratatui::style::Color::Green);
        assert_eq!(Color::Named("blue".into()).to_ratatui(), ratatui::style::Color::Blue);
        assert_eq!(Color::Named("reset".into()).to_ratatui(), ratatui::style::Color::Reset);
        assert_eq!(Color::Named("black".into()).to_ratatui(), ratatui::style::Color::Black);
        assert_eq!(Color::Named("white".into()).to_ratatui(), ratatui::style::Color::White);
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
        assert_eq!(Color::Named("gray".into()).to_ratatui(), ratatui::style::Color::Gray);
        assert_eq!(Color::Named("grey".into()).to_ratatui(), ratatui::style::Color::Gray);
    }

    #[test]
    fn color_case_insensitive() {
        assert_eq!(Color::Named("RED".into()).to_ratatui(), ratatui::style::Color::Red);
    }
}
