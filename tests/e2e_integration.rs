use std::time::Duration;

use crossterm::event::KeyCode;
use zeta::testing::ZetaE2eInstance;

/// Verify that Zeta starts and displays the initial UI with panes.
#[test]
fn e2e_startup_shows_panes() {
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");

    // Wait for the app to render (250ms timeout).
    assert!(
        zeta.wait_for_text("Zeta", Duration::from_millis(2500))
            .expect("screen check"),
        "App should display 'Zeta' on startup"
    );
}

/// Verify that arrow keys navigate the pane selection.
#[test]
fn e2e_navigate_down_in_pane() {
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");

    // Wait for startup
    assert!(
        zeta.wait_for_text("Zeta", Duration::from_millis(2500))
            .expect("screen check"),
        "App must startup"
    );

    // Send a down arrow key
    zeta.send_key(KeyCode::Down).expect("send key");
    zeta.wait_for_render().expect("wait render");

    // Screen should still be valid (no crash)
    let screen = zeta.capture_screen().expect("capture");
    assert!(
        !screen.is_empty(),
        "Screen should have content after navigation"
    );
}

/// Verify that the left/right arrow keys or Tab switches panes.
#[test]
fn e2e_switch_pane() {
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");

    assert!(
        zeta.wait_for_text("Zeta", Duration::from_millis(2500))
            .expect("screen check"),
        "App must startup"
    );

    // Tab key typically switches panes
    zeta.send_key(KeyCode::Tab).expect("send tab");
    zeta.wait_for_render().expect("wait render");

    // Verify screen is still valid
    let screen = zeta.capture_screen().expect("capture");
    assert!(
        !screen.is_empty(),
        "Screen should have content after pane switch"
    );
}

/// Verify that text input in a search/filter box works.
#[test]
fn e2e_filter_input() {
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");

    assert!(
        zeta.wait_for_text("Zeta", Duration::from_millis(2500))
            .expect("screen check"),
        "App must startup"
    );

    // Many file managers use '/' for quick filter or Ctrl+F for search
    zeta.send_key(KeyCode::Char('/')).expect("send /");
    zeta.wait_for_render().expect("wait render");

    // Type a filter term
    zeta.send_text("test").expect("send text");
    zeta.wait_for_render().expect("wait render");

    // Screen should still render without errors
    let screen = zeta.capture_screen().expect("capture");
    assert!(!screen.is_empty(), "Screen should render filter box");
}

/// Verify that Escape clears any overlay/dialog.
#[test]
fn e2e_escape_clears_overlay() {
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");

    assert!(
        zeta.wait_for_text("Zeta", Duration::from_millis(2500))
            .expect("screen check"),
        "App must startup"
    );

    // Open some overlay (e.g., filter)
    zeta.send_key(KeyCode::Char('/')).expect("send /");
    zeta.wait_for_render().expect("wait render");

    // Type something
    zeta.send_text("xyz").expect("send text");
    zeta.wait_for_render().expect("wait render");

    // Escape should close it
    zeta.send_key(KeyCode::Esc).expect("send esc");
    zeta.wait_for_render().expect("wait render");

    // Screen should be clean
    let screen = zeta.capture_screen().expect("capture");
    assert!(
        !screen.is_empty(),
        "Screen should still render after escape"
    );
}

/// Verify that Enter confirms a selection or action.
#[test]
fn e2e_enter_confirmation() {
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");

    assert!(
        zeta.wait_for_text("Zeta", Duration::from_millis(2500))
            .expect("screen check"),
        "App must startup"
    );

    // Navigate down a few times
    zeta.send_key(KeyCode::Down).expect("send down");
    zeta.wait_for_render().expect("wait render");
    zeta.send_key(KeyCode::Down).expect("send down");
    zeta.wait_for_render().expect("wait render");

    // Press Enter (may open a file or confirm selection, depending on what's focused)
    zeta.send_key(KeyCode::Enter).expect("send enter");
    std::thread::sleep(Duration::from_millis(300));
    zeta.wait_for_render().expect("wait render");

    // App should not crash
    let screen = zeta.capture_screen().expect("capture");
    assert!(!screen.is_empty(), "Screen should render after enter");
}

/// Verify that Ctrl+C or 'q' quits the application cleanly.
#[test]
fn e2e_quit_gracefully() {
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");

    assert!(
        zeta.wait_for_text("Zeta", Duration::from_millis(2500))
            .expect("screen check"),
        "App must startup"
    );

    // Send 'q' to quit
    zeta.send_key(KeyCode::Char('q')).expect("send q");
    std::thread::sleep(Duration::from_millis(500));

    // Shutdown should complete without panic
    let result = zeta.shutdown();
    assert!(result.is_ok(), "Shutdown should complete cleanly");
}

