use crate::{
    config::Theme,
    fs::{Entry, EntryKind},
};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState},
    Frame,
};
use std::collections::HashSet;
use std::path::PathBuf;

pub struct Pane {
    pub cwd: PathBuf,
    pub entries: Vec<Entry>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub selected: HashSet<PathBuf>,
    pub list_state: ListState,
    pub is_active: bool,
    pub filter: Option<String>,
}

impl Pane {
    pub fn new(cwd: PathBuf, entries: Vec<Entry>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            cwd,
            entries,
            cursor: 0,
            scroll_offset: 0,
            selected: HashSet::new(),
            list_state,
            is_active: false,
            filter: None,
        }
    }

    pub fn visible_entries(&self) -> Vec<&Entry> {
        match &self.filter {
            None => self.entries.iter().collect(),
            Some(f) => {
                let f = f.to_lowercase();
                self.entries
                    .iter()
                    .filter(|e| e.name.to_lowercase().contains(&f))
                    .collect()
            }
        }
    }

    pub fn focused_entry(&self) -> Option<&Entry> {
        self.visible_entries().get(self.cursor).copied()
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.list_state.select(Some(self.cursor));
        }
    }

    pub fn move_down(&mut self) {
        let len = self.visible_entries().len();
        if self.cursor + 1 < len {
            self.cursor += 1;
            self.list_state.select(Some(self.cursor));
        }
    }

    pub fn goto_top(&mut self) {
        self.cursor = 0;
        self.list_state.select(Some(0));
    }

    pub fn goto_bottom(&mut self) {
        let len = self.visible_entries().len();
        if len > 0 {
            self.cursor = len - 1;
            self.list_state.select(Some(self.cursor));
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.cursor = self.cursor.saturating_sub(page_size);
        self.list_state.select(Some(self.cursor));
    }

    pub fn page_down(&mut self, page_size: usize) {
        let len = self.visible_entries().len();
        self.cursor = (self.cursor + page_size).min(len.saturating_sub(1));
        self.list_state.select(Some(self.cursor));
    }

    pub fn toggle_selection(&mut self) {
        if let Some(entry) = self.focused_entry() {
            let path = entry.path.clone();
            if self.selected.contains(&path) {
                self.selected.remove(&path);
            } else {
                self.selected.insert(path);
            }
        }
    }

    pub fn select_all(&mut self) {
        for e in &self.entries {
            self.selected.insert(e.path.clone());
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    /// Entries to act on: selected set if non-empty, else focused entry.
    pub fn operative_entries(&self) -> Vec<PathBuf> {
        if !self.selected.is_empty() {
            self.selected.iter().cloned().collect()
        } else if let Some(e) = self.focused_entry() {
            vec![e.path.clone()]
        } else {
            vec![]
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, title: &str) {
        let border_color = if self.is_active {
            theme.colors.border_active.to_ratatui()
        } else {
            theme.colors.border_inactive.to_ratatui()
        };

        let border_type = match theme.border_style {
            crate::config::theme::BorderStyle::Rounded  => BorderType::Rounded,
            crate::config::theme::BorderStyle::Double   => BorderType::Double,
            crate::config::theme::BorderStyle::Thick    => BorderType::Thick,
            crate::config::theme::BorderStyle::Plain    => BorderType::Plain,
            crate::config::theme::BorderStyle::None     => BorderType::Plain,
        };

        let block = Block::bordered()
            .border_type(border_type)
            .title(format!(" {} ", title))
            .style(Style::default().fg(border_color));

        let visible = self.visible_entries();
        let items: Vec<ListItem> = visible
            .iter()
            .map(|e| {
                let is_sel = self.selected.contains(&e.path);
                let sel_sym = if is_sel {
                    theme.symbols.selected.as_str()
                } else {
                    theme.symbols.unselected.as_str()
                };

                let icon = entry_icon(e, &theme.symbols);
                let color = entry_color(e, theme);

                let style = if is_sel {
                    Style::default()
                        .fg(theme.colors.selection_fg.to_ratatui())
                        .bg(theme.colors.selection_bg.to_ratatui())
                } else if e.is_hidden {
                    Style::default().fg(theme.colors.hidden.to_ratatui())
                } else {
                    Style::default().fg(color)
                };

                let line = Line::from(vec![
                    Span::raw(sel_sym),
                    Span::raw(icon),
                    Span::styled(e.name.clone(), style),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(theme.colors.selection_bg.to_ratatui())
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
}

fn entry_icon<'a>(e: &Entry, symbols: &'a crate::config::theme::ThemeSymbols) -> &'a str {
    match &e.kind {
        EntryKind::Directory => &symbols.dir_open,
        EntryKind::Symlink { .. } => &symbols.symlink,
        EntryKind::File => &symbols.file,
        EntryKind::Special => &symbols.file,
    }
}

fn entry_color(e: &Entry, theme: &Theme) -> ratatui::style::Color {
    match &e.kind {
        EntryKind::Directory => theme.colors.directory.to_ratatui(),
        EntryKind::Symlink { .. } => theme.colors.symlink.to_ratatui(),
        EntryKind::File => {
            if e.is_archive() {
                theme.colors.archive.to_ratatui()
            } else if e.is_media() {
                theme.colors.media.to_ratatui()
            } else if e.is_executable {
                theme.colors.executable.to_ratatui()
            } else {
                theme.colors.foreground.to_ratatui()
            }
        }
        EntryKind::Special => theme.colors.foreground.to_ratatui(),
    }
}
