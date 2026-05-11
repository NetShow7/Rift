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
