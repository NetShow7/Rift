# Rift — TUI File Manager

Modern terminal file manager built with Rust (ratatui + crossterm).

## About

Rust TUI file manager with Miller columns, Dual pane, Single pane layouts. Configurable keybindings, themes (Tokyo Night default), shell integration, and file operations.

## Architecture

### Module Layout

```
src/
├── main.rs         Entry, terminal setup, panic guard
├── app.rs          App struct, event loop, key dispatch, file ops orchestration
├── input.rs        Key event → canonical string normalization
├── shell.rs        Shell command expansion, capture + takeover modes
├── config/
│   ├── mod.rs      Config loading/saving (TOML), GeneralConfig, LayoutMode, ShellMode
│   ├── keymap.rs   Action enum, KeyBinding (Action | Shell), Keymap with defaults
│   └── theme.rs    Theme, ThemeColors, ThemeSymbols, BorderStyle, Color parsing
├── fs/
│   ├── mod.rs      Re-exports
│   ├── entry.rs    Entry, EntryKind, file metadata, previewability classification
│   ├── ops.rs      copy/move/delete/rename/create (async, conflict resolution)
│   └── watcher.rs  notify-based filesystem watcher (unused in main loop)
└── ui/
    ├── mod.rs      Re-exports
    ├── layout.rs   compute_layout — Single/Dual/Miller + preview area
    ├── pane.rs     Pane struct, list rendering, cursor/selection/filter
    ├── modal.rs    All modals: Confirm, Input, Conflict, Summary, Help, Settings
    ├── preview.rs  Directory listing, text preview, hex dump fallback
    └── statusbar.rs StatusBar with cwd, filter, clipboard, selection count, hints
```

### Data Flow

```
main → App::run → event loop (draw + poll + handle_key)
  ├── Normal mode → keymap lookup → dispatch_action / dispatch_shell
  ├── Search/Filter mode → handle_text_input (live filter)
  └── Modal mode → handle_modal_key (blocks until dismissed)

dispatch_action modifies app state (cursor, selection, clipboard, modal)
dispatch_shell → shell::expand_command + shell::run_takeover / run_capture
File ops (copy/move) use background thread + mpsc channel for non-blocking UI
```

### Key Patterns

- **No nested ifs** — guard clauses throughout
- **Single responsibility** — each module owns its concern
- **Simple over clever** — plain enums, match statements, no macros
- **Async** — tokio current-thread runtime for file operations, `block_on` in event loop
- **Config** — `serde` + `toml`, defaults in code, load from `~/.config/rift/config.toml`
- **Error handling** — `anyhow::Result` everywhere, `thiserror` unused
- **Modal-driven UI** — modal stack blocks key dispatch until resolved

## Build & Run

```sh
cargo build              # debug
cargo build --release    # release (opt-level=3, LTO, stripped)
cargo run                # run from cwd
cargo run /some/path     # start in specific directory
```

## Testing

```sh
cargo test               # all 161 unit tests
cargo test <module>::tests::<test_name>  # single test
```

- **Unit tests** live in `#[cfg(test)] mod tests` at end of each source file
- **File ops tests** use `tempfile::TempDir` + `#[tokio::test]`
- **Entry tests** create real files via `tempfile` (no mocks)
- **Pane tests** construct `Entry` structs directly (no filesystem needed)
- Layout/modal/config tests use pure logic, no I/O

## Conventions

### Code

- **Format**: `rustfmt` (let the user run manually)
- **Naming**: snake_case for fns/vars, CamelCase for types/enums
- **Matches**: exhaustive match on enums, `_ => {}` only for irrelevant variants
- **Imports**: grouped — std → external → crate, one `use` per line
- **No comments** unless explaining why, not what
- **No unwrap** — use `?`, `.ok()`, or `.unwrap_or_else()` in fallible contexts
- **Theme colors**: Tokyo Night palette as defaults, hex `#rrggbb` format

### Git

- Conventional commits: `feat:`, `fix:`, `refactor:`, `chore:`, `docs:`, `style:`
- No force-push to main
- Verify builds before committing

### Config keys

- TOML, `snake_case`
- Keybindings use canonical key strings: `up`, `down`, `ctrl+c`, `f2`, `shift+tab`
- Shell bindings prefixed with `shell:` in config
