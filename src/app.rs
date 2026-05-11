use crate::{
    config::{Action, Config, KeyBinding, LayoutMode, ShellMode},
    fs::{self, read_dir, Conflict, ConflictResolution, OpResult},
    input::key_to_string,
    shell,
    ui::{
        compute_layout, draw_modal, draw_preview,
        modal::{ConflictChoice, ConfirmChoice, ConflictDialogState},
        Modal, Pane, StatusBar,
    },
};
use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    path::PathBuf,
    sync::mpsc,
    time::Duration,
};

/// What's in the clipboard.
#[derive(Clone)]
pub struct Clipboard {
    pub paths: Vec<PathBuf>,
    pub is_cut: bool,
}

/// Top-level input mode.
pub enum InputMode {
    Normal,
    Search(String),
    Filter(String),
    AwaitingModal,
}

pub struct App {
    pub config: Config,
    pub layout: LayoutMode,
    pub show_preview: bool,
    pub show_hidden: bool,

    /// The main pane (always present).
    pub primary: Pane,
    /// Second pane for Dual layout.
    pub secondary: Option<Pane>,
    /// Parent pane for Miller layout.
    pub parent: Option<Pane>,

    /// Which pane is focused (0 = primary, 1 = secondary).
    pub active_pane: usize,

    pub clipboard: Option<Clipboard>,
    pub modal: Option<Modal>,
    pub input_mode: InputMode,

    pub running: bool,
}

impl App {
    pub fn new(config: Config, start_dir: PathBuf) -> Result<Self> {
        let show_hidden = config.general.show_hidden;
        let layout = config.general.layout.clone();

        let entries = read_dir(&start_dir, show_hidden)?;
        let primary = Pane::new(start_dir.clone(), entries);

        let parent = if matches!(layout, LayoutMode::Miller) {
            build_parent_pane(&start_dir, show_hidden)
        } else {
            None
        };

        Ok(Self {
            layout,
            show_preview: true,
            show_hidden,
            primary,
            secondary: None,
            parent,
            active_pane: 0,
            clipboard: None,
            modal: None,
            input_mode: InputMode::Normal,
            running: true,
            config,
        })
    }

