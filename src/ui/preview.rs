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

    const MAX_PREVIEW_BYTES: u64 = 131072;

    // Check file metadata first
    let metadata = match fs::metadata(&entry.path) {
        Ok(m) => m,
        Err(e) => {
            frame.render_widget(
                Paragraph::new(format!("Cannot read: {}", e))
                    .style(Style::default().fg(Color::Rgb(247, 118, 142))),
                inner,
            );
            return;
        }
    };

    // Read file contents (capped)
    let bytes = if metadata.len() > MAX_PREVIEW_BYTES {
        use std::io::Read;
        let mut file = match std::fs::File::open(&entry.path) {
            Ok(f) => f,
            Err(e) => {
                frame.render_widget(
                    Paragraph::new(format!("Cannot read: {}", e))
                        .style(Style::default().fg(Color::Rgb(247, 118, 142))),
                    inner,
                );
                return;
            }
        };
        let mut buf = vec![0u8; MAX_PREVIEW_BYTES as usize];
        let n = file.read(&mut buf).unwrap_or(0);
        buf.truncate(n);
        buf
    } else {
        match std::fs::read(&entry.path) {
            Ok(b) => b,
            Err(e) => {
                frame.render_widget(
                    Paragraph::new(format!("Cannot read: {}", e))
                        .style(Style::default().fg(Color::Rgb(247, 118, 142))),
                    inner,
                );
                return;
            }
        }
    };

    let preview = if let Ok(text) = String::from_utf8(bytes.clone()) {
        text.lines()
            .take(inner.height as usize)
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        bytes
            .chunks(16)
            .take(inner.height as usize)
            .map(|chunk| {
                chunk.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")
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
