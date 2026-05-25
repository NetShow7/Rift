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
}

pub struct SidebarState {
    pub anim: AnimState,
    pub target_width: u16,
    pub cursor: usize,
    pub section: SidebarSection,
    pub favorites: Vec<String>,
    pub drives: Vec<String>,
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
            sidebar_focused: false,
        }
    }

    pub fn toggle(&mut self) {
        match self.anim {
            AnimState::Closed => {
                let cw = self.current_width();
                self.anim = AnimState::Opening { current_width: cw.max(1), frame: 0 };
            }
            _ => {
                let cw = self.current_width();
                self.anim = AnimState::Closing { current_width: cw, frame: 0 };
            }
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

        match self.anim {
            AnimState::Opening { ref mut current_width, ref mut frame } => {
                *current_width = (*current_width + OPEN_STEP).min(self.target_width);
                *frame += 1;
                if *current_width >= self.target_width {
                    self.anim = AnimState::Open;
                }
            }
            AnimState::Closing { ref mut current_width, ref mut frame } => {
                *current_width = current_width.saturating_sub(CLOSE_STEP);
                *frame += 1;
                if *current_width == 0 {
                    self.anim = AnimState::Closed;
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
pub fn render_sidebar(frame: &mut Frame, area: Rect, state: &SidebarState, theme: &Theme, drop_left: bool) {
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
        drop_left,
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
        drop_left,
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
    drop_left: bool,
) {
    let sel_bg = theme.colors.selection_bg.to_ratatui();
    let sel_fg = theme.colors.selection_fg.to_ratatui();
    let dir_color = theme.colors.directory.to_ratatui();

    let border_type = match theme.border_style {
        crate::config::theme::BorderStyle::Rounded => BorderType::Rounded,
        crate::config::theme::BorderStyle::Double  => BorderType::Double,
        crate::config::theme::BorderStyle::Thick   => BorderType::Thick,
        crate::config::theme::BorderStyle::Plain   => BorderType::Plain,
        crate::config::theme::BorderStyle::None    => BorderType::Plain,
    };
    let borders = if drop_left { Borders::TOP | Borders::RIGHT | Borders::BOTTOM } else { Borders::ALL };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(dir_color))
        .borders(borders)
        .border_type(border_type)
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

/// Draw a collapsed sidebar indicator: a thin vertical line at the sidebar edge
/// with a vertical keybinding hint. Only drawn when sidebar is fully closed.
pub fn render_collapsed_indicator(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    sidebar_on_left: bool,
    keybinding: Option<&str>,
) {
    let border_color = theme.colors.border_inactive.to_ratatui();
    let bg = theme.colors.background.to_ratatui();

    let block = Block::default()
        .borders(if sidebar_on_left { Borders::RIGHT } else { Borders::LEFT })
        .border_style(Style::default().fg(border_color))
        .bg(bg);
    frame.render_widget(block, area);

    // Show vertical keybinding hint in the column next to the border
    if area.width > 1 {
        let hint_x = if sidebar_on_left { area.x } else { area.x + 1 };
        let hint_area = Rect { x: hint_x, y: area.y, width: area.width - 1, height: area.height };
        render_vertical_text(frame, hint_area, keybinding.unwrap_or("▸"), border_color, bg);
    }
}

/// Render text vertically, one character per row, centered vertically in the area.
fn render_vertical_text(frame: &mut Frame, area: Rect, text: &str, fg: ratatui::style::Color, bg: ratatui::style::Color) {
    let style = Style::default().fg(fg).bg(bg);
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len() as u16;
    let start_y = if len >= area.height {
        area.y
    } else {
        area.y + ((area.height - len) / 2)
    };

    for (i, &ch) in chars.iter().enumerate() {
        let y = start_y + i as u16;
        if y >= area.y + area.height {
            break;
        }
        let cell = Rect { x: area.x, y, width: 1, height: 1 };
        frame.render_widget(
            Paragraph::new(Line::from(ch.to_string())).style(style),
            cell,
        );
    }
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
