use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use zeta::action::Action;

// Test 1: Tab key creates terminal input with 0x09 byte
#[test]
fn tab_key_creates_terminal_input_action() {
    let key_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![b'\t'])));
    // Verify the byte value is 0x09
    assert_eq!(action, Some(Action::TerminalInput(vec![0x09])));
}

// Test 2: Printable character creates terminal input
#[test]
fn printable_char_creates_terminal_input() {
    let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![b'a'])));
}

// Test 3: Uppercase character creates terminal input
#[test]
fn uppercase_char_creates_terminal_input() {
    let key_event = KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![b'Z'])));
}

// Test 4: Numeric character creates terminal input
#[test]
fn numeric_char_creates_terminal_input() {
    let key_event = KeyEvent::new(KeyCode::Char('5'), KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![b'5'])));
}

// Test 5: Enter key creates platform-specific line ending
#[test]
#[cfg(windows)]
fn enter_key_creates_crlf_on_windows() {
    let key_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![b'\r', b'\n'])));
}

#[test]
#[cfg(not(windows))]
fn enter_key_creates_cr_on_unix() {
    let key_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![b'\r'])));
}

// Test 6: Ctrl+A creates ASCII control byte 1
#[test]
fn ctrl_a_creates_control_byte() {
    let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![1])));
}

// Test 7: Ctrl+Z creates ASCII control byte 26
#[test]
fn ctrl_z_creates_control_byte() {
    let key_event = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL);
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![26])));
}

// Test 8: Ctrl+uppercase letter also creates control byte
#[test]
fn ctrl_uppercase_creates_control_byte() {
    let key_event = KeyEvent::new(KeyCode::Char('B'), KeyModifiers::CONTROL);
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![2])));
}

// Test 9: Backspace creates DEL byte (ASCII 127)
#[test]
fn backspace_creates_del_byte() {
    let key_event = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![127])));
}

// Test 10: Escape creates ESC byte (ASCII 27)
#[test]
fn escape_creates_esc_byte() {
    let key_event = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![27])));
}

// Test 11: Up arrow creates ANSI escape sequence
#[test]
fn up_arrow_creates_ansi_sequence() {
    let key_event = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![27, b'[', b'A'])));
}

// Test 12: Down arrow creates ANSI escape sequence
#[test]
fn down_arrow_creates_ansi_sequence() {
    let key_event = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![27, b'[', b'B'])));
}

// Test 13: Right arrow creates ANSI escape sequence
#[test]
fn right_arrow_creates_ansi_sequence() {
    let key_event = KeyEvent::new(KeyCode::Right, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![27, b'[', b'C'])));
}

// Test 14: Left arrow creates ANSI escape sequence
#[test]
fn left_arrow_creates_ansi_sequence() {
    let key_event = KeyEvent::new(KeyCode::Left, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![27, b'[', b'D'])));
}

// Test 15: F11 toggles terminal fullscreen
#[test]
fn f11_toggles_terminal_fullscreen() {
    let key_event = KeyEvent::new(KeyCode::F(11), KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::ToggleTerminalFullscreen));
}

// Test 16: F2 toggles terminal panel
#[test]
fn f2_toggles_terminal_panel() {
    let key_event = KeyEvent::new(KeyCode::F(2), KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::ToggleTerminal));
}

// Test 17: Ctrl+Backslash toggles terminal panel
#[test]
fn ctrl_backslash_toggles_terminal_panel() {
    let key_event = KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::CONTROL);
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::ToggleTerminal));
}

// Test 18: Ctrl+[ creates ESC (ASCII 27)
#[test]
fn ctrl_bracket_creates_esc() {
    let key_event = KeyEvent::new(KeyCode::Char('['), KeyModifiers::CONTROL);
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![27])));
}

// Test 19: Ctrl+] creates ASCII 29
#[test]
fn ctrl_right_bracket_creates_ascii_29() {
    let key_event = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::CONTROL);
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![29])));
}

// Test 20: Ctrl+^ creates ASCII 30
#[test]
fn ctrl_caret_creates_ascii_30() {
    let key_event = KeyEvent::new(KeyCode::Char('^'), KeyModifiers::CONTROL);
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![30])));
}

// Test 21: Ctrl+_ creates ASCII 31
#[test]
fn ctrl_underscore_creates_ascii_31() {
    let key_event = KeyEvent::new(KeyCode::Char('_'), KeyModifiers::CONTROL);
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![31])));
}

// Test 22: Unsupported key F1 returns None
#[test]
fn f1_unsupported_returns_none() {
    let key_event = KeyEvent::new(KeyCode::F(1), KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, None);
}

// Test 23: Unsupported key Home returns None
#[test]
fn home_unsupported_returns_none() {
    let key_event = KeyEvent::new(KeyCode::Home, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, None);
}

// Test 24: Unsupported key End returns None
#[test]
fn end_unsupported_returns_none() {
    let key_event = KeyEvent::new(KeyCode::End, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, None);
}

// Test 25: Unsupported key PageUp returns None
#[test]
fn page_up_unsupported_returns_none() {
    let key_event = KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, None);
}

// Test 26: Special character space creates terminal input
#[test]
fn space_char_creates_terminal_input() {
    let key_event = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![b' '])));
}

// Test 27: Special character hyphen creates terminal input
#[test]
fn hyphen_char_creates_terminal_input() {
    let key_event = KeyEvent::new(KeyCode::Char('-'), KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    assert_eq!(action, Some(Action::TerminalInput(vec![b'-'])));
}

// Test 28: Verify arrow keys use ESC (27) as first byte
#[test]
fn arrow_keys_start_with_esc_byte() {
    let key_event = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    if let Some(Action::TerminalInput(bytes)) = action {
        assert!(!bytes.is_empty());
        assert_eq!(bytes[0], 27, "First byte should be ESC");
    } else {
        panic!("Expected TerminalInput action");
    }
}

// Test 29: Verify arrow keys use '[' as second byte
#[test]
fn arrow_keys_have_bracket_as_second_byte() {
    let key_event = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    if let Some(Action::TerminalInput(bytes)) = action {
        assert!(bytes.len() >= 2);
        assert_eq!(bytes[1], b'[', "Second byte should be '['");
    } else {
        panic!("Expected TerminalInput action");
    }
}

// Test 30: Verify Backspace is exactly ASCII 127
#[test]
fn backspace_is_exactly_127() {
    let key_event = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
    let action = Action::from_terminal_key_event(key_event);
    if let Some(Action::TerminalInput(bytes)) = action {
        assert_eq!(bytes.len(), 1, "Backspace should be single byte");
        assert_eq!(bytes[0], 127, "Backspace should be ASCII 127");
    } else {
        panic!("Expected TerminalInput action");
    }
}
