use crate::config::{Action, Config, KeyBinding, Keymap, LayoutMode, ShellMode, theme::{BorderStyle, Color as RiftColor}};
use crate::fs::{Conflict, ConflictResolution};
use std::sync::mpsc::Sender;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
    Frame,
};

#[derive(Debug, Clone)]
pub enum InputIntent {
    Rename,
    NewFile,
    NewDir,
}

/// All modal dialogs the app can show.
#[derive(Debug, Clone)]
pub enum Modal {
    /// Conflict during copy/move: show options to user.
    Conflict {
        conflict: ConflictDialogState,
        response: Option<Sender<ConflictResolution>>,
    },
    /// Simple yes/no confirmation (e.g. delete).
    Confirm {
        title: String,
        message: String,
        selected: ConfirmChoice,
    },
    /// Single-line text input (rename, new file/dir).
    Input {
        title: String,
        prompt: String,
        value: String,
        cursor: usize,
        intent: InputIntent,
    },
    /// Operation summary after batch op completes.
    Summary {
        title: String,
        lines: Vec<String>,
        scroll: usize,
    },
    /// Help overlay.
    Help { scroll: usize, keymap: Keymap },

    /// Settings panel.
    Settings(SettingsState),
}

#[derive(Debug, Clone)]
pub struct ConflictDialogState {
    pub conflict: Conflict,
    pub selected: ConflictChoice,
    /// If user picks Rename, they type here.
    pub rename_input: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictChoice {
    Skip,
    Overwrite,
    Rename,
    Abort,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmChoice {
    Yes,
    No,
}

impl Modal {
    pub fn conflict(conflict: Conflict, response: Sender<ConflictResolution>) -> Self {
        Self::Conflict {
            conflict: ConflictDialogState {
                conflict,
                selected: ConflictChoice::Skip,
                rename_input: None,
            },
            response: Some(response),
        }
    }

    pub fn confirm(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Confirm {
            title: title.into(),
            message: message.into(),
            selected: ConfirmChoice::No,
        }
    }

    pub fn input(title: impl Into<String>, prompt: impl Into<String>, prefill: &str, intent: InputIntent) -> Self {
        let value = prefill.to_string();
        let cursor = value.len();
        Self::Input {
            title: title.into(),
            prompt: prompt.into(),
            value,
            cursor,
            intent,
        }
    }
}

/// Centred overlay rect.
pub fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(area.width * percent_x / 100)) / 2;
    let w = area.width * percent_x / 100;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect { x, y, width: w, height: height.min(area.height) }
}

pub fn draw_modal(frame: &mut Frame, modal: &Modal, area: Rect) {
    match modal {
        Modal::Conflict { conflict, .. } => draw_conflict(frame, conflict, area),
        Modal::Confirm { title, message, selected } => {
            draw_confirm(frame, title, message, selected, area)
        }
        Modal::Input { title, prompt, value, cursor, .. } => {
            draw_input(frame, title, prompt, value, *cursor, area)
        }
        Modal::Summary { title, lines, scroll } => {
            draw_summary(frame, title, lines, *scroll, area)
        }
        Modal::Help { scroll, keymap } => draw_help(frame, *scroll, keymap, area),
        Modal::Settings(state) => draw_settings(frame, state, area),
    }
}

fn draw_conflict(frame: &mut Frame, state: &ConflictDialogState, area: Rect) {
    let rect = centered_rect(60, 12, area);
    frame.render_widget(Clear, rect);

    let src_name = state.conflict.src.file_name()
        .unwrap_or_default().to_string_lossy();
    let dst_name = state.conflict.dst.display().to_string();

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Conflict ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(187, 154, 247)));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let choices = [
        (ConflictChoice::Skip,      "[S]kip"),
        (ConflictChoice::Overwrite, "[O]verwrite"),
        (ConflictChoice::Rename,    "[R]ename"),
        (ConflictChoice::Abort,     "[A]bort"),
    ];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(format!("\"{}\" already exists at:", src_name))
            .alignment(Alignment::Center),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(dst_name)
            .style(Style::default().fg(Color::Rgb(224, 175, 104)))
            .alignment(Alignment::Center),
        chunks[1],
    );

    let button_line: Line = choices
        .iter()
        .map(|(choice, label)| {
            let style = if *choice == state.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Rgb(187, 154, 247))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(169, 177, 214))
            };
            Span::styled(format!(" {} ", label), style)
        })
        .collect::<Vec<_>>()
        .into();

    frame.render_widget(
        Paragraph::new(button_line).alignment(Alignment::Center),
        chunks[3],
    );

    if let Some(ref rename) = state.rename_input {
        frame.render_widget(
            Paragraph::new(format!("New name: {}_", rename))
                .style(Style::default().fg(Color::Rgb(158, 206, 106))),
            chunks[4],
        );
    }
}

fn draw_confirm(
    frame: &mut Frame,
    title: &str,
    message: &str,
    selected: &ConfirmChoice,
    area: Rect,
) {
    let rect = centered_rect(50, 8, area);
    frame.render_widget(Clear, rect);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", title))
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(247, 118, 142)));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Length(2)])
        .split(inner);

    frame.render_widget(
        Paragraph::new(message).alignment(Alignment::Center),
        chunks[0],
    );

    let yes_style = if *selected == ConfirmChoice::Yes {
        Style::default().fg(Color::Black).bg(Color::Rgb(247, 118, 142)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(169, 177, 214))
    };
    let no_style = if *selected == ConfirmChoice::No {
        Style::default().fg(Color::Black).bg(Color::Rgb(169, 177, 214)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(169, 177, 214))
    };

    let line = Line::from(vec![
        Span::styled("  [Y]es  ", yes_style),
        Span::raw("  "),
        Span::styled("  [N]o  ", no_style),
    ]);
    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), chunks[1]);
}

