use std::sync::LazyLock;

use ratatui::style::{Color, Modifier, Style};
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    FontStyle as SyntectFontStyle, Style as SyntectStyle, Theme, ThemeSet,
};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

static HIGHLIGHT_THEME: LazyLock<Theme> =
    LazyLock::new(|| THEME_SET.themes["base16-ocean.dark"].clone());

fn syntect_color_to_ratatui(color: syntect::highlighting::Color) -> Color {
    if color.a == 0 {
        return Color::default();
    }
    Color::Rgb(color.r, color.g, color.b)
}

fn syntect_font_style_to_modifier(font_style: SyntectFontStyle) -> Modifier {
    let mut m = Modifier::empty();
    if font_style.contains(SyntectFontStyle::BOLD) {
        m |= Modifier::BOLD;
    }
    if font_style.contains(SyntectFontStyle::ITALIC) {
        m |= Modifier::ITALIC;
    }
    if font_style.contains(SyntectFontStyle::UNDERLINE) {
        m |= Modifier::UNDERLINED;
    }
    m
}

fn syntect_style_to_ratatui(style: &SyntectStyle) -> Style {
    Style::default()
        .fg(syntect_color_to_ratatui(style.foreground))
        .bg(syntect_color_to_ratatui(style.background))
        .add_modifier(syntect_font_style_to_modifier(style.font_style))
}

pub type HighlightedLine = Vec<(Style, String)>;

/// Detect the syntax definition to use for `path` + `content`.
///
/// Resolution order:
/// 1. File extension (e.g. `.rs` → Rust)
/// 2. File name (e.g. `Makefile`, `Dockerfile`)
/// 3. First line / shebang of `content` (e.g. `#!/bin/bash`)
/// 4. Falls back to the plain-text pseudo-syntax.
pub fn detect_syntax(
    path: Option<&std::path::Path>,
    content: &str,
) -> &'static SyntaxReference {
    if let Some(path) = path {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(syntax) = SYNTAX_SET.find_syntax_by_extension(ext) {
                return syntax;
            }
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if let Some(syntax) = SYNTAX_SET.find_syntax_by_name(name) {
                return syntax;
            }
        }
    }

    let first_line = content.lines().next().unwrap_or("");
    SYNTAX_SET
        .find_syntax_by_first_line(first_line)
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text())
}

