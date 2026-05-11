use crate::{config::Theme, fs::Entry};
use humansize::{format_size, BINARY};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::path::Path;

pub struct StatusBar;

impl StatusBar {
    pub fn draw(
        frame: &mut Frame,
        area: Rect,
        theme: &Theme,
        cwd: &Path,
        focused: Option<&Entry>,
        selected_count: usize,
        has_clipboard: bool,
        clipboard_is_cut: bool,
        filter: Option<&str>,
    ) {
        let bg = theme.colors.statusbar_bg.to_ratatui();
        let fg = theme.colors.statusbar_fg.to_ratatui();
        let accent = theme.colors.directory.to_ratatui();
        let warn = theme.colors.warning.to_ratatui();

        let cwd_str = cwd.display().to_string();

        // Left side: cwd
        let mut left = vec![
            Span::styled(" ", Style::default().bg(bg)),
            Span::styled(&cwd_str, Style::default().fg(accent).bg(bg)),
        ];

        if let Some(f) = filter {
            left.push(Span::styled(
                format!("  [filter: {}]", f),
                Style::default().fg(warn).bg(bg),
            ));
        }

        // Right side: selection count, clipboard, file info
        let mut right_parts: Vec<String> = Vec::new();

        if selected_count > 0 {
            right_parts.push(format!("{} selected", selected_count));
        }
        if has_clipboard {
            let op = if clipboard_is_cut { "cut" } else { "copy" };
            right_parts.push(format!("[{}]", op));
        }

        if let Some(entry) = focused {
            if let Some(size) = entry.size {
                right_parts.push(format_size(size, BINARY));
            }
            if let Some(modified) = &entry.modified {
                right_parts.push(modified.format("%Y-%m-%d %H:%M").to_string());
            }
        }

        let right_str = right_parts.join("  ");

        // Pad to fill width
        let left_str: String = left.iter().map(|s| s.content.as_ref()).collect();
        let padding = area
            .width
            .saturating_sub(left_str.len() as u16 + right_str.len() as u16 + 2) as usize;
        let pad = " ".repeat(padding);

        let line = Line::from(vec![
            Span::styled(left_str, Style::default().fg(fg).bg(bg)),
            Span::styled(pad, Style::default().bg(bg)),
            Span::styled(format!("{}  ", right_str), Style::default().fg(fg).bg(bg)),
        ]);

        frame.render_widget(Paragraph::new(line).style(Style::default().bg(bg)), area);
    }
}