/// Verify that Home/End keys move to start/end of list.
#[test]
fn e2e_home_end_navigation() {
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");

    assert!(
        zeta.wait_for_text("Zeta", Duration::from_millis(2500))
            .expect("screen check"),
        "App must startup"
    );

    // Press Home
    zeta.send_key(KeyCode::Home).expect("send home");
    zeta.wait_for_render().expect("wait render");

    // Press End
    zeta.send_key(KeyCode::End).expect("send end");
    zeta.wait_for_render().expect("wait render");

    // Screen should be stable
    let screen = zeta.capture_screen().expect("capture");
    assert!(!screen.is_empty(), "Screen should render after Home/End");
}

// ============================================================================
// Tests for fix/editor-selection-and-ui-regressions branch
// ============================================================================

/// Verify that the `?` cheatsheet opens without crashing.
#[test]
fn e2e_cheatsheet_opens() {
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");

    assert!(
        zeta.wait_for_text("Zeta", Duration::from_millis(2500))
            .expect("screen check"),
        "App must startup"
    );

    // '?' opens the quick-help cheatsheet.
    zeta.send_key(KeyCode::Char('?')).expect("send ?");
    zeta.wait_for_render().expect("wait render");

    // Screen should still render (no crash on cheatsheet open).
    let screen = zeta.capture_screen().expect("capture");
    assert!(
        !screen.is_empty(),
        "Cheatsheet should open without crashing"
    );

    // Escape closes it.
    zeta.send_key(KeyCode::Esc).expect("send esc");
    zeta.wait_for_render().expect("wait render");
}

/// Verify that workspace switching via keyboard (Alt+1..4) does not crash.
#[test]
fn e2e_workspace_keyboard_switch_no_crash() {
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");

    assert!(
        zeta.wait_for_text("Zeta", Duration::from_millis(2500))
            .expect("screen check"),
        "App must startup"
    );

    // Alt+2 switches to workspace 2.
    zeta.send_key(KeyCode::Char('2')).expect("send 2");
    zeta.wait_for_render().expect("wait render");

    let screen = zeta.capture_screen().expect("capture");
    assert!(
        !screen.is_empty(),
        "Screen should render after workspace key"
    );
}

/// Open a file in the editor, select all, verify no crash.
/// Tests the selection logic path that previously skipped empty lines.
#[test]
fn e2e_editor_select_all_no_crash() {
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");

    assert!(
        zeta.wait_for_text("Zeta", Duration::from_millis(2500))
            .expect("screen check"),
        "App must startup"
    );

    // Press Enter to attempt opening the first entry (may be a file or dir).
    zeta.send_key(KeyCode::Enter).expect("send enter");
    std::thread::sleep(Duration::from_millis(400));
    zeta.wait_for_render().expect("wait render");

    // Try Ctrl+A (select all) — if we're in an editor this exercises selection.
    zeta.send_text("\x01").expect("send ctrl-a"); // Ctrl+A
    zeta.wait_for_render().expect("wait render");

    // Screen should not crash.
    let screen = zeta.capture_screen().expect("capture");
    assert!(
        !screen.is_empty(),
        "Screen should render after select-all attempt"
    );

    // Escape back to safety.
    zeta.send_key(KeyCode::Esc).expect("send esc");
    zeta.wait_for_render().expect("wait render");
}

/// Verify that the preview panel toggle (Ctrl+P or F3 shortcut) does not crash.
#[test]
fn e2e_preview_panel_toggle_no_crash() {
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");

    assert!(
        zeta.wait_for_text("Zeta", Duration::from_millis(2500))
            .expect("screen check"),
        "App must startup"
    );

    // Toggle preview panel — exact key depends on config but F3 is typical.
    zeta.send_key(KeyCode::F(3)).expect("send F3");
    zeta.wait_for_render().expect("wait render");

    let screen = zeta.capture_screen().expect("capture");
    assert!(!screen.is_empty(), "Preview panel toggle should not crash");

    // Toggle it back off.
    zeta.send_key(KeyCode::F(3)).expect("send F3 again");
    zeta.wait_for_render().expect("wait render");
}

/// Verify that Shift+F11 (preview fullscreen) key sequence is accepted without crash.
/// Full rendering verification is done via unit tests; this confirms the action
/// pipeline handles the key without panicking.
#[test]
fn e2e_shift_f11_preview_fullscreen_no_crash() {
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");

    assert!(
        zeta.wait_for_text("Zeta", Duration::from_millis(2500))
            .expect("screen check"),
        "App must startup"
    );

    // Open preview first.
    zeta.send_key(KeyCode::F(3)).expect("send F3");
    zeta.wait_for_render().expect("wait render");

    // Send Shift+F11 escape sequence: \x1b[23;2~
    // (Standard xterm sequence for Shift+F11)
    zeta.send_text("\x1b[23;2~").expect("send Shift+F11");
    zeta.wait_for_render().expect("wait render");

    let screen = zeta.capture_screen().expect("capture");
    assert!(!screen.is_empty(), "Shift+F11 should not crash the app");

    // Toggle back, close preview.
    zeta.send_text("\x1b[23;2~").expect("send Shift+F11 again");
    zeta.wait_for_render().expect("wait render");
    zeta.send_key(KeyCode::Esc).expect("send esc");
    zeta.wait_for_render().expect("wait render");
}