fn draw_input(
    frame: &mut Frame,
    title: &str,
    prompt: &str,
    value: &str,
    _cursor: usize,
    area: Rect,
) {
    let rect = centered_rect(55, 7, area);
    frame.render_widget(Clear, rect);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", title))
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(122, 162, 247)));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    frame.render_widget(Paragraph::new(prompt), chunks[0]);
    frame.render_widget(
        Paragraph::new(format!("{}_", value))
            .style(Style::default().fg(Color::Rgb(192, 202, 245))),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new("Enter to confirm • Esc to cancel")
            .style(Style::default().fg(Color::Rgb(86, 95, 137)))
            .alignment(Alignment::Center),
        chunks[2],
    );
}

fn draw_summary(frame: &mut Frame, title: &str, lines: &[String], scroll: usize, area: Rect) {
    let rect = centered_rect(65, 20, area);
    frame.render_widget(Clear, rect);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", title))
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(158, 206, 106)));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let visible: Vec<Line> = lines
        .iter()
        .skip(scroll)
        .take(inner.height as usize)
        .map(|l| Line::raw(l.clone()))
        .collect();

    frame.render_widget(Paragraph::new(visible), inner);
}

fn draw_help(frame: &mut Frame, scroll: usize, keymap: &Keymap, area: Rect) {
    let rect = centered_rect(70, 30, area);
    frame.render_widget(Clear, rect);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Help — ? to close ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(122, 162, 247)));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let mut lines: Vec<Line> = Vec::new();
    for (section, bindings) in help_sections() {
        lines.push(Line::from(Span::styled(
            section,
            Style::default()
                .fg(Color::Rgb(187, 154, 247))
                .add_modifier(Modifier::BOLD),
        )));
        for (action, desc) in bindings {
            let keys: Vec<&str> = keymap.0.iter()
                .filter(|(_, v)| matches!(v, KeyBinding::Action(a) if *a == action))
                .map(|(k, _)| k.as_str())
                .collect();
            let key_str = if keys.is_empty() { "—".into() } else { keys.join(", ") };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:20}", key_str), Style::default().fg(Color::Rgb(224, 175, 104))),
                Span::raw(desc),
            ]));
        }
        lines.push(Line::raw(""));
    }

    let visible: Vec<Line> = lines.into_iter().skip(scroll).take(inner.height as usize).collect();
    frame.render_widget(Paragraph::new(visible), inner);
}

fn help_sections() -> Vec<(&'static str, Vec<(Action, &'static str)>)> {
    use Action::*;
    vec![
        ("Navigation", vec![
            (MoveUp, "Move up"),
            (MoveDown, "Move down"),
            (MoveLeft, "Move left"),
            (MoveRight, "Move right"),
            (PageUp, "Page up"),
            (PageDown, "Page down"),
            (GotoTop, "Go to top"),
            (GotoBottom, "Go to bottom"),
            (OpenEntry, "Open entry"),
            (GoParent, "Go to parent"),
        ]),
        ("Selection", vec![
            (SelectToggle, "Toggle selection"),
            (SelectAll, "Select all"),
            (SelectNone, "Clear selection"),
            (InvertSelection, "Invert selection"),
        ]),
        ("File Operations", vec![
            (Copy, "Copy"),
            (Cut, "Cut"),
            (Paste, "Paste"),
            (Delete, "Delete"),
            (Rename, "Rename"),
            (NewFile, "New file"),
            (NewDir, "New directory"),
        ]),
        ("View", vec![
            (ToggleHidden, "Toggle hidden"),
            (TogglePreview, "Toggle preview"),
            (CycleLayout, "Cycle layout"),
            (SetLayoutSingle, "Layout: single"),
            (SetLayoutDual, "Layout: dual"),
            (SetLayoutMiller, "Layout: miller"),
            (Refresh, "Refresh"),
        ]),
        ("Search / Filter", vec![
            (Search, "Search"),
            (Filter, "Filter"),
            (ClearFilter, "Clear filter"),
        ]),
        ("App", vec![
            (Help, "Help"),
            (OpenSettings, "Settings"),
            (OpenConfig, "Open config"),
            (OpenShell, "Open shell"),
            (Quit, "Quit"),
        ]),
    ]
}

// --- Settings panel -----------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Theme,
    Keymap,
}

