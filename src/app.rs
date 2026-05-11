use crate::{
    config::{Action, Config, KeyBinding, LayoutMode, ShellMode},
    fs::{self, read_dir, Conflict, ConflictResolution, OpResult},
    input::key_to_string,
    shell,
    ui::{
        compute_layout, draw_modal, draw_preview,
        modal::{ConflictChoice, ConfirmChoice, SettingsEdit, SettingsState, SettingsTab},
        InputIntent, Modal, Pane, PreviewCache, StatusBar,
    },
};
use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    collections::HashMap,
    io,
    path::PathBuf,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

/// What's in the clipboard.
#[derive(Clone)]
pub struct Clipboard {
    pub paths: Vec<PathBuf>,
    pub is_cut: bool,
}

/// A paste operation running on a background thread.
pub(crate) struct BackgroundPaste {
    result_rx: mpsc::Receiver<Vec<OpResult>>,
    sources: Vec<PathBuf>,
    dest: PathBuf,
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
    pub pending_paste: Option<BackgroundPaste>,
    pub modal: Option<Modal>,
    pub input_mode: InputMode,

    pub running: bool,
    pub rt: tokio::runtime::Runtime,
    pub preview_cache: PreviewCache,
    /// Transient one-line message shown in the status bar (auto-clears after 2 s).
    pub status_message: Option<(String, std::time::Instant)>,
}

/// Top-level input mode.
pub enum InputMode {
    Normal,
    Search(String),
    Filter(String),
    AwaitingModal,
}

