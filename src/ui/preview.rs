pub struct PreviewCache {
    pub path: Option<std::path::PathBuf>,
    pub content: PreviewContent,
    pub pending_rx:
        Option<std::sync::mpsc::Receiver<(std::path::PathBuf, PreviewContent)>>,
    pub loading_path: Option<std::path::PathBuf>,
}

#[derive(Debug, PartialEq)]
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
        Self {
            path: None,
            content: PreviewContent::Empty,
            pending_rx: None,
            loading_path: None,
        }
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

    /// Initiate async loading on a background thread.
    pub fn start_load(&mut self, entry: Option<&crate::fs::Entry>) {
        use std::thread;

        let Some(entry) = entry else {
            self.path = None;
            self.content = PreviewContent::Empty;
            self.loading_path = None;
            self.pending_rx = None;
            return;
        };

        // If already loading this exact path, no-op (avoids duplicate loads)
        if self.loading_path.as_deref() == Some(&entry.path) {
            return;
        }

        let entry_path = entry.path.clone();
        let is_dir = entry.is_dir();
        let is_previewable = entry.is_previewable();
        let (tx, rx) = std::sync::mpsc::channel();

        self.path = Some(entry_path.clone());
        self.content = PreviewContent::Loading;
        self.loading_path = Some(entry_path.clone());
        self.pending_rx = Some(rx);

        thread::spawn(move || {
            let content = if is_dir {
                load_content_for_dir(&entry_path)
            } else if !is_previewable {
                PreviewContent::NotPreviewable
            } else {
                load_content_for_file(&entry_path)
            };
            let _ = tx.send((entry_path, content));
        });
    }

    /// Poll for completed async load.
    pub fn check_completion(&mut self) {
        use std::sync::mpsc::TryRecvError;
        let Some(rx) = &self.pending_rx else { return };
        match rx.try_recv() {
            Ok((path, content)) => {
                // Only accept result matching the current loading_path
                if self.loading_path.as_deref() == Some(&path) {
                    self.content = content;
                    self.loading_path = None;
                    self.pending_rx = None;
                }
                // Stale result for a different path → silently drop
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.pending_rx = None;
                self.loading_path = None;
            }
        }
    }

    /// Cancel any in-flight load and reset pending state.
    pub fn cancel_load(&mut self) {
        self.pending_rx = None;
        self.loading_path = None;
    }
}

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use crate::ui::preview::{draw_preview, PreviewCache, PreviewContent};

    #[test]
    fn test_draw_preview_loading_shows_text() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut cache = PreviewCache::new();
        cache.path = Some(std::path::PathBuf::from("/test"));
        cache.content = PreviewContent::Loading;

        terminal
            .draw(|frame| {
                draw_preview(frame, frame.size(), &cache);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let content: String = buf
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<Vec<&str>>()
            .concat();
        assert!(
            content.contains("Loading"),
            "Loading variant should display 'Loading' text"
        );
    }

    #[test]
    fn test_draw_preview_empty_shows_nothing() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let cache = PreviewCache::new();

        terminal
            .draw(|frame| {
                draw_preview(frame, frame.size(), &cache);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let content: String = buf
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<Vec<&str>>()
            .concat();
        assert!(
            !content.contains("Loading"),
            "Empty variant should not contain 'Loading'"
        );
    }

    #[test]
    fn test_draw_preview_not_previewable_shows_message() {
        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut cache = PreviewCache::new();
        cache.path = Some(std::path::PathBuf::from("/test.bin"));
        cache.content = PreviewContent::NotPreviewable;

        terminal
            .draw(|frame| {
                draw_preview(frame, frame.size(), &cache);
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let content: String = buf
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<Vec<&str>>()
            .concat();
        assert!(
            content.contains("preview"),
            "NotPreviewable should show preview message"
        );
    }
}

fn load_content(entry: &crate::fs::Entry) -> PreviewContent {
    if entry.is_dir() {
        load_content_for_dir(&entry.path)
    } else if !entry.is_previewable() {
        PreviewContent::NotPreviewable
    } else {
        load_content_for_file(&entry.path)
    }
}

fn load_content_for_dir(path: &std::path::PathBuf) -> PreviewContent {
    use std::fs;

    match fs::read_dir(path) {
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
}

fn load_content_for_file(path: &std::path::PathBuf) -> PreviewContent {
    use std::fs;
    use std::io::Read;

    const MAX_PREVIEW_BYTES: u64 = 131072;
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return PreviewContent::Error(format!("Cannot read: {}", e)),
    };
    let bytes = if meta.len() > MAX_PREVIEW_BYTES {
        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => return PreviewContent::Error(format!("Cannot read: {}", e)),
        };
        let mut buf = vec![0u8; MAX_PREVIEW_BYTES as usize];
        let n = file.read(&mut buf).unwrap_or(0);
        buf.truncate(n);
        buf
    } else {
        match std::fs::read(path) {
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
        PreviewContent::Empty => {
            ("".to_string(), Color::Rgb(169, 177, 214))
        }
        PreviewContent::Loading => {
            (" Loading...".to_string(), Color::Rgb(122, 162, 247))
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

#[cfg(test)]
mod async_loading_tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use crate::fs::{Entry, EntryKind};

    fn make_entry(path: &str) -> Entry {
        Entry {
            path: PathBuf::from(path),
            name: PathBuf::from(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            kind: EntryKind::File,
            size: Some(42),
            modified: None,
            permissions: None,
            is_hidden: false,
            is_executable: false,
            extension: PathBuf::from(path)
                .extension()
                .map(|e| e.to_string_lossy().to_string()),
        }
    }

    /// start_load sets content to Loading, stores path and loading_path.
    #[test]
    fn test_start_load_sets_loading() {
        let mut cache = PreviewCache::new();
        let entry = make_entry("/tmp/test.txt");
        cache.start_load(Some(&entry));

        assert_eq!(cache.path, Some(PathBuf::from("/tmp/test.txt")));
        assert_eq!(cache.content, PreviewContent::Loading);
        assert_eq!(cache.loading_path, Some(PathBuf::from("/tmp/test.txt")));
    }

    /// start_load stores a different path correctly.
    #[test]
    fn test_start_load_stores_path() {
        let mut cache = PreviewCache::new();
        let entry = make_entry("/tmp/other.rs");
        cache.start_load(Some(&entry));

        assert_eq!(cache.path, Some(PathBuf::from("/tmp/other.rs")));
        assert_eq!(cache.loading_path, Some(PathBuf::from("/tmp/other.rs")));
    }

    /// check_completion is a no-op when no pending_rx is set.
    #[test]
    fn test_check_completion_noop_when_idle() {
        let mut cache = PreviewCache::new();
        cache.content = PreviewContent::Text("original".to_string());
        cache.check_completion();
        assert_eq!(cache.content, PreviewContent::Text("original".to_string()));
    }

    /// check_completion receives a result from the channel and swaps Loading → content.
    #[test]
    fn test_check_completion_swaps_content() {
        let mut cache = PreviewCache::new();
        let (tx, rx) = mpsc::channel();
        let test_path = PathBuf::from("/tmp/test.rs");

        cache.path = Some(test_path.clone());
        cache.content = PreviewContent::Loading;
        cache.loading_path = Some(test_path.clone());
        cache.pending_rx = Some(rx);

        tx.send((test_path.clone(), PreviewContent::Text("loaded".to_string())))
            .unwrap();
        cache.check_completion();

        assert_eq!(cache.content, PreviewContent::Text("loaded".to_string()));
        assert!(cache.loading_path.is_none());
        assert!(cache.pending_rx.is_none());
    }

    /// check_completion rejects a stale result whose path doesn't match loading_path.
    #[test]
    fn test_stale_result_rejected() {
        let mut cache = PreviewCache::new();
        let (tx, rx) = mpsc::channel();
        let path_a = PathBuf::from("/tmp/a.rs");
        let path_b = PathBuf::from("/tmp/b.rs");

        cache.path = Some(path_b.clone());
        cache.content = PreviewContent::Loading;
        cache.loading_path = Some(path_b.clone());
        cache.pending_rx = Some(rx);

        tx.send((path_a, PreviewContent::Text("from_a".to_string())))
            .unwrap();
        cache.check_completion();

        // Still loading — stale result silently dropped
        assert_eq!(cache.content, PreviewContent::Loading);
        assert_eq!(cache.loading_path, Some(path_b));
    }

    /// start_load with same path while already loading is a no-op.
    #[test]
    fn test_start_load_same_path_reentry() {
        let mut cache = PreviewCache::new();
        let entry = make_entry("/tmp/test.txt");

        cache.content = PreviewContent::Loading;
        cache.loading_path = Some(PathBuf::from("/tmp/test.txt"));

        cache.start_load(Some(&entry));

        assert_eq!(cache.content, PreviewContent::Loading);
        assert_eq!(cache.loading_path, Some(PathBuf::from("/tmp/test.txt")));
    }
}