#[derive(Debug, Clone)]
pub enum SettingsEdit {
    Text { value: String, cursor: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingId {
    Layout,
    ShowHidden,
    FollowSymlinks,
    ScrollThreshold,
    Shell,
    ShellMode,
    ConfirmDelete,
    TrashDir,
    ShowShortcutHints,
    BorderStyle,
    ColorBackground,
    ColorForeground,
    ColorSelectionBg,
    ColorSelectionFg,
    ColorDirectory,
    ColorSymlink,
    ColorExecutable,
    ColorArchive,
    ColorMedia,
    ColorHidden,
    ColorBorderActive,
    ColorBorderInactive,
    ColorStatusbarBg,
    ColorStatusbarFg,
    ColorError,
    ColorWarning,
    ColorSuccess,
    ColorModalBg,
    ColorModalBorder,
    SymbolDirOpen,
    SymbolDirClosed,
    SymbolSymlink,
    SymbolFile,
    SymbolSelected,
    SymbolUnselected,
    SymbolCopyMarker,
    SymbolCutMarker,
    SymbolErrorMarker,
    KeyAction(Action),
}

#[derive(Debug, Clone)]
pub enum TabItem {
    Setting(SettingId),
    Section(&'static str),
}

#[derive(Debug, Clone)]
pub struct SettingsState {
    pub tab: SettingsTab,
    pub cursor: usize,
    pub scroll: usize,
    pub editing: Option<SettingsEdit>,
    pub capturing: Option<Action>,
    pub config: Config,
    pub changed: bool,
}

impl SettingsState {
    pub fn new(config: Config) -> Self {
        Self {
            tab: SettingsTab::General,
            cursor: 0,
            scroll: 0,
            editing: None,
            capturing: None,
            config,
            changed: false,
        }
    }

    pub fn tab_items(&self) -> Vec<TabItem> {
        use SettingId::*;
        use Action::*;
        match self.tab {
            SettingsTab::General => vec![
                TabItem::Setting(Layout),
                TabItem::Setting(ShowHidden),
                TabItem::Setting(FollowSymlinks),
                TabItem::Setting(ScrollThreshold),
                TabItem::Setting(Shell),
                TabItem::Setting(ShellMode),
                TabItem::Setting(ConfirmDelete),
                TabItem::Setting(TrashDir),
                TabItem::Setting(ShowShortcutHints),
            ],
            SettingsTab::Theme => vec![
                TabItem::Setting(BorderStyle),
                TabItem::Section("Colors"),
                TabItem::Setting(ColorForeground),
                TabItem::Setting(ColorBackground),
                TabItem::Setting(ColorSelectionBg),
                TabItem::Setting(ColorSelectionFg),
                TabItem::Setting(ColorDirectory),
                TabItem::Setting(ColorSymlink),
                TabItem::Setting(ColorExecutable),
                TabItem::Setting(ColorArchive),
                TabItem::Setting(ColorMedia),
                TabItem::Setting(ColorHidden),
                TabItem::Setting(ColorBorderActive),
                TabItem::Setting(ColorBorderInactive),
                TabItem::Setting(ColorStatusbarBg),
                TabItem::Setting(ColorStatusbarFg),
                TabItem::Setting(ColorError),
                TabItem::Setting(ColorWarning),
                TabItem::Setting(ColorSuccess),
                TabItem::Setting(ColorModalBg),
                TabItem::Setting(ColorModalBorder),
                TabItem::Section("Symbols"),
                TabItem::Setting(SymbolDirOpen),
                TabItem::Setting(SymbolDirClosed),
                TabItem::Setting(SymbolSymlink),
                TabItem::Setting(SymbolFile),
                TabItem::Setting(SymbolSelected),
                TabItem::Setting(SymbolUnselected),
                TabItem::Setting(SymbolCopyMarker),
                TabItem::Setting(SymbolCutMarker),
                TabItem::Setting(SymbolErrorMarker),
            ],
            SettingsTab::Keymap => {
                vec![
                    TabItem::Section("Navigation"),
                    TabItem::Setting(KeyAction(MoveUp)),
                    TabItem::Setting(KeyAction(MoveDown)),
                    TabItem::Setting(KeyAction(MoveLeft)),
                    TabItem::Setting(KeyAction(MoveRight)),
                    TabItem::Setting(KeyAction(PageUp)),
                    TabItem::Setting(KeyAction(PageDown)),
                    TabItem::Setting(KeyAction(GotoTop)),
                    TabItem::Setting(KeyAction(GotoBottom)),
                    TabItem::Setting(KeyAction(OpenEntry)),
                    TabItem::Setting(KeyAction(GoParent)),
                    TabItem::Section("Selection"),
                    TabItem::Setting(KeyAction(SelectToggle)),
                    TabItem::Setting(KeyAction(SelectAll)),
                    TabItem::Setting(KeyAction(SelectNone)),
                    TabItem::Setting(KeyAction(InvertSelection)),
                    TabItem::Section("File Operations"),
                    TabItem::Setting(KeyAction(Copy)),
                    TabItem::Setting(KeyAction(Cut)),
                    TabItem::Setting(KeyAction(Paste)),
                    TabItem::Setting(KeyAction(Delete)),
                    TabItem::Setting(KeyAction(Rename)),
                    TabItem::Setting(KeyAction(NewFile)),
                    TabItem::Setting(KeyAction(NewDir)),
                    TabItem::Section("View"),
                    TabItem::Setting(KeyAction(ToggleHidden)),
                    TabItem::Setting(KeyAction(TogglePreview)),
                    TabItem::Setting(KeyAction(CycleLayout)),
                    TabItem::Setting(KeyAction(SetLayoutSingle)),
                    TabItem::Setting(KeyAction(SetLayoutDual)),
                    TabItem::Setting(KeyAction(SetLayoutMiller)),
                    TabItem::Setting(KeyAction(Refresh)),
                    TabItem::Section("Search / Filter"),
                    TabItem::Setting(KeyAction(Search)),
                    TabItem::Setting(KeyAction(Filter)),
                    TabItem::Setting(KeyAction(ClearFilter)),
                    TabItem::Section("App"),
                    TabItem::Setting(KeyAction(OpenConfig)),
                    TabItem::Setting(KeyAction(OpenShell)),
                    TabItem::Setting(KeyAction(Quit)),
                    TabItem::Setting(KeyAction(Help)),
                    TabItem::Setting(KeyAction(OpenSettings)),
                ]
            }
        }
    }

    pub fn max_cursor(&self) -> usize {
        let items = self.tab_items();
        items.len().saturating_sub(1)
    }

    pub fn value_for(&self, id: &SettingId) -> String {
        use SettingId::*;
        match id {
            Layout => format!("{:?}", self.config.general.layout).to_lowercase(),
            ShowHidden => yesno(self.config.general.show_hidden),
            FollowSymlinks => yesno(self.config.general.follow_symlinks),
            ScrollThreshold => self.config.general.scroll_threshold.to_string(),
            Shell => self.config.general.shell.clone(),
            ShellMode => format!("{:?}", self.config.general.shell_mode).to_lowercase(),
            ConfirmDelete => yesno(self.config.general.confirm_delete),
            TrashDir => self.config.general.trash_dir
                .as_ref().map(|p| p.display().to_string())
                .unwrap_or_else(|| "none".into()),
            ShowShortcutHints => yesno(self.config.general.show_shortcut_hints),
            BorderStyle => format!("{:?}", self.config.theme.border_style).to_lowercase(),
            ColorForeground => color_val(&self.config.theme.colors.foreground),
            ColorBackground => color_val(&self.config.theme.colors.background),
            ColorSelectionBg => color_val(&self.config.theme.colors.selection_bg),
            ColorSelectionFg => color_val(&self.config.theme.colors.selection_fg),
            ColorDirectory => color_val(&self.config.theme.colors.directory),
            ColorSymlink => color_val(&self.config.theme.colors.symlink),
            ColorExecutable => color_val(&self.config.theme.colors.executable),
            ColorArchive => color_val(&self.config.theme.colors.archive),
            ColorMedia => color_val(&self.config.theme.colors.media),
            ColorHidden => color_val(&self.config.theme.colors.hidden),
            ColorBorderActive => color_val(&self.config.theme.colors.border_active),
            ColorBorderInactive => color_val(&self.config.theme.colors.border_inactive),
            ColorStatusbarBg => color_val(&self.config.theme.colors.statusbar_bg),
            ColorStatusbarFg => color_val(&self.config.theme.colors.statusbar_fg),
            ColorError => color_val(&self.config.theme.colors.error),
            ColorWarning => color_val(&self.config.theme.colors.warning),
            ColorSuccess => color_val(&self.config.theme.colors.success),
            ColorModalBg => color_val(&self.config.theme.colors.modal_bg),
            ColorModalBorder => color_val(&self.config.theme.colors.modal_border),
            SymbolDirOpen => self.config.theme.symbols.dir_open.clone(),
            SymbolDirClosed => self.config.theme.symbols.dir_closed.clone(),
            SymbolSymlink => self.config.theme.symbols.symlink.clone(),
            SymbolFile => self.config.theme.symbols.file.clone(),
            SymbolSelected => self.config.theme.symbols.selected.clone(),
            SymbolUnselected => self.config.theme.symbols.unselected.clone(),
            SymbolCopyMarker => self.config.theme.symbols.copy_marker.clone(),
            SymbolCutMarker => self.config.theme.symbols.cut_marker.clone(),
            SymbolErrorMarker => self.config.theme.symbols.error_marker.clone(),
            KeyAction(a) => {
                let keys: Vec<String> = self.config.keymap.0.iter()
                    .filter(|(_, v)| matches!(v, KeyBinding::Action(ka) if ka == a))
                    .map(|(k, _)| k.clone())
                    .collect();
                if keys.is_empty() { "—".into() } else { keys.join(", ") }
            }
        }
    }

    pub fn activate(&mut self) {
        let items = self.tab_items();
        let idx = self.cursor.min(items.len().saturating_sub(1));
        let Some(TabItem::Setting(id)) = items.get(idx) else { return };
        self.activate_id(id);
    }

    fn activate_id(&mut self, id: &SettingId) {
        use SettingId::*;
        match id {
            ShowHidden | FollowSymlinks | ConfirmDelete | ShowShortcutHints => {
                self.toggle_bool(id);
            }
            Layout | ShellMode | BorderStyle => {
                self.cycle_enum(id);
            }
            KeyAction(_) => {
                self.capturing = Some(match id {
                    KeyAction(a) => a.clone(),
                    _ => unreachable!(),
                });
            }
            _ => {
                let val = self.value_for(id);
                self.editing = Some(SettingsEdit::Text { value: val.clone(), cursor: val.len() });
            }
        }
    }

    fn toggle_bool(&mut self, id: &SettingId) {
        use SettingId::*;
        self.changed = true;
        match id {
            ShowHidden => self.config.general.show_hidden = !self.config.general.show_hidden,
            FollowSymlinks => self.config.general.follow_symlinks = !self.config.general.follow_symlinks,
            ConfirmDelete => self.config.general.confirm_delete = !self.config.general.confirm_delete,
            ShowShortcutHints => self.config.general.show_shortcut_hints = !self.config.general.show_shortcut_hints,
            _ => {}
        }
    }

    fn cycle_enum(&mut self, id: &SettingId) {
        self.changed = true;
        match id {
            SettingId::Layout => {
                self.config.general.layout = match self.config.general.layout {
                    LayoutMode::Single => LayoutMode::Dual,
                    LayoutMode::Dual => LayoutMode::Miller,
                    LayoutMode::Miller => LayoutMode::Single,
                };
            }
            SettingId::ShellMode => {
                self.config.general.shell_mode = match self.config.general.shell_mode {
                    ShellMode::Capture => ShellMode::Takeover,
                    ShellMode::Takeover => ShellMode::Capture,
                };
            }
            SettingId::BorderStyle => {
                self.config.theme.border_style = match self.config.theme.border_style {
                    BorderStyle::Plain => BorderStyle::Rounded,
                    BorderStyle::Rounded => BorderStyle::Double,
                    BorderStyle::Double => BorderStyle::Thick,
                    BorderStyle::Thick => BorderStyle::None,
                    BorderStyle::None => BorderStyle::Plain,
                };
            }
            _ => {}
        }
    }

    pub fn apply_edit(&mut self, value: &str) {
        self.changed = true;
        let items = self.tab_items();
        let idx = self.cursor.min(items.len().saturating_sub(1));
        let Some(TabItem::Setting(id)) = items.get(idx) else { return };
        self.apply_value(id, value);
    }

    fn apply_value(&mut self, id: &SettingId, value: &str) {
        use SettingId::*;
        match id {
            Shell => self.config.general.shell = value.to_string(),
            ScrollThreshold => {
                if let Ok(n) = value.parse::<usize>() {
                    self.config.general.scroll_threshold = n;
                }
            }
            TrashDir => {
                if value.is_empty() || value == "none" {
                    self.config.general.trash_dir = None;
                } else {
                    self.config.general.trash_dir = Some(value.into());
                }
            }
            _ => {
                if let Some(c) = self.id_to_color(id) {
                    *c = parse_color(value);
                } else if let Some(s) = self.id_to_symbol(id) {
                    *s = value.to_string();
                }
            }
        }
    }

    fn id_to_color(&mut self, id: &SettingId) -> Option<&mut RiftColor> {
        use SettingId::*;
        let c = &mut self.config.theme.colors;
        Some(match id {
            ColorForeground => &mut c.foreground,
            ColorBackground => &mut c.background,
            ColorSelectionBg => &mut c.selection_bg,
            ColorSelectionFg => &mut c.selection_fg,
            ColorDirectory => &mut c.directory,
            ColorSymlink => &mut c.symlink,
            ColorExecutable => &mut c.executable,
            ColorArchive => &mut c.archive,
            ColorMedia => &mut c.media,
            ColorHidden => &mut c.hidden,
            ColorBorderActive => &mut c.border_active,
            ColorBorderInactive => &mut c.border_inactive,
            ColorStatusbarBg => &mut c.statusbar_bg,
            ColorStatusbarFg => &mut c.statusbar_fg,
            ColorError => &mut c.error,
            ColorWarning => &mut c.warning,
            ColorSuccess => &mut c.success,
            ColorModalBg => &mut c.modal_bg,
            ColorModalBorder => &mut c.modal_border,
            _ => return None,
        })
    }

    fn id_to_symbol(&mut self, id: &SettingId) -> Option<&mut String> {
        use SettingId::*;
        let s = &mut self.config.theme.symbols;
        Some(match id {
            SymbolDirOpen => &mut s.dir_open,
            SymbolDirClosed => &mut s.dir_closed,
            SymbolSymlink => &mut s.symlink,
            SymbolFile => &mut s.file,
            SymbolSelected => &mut s.selected,
            SymbolUnselected => &mut s.unselected,
            SymbolCopyMarker => &mut s.copy_marker,
            SymbolCutMarker => &mut s.cut_marker,
            SymbolErrorMarker => &mut s.error_marker,
            _ => return None,
        })
    }

    pub fn apply_capture(&mut self, key_str: &str) {
        let Some(ref action) = self.capturing else { return };
        self.config.keymap.0.retain(|_, v| match v {
            KeyBinding::Action(a) => a != action,
            _ => true,
        });
        self.config.keymap.0.remove(key_str);
        self.config.keymap.0.insert(key_str.to_string(), KeyBinding::Action(action.clone()));
        self.changed = true;
        self.capturing = None;
    }
}

fn yesno(v: bool) -> String {
    if v { "yes".into() } else { "no".into() }
}

fn color_val(c: &RiftColor) -> String {
    match c {
        RiftColor::Hex(s) | RiftColor::Named(s) => s.clone(),
    }
}

pub fn parse_color(s: &str) -> RiftColor {
    let s = s.trim();
    if s.starts_with('#') {
        RiftColor::Hex(s.to_string())
    } else if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        RiftColor::Hex(format!("#{}", s))
    } else {
        RiftColor::Named(s.to_lowercase())
    }
}

pub fn draw_settings(frame: &mut Frame, state: &SettingsState, area: Rect) {
    let rect = centered_rect(72, 80, area);
    frame.render_widget(Clear, rect);

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(" Settings — Esc to close ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(187, 154, 247)));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    draw_tab_bar(frame, chunks[0], state);
    draw_settings_content(frame, chunks[1], state);
    draw_settings_footer(frame, chunks[2], state);
}

fn draw_tab_bar(frame: &mut Frame, area: Rect, state: &SettingsState) {
    let tabs = [
        (SettingsTab::General, "General"),
        (SettingsTab::Theme, "Theme"),
        (SettingsTab::Keymap, "Keymap"),
    ];
    let n = tabs.len() as u16;
    let total_w = area.width;
    let tab_w = total_w / n;

    for (i, (tab_id, label)) in tabs.iter().enumerate() {
        let x = area.x + i as u16 * tab_w;
        let r = Rect { x, y: area.y + 1, width: tab_w, height: 1 };
        let is_active = state.tab == *tab_id;
        let style = if is_active {
            Style::default()
                .fg(Color::Rgb(187, 154, 247))
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(Color::Rgb(169, 177, 214))
        };
        let text = if is_active {
            format!("  {}  ", label)
        } else {
            format!("  {}  ", label)
        };
        frame.render_widget(Paragraph::new(text).style(style).alignment(Alignment::Center), r);
    }
}

fn draw_settings_content(frame: &mut Frame, area: Rect, state: &SettingsState) {
    let items = state.tab_items();
    let max_rows = area.height as usize;
    if max_rows == 0 { return; }

    let mut scroll = state.scroll;
    let cursor = state.cursor;

    // Ensure cursor is visible
    if cursor < scroll {
        scroll = cursor;
    } else if cursor >= scroll + max_rows {
        scroll = cursor.saturating_sub(max_rows.saturating_sub(1));
    }

    let mut lines: Vec<Line> = Vec::new();

    for (i, item) in items.iter().enumerate().skip(scroll).take(max_rows) {
        let selected = i == cursor;
        match item {
            TabItem::Section(label) => {
                lines.push(Line::from(Span::styled(
                    format!(" ── {} ──", label),
                    Style::default()
                        .fg(Color::Rgb(86, 95, 137))
                        .add_modifier(Modifier::DIM),
                )));
            }
            TabItem::Setting(id) => {
                let val = state.value_for(id);
                let capturing = state.capturing.is_some()
                    && matches!(id, SettingId::KeyAction(_));

                let label_width = 20usize;
                let val_space = area.width.saturating_sub(label_width as u16 + 2) as usize;

                let display_val = if capturing {
                    " press key combo...".into()
                } else if val.len() > val_space {
                    format!("{}…", &val[..val_space.saturating_sub(1)])
                } else {
                    val.clone()
                };

                let label_style = if selected {
                    Style::default().fg(Color::Rgb(187, 154, 247)).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Rgb(192, 202, 245))
                };

                let val_style = if capturing {
                    Style::default().fg(Color::Rgb(158, 206, 106))                    .add_modifier(Modifier::SLOW_BLINK)
                } else if selected {
                    Style::default().fg(Color::Rgb(224, 175, 104)).bg(Color::Rgb(31, 35, 53))
                } else {
                    Style::default().fg(Color::Rgb(169, 177, 214))
                };

                if selected && state.editing.is_some() {
                    if let Some(SettingsEdit::Text { value, cursor }) = &state.editing {
                        let (before, after) = value.split_at(*cursor.min(&value.len()));
                        lines.push(Line::from(vec![
                            Span::styled(format!(" {:w$}", id_label(id), w = label_width), label_style),
                            Span::raw(" "),
                            Span::styled(before, Style::default().fg(Color::Rgb(158, 206, 106))),
                            Span::styled("▎", Style::default().fg(Color::Rgb(158, 206, 106)).add_modifier(Modifier::BOLD)),
                            Span::styled(after, Style::default().fg(Color::Rgb(158, 206, 106))),
                        ]));
                        continue;
                    }
                }

                lines.push(Line::from(vec![
                    Span::styled(format!(" {:w$}", id_label(id), w = label_width), label_style),
                    Span::raw(" "),
                    Span::styled(format!("{:w$}", display_val, w = val_space), val_style),
                ]));
            }
        }
    }

    while lines.len() < max_rows {
        lines.push(Line::raw(""));
    }

    frame.render_widget(Paragraph::new(lines).scroll((scroll as u16, 0)), area);
}

fn id_label(id: &SettingId) -> &'static str {
    use SettingId::*;
    match id {
        Layout => "Layout",
        ShowHidden => "Show hidden",
        FollowSymlinks => "Follow symlinks",
        ScrollThreshold => "Scroll threshold",
        Shell => "Shell",
        ShellMode => "Shell mode",
        ConfirmDelete => "Confirm delete",
        TrashDir => "Trash dir",
        ShowShortcutHints => "Show shortcut hints",
        BorderStyle => "Border style",
        ColorForeground => "  Foreground",
        ColorBackground => "  Background",
        ColorSelectionBg => "  Selection bg",
        ColorSelectionFg => "  Selection fg",
        ColorDirectory => "  Directory",
        ColorSymlink => "  Symlink",
        ColorExecutable => "  Executable",
        ColorArchive => "  Archive",
        ColorMedia => "  Media",
        ColorHidden => "  Hidden",
        ColorBorderActive => "  Border active",
        ColorBorderInactive => "  Border inactive",
        ColorStatusbarBg => "  Statusbar bg",
        ColorStatusbarFg => "  Statusbar fg",
        ColorError => "  Error",
        ColorWarning => "  Warning",
        ColorSuccess => "  Success",
        ColorModalBg => "  Modal bg",
        ColorModalBorder => "  Modal border",
        SymbolDirOpen => "  Dir open",
        SymbolDirClosed => "  Dir closed",
        SymbolSymlink => "  Symlink",
        SymbolFile => "  File",
        SymbolSelected => "  Selected",
        SymbolUnselected => "  Unselected",
        SymbolCopyMarker => "  Copy marker",
        SymbolCutMarker => "  Cut marker",
        SymbolErrorMarker => "  Error marker",
        KeyAction(a) => {
            use Action::*;
            match a {
                MoveUp => "Move up",
                MoveDown => "Move down",
                MoveLeft => "Move left",
                MoveRight => "Move right",
                PageUp => "Page up",
                PageDown => "Page down",
                GotoTop => "Go to top",
                GotoBottom => "Go to bottom",
                OpenEntry => "Open entry",
                GoParent => "Go to parent",
                SelectToggle => "Toggle selection",
                SelectAll => "Select all",
                SelectNone => "Select none",
                InvertSelection => "Invert selection",
                Copy => "Copy",
                Cut => "Cut",
                Paste => "Paste",
                Delete => "Delete",
                Rename => "Rename",
                NewFile => "New file",
                NewDir => "New directory",
                ToggleHidden => "Toggle hidden",
                TogglePreview => "Toggle preview",
                CycleLayout => "Cycle layout",
                SetLayoutSingle => "Layout: single",
                SetLayoutDual => "Layout: dual",
                SetLayoutMiller => "Layout: miller",
                Refresh => "Refresh",
                Search => "Search",
                Filter => "Filter",
                ClearFilter => "Clear filter",
                OpenConfig => "Open config",
                OpenShell => "Open shell",
                OpenSettings => "Settings",
                Quit => "Quit",
                Help => "Help",
                SwitchPane => "Switch pane",
            }
        }
    }
}

fn draw_settings_footer(frame: &mut Frame, area: Rect, state: &SettingsState) {
    let help = if state.capturing.is_some() {
        " Press any key to bind... Esc to cancel "
    } else if state.editing.is_some() {
        " Enter to confirm  Esc to cancel "
    } else {
        " ↑↓ navigate  Enter edit / toggle / capture  Tab switch tab  Esc close "
    };
    frame.render_widget(
        Paragraph::new(help)
            .style(Style::default().fg(Color::Rgb(86, 95, 137)))
            .alignment(Alignment::Center),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Action, Config, KeyBinding, LayoutMode, ShellMode, theme::BorderStyle};
    use std::path::PathBuf;

    #[test]
    fn modal_confirm_constructor() {
        let m = Modal::confirm("Delete", "Delete 3 items?");
        assert!(matches!(&m, Modal::Confirm { title, message, selected }
            if title == "Delete" && message == "Delete 3 items?" && selected == &ConfirmChoice::No));
    }

    #[test]
    fn modal_input_constructor() {
        let m = Modal::input("Rename", "New name:", "old.txt", InputIntent::Rename);
        match &m {
            Modal::Input { title, prompt, value, cursor, .. } => {
                assert_eq!(title, "Rename");
                assert_eq!(prompt, "New name:");
                assert_eq!(value, "old.txt");
                assert_eq!(*cursor, 7);
            }
            _ => panic!("Expected Input modal"),
        }
    }

    #[test]
    fn modal_input_empty_prefill() {
        let m = Modal::input("New File", "File name:", "", InputIntent::NewFile);
        match &m {
            Modal::Input { value, cursor, .. } => {
                assert_eq!(value, "");
                assert_eq!(*cursor, 0);
            }
            _ => panic!("Expected Input modal"),
        }
    }

    #[test]
    fn centered_rect_basic() {
        let area = Rect { x: 0, y: 0, width: 100, height: 50 };
        let r = centered_rect(50, 20, area);
        assert_eq!(r.x, 25);
        assert_eq!(r.y, 15);
        assert_eq!(r.width, 50);
        assert_eq!(r.height, 20);
    }

    #[test]
    fn centered_rect_full_width() {
        let area = Rect { x: 0, y: 0, width: 100, height: 50 };
        let r = centered_rect(100, 10, area);
        assert_eq!(r.x, 0);
        assert_eq!(r.width, 100);
    }

    #[test]
    fn centered_rect_taller_than_area() {
        let area = Rect { x: 0, y: 0, width: 100, height: 10 };
        let r = centered_rect(50, 20, area);
        assert_eq!(r.height, 10);
    }

    #[test]
    fn centered_rect_with_offset() {
        let area = Rect { x: 10, y: 10, width: 100, height: 50 };
        let r = centered_rect(50, 20, area);
        assert_eq!(r.x, 35);
        assert_eq!(r.y, 25);
    }

    #[test]
    fn settings_state_new() {
        let config = Config::default();
        let state = SettingsState::new(config);
        assert_eq!(state.tab, SettingsTab::General);
        assert_eq!(state.cursor, 0);
        assert_eq!(state.scroll, 0);
        assert!(state.editing.is_none());
        assert!(state.capturing.is_none());
        assert!(!state.changed);
    }

    #[test]
    fn settings_tab_items_general() {
        let state = SettingsState::new(Config::default());
        let items = state.tab_items();
        assert!(matches!(items[0], TabItem::Setting(SettingId::Layout)));
        assert_eq!(items.len(), 9);
    }

    #[test]
    fn settings_tab_items_theme() {
        let mut state = SettingsState::new(Config::default());
        state.tab = SettingsTab::Theme;
        let items = state.tab_items();
        assert!(items.len() > 25);
        assert!(items.iter().any(|i| matches!(i, TabItem::Section(s) if *s == "Colors")));
        assert!(items.iter().any(|i| matches!(i, TabItem::Section(s) if *s == "Symbols")));
    }

    #[test]
    fn settings_tab_items_keymap() {
        let mut state = SettingsState::new(Config::default());
        state.tab = SettingsTab::Keymap;
        let items = state.tab_items();
        assert!(items.iter().any(|i| matches!(i, TabItem::Setting(SettingId::KeyAction(Action::MoveUp)))));
        assert!(items.iter().any(|i| matches!(i, TabItem::Setting(SettingId::KeyAction(Action::Quit)))));
    }

    #[test]
    fn settings_max_cursor() {
        let state = SettingsState::new(Config::default());
        assert!(state.max_cursor() > 0);
    }

    #[test]
    fn settings_value_for_layout() {
        let state = SettingsState::new(Config::default());
        assert_eq!(state.value_for(&SettingId::Layout), "miller");
    }

    #[test]
    fn settings_value_for_show_hidden() {
        let state = SettingsState::new(Config::default());
        assert_eq!(state.value_for(&SettingId::ShowHidden), "no");
    }

    #[test]
    fn settings_value_for_scroll_threshold() {
        let state = SettingsState::new(Config::default());
        assert_eq!(state.value_for(&SettingId::ScrollThreshold), "500");
    }

    #[test]
    fn settings_value_for_trash_dir() {
        let state = SettingsState::new(Config::default());
        assert_eq!(state.value_for(&SettingId::TrashDir), "none");
    }

    #[test]
    fn settings_value_for_color() {
        let state = SettingsState::new(Config::default());
        assert_eq!(state.value_for(&SettingId::ColorForeground), "#c0caf5");
    }

    #[test]
    fn settings_value_for_symbol() {
        let state = SettingsState::new(Config::default());
        assert_eq!(state.value_for(&SettingId::SymbolSelected), "󰄬 ");
    }

    #[test]
    fn settings_toggle_bool() {
        let mut state = SettingsState::new(Config::default());
        assert!(!state.config.general.show_hidden);
        state.toggle_bool(&SettingId::ShowHidden);
        assert!(state.config.general.show_hidden);
        assert!(state.changed);
    }

    #[test]
    fn settings_toggle_bool_twice() {
        let mut state = SettingsState::new(Config::default());
        state.toggle_bool(&SettingId::ShowHidden);
        state.toggle_bool(&SettingId::ShowHidden);
        assert!(!state.config.general.show_hidden);
    }

    #[test]
    fn settings_cycle_layout() {
        let mut state = SettingsState::new(Config::default());
        assert_eq!(state.config.general.layout, LayoutMode::Miller);
        state.cycle_enum(&SettingId::Layout);
        assert_eq!(state.config.general.layout, LayoutMode::Single);
        state.cycle_enum(&SettingId::Layout);
        assert_eq!(state.config.general.layout, LayoutMode::Dual);
        state.cycle_enum(&SettingId::Layout);
        assert_eq!(state.config.general.layout, LayoutMode::Miller);
    }

    #[test]
    fn settings_cycle_shell_mode() {
        let mut state = SettingsState::new(Config::default());
        assert_eq!(state.config.general.shell_mode, ShellMode::Capture);
        state.cycle_enum(&SettingId::ShellMode);
        assert_eq!(state.config.general.shell_mode, ShellMode::Takeover);
        state.cycle_enum(&SettingId::ShellMode);
        assert_eq!(state.config.general.shell_mode, ShellMode::Capture);
    }

    #[test]
    fn settings_cycle_border_style() {
        let mut state = SettingsState::new(Config::default());
        assert_eq!(state.config.theme.border_style, BorderStyle::Rounded);
        state.cycle_enum(&SettingId::BorderStyle);
        assert_eq!(state.config.theme.border_style, BorderStyle::Double);
        state.cycle_enum(&SettingId::BorderStyle);
        assert_eq!(state.config.theme.border_style, BorderStyle::Thick);
        state.cycle_enum(&SettingId::BorderStyle);
        assert_eq!(state.config.theme.border_style, BorderStyle::None);
        state.cycle_enum(&SettingId::BorderStyle);
        assert_eq!(state.config.theme.border_style, BorderStyle::Plain);
        state.cycle_enum(&SettingId::BorderStyle);
        assert_eq!(state.config.theme.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn settings_apply_shell_value() {
        let mut state = SettingsState::new(Config::default());
        state.apply_value(&SettingId::Shell, "/bin/zsh");
        assert_eq!(state.config.general.shell, "/bin/zsh");
    }

    #[test]
    fn settings_apply_scroll_threshold() {
        let mut state = SettingsState::new(Config::default());
        state.apply_value(&SettingId::ScrollThreshold, "1000");
        assert_eq!(state.config.general.scroll_threshold, 1000);
    }

    #[test]
    fn settings_apply_invalid_scroll_threshold() {
        let mut state = SettingsState::new(Config::default());
        state.apply_value(&SettingId::ScrollThreshold, "not_a_number");
        assert_eq!(state.config.general.scroll_threshold, 500);
    }

    #[test]
    fn settings_apply_trash_dir() {
        let mut state = SettingsState::new(Config::default());
        state.apply_value(&SettingId::TrashDir, "/tmp/trash");
        assert_eq!(state.config.general.trash_dir, Some(PathBuf::from("/tmp/trash")));
    }

    #[test]
    fn settings_apply_trash_dir_clear() {
        let mut state = SettingsState::new(Config::default());
        state.config.general.trash_dir = Some(PathBuf::from("/tmp/trash"));
        state.apply_value(&SettingId::TrashDir, "none");
        assert_eq!(state.config.general.trash_dir, None);
    }

    #[test]
    fn settings_apply_color() {
        let mut state = SettingsState::new(Config::default());
        state.apply_value(&SettingId::ColorForeground, "#ff0000");
        assert!(matches!(&state.config.theme.colors.foreground, super::RiftColor::Hex(s) if s == "#ff0000"));
    }

    #[test]
    fn settings_apply_symbol() {
        let mut state = SettingsState::new(Config::default());
        state.apply_value(&SettingId::SymbolDirOpen, ">");
        assert_eq!(state.config.theme.symbols.dir_open, ">");
    }

    #[test]
    fn settings_apply_capture_binds_key() {
        let mut state = SettingsState::new(Config::default());
        state.capturing = Some(Action::MoveUp);
        state.apply_capture("ctrl+u");
        assert!(state.config.keymap.0.contains_key("ctrl+u"));
        assert!(matches!(
            state.config.keymap.0.get("ctrl+u"),
            Some(KeyBinding::Action(Action::MoveUp))
        ));
        assert!(state.changed);
        assert!(state.capturing.is_none());
    }

    #[test]
    fn settings_apply_capture_removes_old() {
        let mut state = SettingsState::new(Config::default());
        state.capturing = Some(Action::MoveDown);
        state.apply_capture("alt+j");
        let moves_down: Vec<&String> = state.config.keymap.0.iter()
            .filter(|(_, v)| matches!(v, KeyBinding::Action(a) if *a == Action::MoveDown))
            .map(|(k, _)| k)
            .collect();
        assert_eq!(moves_down.len(), 1, "MoveDown should have exactly 1 binding");
        assert_eq!(moves_down[0], "alt+j");
    }

    #[test]
    fn settings_apply_capture_noop_when_not_capturing() {
        let mut state = SettingsState::new(Config::default());
        state.capturing = None;
        state.apply_capture("ctrl+u");
        assert!(!state.changed);
    }

    #[test]
    fn settings_apply_capture_replaces_existing_key() {
        let mut state = SettingsState::new(Config::default());
        state.capturing = Some(Action::MoveUp);
        state.apply_capture("q");
        assert!(matches!(
            state.config.keymap.0.get("q"),
            Some(KeyBinding::Action(Action::MoveUp))
        ));
    }

    #[test]
    fn yesno_true() {
        assert_eq!(super::yesno(true), "yes");
    }

    #[test]
    fn yesno_false() {
        assert_eq!(super::yesno(false), "no");
    }

    #[test]
    fn color_val_hex() {
        let c = super::RiftColor::Hex("#c0caf5".into());
        assert_eq!(super::color_val(&c), "#c0caf5");
    }

    #[test]
    fn color_val_named() {
        let c = super::RiftColor::Named("red".into());
        assert_eq!(super::color_val(&c), "red");
    }

    #[test]
    fn parse_color_with_hash() {
        let c = super::parse_color("#ff6600");
        assert!(matches!(c, super::RiftColor::Hex(s) if s == "#ff6600"));
    }

    #[test]
    fn parse_color_without_hash() {
        let c = super::parse_color("ff6600");
        assert!(matches!(c, super::RiftColor::Hex(s) if s == "#ff6600"));
    }

    #[test]
    fn parse_color_named() {
        let c = super::parse_color("red");
        assert!(matches!(c, super::RiftColor::Named(s) if s == "red"));
    }
}
