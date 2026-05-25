use crate::{
    config::Theme,
    fs::{Entry, EntryKind},
};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
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
    pub scroll_threshold: usize,
    pub visible_height: usize,
}

impl Pane {
    pub fn new(cwd: PathBuf, entries: Vec<Entry>, scroll_threshold: usize) -> Self {
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
            scroll_threshold,
            visible_height: 20,
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

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, title: &str, drop_left: bool) {
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

        let borders = if drop_left { Borders::TOP | Borders::RIGHT | Borders::BOTTOM } else { Borders::ALL };
        let block = Block::new()
            .borders(borders)
            .border_type(border_type)
            .title(format!(" {} ", title))
            .style(Style::default().fg(border_color));

        let visible_height = area.height.saturating_sub(2) as usize;
        self.visible_height = visible_height;

        let threshold = if visible_height > 0 {
            self.scroll_threshold.min(visible_height / 2)
        } else {
            0
        };

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
            )
            .scroll_padding(threshold);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entries() -> Vec<Entry> {
        vec![
            Entry {
                path: PathBuf::from("/projects"),
                name: "projects".into(),
                kind: EntryKind::Directory,
                size: None,
                modified: None,
                permissions: None,
                is_hidden: false,
                is_executable: false,
                extension: None,
            },
            Entry {
                path: PathBuf::from("/readme.md"),
                name: "readme.md".into(),
                kind: EntryKind::File,
                size: Some(100),
                modified: None,
                permissions: None,
                is_hidden: false,
                is_executable: false,
                extension: Some("md".into()),
            },
            Entry {
                path: PathBuf::from("/src"),
                name: "src".into(),
                kind: EntryKind::Directory,
                size: None,
                modified: None,
                permissions: None,
                is_hidden: false,
                is_executable: false,
                extension: None,
            },
            Entry {
                path: PathBuf::from("/test.rs"),
                name: "test.rs".into(),
                kind: EntryKind::File,
                size: Some(200),
                modified: None,
                permissions: None,
                is_hidden: false,
                is_executable: false,
                extension: Some("rs".into()),
            },
        ]
    }

    #[test]
    fn pane_new() {
        let entries = make_entries();
        let pane = Pane::new(PathBuf::from("/"), entries, 3);
        assert_eq!(pane.cwd, PathBuf::from("/"));
        assert_eq!(pane.entries.len(), 4);
        assert_eq!(pane.cursor, 0);
        assert!(pane.selected.is_empty());
        assert!(pane.filter.is_none());
        assert!(!pane.is_active);
    }

    #[test]
    fn visible_entries_no_filter() {
        let pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        assert_eq!(pane.visible_entries().len(), 4);
    }

    #[test]
    fn visible_entries_with_filter() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.filter = Some("readme".into());
        assert_eq!(pane.visible_entries().len(), 1);
        assert_eq!(pane.visible_entries()[0].name, "readme.md");
    }

    #[test]
    fn visible_entries_filter_no_match() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.filter = Some("nonexistent".into());
        assert!(pane.visible_entries().is_empty());
    }

    #[test]
    fn focused_entry() {
        let pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        assert_eq!(pane.focused_entry().unwrap().name, "projects");
    }

    #[test]
    fn focused_entry_empty() {
        let pane = Pane::new(PathBuf::from("/"), vec![], 3);
        assert!(pane.focused_entry().is_none());
    }

    #[test]
    fn move_up_stays_at_top() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.move_up();
        assert_eq!(pane.cursor, 0);
    }

    #[test]
    fn move_down_stays_at_bottom() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.cursor = 3;
        pane.move_down();
        assert_eq!(pane.cursor, 3);
    }

    #[test]
    fn move_up_down() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.move_down();
        assert_eq!(pane.cursor, 1);
        pane.move_down();
        assert_eq!(pane.cursor, 2);
        pane.move_up();
        assert_eq!(pane.cursor, 1);
    }

    #[test]
    fn goto_top() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.cursor = 2;
        pane.goto_top();
        assert_eq!(pane.cursor, 0);
    }

    #[test]
    fn goto_bottom() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.goto_bottom();
        assert_eq!(pane.cursor, 3);
    }

    #[test]
    fn goto_bottom_empty() {
        let mut pane = Pane::new(PathBuf::from("/"), vec![], 3);
        pane.goto_bottom();
        assert_eq!(pane.cursor, 0);
    }

    #[test]
    fn page_up_saturates() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.page_up(10);
        assert_eq!(pane.cursor, 0);
    }

    #[test]
    fn page_down_bounded() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.page_down(100);
        assert_eq!(pane.cursor, 3);
    }

    #[test]
    fn page_up_partial() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.cursor = 3;
        pane.page_up(2);
        assert_eq!(pane.cursor, 1);
    }

    #[test]
    fn page_down_partial() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.page_down(2);
        assert_eq!(pane.cursor, 2);
    }

    #[test]
    fn toggle_selection() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.toggle_selection();
        assert_eq!(pane.selected.len(), 1);
        assert!(pane.selected.contains(&PathBuf::from("/projects")));
        pane.toggle_selection();
        assert!(pane.selected.is_empty());
    }

    #[test]
    fn toggle_selection_no_focused_entry() {
        let mut pane = Pane::new(PathBuf::from("/"), vec![], 3);
        pane.toggle_selection();
        assert!(pane.selected.is_empty());
    }

    #[test]
    fn select_all() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.select_all();
        assert_eq!(pane.selected.len(), 4);
    }

    #[test]
    fn clear_selection() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.select_all();
        pane.clear_selection();
        assert!(pane.selected.is_empty());
    }

    #[test]
    fn operative_entries_uses_selected_when_present() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.selected.insert(PathBuf::from("/readme.md"));
        pane.selected.insert(PathBuf::from("/test.rs"));
        let ops = pane.operative_entries();
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn operative_entries_uses_focused_when_no_selection() {
        let pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        let ops = pane.operative_entries();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0], PathBuf::from("/projects"));
    }

    #[test]
    fn operative_entries_empty_when_no_entries() {
        let pane = Pane::new(PathBuf::from("/"), vec![], 3);
        assert!(pane.operative_entries().is_empty());
    }

    #[test]
    fn filter_is_case_insensitive() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.filter = Some("README".into());
        assert_eq!(pane.visible_entries().len(), 1);
        pane.filter = Some("TEST".into());
        assert_eq!(pane.visible_entries().len(), 1);
    }

    #[test]
    fn filter_partial_match() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.filter = Some("ro".into());
        assert_eq!(pane.visible_entries().len(), 1);
        assert_eq!(pane.visible_entries()[0].name, "projects");
    }

    #[test]
    fn move_up_down_with_filter() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        pane.filter = Some("src".into());
        assert_eq!(pane.visible_entries().len(), 1);
        pane.move_down();
        assert_eq!(pane.cursor, 0);
    }

    #[test]
    fn list_state_updated_on_move() {
        let mut pane = Pane::new(PathBuf::from("/"), make_entries(), 3);
        assert_eq!(pane.list_state.selected(), Some(0));
        pane.move_down();
        assert_eq!(pane.list_state.selected(), Some(1));
        pane.move_up();
        assert_eq!(pane.list_state.selected(), Some(0));
    }
}