/// Highlight `content` line-by-line and return styled fragments for each line.
///
/// `path` is optional and used for language detection. When the language cannot
/// be detected, plain-text highlighting is applied (uniform style).
pub fn highlight_text(
    content: &str,
    path: Option<&std::path::Path>,
) -> Vec<HighlightedLine> {
    let syntax = detect_syntax(path, content);

        if syntax.name == "Plain Text" {
        return content
            .lines()
            .map(|line| vec![(Style::default(), line.to_string())])
            .collect();
    }

    let mut highlighter = HighlightLines::new(syntax, &HIGHLIGHT_THEME);
    let mut result = Vec::with_capacity(content.lines().count());

    for line in LinesWithEndings::from(content) {
        let ranges = match highlighter.highlight_line(line, &SYNTAX_SET) {
            Ok(r) => r,
            Err(_) => {
                result.push(vec![(Style::default(), line.to_string())]);
                continue;
            }
        };

        let styled: HighlightedLine = ranges
            .iter()
            .map(|(style, text)| (syntect_style_to_ratatui(style), text.to_string()))
            .collect();
        result.push(styled);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Modifier;

    #[test]
    fn fully_transparent_color_becomes_default() {
        let transparent = syntect::highlighting::Color { r: 0, g: 0, b: 0, a: 0 };
        assert_eq!(syntect_color_to_ratatui(transparent), Color::default());
    }

    #[test]
    fn opaque_color_maps_to_rgb() {
        let c = syntect::highlighting::Color { r: 0xab, g: 0xcd, b: 0xef, a: 0xff };
        assert_eq!(syntect_color_to_ratatui(c), Color::Rgb(0xab, 0xcd, 0xef));
    }

    #[test]
    fn font_style_empty_no_modifier() {
        let fs = SyntectFontStyle::empty();
        assert_eq!(syntect_font_style_to_modifier(fs), Modifier::empty());
    }

    #[test]
    fn font_style_bold() {
        assert!(syntect_font_style_to_modifier(SyntectFontStyle::BOLD).contains(Modifier::BOLD));
    }

    #[test]
    fn font_style_italic() {
        assert!(syntect_font_style_to_modifier(SyntectFontStyle::ITALIC).contains(Modifier::ITALIC));
    }

    #[test]
    fn font_style_underline() {
        assert!(syntect_font_style_to_modifier(SyntectFontStyle::UNDERLINE).contains(Modifier::UNDERLINED));
    }

    #[test]
    fn font_style_combined() {
        let combined = SyntectFontStyle::BOLD | SyntectFontStyle::ITALIC;
        let m = syntect_font_style_to_modifier(combined);
        assert!(m.contains(Modifier::BOLD));
        assert!(m.contains(Modifier::ITALIC));
    }

    #[test]
    fn style_conversion_round_trip() {
        let syntect_style = SyntectStyle {
            foreground: syntect::highlighting::Color { r: 0xff, g: 0x00, b: 0x00, a: 0xff },
            background: syntect::highlighting::Color { r: 0x00, g: 0x00, b: 0x00, a: 0xff },
            font_style: SyntectFontStyle::BOLD,
        };
        let rstyle = syntect_style_to_ratatui(&syntect_style);
        assert_eq!(rstyle.fg, Some(Color::Rgb(0xff, 0x00, 0x00)));
        assert_eq!(rstyle.bg, Some(Color::Rgb(0x00, 0x00, 0x00)));
        assert!(rstyle.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn detect_syntax_known_extension() {
        let p = std::path::Path::new("main.rs");
        let syntax = detect_syntax(Some(p), "");
        assert_eq!(syntax.name, "Rust");
    }

    #[test]
    fn detect_syntax_known_filename() {
        let p = std::path::Path::new("Makefile");
        let syntax = detect_syntax(Some(p), "");
        assert_eq!(syntax.name, "Makefile");
    }

    #[test]
    fn detect_syntax_shebang() {
        let syntax = detect_syntax(None, "#!/usr/bin/env python3\nprint('hi')");
        assert_eq!(syntax.name, "Python");
    }

    #[test]
    fn detect_syntax_unknown_falls_to_plain_text() {
        let syntax = detect_syntax(None, "some random text without a known pattern");
        assert_eq!(syntax.name, "Plain Text");
    }

    #[test]
    fn detect_syntax_extension_takes_priority_over_shebang() {
        let p = std::path::Path::new("README.md");
        let syntax = detect_syntax(Some(p), "#!/usr/bin/env python3");
        assert_eq!(syntax.name, "Markdown");
    }

    #[test]
    fn highlight_empty_content() {
        let lines = highlight_text("", None);
        assert!(lines.is_empty());
    }

    #[test]
    fn highlight_single_word_plain_text() {
        let lines = highlight_text("hello", None);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 1);
        assert_eq!(lines[0][0].1, "hello");
    }

    #[test]
    fn highlight_rust_code_produces_multiple_tokens() {
        let lines = highlight_text("fn main() {}", Some(std::path::Path::new("test.rs")));
        assert!(lines.len() >= 1);
        assert!(lines[0].len() >= 3, "expected ≥3 highlighted fragments on Rust code");
    }

    #[test]
    fn highlight_rust_multi_line() {
        let code = "fn main() {\n    println!(\"hi\");\n}\n";
        let lines = highlight_text(code, Some(std::path::Path::new("test.rs")));
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn highlight_python_shebang_detection() {
        let code = "#!/usr/bin/env python3\nprint('hello')\n";
        let lines = highlight_text(code, None);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].len() >= 1);
        assert!(lines[1].len() >= 2, "expected ≥2 fragments for Python print line");
    }

    #[test]
    fn detect_syntax_by_name_for_hardcoded_vs() {
        let syntax = SYNTAX_SET.find_syntax_by_name("Rust");
        assert!(syntax.is_some(), "Rust syntax should be loaded");
    }
}
