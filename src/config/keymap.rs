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
    OpenSettings,
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
        m.insert("ctrl+.".into(),      KeyBinding::Action(Action::OpenSettings));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn all_actions() -> Vec<Action> {
        use Action::*;
        vec![
            MoveUp, MoveDown, MoveLeft, MoveRight,
            PageUp, PageDown, GotoTop, GotoBottom,
            OpenEntry, GoParent,
            SelectToggle, SelectAll, SelectNone,
            Copy, Cut, Paste, Delete, Rename, NewFile, NewDir,
            ToggleHidden, TogglePreview, CycleLayout, Refresh,
            Search, Filter, OpenSettings, Quit, Help,
        ]
    }

    #[test]
    fn default_keymap_has_all_actions_bound() {
        let km = Keymap::default();
        for action in all_actions() {
            let has = km.0.values().any(|v| matches!(v, KeyBinding::Action(a) if *a == action));
            assert!(has, "Action {:?} is not bound in default keymap", action);
        }
    }

    #[test]
    fn keymap_get_returns_some_for_bound_key() {
        let km = Keymap::default();
        assert!(km.get("j").is_some());
        assert!(km.get("k").is_some());
        assert!(km.get("enter").is_some());
        assert!(km.get("ctrl+c").is_some());
        assert!(km.get("f2").is_some());
    }

    #[test]
    fn keymap_get_returns_none_for_unbound_key() {
        let km = Keymap::default();
        assert!(km.get("ctrl+z").is_none());
        assert!(km.get("x").is_none());
    }

    #[test]
    fn keymap_has_shell_bindings() {
        let km = Keymap::default();
        let shell_count: usize = km.0.values()
            .filter(|v| matches!(v, KeyBinding::Shell(_)))
            .count();
        assert!(shell_count > 0, "Expected at least one shell binding");
        assert!(matches!(km.get("ctrl+e"), Some(KeyBinding::Shell(_))));
        assert!(matches!(km.get("ctrl+t"), Some(KeyBinding::Shell(_))));
    }

    #[test]
    fn keymap_get_returns_correct_action() {
        let km = Keymap::default();
        assert!(matches!(km.get("j"), Some(KeyBinding::Action(Action::MoveDown))));
        assert!(matches!(km.get("k"), Some(KeyBinding::Action(Action::MoveUp))));
        assert!(matches!(km.get("q"), Some(KeyBinding::Action(Action::Quit))));
        assert!(matches!(km.get("enter"), Some(KeyBinding::Action(Action::OpenEntry))));
    }

    #[test]
    fn keymap_action_count() {
        let km = Keymap::default();
        let count: usize = km.0.values()
            .filter(|v| matches!(v, KeyBinding::Action(_)))
            .count();
        assert!(count >= 27, "Expected at least 27 action bindings, got {}", count);
    }
}
