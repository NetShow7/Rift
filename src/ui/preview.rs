use crate::fs::Entry;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Paragraph},
    Frame,
};
use std::fs;

pub fn draw_preview(frame: &mut Frame, area: Rect, entry: Option<&Entry>) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Preview ")
        .style(Style::default().fg(Color::Rgb(65, 72, 104)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(entry) = entry else { return };

    if entry.is_dir() {
        // Show child count
        let count = fs::read_dir(&entry.path)
            .map(|d| d.count())
            .unwrap_or(0);
        frame.render_widget(
            Paragraph::new(format!(" {} items", count))
                .style(Style::default().fg(Color::Rgb(122, 162, 247))),
            inner,
        );
        return;
    }

    // Try to read as text
    match fs::read(&entry.path) {
        Ok(bytes) => {
            let preview = if let Ok(text) = String::from_utf8(bytes.clone()) {
                let lines: String = text
                    .lines()
                    .take(inner.height as usize)
                    .collect::<Vec<_>>()
                    .join("\n");
                lines
            } else {
                // Binary: hex preview
                bytes
                    .chunks(16)
                    .take(inner.height as usize)
                    .map(|chunk| {
                        chunk
                            .iter()
                            .map(|b| format!("{:02x}", b))
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            frame.render_widget(
                Paragraph::new(preview)
                    .style(Style::default().fg(Color::Rgb(169, 177, 214))),
                inner,
            );
        }
        Err(e) => {
            frame.render_widget(
                Paragraph::new(format!("Cannot read: {}", e))
                    .style(Style::default().fg(Color::Rgb(247, 118, 142))),
                inner,
            );
        }
    }
}
