use crate::config::SidebarPosition;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarSection {
    Favorites,
    Drives,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimState {
    Closed,
    Opening { current_width: u16, frame: u8 },
    Open,
    Closing { current_width: u16, frame: u8 },
    Peeking { current_width: u16 },
}

pub struct SidebarState {
    pub anim: AnimState,
    pub target_width: u16,
    pub cursor: usize,
    pub section: SidebarSection,
    pub favorites: Vec<String>,
    pub drives: Vec<String>,
    pub can_peek: bool,
    pub peek_held: bool,
    pub debounce_start: Option<std::time::Instant>,
    pub sidebar_focused: bool,
}

impl SidebarState {
    pub fn new(target_width: u16) -> Self {
        Self {
            anim: AnimState::Closed,
            target_width,
            cursor: 0,
            section: SidebarSection::Favorites,
            favorites: Vec::new(),
            drives: Vec::new(),
            can_peek: true,
            peek_held: false,
            debounce_start: None,
            sidebar_focused: false,
        }
    }

    pub fn toggle(&mut self) {
        match self.anim {
            AnimState::Closed | AnimState::Peeking { .. } => {
                let cw = self.current_width();
                self.anim = AnimState::Opening { current_width: cw.max(1), frame: 0 };
            }
            _ => {
                let cw = self.current_width();
                self.anim = AnimState::Closing { current_width: cw, frame: 0 };
            }
        }
        self.peek_held = false;
    }

    pub fn start_peek(&mut self) {
        if !self.can_peek { return; }
        self.peek_held = true;
        if matches!(self.anim, AnimState::Closed) {
            let cw = self.current_width();
            self.anim = AnimState::Opening { current_width: cw.max(1), frame: 0 };
        }
    }

    pub fn end_peek(&mut self) {
        self.peek_held = false;
        if let AnimState::Peeking { current_width } = self.anim {
            self.anim = AnimState::Closing { current_width, frame: 0 };
        }
    }

    pub fn is_open(&self) -> bool {
        matches!(self.anim, AnimState::Open)
    }

    pub fn current_width(&self) -> u16 {
        match self.anim {
            AnimState::Closed => 0,
            AnimState::Opening { current_width, .. } => current_width,
            AnimState::Open => self.target_width,
            AnimState::Closing { current_width, .. } => current_width,
            AnimState::Peeking { current_width } => current_width,
        }
    }

    pub fn is_animating(&self) -> bool {
        !matches!(self.anim, AnimState::Closed | AnimState::Open)
    }

    pub fn cursor_up(&mut self, fav_len: usize, drive_len: usize) {
        let total = fav_len + drive_len;
        if total == 0 { return; }
        if self.cursor == 0 {
            self.cursor = total - 1;
        } else {
            self.cursor -= 1;
        }
        self.update_section(fav_len);
    }

    pub fn cursor_down(&mut self, fav_len: usize, drive_len: usize) {
        let total = fav_len + drive_len;
        if total == 0 { return; }
        if self.cursor + 1 >= total {
            self.cursor = 0;
        } else {
            self.cursor += 1;
        }
        self.update_section(fav_len);
    }

    fn update_section(&mut self, fav_len: usize) {
        self.section = if self.cursor < fav_len {
            SidebarSection::Favorites
        } else {
            SidebarSection::Drives
        };
    }

    pub fn selected_path(&self, fav_len: usize) -> Option<&str> {
        if self.cursor < fav_len {
            self.favorites.get(self.cursor).map(|s| s.as_str())
        } else {
            self.drives.get(self.cursor - fav_len).map(|s| s.as_str())
        }
    }

    pub fn tick(&mut self) {
        const OPEN_STEP: u16 = 7;
        const CLOSE_STEP: u16 = 14;
        const PEEK_STEP: u16 = 10;
        const PEEK_MAX_WIDTH: u16 = 12;

        match self.anim {
            AnimState::Opening { ref mut current_width, ref mut frame } => {
                let step = if self.peek_held { PEEK_STEP } else { OPEN_STEP };
                let target = if self.peek_held {
                    PEEK_MAX_WIDTH.min(self.target_width)
                } else {
                    self.target_width
                };
                *current_width = (*current_width + step).min(target);
                *frame += 1;
                if *current_width >= target {
                    self.anim = if self.peek_held {
                        AnimState::Peeking { current_width: *current_width }
                    } else {
                        AnimState::Open
                    };
                }
            }
            AnimState::Closing { ref mut current_width, ref mut frame } => {
                *current_width = current_width.saturating_sub(CLOSE_STEP);
                *frame += 1;
                if *current_width == 0 {
                    self.anim = AnimState::Closed;
                }
            }
            AnimState::Peeking { ref mut current_width } => {
                if !self.peek_held {
                    self.anim = AnimState::Closing { current_width: *current_width, frame: 0 };
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_state_initialized_closed() {
        let s = SidebarState::new(28);
        assert_eq!(s.anim, AnimState::Closed);
        assert_eq!(s.current_width(), 0);
        assert!(!s.is_open());
        assert!(!s.is_animating());
    }

    #[test]
    fn toggle_opens_closed_sidebar() {
        let mut s = SidebarState::new(28);
        s.toggle();
        assert!(matches!(s.anim, AnimState::Opening { .. }));
        assert!(s.is_animating());
    }

    #[test]
    fn toggle_closes_open_sidebar() {
        let mut s = SidebarState::new(28);
        s.anim = AnimState::Open;
        s.toggle();
        assert!(matches!(s.anim, AnimState::Closing { .. }));
    }

    #[test]
    fn full_open_animation() {
        let mut s = SidebarState::new(28);
        s.toggle(); // Opening { cw: 1, frame: 0 }
        // Tick 1: 1+7=8
        s.tick();
        assert_eq!(s.current_width(), 8);
        // Tick 2: 8+7=15
        s.tick();
        assert_eq!(s.current_width(), 15);
        // Tick 3: 15+7=22
        s.tick();
        assert_eq!(s.current_width(), 22);
        // Tick 4: 22+7=29 cap at 28
        s.tick();
        assert_eq!(s.current_width(), 28);
        assert_eq!(s.anim, AnimState::Open);
    }

    #[test]
    fn full_close_animation() {
        let mut s = SidebarState::new(28);
        s.anim = AnimState::Open;
        s.toggle(); // Closing { cw: 28, frame: 0 }
        // Tick 1: 28-14=14
        s.tick();
        assert_eq!(s.current_width(), 14);
        // Tick 2: 14-14=0
        s.tick();
        assert_eq!(s.current_width(), 0);
        assert_eq!(s.anim, AnimState::Closed);
    }

    #[test]
    fn peek_open_and_release() {
        let mut s = SidebarState::new(28);
        s.start_peek();
        assert!(s.peek_held);
        assert!(matches!(s.anim, AnimState::Opening { .. }));
        // Tick x2 to reach PEEK_MAX_WIDTH (12)
        s.tick();
        s.tick();
        // Should be at PEEK_MAX_WIDTH or close (step=10, so 1+10=11, 11+10=21 cap at 12)
        // Actually: Opening(1) → tick(11) → Opening(11) → tick(cap at 12) → Peeking(12)
        assert!(matches!(s.anim, AnimState::Peeking { .. }));
        assert_eq!(s.current_width(), 12);
        s.end_peek();
        assert!(matches!(s.anim, AnimState::Closing { .. }));
        s.tick();
        assert_eq!(s.current_width(), 0);
        assert_eq!(s.anim, AnimState::Closed);
    }

    #[test]
    fn cursor_wrap_favorites_to_drives() {
        let mut s = SidebarState::new(28);
        s.favorites = vec!["/a".into(), "/b".into(), "/c".into()];
        s.drives = vec!["/d".into(), "/e".into()];
        // Start at cursor 0
        s.cursor = 2; // last favorite
        assert_eq!(s.section, SidebarSection::Favorites);
        s.cursor_down(3, 2);
        assert_eq!(s.cursor, 3);
        assert_eq!(s.section, SidebarSection::Drives);
        assert_eq!(s.selected_path(3), Some("/d"));
    }

    #[test]
    fn cursor_wrap_drives_to_favorites() {
        let mut s = SidebarState::new(28);
        s.favorites = vec!["/a".into(), "/b".into(), "/c".into()];
        s.drives = vec!["/d".into(), "/e".into()];
        s.cursor = 4; // last drive
        s.cursor_down(3, 2);
        assert_eq!(s.cursor, 0);
        assert_eq!(s.section, SidebarSection::Favorites);
        assert_eq!(s.selected_path(3), Some("/a"));
    }

    #[test]
    fn cursor_up_wrap_from_top() {
        let mut s = SidebarState::new(28);
        s.favorites = vec!["/a".into(), "/b".into()];
        s.drives = vec!["/d".into()];
        s.cursor = 0;
        s.cursor_up(2, 1);
        assert_eq!(s.cursor, 2); // wraps to last item (drives.last)
        assert_eq!(s.section, SidebarSection::Drives);
    }

    #[test]
    fn empty_favorites_initial_cursor_is_valid() {
        let mut s = SidebarState::new(28);
        s.drives = vec!["/d".into(), "/e".into()];
        s.cursor_down(0, 2);
        assert_eq!(s.cursor, 1);
        assert_eq!(s.section, SidebarSection::Drives);
    }

    #[test]
    fn no_items_cursor_does_nothing() {
        let mut s = SidebarState::new(28);
        s.cursor_down(0, 0);
        assert_eq!(s.cursor, 0);
        s.cursor_up(0, 0);
        assert_eq!(s.cursor, 0);
    }
}

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use crate::config::theme::Theme;

/// Render the sidebar into the given area.
pub fn render_sidebar(frame: &mut Frame, area: Rect, state: &SidebarState, theme: &Theme) {
    let width = state.current_width();

    // If fully closed and nothing animating: draw nothing
    if width == 0 && !state.is_animating() {
        return;
    }

    // If width is very small (< 3), render just the handle bar
    if width < 3 {
        render_handle(frame, area, theme);
        return;
    }

    let bg = theme.colors.background.to_ratatui();
    let fg = theme.colors.foreground.to_ratatui();
    let border_color = if state.sidebar_focused {
        theme.colors.border_active.to_ratatui()
    } else {
        theme.colors.border_inactive.to_ratatui()
    };

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),       // Favorites section
            Constraint::Length(5),    // Drives section (fixed small height)
        ])
        .split(area);

    let fav_len = state.favorites.len();
    render_sidebar_section(
        frame,
        inner[0],
        " Favorites ",
        &state.favorites,
        &state.drives,
        state.cursor,
        fav_len,
        0,
        theme,
        border_color,
        bg,
        fg,
        width,
    );

    let drive_len = state.drives.len();
    render_sidebar_section(
        frame,
        inner[1],
        " Drives ",
        &state.drives,
        &[],
        if state.cursor >= fav_len { state.cursor - fav_len } else { 0 },
        drive_len,
        fav_len,
        theme,
        border_color,
        bg,
        fg,
        width,
    );
}

fn render_sidebar_section(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: &[String],
    _secondary: &[String],
    cursor_in_section: usize,
    item_count: usize,
    _global_offset: usize,
    theme: &Theme,
    border_color: ratatui::style::Color,
    bg: ratatui::style::Color,
    fg: ratatui::style::Color,
    width: u16,
) {
    let sel_bg = theme.colors.selection_bg.to_ratatui();
    let sel_fg = theme.colors.selection_fg.to_ratatui();
    let dir_color = theme.colors.directory.to_ratatui();

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(dir_color))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .bg(bg);

    let mut rows = Vec::with_capacity(item_count);
    for i in 0..item_count {
        let is_cursor = i == cursor_in_section;
        let text = abbreviate_path(&items[i], width.saturating_sub(4) as usize);
        let style = if is_cursor {
            Style::default().fg(sel_fg).bg(sel_bg)
        } else {
            Style::default().fg(fg).bg(bg)
        };
        rows.push(Line::styled(format!(" {} ", text), style));
    }

    if rows.is_empty() {
        rows.push(Line::styled(
            " (empty) ",
            Style::default().fg(theme.colors.hidden.to_ratatui()).bg(bg),
        ));
    }

    let content = Paragraph::new(rows).block(block).bg(bg);
    frame.render_widget(content, area);
}

fn render_handle(frame: &mut Frame, area: Rect, theme: &Theme) {
    let handle_color = theme.colors.directory.to_ratatui();
    let bg = theme.colors.background.to_ratatui();

    let block = Block::default()
        .borders(Borders::NONE)
        .bg(handle_color);

    let label = Paragraph::new(Line::from(" ▸ "))
        .style(Style::default().fg(bg).bg(handle_color))
        .block(block);
    frame.render_widget(label, area);
}

fn abbreviate_path(path: &str, max_width: usize) -> String {
    if max_width < 2 {
        return String::new();
    }
    let display = if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy().to_string();
        path.replace(&home_str, "~")
    } else {
        path.to_string()
    };

    if display.len() > max_width {
        let mut truncated: String = display.chars().take(max_width.saturating_sub(1)).collect();
        truncated.push('…');
        truncated
    } else {
        display
    }
}
