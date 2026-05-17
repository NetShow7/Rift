[![Rust](https://github.com/NetShow7/Rift/actions/workflows/rust.yml/badge.svg)](https://github.com/NetShow7/Rift/actions/workflows/rust.yml)

# Rift

Modern terminal file manager built with Rust (ratatui + crossterm).

Miller columns, Dual pane, Single pane layouts. Fully configurable keybindings, themes (Tokyo Night default), shell integration, and async file operations.

## Features

- **Three layout modes**: Single pane, Dual pane, Miller columns
- **Configurable keybindings**: TOML file, bind actions or shell commands to any key
- **Theming**: Tokyo Night default, customizable colors and symbols (Nerd Font required for icons)
- **Shell integration**: Run commands inline (capture mode) or take over the terminal (takeover mode)
- **File operations**: Copy, cut, paste, delete, rename, create files/directories -- async with conflict resolution
- **File preview**: Directory listing, text preview with syntax highlighting (syntect), hex dump fallback
- **Search / filter**: Live filename filtering and search
- **Sidebar**: Quick navigation with favorites
- **Clipboard**: Multi-file cut/copy operations

## Installation

### From source

Requires Rust 1.75+.

```sh
cargo install --git https://github.com/netshow/rift
```

### Build locally

```sh
git clone https://github.com/netshow/rift.git
cd rift
cargo build --release
```

The binary is `target/release/rift`.

## Usage

```sh
# Start in current directory
rift

# Start in a specific directory
rift /some/path

# Default keybindings
#   h/j/k/l or arrows   navigate
#   enter                open file/directory
#   backspace            go to parent
#   /                    search
#   f                    filter
#   tab                  cycle layout (single -> dual -> miller)
#   q                    quit
#   ?                    help
```

## Configuration

Config is loaded from `~/.config/rift/config.toml`. See `config/config.toml` in the repository for a
complete example with defaults.

### General options

```toml
[general]
layout = "miller"          # single | dual | miller
show_hidden = false
follow_symlinks = true
scroll_threshold = 500
shell = "/bin/bash"
shell_mode = "capture"     # capture | takeover
confirm_delete = true
show_shortcut_hints = true
# trash_dir = "~/.local/share/rift/trash"
```

### Colors

All colors are configurable via `[theme.colors]` using `#rrggbb` hex or named ANSI values:

```toml
[theme.colors]
foreground      = "#c0caf5"
selection_bg    = "#364a82"
directory       = "#7aa2f7"
symlink         = "#b4f9f8"
executable      = "#9ece6a"
error           = "#f7768e"
```

### Symbols

Icons require a Nerd Font. Plain ASCII fallback is available in the default config.

```toml
[theme.symbols]
dir_open     = " "
selected     = "󰄬 "
```

### Keybindings

Keys are mapped to built-in actions or shell commands:

```toml
[keymap]
"up"        = "move_up"
"down"      = "move_down"
"ctrl+c"    = "copy"
"ctrl+v"    = "paste"
"ctrl+e"    = "shell:$EDITOR {}"   # {} is replaced with focused file path
"ctrl+t"    = "shell:$SHELL"
```

### Available actions

| Category | Actions |
|---|---|
| Navigation | `move_up`, `move_down`, `move_left`, `move_right`, `page_up`, `page_down`, `goto_top`, `goto_bottom`, `open_entry`, `go_parent`, `switch_pane` |
| Selection | `select_toggle`, `select_all`, `select_none`, `invert_selection` |
| File ops | `copy`, `cut`, `paste`, `delete`, `rename`, `new_file`, `new_dir` |
| View | `toggle_hidden`, `toggle_preview`, `cycle_layout`, `set_layout_single/dual/miller`, `refresh`, `toggle_sidebar` |
| Search | `search`, `filter`, `clear_filter` |
| App | `open_config`, `open_editor`, `open_shell`, `open_settings`, `quit`, `help` |

## Layout modes

| Mode | Description |
|---|---|
| Single | One pane fills the terminal. |
| Dual | Two side-by-side panes. Switch focus with the key bound to `switch_pane`. |
| Miller | Parent-child directory hierarchy displayed left to right. Navigate right to drill in, left to go up. |

## Building

```sh
cargo build              # debug
cargo build --release    # release (opt-level=3, LTO, stripped)
```

## Testing

```sh
cargo test               # run all unit tests
cargo test -- --nocapture  # with stdout
```

## Architecture

```
src/
├── main.rs         Entry, terminal setup, panic guard
├── app.rs          App struct, event loop, key dispatch, file ops
├── input.rs        Key event to canonical string normalization
├── shell.rs        Shell command expansion, capture + takeover modes
├── config/
│   ├── mod.rs      Config loading/saving (TOML), layout/shell modes
│   ├── keymap.rs   Action enum, KeyBinding, default keymap
│   └── theme.rs    Theme, colors, symbols, border styles
├── fs/
│   ├── mod.rs      Re-exports
│   ├── entry.rs    File metadata, entry classification
│   ├── ops.rs      Copy/move/delete/rename/create (async)
│   └── watcher.rs  Filesystem change watcher
└── ui/
    ├── mod.rs      Re-exports
    ├── layout.rs   Compute layout by mode + preview
    ├── pane.rs     List rendering, cursor, selection, filter
    ├── modal.rs    Confirm, input, conflict, help, settings modals
    ├── preview.rs  Text/hex/directory previews
    └── statusbar.rs Status bar with cwd, filter, clipboard, hints
```
