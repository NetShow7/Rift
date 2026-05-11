use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Normalise a crossterm KeyEvent into a canonical string like
/// "ctrl+c", "shift+f2", "enter", "backspace", "a", "A", etc.
pub fn key_to_string(event: &KeyEvent) -> String {
    let mut parts = Vec::new();

    if event.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("ctrl".to_string());
    }
    if event.modifiers.contains(KeyModifiers::ALT) {
        parts.push("alt".to_string());
    }
    if event.modifiers.contains(KeyModifiers::SHIFT) {
        // Only add explicit shift for non-character keys — for chars
        // we rely on the uppercase letter itself.
        match event.code {
            KeyCode::Char(_) => {}
            _ => parts.push("shift".to_string()),
        }
    }

    let key_name = match event.code {
        KeyCode::Char(' ') => "space".to_string(),
        KeyCode::Char(c)   => c.to_string(),
        KeyCode::Enter     => "enter".to_string(),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Delete    => "delete".to_string(),
        KeyCode::Esc       => "escape".to_string(),
        KeyCode::Tab       => "tab".to_string(),
        KeyCode::BackTab   => "shift+tab".to_string(),
        KeyCode::Up        => "up".to_string(),
        KeyCode::Down      => "down".to_string(),
        KeyCode::Left      => "left".to_string(),
        KeyCode::Right     => "right".to_string(),
        KeyCode::PageUp    => "page_up".to_string(),
        KeyCode::PageDown  => "page_down".to_string(),
        KeyCode::Home      => "home".to_string(),
        KeyCode::End       => "end".to_string(),
        KeyCode::Insert    => "insert".to_string(),
        KeyCode::F(n)      => format!("f{}", n),
        _                  => return String::new(),
    };

    parts.push(key_name);
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn simple_char() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)), "a");
    }

    #[test]
    fn uppercase_char() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE)), "A");
    }

    #[test]
    fn ctrl_c() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)), "ctrl+c");
    }

    #[test]
    fn alt_x() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT)), "alt+x");
    }

    #[test]
    fn ctrl_alt_delete() {
        let e = KeyEvent::new(KeyCode::Delete, KeyModifiers::CONTROL | KeyModifiers::ALT);
        assert_eq!(key_to_string(&e), "ctrl+alt+delete");
    }

    #[test]
    fn space() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)), "space");
    }

    #[test]
    fn enter() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)), "enter");
    }

    #[test]
    fn backspace() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)), "backspace");
    }

    #[test]
    fn escape() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)), "escape");
    }

    #[test]
    fn tab() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)), "tab");
    }

    #[test]
    fn shift_tab() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE)), "shift+tab");
    }

    #[test]
    fn arrow_keys() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)), "up");
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)), "down");
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)), "left");
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)), "right");
    }

    #[test]
    fn function_keys() {
        for n in 1..=12 {
            assert_eq!(key_to_string(&KeyEvent::new(KeyCode::F(n), KeyModifiers::NONE)), format!("f{}", n));
        }
    }

    #[test]
    fn shift_function_key() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::F(1), KeyModifiers::SHIFT)), "shift+f1");
    }

    #[test]
    fn ctrl_function_key() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::F(5), KeyModifiers::CONTROL)), "ctrl+f5");
    }

    #[test]
    fn page_keys() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)), "page_up");
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)), "page_down");
    }

    #[test]
    fn home_end_insert() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)), "home");
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::End, KeyModifiers::NONE)), "end");
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE)), "insert");
    }

    #[test]
    fn delete_key() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)), "delete");
    }

    #[test]
    fn unknown_key_returns_empty() {
        assert_eq!(key_to_string(&KeyEvent::new(KeyCode::Null, KeyModifiers::NONE)), "");
    }
}
