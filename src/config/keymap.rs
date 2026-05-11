use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Every built-in action the app understands.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // Navigation
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    PageUp,
    PageDown,
    GotoTop,
    GotoBottom,
    OpenEntry,
    GoParent,

    // Selection
    SelectToggle,
    SelectAll,
    SelectNone,
    InvertSelection,

    // File operations
    Copy,
    Cut,
    Paste,
    Delete,
    Rename,
    NewFile,
    NewDir,

    // View
    ToggleHidden,
    TogglePreview,
    CycleLayout,
    SetLayoutSingle,
    SetLayoutDual,
    SetLayoutMiller,
    Refresh,

    // Search / filter
    Search,
    Filter,
    ClearFilter,

    // App
    OpenConfig,
    OpenShell,
    Quit,
    Help,
}

/// A key binding is either a built-in action or a shell command string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyBinding {
    Action(Action),
    Shell(String),
}

/// Thin wrapper so TOML deserialization works cleanly.
/// In config.toml:
///
/// [keymap]
/// "ctrl+c" = "copy"
/// "ctrl+e" = "shell:$EDITOR {}"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keymap(pub HashMap<String, KeyBinding>);

impl Default for Keymap {
    fn default() -> Self {
        let mut m: HashMap<String, KeyBinding> = HashMap::new();

        // Navigation — modern defaults
        m.insert("up".into(),          KeyBinding::Action(Action::MoveUp));
        m.insert("down".into(),        KeyBinding::Action(Action::MoveDown));
        m.insert("left".into(),        KeyBinding::Action(Action::MoveLeft));
        m.insert("right".into(),       KeyBinding::Action(Action::MoveRight));
        m.insert("k".into(),           KeyBinding::Action(Action::MoveUp));
        m.insert("j".into(),           KeyBinding::Action(Action::MoveDown));
        m.insert("h".into(),           KeyBinding::Action(Action::MoveLeft));
        m.insert("l".into(),           KeyBinding::Action(Action::MoveRight));
        m.insert("enter".into(),       KeyBinding::Action(Action::OpenEntry));
        m.insert("backspace".into(),   KeyBinding::Action(Action::GoParent));
        m.insert("page_up".into(),     KeyBinding::Action(Action::PageUp));
        m.insert("page_down".into(),   KeyBinding::Action(Action::PageDown));
        m.insert("g".into(),           KeyBinding::Action(Action::GotoTop));
        m.insert("G".into(),           KeyBinding::Action(Action::GotoBottom));

        // Selection
        m.insert("space".into(),       KeyBinding::Action(Action::SelectToggle));
        m.insert("ctrl+a".into(),      KeyBinding::Action(Action::SelectAll));
        m.insert("escape".into(),      KeyBinding::Action(Action::SelectNone));

        // File ops — modern shortcuts
        m.insert("ctrl+c".into(),      KeyBinding::Action(Action::Copy));
        m.insert("ctrl+x".into(),      KeyBinding::Action(Action::Cut));
        m.insert("ctrl+v".into(),      KeyBinding::Action(Action::Paste));
        m.insert("delete".into(),      KeyBinding::Action(Action::Delete));
        m.insert("f2".into(),          KeyBinding::Action(Action::Rename));
        m.insert("ctrl+n".into(),      KeyBinding::Action(Action::NewFile));
        m.insert("ctrl+shift+n".into(),KeyBinding::Action(Action::NewDir));

        // View
        m.insert("ctrl+h".into(),      KeyBinding::Action(Action::ToggleHidden));
        m.insert("ctrl+p".into(),      KeyBinding::Action(Action::TogglePreview));
        m.insert("tab".into(),         KeyBinding::Action(Action::CycleLayout));
        m.insert("r".into(),           KeyBinding::Action(Action::Refresh));

        // Search
        m.insert("/".into(),           KeyBinding::Action(Action::Search));
        m.insert("f".into(),           KeyBinding::Action(Action::Filter));

        // Shell passthrough example (users can override/add their own)
        m.insert("ctrl+e".into(),      KeyBinding::Shell("$EDITOR {}".into()));
        m.insert("ctrl+t".into(),      KeyBinding::Shell("$SHELL".into()));

        // App
        m.insert("?".into(),           KeyBinding::Action(Action::Help));
        m.insert("q".into(),           KeyBinding::Action(Action::Quit));
        m.insert("ctrl+q".into(),      KeyBinding::Action(Action::Quit));

        Self(m)
    }
}

impl Keymap {
    pub fn get(&self, key: &str) -> Option<&KeyBinding> {
        self.0.get(key)
    }
}
