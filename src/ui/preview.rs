pub struct PreviewCache {
    pub path: Option<std::path::PathBuf>,
    pub content: PreviewContent,
}

pub enum PreviewContent {
    Empty,
    Loading,
    Text(String),
    Hex(String),
    DirListing(String),
    Error(String),
    NotPreviewable,
}

impl PreviewCache {
    pub fn new() -> Self {
        Self { path: None, content: PreviewContent::Empty }
    }

    /// Returns true if the cache needs to be refreshed for this entry.
    pub fn needs_refresh(&self, entry: Option<&crate::fs::Entry>) -> bool {
        match entry {
            None => self.path.is_some(),
            Some(e) => self.path.as_deref() != Some(&e.path),
        }
    }

    /// Load preview content synchronously.
    pub fn load(&mut self, entry: Option<&crate::fs::Entry>) {
        let Some(entry) = entry else {
            self.path = None;
            self.content = PreviewContent::Empty;
            return;
        };

        self.path = Some(entry.path.clone());
        self.content = load_content(entry);
    }
}

fn load_content(entry: &crate::fs::Entry) -> PreviewContent {
    use std::fs;
    use std::io::Read;

    if entry.is_dir() {
        match fs::read_dir(&entry.path) {
            Ok(rd) => {
                let mut lines: Vec<String> = rd
                    .filter_map(|r| r.ok())
                    .map(|de| {
                        let name = de.file_name().to_string_lossy().to_string();
                        let is_subdir = de.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        if is_subdir { format!(" {}/", name) } else { format!(" {}", name) }
                    })
                    .collect();
                lines.sort();
                if lines.is_empty() {
                    PreviewContent::DirListing(" (empty)".to_string())
                } else {
                    PreviewContent::DirListing(lines.join("\n"))
                }
            }
            Err(e) => PreviewContent::Error(format!("Cannot read: {}", e)),
        }
    } else if !entry.is_previewable() {
        PreviewContent::NotPreviewable
    } else {
        const MAX_PREVIEW_BYTES: u64 = 131072;
        let meta = match fs::metadata(&entry.path) {
            Ok(m) => m,
            Err(e) => return PreviewContent::Error(format!("Cannot read: {}", e)),
        };
        let bytes = if meta.len() > MAX_PREVIEW_BYTES {
            let mut file = match std::fs::File::open(&entry.path) {
                Ok(f) => f,
                Err(e) => return PreviewContent::Error(format!("Cannot read: {}", e)),
            };
            let mut buf = vec![0u8; MAX_PREVIEW_BYTES as usize];
            let n = file.read(&mut buf).unwrap_or(0);
            buf.truncate(n);
            buf
        } else {
            match std::fs::read(&entry.path) {
                Ok(b) => b,
                Err(e) => return PreviewContent::Error(format!("Cannot read: {}", e)),
            }
        };

        if let Ok(text) = String::from_utf8(bytes.clone()) {
            PreviewContent::Text(text)
        } else {
            let hex = bytes
                .chunks(16)
                .map(|chunk| chunk.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>()
                .join("\n");
            PreviewContent::Hex(hex)
        }
    }
}

pub fn draw_preview(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, cache: &PreviewCache) {
    use ratatui::{
        style::{Color, Style},
        text::{Line, Span},
        widgets::{Block, BorderType, Paragraph},
    };
    use crate::ui::syntax_highlighter;

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Preview ")
        .style(Style::default().fg(Color::Rgb(65, 72, 104)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (text, color) = match &cache.content {
        PreviewContent::Empty | PreviewContent::Loading => {
            ("".to_string(), Color::Rgb(169, 177, 214))
        }
        PreviewContent::DirListing(s) => {
            let lines: String = s.lines()
                .take(inner.height as usize)
                .collect::<Vec<_>>()
                .join("\n");
            (lines, Color::Rgb(122, 162, 247))
        }
        PreviewContent::Text(s) => {
            let max_lines = inner.height as usize;
            let highlighted = syntax_highlighter::highlight_text(s, cache.path.as_deref());
            let styled_lines: Vec<Line> = highlighted
                .into_iter()
                .take(max_lines)
                .map(|fragments| {
                    Line::from(
                        fragments
                            .into_iter()
                            .map(|(style, text)| Span::styled(text, style))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect();

            if !styled_lines.is_empty() {
                frame.render_widget(Paragraph::new(styled_lines), inner);
                return;
            }
            let lines: String = s.lines()
                .take(inner.height as usize)
                .collect::<Vec<_>>()
                .join("\n");
            (lines, Color::Rgb(169, 177, 214))
        }
        PreviewContent::Hex(s) => {
            let lines: String = s.lines()
                .take(inner.height as usize)
                .collect::<Vec<_>>()
                .join("\n");
            (lines, Color::Rgb(169, 177, 214))
        }
        PreviewContent::NotPreviewable => {
            (" Binary file \u{2014} preview unavailable".to_string(), Color::Rgb(247, 118, 142))
        }
        PreviewContent::Error(e) => {
            (format!("Cannot read: {}", e), Color::Rgb(247, 118, 142))
        }
    };

    if !text.is_empty() {
        frame.render_widget(
            Paragraph::new(text).style(Style::default().fg(color)),
            inner,
        );
    }
}