    pub fn run(mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        while self.running {
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.size();
        let layout_areas = compute_layout(area, &self.layout, self.show_preview);

        // Draw parent pane (Miller left column)
        if let (Some(parent), Some(parent_area)) = (&mut self.parent, layout_areas.panes.first().copied()) {
            if matches!(self.layout, LayoutMode::Miller) && layout_areas.panes.len() >= 2 {
                parent.is_active = false;
                parent.draw(frame, parent_area, &self.config.theme, "Parent");
            }
        }

        // Draw primary pane
        let primary_area = if matches!(self.layout, LayoutMode::Miller) {
            layout_areas.panes.get(1).copied().unwrap_or(layout_areas.panes[0])
        } else {
            layout_areas.panes[0]
        };

        self.primary.is_active = self.active_pane == 0;
        let cwd_title = self.primary.cwd.display().to_string();
        self.primary.draw(frame, primary_area, &self.config.theme, &cwd_title);

        // Draw secondary pane (Dual)
        if let (Some(secondary), Some(sec_area)) = (&mut self.secondary, layout_areas.panes.get(1).copied()) {
            if matches!(self.layout, LayoutMode::Dual) {
                secondary.is_active = self.active_pane == 1;
                let sec_title = secondary.cwd.display().to_string();
                secondary.draw(frame, sec_area, &self.config.theme, &sec_title);
            }
        }

        // Draw preview
        if let Some(preview_area) = layout_areas.preview {
            let focused_entry = self.primary.focused_entry().cloned();
            draw_preview(frame, preview_area, focused_entry.as_ref());
        }

        // Draw status bar
        let focused = self.primary.focused_entry().cloned();
        let selected_count = self.primary.selected.len();
        let filter = self.primary.filter.clone();
        StatusBar::draw(
            frame,
            layout_areas.statusbar,
            &self.config.theme,
            &self.primary.cwd.clone(),
            focused.as_ref(),
            selected_count,
            self.clipboard.is_some(),
            self.clipboard.as_ref().map(|c| c.is_cut).unwrap_or(false),
            filter.as_deref(),
        );

        // Draw modal on top
        if let Some(modal) = &self.modal {
            draw_modal(frame, modal, area);
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        // Modal gets priority
        if self.modal.is_some() {
            return self.handle_modal_key(key);
        }

        match &self.input_mode {
            InputMode::Search(_) | InputMode::Filter(_) => {
                return self.handle_text_input(key);
            }
            InputMode::Normal | InputMode::AwaitingModal => {}
        }

        let key_str = key_to_string(&key);
        if key_str.is_empty() {
            return Ok(());
        }

        if let Some(binding) = self.config.keymap.get(&key_str).cloned() {
            match binding {
                KeyBinding::Action(action) => self.dispatch_action(action)?,
                KeyBinding::Shell(cmd) => self.dispatch_shell(&cmd)?,
            }
        }

        Ok(())
    }

    fn dispatch_action(&mut self, action: Action) -> Result<()> {
        let pane = &mut self.primary;

        match action {
            Action::MoveUp      => pane.move_up(),
            Action::MoveDown    => pane.move_down(),
            Action::GotoTop     => pane.goto_top(),
            Action::GotoBottom  => pane.goto_bottom(),
            Action::PageUp      => pane.page_up(20),
            Action::PageDown    => pane.page_down(20),

            Action::MoveLeft | Action::GoParent => {
                if let Some(parent) = self.primary.cwd.parent().map(|p| p.to_path_buf()) {
                    self.navigate_to(parent)?;
                }
            }

            Action::MoveRight | Action::OpenEntry => {
                if let Some(entry) = pane.focused_entry().cloned() {
                    if entry.is_dir() {
                        let path = entry.path.clone();
                        self.navigate_to(path)?;
                    }
                    // TODO: open file with $OPENER
                }
            }

            Action::SelectToggle  => self.primary.toggle_selection(),
            Action::SelectAll     => self.primary.select_all(),
            Action::SelectNone    => {
                self.primary.clear_selection();
                if let InputMode::Filter(_) = &self.input_mode {
                    self.primary.filter = None;
                }
                self.input_mode = InputMode::Normal;
            }

            Action::Copy => {
                let paths = self.primary.operative_entries();
                if !paths.is_empty() {
                    self.clipboard = Some(Clipboard { paths, is_cut: false });
                }
            }
            Action::Cut => {
                let paths = self.primary.operative_entries();
                if !paths.is_empty() {
                    self.clipboard = Some(Clipboard { paths, is_cut: true });
                }
            }
            Action::Paste => self.do_paste()?,

            Action::Delete => {
                let paths = self.primary.operative_entries();
                if !paths.is_empty() && self.config.general.confirm_delete {
                    self.modal = Some(Modal::confirm(
                        "Delete",
                        format!("Delete {} item(s)?", paths.len()),
                    ));
                } else {
                    self.do_delete()?;
                }
            }

            Action::Rename => {
                if let Some(entry) = self.primary.focused_entry() {
                    let name = entry.name.clone();
                    self.modal = Some(Modal::input("Rename", "New name:", &name));
                }
            }

            Action::NewFile => {
                self.modal = Some(Modal::input("New File", "File name:", ""));
            }

            Action::NewDir => {
                self.modal = Some(Modal::input("New Directory", "Directory name:", ""));
            }

            Action::ToggleHidden => {
                self.show_hidden = !self.show_hidden;
                self.refresh_primary()?;
            }

            Action::TogglePreview => {
                self.show_preview = !self.show_preview;
            }

            Action::CycleLayout => {
                self.layout = match self.layout {
                    LayoutMode::Single => LayoutMode::Dual,
                    LayoutMode::Dual   => LayoutMode::Miller,
                    LayoutMode::Miller => LayoutMode::Single,
                };
                self.sync_secondary_pane()?;
            }
            Action::SetLayoutSingle => { self.layout = LayoutMode::Single; }
            Action::SetLayoutDual   => { self.layout = LayoutMode::Dual; self.sync_secondary_pane()?; }
            Action::SetLayoutMiller => { self.layout = LayoutMode::Miller; self.sync_secondary_pane()?; }

            Action::Refresh => self.refresh_primary()?,

            Action::Search => {
                self.input_mode = InputMode::Search(String::new());
            }
            Action::Filter => {
                self.input_mode = InputMode::Filter(String::new());
            }
            Action::ClearFilter => {
                self.primary.filter = None;
                self.input_mode = InputMode::Normal;
            }

            Action::Help => {
                self.modal = Some(Modal::Help { scroll: 0 });
            }

            Action::Quit => {
                self.running = false;
            }

            Action::OpenConfig => {
                let path = Config::config_path();
                let cmd = format!("$EDITOR {}", path.display());
                self.dispatch_shell(&cmd)?;
            }

            Action::OpenShell => {
                self.dispatch_shell("$SHELL")?;
            }

            Action::InvertSelection => {
                let all: Vec<PathBuf> = self.primary.entries.iter().map(|e| e.path.clone()).collect();
                for p in all {
                    if self.primary.selected.contains(&p) {
                        self.primary.selected.remove(&p);
                    } else {
                        self.primary.selected.insert(p);
                    }
                }
            }
        }

        Ok(())
    }

    fn dispatch_shell(&mut self, template: &str) -> Result<()> {
        let focused = self.primary.focused_entry().map(|e| e.path.clone());
        let cmd = shell::expand_command(template, focused.as_deref());
        let cwd = self.primary.cwd.clone();
        let shell_bin = self.config.general.shell.clone();

        match self.config.general.shell_mode {
            ShellMode::Takeover => {
                // Suspend TUI, run, resume
                crossterm::terminal::disable_raw_mode()?;
                crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

                shell::run_takeover(&shell_bin, &cmd, &cwd).ok();

                crossterm::terminal::enable_raw_mode()?;
                crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
                self.refresh_primary()?;
            }
            ShellMode::Capture => {
                let rt = tokio::runtime::Handle::current();
                let output = rt.block_on(shell::run_capture(&shell_bin, &cmd, &cwd));
                let lines = match output {
                    Ok(out) => out.lines().map(|l| l.to_string()).collect(),
                    Err(e)  => vec![format!("Error: {}", e)],
                };
                self.modal = Some(Modal::Summary {
                    title: format!("$ {}", cmd),
                    lines,
                    scroll: 0,
                });
            }
        }

        Ok(())
    }

    fn do_paste(&mut self) -> Result<()> {
        let Some(clip) = self.clipboard.clone() else { return Ok(()) };
        let dest = self.primary.cwd.clone();

        // Collect conflict resolutions interactively
        // For now we queue the op and let the async runtime handle it;
        // conflicts will queue modals via a channel in a fuller implementation.
        // Here we do a synchronous resolution loop for the MVP.
        let (tx, rx) = mpsc::channel::<ConflictResolution>();

        // We resolve conflicts by showing a modal — but since our event loop
        // is single-threaded, we use a simpler approach: run the op synchronously
        // with a blocking resolver that shows the modal and spins until resolved.
        //
        // In a production version this would be an async task with a oneshot channel.

        let sources = clip.paths.clone();
        let is_cut = clip.is_cut;

        // Synchronous paste with inline conflict handling
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        let results = if is_cut {
            rt.block_on(fs::move_entries(&sources, &dest, |conflict| {
                // TODO: surface modal — for MVP, default to skip
                let _ = tx.send(ConflictResolution::Skip);
                ConflictResolution::Skip
            }))
        } else {
            rt.block_on(fs::copy_entries(&sources, &dest, |_conflict| {
                ConflictResolution::Skip
            }))
        };

        if is_cut {
            self.clipboard = None;
        }

        // Show summary if any failures
        let failures: Vec<String> = results
            .iter()
            .filter_map(|r| {
                if let OpResult::Failed { src, error } = r {
                    Some(format!("✗ {}: {}", src.display(), error))
                } else {
                    None
                }
            })
            .collect();

        if !failures.is_empty() {
            self.modal = Some(Modal::Summary {
                title: "Paste — Errors".into(),
                lines: failures,
                scroll: 0,
            });
        }

        self.refresh_primary()?;
        Ok(())
    }

    fn do_delete(&mut self) -> Result<()> {
        let paths = self.primary.operative_entries();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let results = rt.block_on(fs::delete_entries(&paths));

        let failures: Vec<String> = results
            .iter()
            .filter_map(|r| {
                if let OpResult::Failed { src, error } = r {
                    Some(format!("✗ {}: {}", src.display(), error))
                } else {
                    None
                }
            })
            .collect();

        if !failures.is_empty() {
            self.modal = Some(Modal::Summary {
                title: "Delete — Errors".into(),
                lines: failures,
                scroll: 0,
            });
        }

        self.primary.clear_selection();
        self.refresh_primary()?;
        Ok(())
    }

    fn handle_modal_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;

        let Some(modal) = &mut self.modal else { return Ok(()) };

        match modal {
            Modal::Confirm { selected, .. } => match key.code {
                KeyCode::Left | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    *selected = ConfirmChoice::Yes;
                    let confirmed = matches!(selected, ConfirmChoice::Yes);
                    self.modal = None;
                    if confirmed {
                        self.do_delete()?;
                    }
                }
                KeyCode::Right | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.modal = None;
                }
                KeyCode::Enter => {
                    let is_yes = matches!(selected, ConfirmChoice::Yes);
                    self.modal = None;
                    if is_yes {
                        self.do_delete()?;
                    }
                }
                _ => {}
            },

            Modal::Input { title, value, cursor, .. } => {
                let title = title.clone();
                match key.code {
                    KeyCode::Char(c) => {
                        value.insert(*cursor, c);
                        *cursor += 1;
                    }
                    KeyCode::Backspace => {
                        if *cursor > 0 {
                            *cursor -= 1;
                            value.remove(*cursor);
                        }
                    }
                    KeyCode::Left  => { *cursor = cursor.saturating_sub(1); }
                    KeyCode::Right => { *cursor = (*cursor + 1).min(value.len()); }
                    KeyCode::Esc   => { self.modal = None; }
                    KeyCode::Enter => {
                        let val = value.clone();
                        self.modal = None;
                        self.apply_input(&title, &val)?;
                    }
                    _ => {}
                }
            }

            Modal::Conflict { conflict } => {
                use crossterm::event::KeyCode;
                match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        conflict.selected = match conflict.selected {
                            ConflictChoice::Overwrite => ConflictChoice::Skip,
                            ConflictChoice::Rename    => ConflictChoice::Overwrite,
                            ConflictChoice::Abort     => ConflictChoice::Rename,
                            ConflictChoice::Skip      => ConflictChoice::Skip,
                        };
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        conflict.selected = match conflict.selected {
                            ConflictChoice::Skip      => ConflictChoice::Overwrite,
                            ConflictChoice::Overwrite => ConflictChoice::Rename,
                            ConflictChoice::Rename    => ConflictChoice::Abort,
                            ConflictChoice::Abort     => ConflictChoice::Abort,
                        };
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => { self.modal = None; }
                    KeyCode::Char('o') | KeyCode::Char('O') => { self.modal = None; }
                    KeyCode::Char('a') | KeyCode::Char('A') => { self.modal = None; }
                    KeyCode::Esc => { self.modal = None; }
                    KeyCode::Enter => { self.modal = None; }
                    _ => {}
                }
            }