impl App {
    pub fn new(config: Config, start_dir: PathBuf) -> Result<Self> {
        let show_hidden = config.general.show_hidden;
        let layout = config.general.layout.clone();

        let scroll_threshold = config.general.scroll_threshold;
        let entries = read_dir(&start_dir, show_hidden)?;
        let primary = Pane::new(start_dir.clone(), entries, scroll_threshold);

        let parent = if matches!(layout, LayoutMode::Miller) {
            build_parent_pane(&start_dir, show_hidden, scroll_threshold)
        } else {
            None
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        Ok(Self {
            layout,
            show_preview: true,
            show_hidden,
            primary,
            secondary: None,
            parent,
            active_pane: 0,
            clipboard: None,
            pending_paste: None,
            modal: None,
            input_mode: InputMode::Normal,
            running: true,
            config,
            rt,
            preview_cache: PreviewCache::new(),
            status_message: None,
        })
    }

    /// Return a mutable reference to whichever pane is currently active.
    fn active_pane_mut(&mut self) -> &mut Pane {
        if self.active_pane == 1 {
            self.secondary.as_mut().unwrap_or(&mut self.primary)
        } else {
            &mut self.primary
        }
    }

    /// Find the key string bound to a built-in action, if any.
    fn key_for_action(&self, action: &Action) -> Option<String> {
        self.config.keymap.0.iter()
            .find(|(_, v)| matches!(v, KeyBinding::Action(a) if a == action))
            .map(|(k, _)| k.clone())
    }

    pub fn run(mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        while self.running {
            // Refresh preview cache from whichever pane is active, not always primary
            let focused = if self.active_pane == 1 {
                self.secondary.as_ref().and_then(|p| p.focused_entry()).cloned()
            } else {
                self.primary.focused_entry().cloned()
            };
            if self.preview_cache.needs_refresh(focused.as_ref()) {
                self.preview_cache.load(focused.as_ref());
            }

            // Expire transient status messages after 2 seconds
            if let Some((_, ts)) = &self.status_message {
                if ts.elapsed() >= Duration::from_secs(2) {
                    self.status_message = None;
                }
            }

            terminal.draw(|frame| self.draw(frame))?;

            self.check_pending_paste()?;

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
            draw_preview(frame, preview_area, &self.preview_cache);
        }

        // Draw status bar — use active pane for focused entry and selection count
        let focused = if self.active_pane == 1 {
            self.secondary.as_ref().and_then(|p| p.focused_entry()).cloned()
        } else {
            self.primary.focused_entry().cloned()
        };
        let active = if self.active_pane == 1 {
            self.secondary.as_ref().unwrap_or(&self.primary)
        } else {
            &self.primary
        };
        let selected_count = active.selected.len();
        let filter = active.filter.clone();
        let status_msg = self.status_message.as_ref().map(|(m, _)| m.as_str());
        // Resolve shortcut hints
        let show_hints = self.config.general.show_shortcut_hints;
        let key_help = if show_hints { self.key_for_action(&Action::Help) } else { None };
        let key_quit = if show_hints { self.key_for_action(&Action::Quit) } else { None };
        let key_settings = if show_hints { self.key_for_action(&Action::OpenSettings) } else { None };

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
            status_msg,
            show_hints,
            key_help.as_deref(),
            key_quit.as_deref(),
            key_settings.as_deref(),
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
        match action {
            Action::MoveUp      => self.active_pane_mut().move_up(),
            Action::MoveDown    => self.active_pane_mut().move_down(),
            Action::GotoTop     => self.active_pane_mut().goto_top(),
            Action::GotoBottom  => self.active_pane_mut().goto_bottom(),
            Action::PageUp => {
                let pane = self.active_pane_mut();
                let h = pane.visible_height.saturating_sub(1).max(1);
                pane.page_up(h);
            }
            Action::PageDown => {
                let pane = self.active_pane_mut();
                let h = pane.visible_height.saturating_sub(1).max(1);
                pane.page_down(h);
            }

            Action::SwitchPane => {
                if self.secondary.is_some() {
                    self.active_pane = 1 - self.active_pane;
                } else {
                    self.status_message = Some((
                        "Dual layout required to switch panes (use CycleLayout to switch)".into(),
                        std::time::Instant::now(),
                    ));
                }
            }

            Action::MoveLeft | Action::GoParent => {
                if let Some(parent) = self.primary.cwd.parent().map(|p| p.to_path_buf()) {
                    self.navigate_to(parent)?;
                }
            }

            Action::MoveRight | Action::OpenEntry => {
                if let Some(entry) = self.active_pane_mut().focused_entry().cloned() {
                    if entry.is_dir() {
                        let path = entry.path.clone();
                        let _ = self.navigate_to(path);
                    }
                    // TODO: open file with $OPENER
                }
            }

            Action::SelectToggle  => self.active_pane_mut().toggle_selection(),
            Action::SelectAll     => self.active_pane_mut().select_all(),
            Action::SelectNone => {
                self.active_pane_mut().clear_selection();
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
                    self.modal = Some(Modal::input("Rename", "New name:", &name, InputIntent::Rename));
                }
            }

            Action::NewFile => {
                self.modal = Some(Modal::input("New File", "File name:", "", InputIntent::NewFile));
            }

            Action::NewDir => {
                self.modal = Some(Modal::input("New Directory", "Directory name:", "", InputIntent::NewDir));
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
                let keymap = self.config.keymap.clone();
                self.modal = Some(Modal::Help { scroll: 0, keymap });
            }

            Action::OpenSettings => {
                // Sync live app state into config before editing
                self.config.general.layout = self.layout.clone();
                self.config.general.show_hidden = self.show_hidden;
                self.modal = Some(Modal::Settings(SettingsState::new(self.config.clone())));
            }

            Action::Quit => {
                self.running = false;
            }

            Action::OpenConfig => {
                let path = Config::config_path();
                let path_str = path.to_string_lossy();
                let quoted = format!("'{}'", path_str.replace('\'', r"'\''"));
                let cmd = format!("$EDITOR {}", quoted);
                self.dispatch_shell(&cmd)?;
            }

            Action::OpenShell => {
                self.dispatch_shell("$SHELL")?;
            }

            Action::InvertSelection => {
                let pane = self.active_pane_mut();
                let paths: Vec<PathBuf> = pane.visible_entries().iter().map(|e| e.path.clone()).collect();
                for p in paths {
                    if pane.selected.contains(&p) {
                        pane.selected.remove(&p);
                    } else {
                        pane.selected.insert(p);
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
                let output = self.rt.block_on(shell::run_capture(&shell_bin, &cmd, &cwd));
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
        let sources = clip.paths.clone();
        let is_cut = clip.is_cut;

        // Phase 1: detect conflicts and collect user resolutions interactively
        let mut resolution_map: HashMap<PathBuf, ConflictResolution> = HashMap::new();
        for src in &sources {
            let name = src.file_name().unwrap_or_default();
            let dst = dest.join(name);

            // Same path — pasting into origin directory
            if src == &dst {
                if is_cut {
                    continue;
                }
                let new_name = auto_rename(&dst);
                resolution_map.insert(src.clone(), ConflictResolution::Rename(new_name));
                continue;
            }

            if dst.exists() {
                let (tx, rx) = mpsc::channel();
                self.modal = Some(Modal::conflict(
                    Conflict { src: src.clone(), dst: dst.clone() },
                    tx,
                ));
                while self.modal.is_some() {
                    if event::poll(Duration::from_millis(50))? {
                        if let Event::Key(key) = event::read()? {
                            if key.kind == KeyEventKind::Press {
                                self.handle_modal_key(key)?;
                            }
                        }
                    }
                }
                let res = rx.try_recv().unwrap_or(ConflictResolution::Skip);
                if matches!(res, ConflictResolution::Abort) {
                    resolution_map.insert(src.clone(), ConflictResolution::Abort);
                    break;
                }
                resolution_map.insert(src.clone(), res);
            }
        }

        if is_cut {
            self.clipboard = None;
        }

        // Phase 2: spawn background thread so the UI stays responsive
        let resolutions = Arc::new(resolution_map);
        let sources_for_thread = sources.clone();
        let dest_for_thread = dest.clone();
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build background tokio runtime");

            let resolve = {
                let resolutions = Arc::clone(&resolutions);
                move |conflict: Conflict| -> ConflictResolution {
                    resolutions
                        .get(&conflict.src)
                        .cloned()
                        .unwrap_or(ConflictResolution::Overwrite)
                }
            };

            let results = if is_cut {
                rt.block_on(fs::move_entries(&sources_for_thread, &dest_for_thread, resolve))
            } else {
                rt.block_on(fs::copy_entries(&sources_for_thread, &dest_for_thread, resolve))
            };

            let _ = tx.send(results);
        });

        self.pending_paste = Some(BackgroundPaste {
            result_rx: rx,
            sources,
            dest,
        });

        Ok(())
    }

    fn check_pending_paste(&mut self) -> Result<()> {
        let Some(pending) = &self.pending_paste else { return Ok(()) };

        match pending.result_rx.try_recv() {
            Ok(results) => {
                let pending = self.pending_paste.take().unwrap();
                self.handle_paste_results(results, &pending.sources, &pending.dest)?;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending_paste = None;
            }
        }

        Ok(())
    }

    fn handle_paste_results(
        &mut self,
        results: Vec<OpResult>,
        _sources: &[PathBuf],
        _dest: &PathBuf,
    ) -> Result<()> {
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

        let skips: Vec<String> = results
            .iter()
            .filter_map(|r| {
                if let OpResult::Skipped { src } = r {
                    Some(format!("→ {}: skipped", src.display()))
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
        } else if !skips.is_empty() {
            self.modal = Some(Modal::Summary {
                title: "Paste — Skipped".into(),
                lines: skips,
                scroll: 0,
            });
        }

        self.refresh_primary()?;
        Ok(())
    }

    fn do_delete(&mut self) -> Result<()> {
        let paths = self.primary.operative_entries();
        let results = self.rt.block_on(fs::delete_entries(&paths));

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
                KeyCode::Left | KeyCode::Right => {
                    *selected = match selected {
                        ConfirmChoice::Yes => ConfirmChoice::No,
                        ConfirmChoice::No  => ConfirmChoice::Yes,
                    };
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    *selected = ConfirmChoice::Yes;
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    *selected = ConfirmChoice::No;
                }
                KeyCode::Enter => {
                    let is_yes = matches!(selected, ConfirmChoice::Yes);
                    self.modal = None;
                    if is_yes {
                        self.do_delete()?;
                    }
                }
                KeyCode::Esc => {
                    self.modal = None;
                }
                _ => {}
            },

            Modal::Input { value, cursor, intent, .. } => {
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
                        let int = intent.clone();
                        self.modal = None;
                        self.apply_input(int, &val)?;
                    }
                    _ => {}
                }
            }

            Modal::Conflict { conflict, response } => {
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
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        if let Some(tx) = response.take() {
                            let _ = tx.send(ConflictResolution::Skip);
                        }
                        self.modal = None;
                    }
                    KeyCode::Char('o') | KeyCode::Char('O') => {
                        if let Some(tx) = response.take() {
                            let _ = tx.send(ConflictResolution::Overwrite);
                        }
                        self.modal = None;
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        if let Some(tx) = response.take() {
                            let new_name = auto_rename(&conflict.conflict.dst);
                            let _ = tx.send(ConflictResolution::Rename(new_name));
                        }
                        self.modal = None;
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        if let Some(tx) = response.take() {
                            let _ = tx.send(ConflictResolution::Abort);
                        }
                        self.modal = None;
                    }
                    KeyCode::Esc => {
                        if let Some(tx) = response.take() {
                            let _ = tx.send(ConflictResolution::Abort);
                        }
                        self.modal = None;
                    }
                    KeyCode::Enter => {
                        if let Some(tx) = response.take() {
                            let resolution = match conflict.selected {
                                ConflictChoice::Skip      => ConflictResolution::Skip,
                                ConflictChoice::Overwrite => ConflictResolution::Overwrite,
                                ConflictChoice::Rename    => ConflictResolution::Rename(
                                    auto_rename(&conflict.conflict.dst),
                                ),
                                ConflictChoice::Abort     => ConflictResolution::Abort,
                            };
                            let _ = tx.send(resolution);
                        }
                        self.modal = None;
                    }
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

            Modal::Help { scroll, .. } => {
                use crossterm::event::KeyCode;
                match key.code {
                    KeyCode::Up   | KeyCode::Char('k') => { *scroll = scroll.saturating_sub(1); }
                    KeyCode::Down | KeyCode::Char('j') => { *scroll += 1; }
                    KeyCode::Esc  | KeyCode::Char('q') | KeyCode::Char('?') => { self.modal = None; }
                    _ => {}
                }
            }

            Modal::Settings(state) => {
                // Capture mode: next keypress binds to action
                if state.capturing.is_some() {
                    let key_str = crate::input::key_to_string(&key);
                    if !key_str.is_empty() {
                        state.apply_capture(&key_str);
                    } else {
                        state.capturing = None;
                    }
                    return Ok(());
                }

                // Inline text editing
                if let Some(ref mut edit) = state.editing {
                    let SettingsEdit::Text { value, cursor } = edit;
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
                        KeyCode::Left => {
                            *cursor = cursor.saturating_sub(1);
                        }
                        KeyCode::Right => {
                            *cursor = (*cursor + 1).min(value.len());
                        }
                        KeyCode::Enter => {
                            let val = value.clone();
                            state.apply_edit(&val);
                            state.editing = None;
                        }
                        KeyCode::Esc => {
                            state.editing = None;
                        }
                        _ => {}
                    }
                    return Ok(());
                }

                // Normal navigation within settings
                match key.code {
                    KeyCode::Tab | KeyCode::Char('\t') => {
                        state.tab = match state.tab {
                            SettingsTab::General => SettingsTab::Theme,
                            SettingsTab::Theme => SettingsTab::Keymap,
                            SettingsTab::Keymap => SettingsTab::General,
                        };
                        state.cursor = 0;
                        state.scroll = 0;
                    }
                    KeyCode::BackTab => {
                        state.tab = match state.tab {
                            SettingsTab::General => SettingsTab::Keymap,
                            SettingsTab::Theme => SettingsTab::General,
                            SettingsTab::Keymap => SettingsTab::Theme,
                        };
                        state.cursor = 0;
                        state.scroll = 0;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if state.cursor > 0 {
                            state.cursor -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if state.cursor < state.max_cursor() {
                            state.cursor += 1;
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        state.activate();
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        let changed = state.changed;
                        let config = state.config.clone();
                        self.modal = None;
                        if changed {
                            let old_hidden = self.show_hidden;
                            let old_layout = self.layout.clone();
                            self.config = config;
                            let _ = self.config.save();
                            if self.config.general.show_hidden != old_hidden {
                                self.show_hidden = self.config.general.show_hidden;
                                self.refresh_primary()?;
                            }
                            if self.config.general.layout != old_layout {
                                self.layout = self.config.general.layout.clone();
                                self.sync_secondary_pane()?;
                            }
                        }
                    }
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

    fn apply_input(&mut self, intent: InputIntent, value: &str) -> Result<()> {
        if value.is_empty() {
            return Ok(());
        }

        match intent {
            InputIntent::Rename => {
                if let Some(entry) = self.primary.focused_entry() {
                    let src = entry.path.clone();
                    self.rt.block_on(fs::rename_entry(&src, value))?;
                }
            }
            InputIntent::NewFile => {
                let cwd = self.primary.cwd.clone();
                self.rt.block_on(fs::create_file(&cwd, value))?;
            }
            InputIntent::NewDir => {
                let cwd = self.primary.cwd.clone();
                self.rt.block_on(fs::create_dir(&cwd, value))?;
            }
        }

        self.refresh_primary()?;
        Ok(())
    }

    fn navigate_to(&mut self, path: PathBuf) -> Result<()> {
        let old_cwd = self.primary.cwd.clone();
        let threshold = self.config.general.scroll_threshold;
        let entries = read_dir(&path, self.show_hidden)?;
        self.primary = Pane::new(path.clone(), entries, threshold);
        self.primary.clear_selection();

        // Update parent pane for Miller
        if matches!(self.layout, LayoutMode::Miller) {
            self.parent = build_parent_pane(&path, self.show_hidden, threshold);
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
        let threshold = self.config.general.scroll_threshold;
        match self.layout {
            LayoutMode::Dual => {
                if self.secondary.is_none() {
                    let cwd = self.primary.cwd.clone();
                    let entries = read_dir(&cwd, self.show_hidden)?;
                    self.secondary = Some(Pane::new(cwd, entries, threshold));
                }
                self.parent = None;
            }
            LayoutMode::Miller => {
                self.secondary = None;
                self.parent = build_parent_pane(&self.primary.cwd, self.show_hidden, threshold);
            }
            LayoutMode::Single => {
                self.secondary = None;
                self.parent = None;
            }
        }
        Ok(())
    }
}

fn build_parent_pane(cwd: &PathBuf, show_hidden: bool, scroll_threshold: usize) -> Option<Pane> {
    let parent_path = cwd.parent()?;
    let entries = read_dir(parent_path, show_hidden).ok()?;
    let mut pane = Pane::new(parent_path.to_path_buf(), entries, scroll_threshold);
    // Highlight cwd in parent
    if let Some(idx) = pane.entries.iter().position(|e| &e.path == cwd) {
        pane.cursor = idx;
        pane.list_state.select(Some(idx));
    }
    Some(pane)
}

/// Generate a non-colliding filename like `file_copy.txt`, `file_copy_2.txt`, etc.
fn auto_rename(path: &std::path::Path) -> String {
    let parent = path.parent().unwrap_or(std::path::Path::new("."));
    let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();

    let candidate = format!("{}_copy{}", name, ext);
    if !parent.join(&candidate).exists() {
        return candidate;
    }
    for i in 2.. {
        let candidate = format!("{}_copy_{}{}", name, i, ext);
        if !parent.join(&candidate).exists() {
            return candidate;
        }
    }
    format!("{}_copy_{}{}", name, std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs(), ext)
}

#[cfg(test)]
mod tests {
    use super::auto_rename;
    use std::path::Path;

    #[test]
    fn auto_rename_simple() {
        let name = auto_rename(Path::new("/tmp/file.txt"));
        assert_eq!(name, "file_copy.txt");
    }

    #[test]
    fn auto_rename_no_extension() {
        let name = auto_rename(Path::new("/tmp/Makefile"));
        assert_eq!(name, "Makefile_copy");
    }

    #[test]
    fn auto_rename_hidden_file() {
        let name = auto_rename(Path::new("/tmp/.hidden"));
        assert_eq!(name, ".hidden_copy");
    }

    #[test]
    fn auto_rename_with_existing_copy() {
        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path().join("file.txt");
        std::fs::write(&p, "").unwrap();
        let copy1 = dir.path().join("file_copy.txt");
        std::fs::write(&copy1, "").unwrap();
        let name = auto_rename(&p);
        assert_eq!(name, "file_copy_2.txt");
    }

    #[test]
    fn auto_rename_multiple_copies() {
        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path().join("file.txt");
        std::fs::write(&p, "").unwrap();
        std::fs::write(dir.path().join("file_copy.txt"), "").unwrap();
        for i in 2..=4 {
            std::fs::write(dir.path().join(format!("file_copy_{}.txt", i)), "").unwrap();
        }
        let name = auto_rename(&p);
        assert_eq!(name, "file_copy_5.txt");
    }

    #[test]
    fn auto_rename_no_extension_with_copy_exists() {
        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path().join("Makefile");
        std::fs::write(&p, "").unwrap();
        let copy1 = dir.path().join("Makefile_copy");
        std::fs::write(&copy1, "").unwrap();
        let name = auto_rename(&p);
        assert_eq!(name, "Makefile_copy_2");
    }
}