            Modal::Summary { scroll, lines, .. } => {
                use crossterm::event::KeyCode;
                let max = lines.len().saturating_sub(1);
                match key.code {
                    KeyCode::Up   | KeyCode::Char('k') => { *scroll = scroll.saturating_sub(1); }
                    KeyCode::Down | KeyCode::Char('j') => { *scroll = (*scroll + 1).min(max); }
                    KeyCode::Esc  | KeyCode::Char('q') | KeyCode::Enter => { self.modal = None; }
                    _ => {}
                }
            }

            Modal::Help { scroll } => {
                use crossterm::event::KeyCode;
                match key.code {
                    KeyCode::Up   | KeyCode::Char('k') => { *scroll = scroll.saturating_sub(1); }
                    KeyCode::Down | KeyCode::Char('j') => { *scroll += 1; }
                    KeyCode::Esc  | KeyCode::Char('q') | KeyCode::Char('?') => { self.modal = None; }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn handle_text_input(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;

        match &mut self.input_mode {
            InputMode::Search(ref mut s) | InputMode::Filter(ref mut s) => {
                match key.code {
                    KeyCode::Char(c) => s.push(c),
                    KeyCode::Backspace => { s.pop(); }
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                        self.primary.filter = None;
                        return Ok(());
                    }
                    KeyCode::Enter => {
                        let query = s.clone();
                        let is_filter = matches!(self.input_mode, InputMode::Filter(_));
                        self.input_mode = InputMode::Normal;
                        if is_filter {
                            self.primary.filter = if query.is_empty() { None } else { Some(query) };
                        }
                        return Ok(());
                    }
                    _ => {}
                }
                // Live filter
                let query = s.clone();
                if matches!(self.input_mode, InputMode::Filter(_)) {
                    self.primary.filter = if query.is_empty() { None } else { Some(query) };
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn apply_input(&mut self, title: &str, value: &str) -> Result<()> {
        if value.is_empty() {
            return Ok(());
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        match title {
            "Rename" => {
                if let Some(entry) = self.primary.focused_entry() {
                    let src = entry.path.clone();
                    rt.block_on(fs::rename_entry(&src, value))?;
                }
            }
            "New File" => {
                let cwd = self.primary.cwd.clone();
                rt.block_on(fs::create_file(&cwd, value))?;
            }
            "New Directory" => {
                let cwd = self.primary.cwd.clone();
                rt.block_on(fs::create_dir(&cwd, value))?;
            }
            _ => {}
        }

        self.refresh_primary()?;
        Ok(())
    }

    fn navigate_to(&mut self, path: PathBuf) -> Result<()> {
        let old_cwd = self.primary.cwd.clone();
        let entries = read_dir(&path, self.show_hidden)?;
        self.primary = Pane::new(path.clone(), entries);

        // Update parent pane for Miller
        if matches!(self.layout, LayoutMode::Miller) {
            self.parent = build_parent_pane(&path, self.show_hidden);
            // Highlight the dir we came from in the parent
            if let Some(ref mut parent_pane) = self.parent {
                if let Some(idx) = parent_pane.entries.iter().position(|e| e.path == old_cwd) {
                    parent_pane.cursor = idx;
                    parent_pane.list_state.select(Some(idx));
                }
            }
        }

        Ok(())
    }

    fn refresh_primary(&mut self) -> Result<()> {
        let cwd = self.primary.cwd.clone();
        let cursor = self.primary.cursor;
        let entries = read_dir(&cwd, self.show_hidden)?;
        self.primary.entries = entries;
        self.primary.cursor = cursor.min(self.primary.entries.len().saturating_sub(1));
        self.primary.list_state.select(Some(self.primary.cursor));
        Ok(())
    }

    fn sync_secondary_pane(&mut self) -> Result<()> {
        match self.layout {
            LayoutMode::Dual => {
                if self.secondary.is_none() {
                    let cwd = self.primary.cwd.clone();
                    let entries = read_dir(&cwd, self.show_hidden)?;
                    self.secondary = Some(Pane::new(cwd, entries));
                }
                self.parent = None;
            }
            LayoutMode::Miller => {
                self.secondary = None;
                self.parent = build_parent_pane(&self.primary.cwd, self.show_hidden);
            }
            LayoutMode::Single => {
                self.secondary = None;
                self.parent = None;
            }
        }
        Ok(())
    }
}

fn build_parent_pane(cwd: &PathBuf, show_hidden: bool) -> Option<Pane> {
    let parent_path = cwd.parent()?;
    let entries = read_dir(parent_path, show_hidden).ok()?;
    let mut pane = Pane::new(parent_path.to_path_buf(), entries);
    // Highlight cwd in parent
    if let Some(idx) = pane.entries.iter().position(|e| &e.path == cwd) {
        pane.cursor = idx;
        pane.list_state.select(Some(idx));
    }
    Some(pane)
}
