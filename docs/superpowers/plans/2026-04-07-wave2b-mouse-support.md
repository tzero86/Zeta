# Wave 2B — Full Mouse Support

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable full mouse interaction — click to focus panes, click menu bar items to open menus, scroll wheel to navigate pane entries and scroll the preview/editor, and click overlay buttons for collision/dialog resolution.

**Architecture:**
- `TerminalSession::enter()` emits `EnableMouseCapture`; `Drop` emits `DisableMouseCapture`.
- `AppEvent` gains a `Mouse(crossterm::event::MouseEvent)` variant.
- `next_event()` in `app.rs` handles `Event::Mouse(…)` alongside `Event::Key(…)`.
- A new `route_mouse_event()` function in `app.rs` inspects the `LayoutCache` (populated by Wave 1B's `render()` return value) and the current `FocusLayer` to translate `MouseEvent` into `Action`.
- `handle_event()` dispatches the resulting action through the existing `dispatch()` path — no new state machinery required.
- Scroll events in the pane produce `MoveSelectionUp/Down`; scroll in the preview produces `ScrollPreviewUp/Down`; scroll in the editor produces `EditorMoveUp/Down`.
- Left-click on a pane focuses it (`FocusNextPane` / pane-specific focus action). Left-click on the menu bar opens the corresponding menu.

**Tech Stack:** crossterm 0.28 (`EnableMouseCapture`, `MouseEvent`, `MouseEventKind`, `MouseButton`), ratatui 0.29 `Rect`, existing `LayoutCache` from Wave 1B, `FocusLayer` from Wave 2A.

**Jira:** ZTA-85, ZTA-86 (ZTA-116 through ZTA-121)

**Wave dependency:** Starts AFTER Wave 2A is merged. Requires `LayoutCache` on `App` (Wave 1B) and `AppState::focus_layer()` (Wave 2A).

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/event.rs` | Add `AppEvent::Mouse` variant |
| Modify | `src/app.rs` | Enable mouse capture; handle mouse events; `route_mouse_event()` |

---

## Task 1: Enable mouse capture in TerminalSession

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1.1: Write the failing test**

Add to `src/app.rs` test module:

```rust
#[test]
fn mouse_event_variant_exists_in_app_event() {
    use crossterm::event::{MouseEvent, MouseEventKind, MouseButton, KeyModifiers};
    let ev = crossterm::event::MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 5,
        row: 3,
        modifiers: KeyModifiers::NONE,
    };
    let app_event = crate::event::AppEvent::Mouse(ev);
    assert!(matches!(app_event, crate::event::AppEvent::Mouse(_)));
}
```

- [ ] **Step 1.2: Confirm it fails**

```bash
cargo test mouse_event_variant_exists 2>&1 | head -5
```

Expected: compile error — `AppEvent::Mouse` doesn't exist yet.

- [ ] **Step 1.3: Update `src/event.rs`**

```rust
use crossterm::event::{KeyEvent, MouseEvent};

use crate::jobs::JobResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppEvent {
    Input(KeyEvent),
    Mouse(MouseEvent),
    Resize { width: u16, height: u16 },
    Job(JobResult),
}
```

- [ ] **Step 1.4: Update `src/app.rs` — enable mouse capture**

Add the crossterm imports for mouse:

```rust
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseEventKind,
};
```

In `TerminalSession::enter()`, add mouse capture after entering alternate screen:

```rust
fn enter() -> Result<Self> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("failed to enter alternate screen and enable mouse")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to create terminal backend")?;
    terminal.clear().context("failed to clear terminal")?;
    Ok(Self { terminal })
}
```

In `TerminalSession`'s `Drop` impl, add `DisableMouseCapture`:

```rust
impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}
```

- [ ] **Step 1.5: Run the test**

```bash
cargo test mouse_event_variant_exists
```

Expected: passes.

- [ ] **Step 1.6: Commit**

```bash
git add src/event.rs src/app.rs
git commit -m "feat(app): enable mouse capture; add AppEvent::Mouse variant"
```

---

## Task 2: Handle mouse events in next_event()

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 2.1: Write the failing test**

Add to `src/app.rs` test module:

```rust
#[test]
fn route_mouse_left_click_on_left_pane_focuses_left() {
    use crate::state::FocusLayer;
    use crate::ui::layout_cache::LayoutCache;
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind, KeyModifiers};
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        left_pane: Rect { x: 0, y: 1, width: 40, height: 20 },
        right_pane: Rect { x: 40, y: 1, width: 40, height: 20 },
        menu_bar: Rect { x: 0, y: 0, width: 80, height: 1 },
        tools_panel: None,
        status_bar: Rect { x: 0, y: 21, width: 80, height: 1 },
    };
    let mouse = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 5,
        modifiers: KeyModifiers::NONE,
    };
    let action = route_mouse_event(mouse, &cache, FocusLayer::Pane);
    // A click in the left pane should produce some focus-related action.
    assert!(
        action.is_some(),
        "expected an action for left-pane click, got None"
    );
}

#[test]
fn route_mouse_scroll_up_in_pane_produces_move_selection_up() {
    use crate::state::FocusLayer;
    use crate::ui::layout_cache::LayoutCache;
    use crossterm::event::{MouseEvent, MouseEventKind, KeyModifiers};
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        left_pane: Rect { x: 0, y: 1, width: 40, height: 20 },
        right_pane: Rect { x: 40, y: 1, width: 40, height: 20 },
        menu_bar: Rect { x: 0, y: 0, width: 80, height: 1 },
        tools_panel: None,
        status_bar: Rect { x: 0, y: 21, width: 80, height: 1 },
    };
    let mouse = MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 10,
        row: 5,
        modifiers: KeyModifiers::NONE,
    };
    let action = route_mouse_event(mouse, &cache, FocusLayer::Pane);
    assert_eq!(action, Some(Action::MoveSelectionUp));
}
```

- [ ] **Step 2.2: Confirm they fail**

```bash
cargo test route_mouse_left_click route_mouse_scroll 2>&1 | head -5
```

Expected: compile errors — `route_mouse_event` not yet defined.

- [ ] **Step 2.3: Add `route_mouse_event` to `src/app.rs`**

```rust
use crate::ui::layout_cache::{rect_contains, LayoutCache};

/// Translate a raw mouse event into an `Action` using the last-rendered
/// `LayoutCache` for hit-testing. Returns `None` for unhandled events.
fn route_mouse_event(
    event: crossterm::event::MouseEvent,
    cache: &LayoutCache,
    focus: FocusLayer,
) -> Option<Action> {
    use crossterm::event::{MouseButton, MouseEventKind};

    let col = event.column;
    let row = event.row;

    match event.kind {
        // -----------------------------------------------------------------------
        // Scroll wheel
        // -----------------------------------------------------------------------
        MouseEventKind::ScrollUp => {
            if focus == FocusLayer::Preview
                || cache.tools_panel.map_or(false, |r| rect_contains(r, col, row))
            {
                return Some(Action::ScrollPreviewUp);
            }
            if matches!(focus, FocusLayer::Editor) {
                return Some(Action::EditorMoveUp);
            }
            // Scroll in either pane → move selection.
            if rect_contains(cache.left_pane, col, row)
                || rect_contains(cache.right_pane, col, row)
            {
                return Some(Action::MoveSelectionUp);
            }
            None
        }
        MouseEventKind::ScrollDown => {
            if focus == FocusLayer::Preview
                || cache.tools_panel.map_or(false, |r| rect_contains(r, col, row))
            {
                return Some(Action::ScrollPreviewDown);
            }
            if matches!(focus, FocusLayer::Editor) {
                return Some(Action::EditorMoveDown);
            }
            if rect_contains(cache.left_pane, col, row)
                || rect_contains(cache.right_pane, col, row)
            {
                return Some(Action::MoveSelectionDown);
            }
            None
        }

        // -----------------------------------------------------------------------
        // Left click
        // -----------------------------------------------------------------------
        MouseEventKind::Down(MouseButton::Left) => {
            // Modals absorb all clicks — don't route through layout.
            if matches!(focus, FocusLayer::Modal(_)) {
                return None;
            }

            // Click on menu bar item.
            if rect_contains(cache.menu_bar, col, row) {
                return route_menu_bar_click(col, cache.menu_bar.x);
            }

            // Click on the tools panel (editor or preview).
            if let Some(tools_rect) = cache.tools_panel {
                if rect_contains(tools_rect, col, row) {
                    // If preview is in the tools panel, focus it.
                    if focus != FocusLayer::Editor {
                        return Some(Action::FocusPreviewPanel);
                    }
                    return None; // editor already focused
                }
            }

            // Click on left pane.
            if rect_contains(cache.left_pane, col, row) {
                if focus == FocusLayer::Editor || focus == FocusLayer::Preview {
                    return Some(Action::CycleFocus); // leave tools panel first
                }
                return Some(Action::FocusNextPane); // switch to left if right was focused
            }

            // Click on right pane.
            if rect_contains(cache.right_pane, col, row) {
                if focus == FocusLayer::Editor || focus == FocusLayer::Preview {
                    return Some(Action::CycleFocus);
                }
                return Some(Action::FocusNextPane);
            }

            None
        }

        // Other mouse events (drag, release, move) — ignored for now.
        _ => None,
    }
}

/// Map an x-coordinate in the menu bar row to the appropriate `OpenMenu` action.
/// Offsets match the hardcoded menu item positions in `render_menu_bar`:
/// " [Z]eta " = 8 chars, " File " at +1, " Navigate " at +8, " View " at +19, " Help " at +26.
fn route_menu_bar_click(col: u16, bar_x: u16) -> Option<Action> {
    use crate::action::MenuId;
    let offset = col.saturating_sub(bar_x);
    match offset {
        1..=7 => Some(Action::OpenMenu(MenuId::File)),
        8..=18 => Some(Action::OpenMenu(MenuId::Navigate)),
        19..=25 => Some(Action::OpenMenu(MenuId::View)),
        26..=32 => Some(Action::OpenMenu(MenuId::Help)),
        _ => None,
    }
}
```

- [ ] **Step 2.4: Run the tests**

```bash
cargo test route_mouse_left_click route_mouse_scroll
```

Expected: both pass.

- [ ] **Step 2.5: Commit**

```bash
git add src/app.rs
git commit -m "feat(app): add route_mouse_event with LayoutCache hit-testing"
```

---

## Task 3: Wire mouse events into the event loop

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 3.1: Update `next_event()` to return mouse events**

In `next_event()`, the `_ => Ok(None)` arm currently discards all mouse events. Change it to:

```rust
fn next_event(&mut self) -> Result<Option<AppEvent>> {
    match self.job_results.try_recv() {
        Ok(result) => return Ok(Some(AppEvent::Job(result))),
        Err(TryRecvError::Disconnected) => anyhow::bail!("background worker disconnected"),
        Err(TryRecvError::Empty) => {}
    }

    if !event::poll(Duration::from_millis(250)).context("failed to poll terminal events")? {
        return Ok(None);
    }

    match event::read().context("failed to read terminal event")? {
        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
            Ok(Some(AppEvent::Input(key_event)))
        }
        Event::Mouse(mouse_event) => Ok(Some(AppEvent::Mouse(mouse_event))),
        Event::Resize(width, height) => Ok(Some(AppEvent::Resize { width, height })),
        _ => Ok(None),
    }
}
```

- [ ] **Step 3.2: Update `handle_event()` to dispatch mouse events**

Add the `AppEvent::Mouse` arm:

```rust
fn handle_event(&mut self, event: AppEvent) -> Result<()> {
    match event {
        AppEvent::Input(key_event) => {
            let focus = self.state.focus_layer();
            let is_preview_open = self.state.is_preview_panel_open();
            if let Some(action) = route_key_event(key_event, &self.keymap, focus, is_preview_open) {
                self.dispatch(action)?;
            }
        }
        AppEvent::Mouse(mouse_event) => {
            let focus = self.state.focus_layer();
            if let Some(action) = route_mouse_event(mouse_event, &self.layout_cache, focus) {
                self.dispatch(action)?;
            }
        }
        AppEvent::Resize { width, height } => {
            self.dispatch(Action::Resize { width, height })?;
        }
        AppEvent::Job(result) => {
            self.state.apply_job_result(result);
        }
    }
    Ok(())
}
```

- [ ] **Step 3.3: Run the full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 3.4: Commit**

```bash
git add src/app.rs
git commit -m "feat(app): route AppEvent::Mouse through dispatch pipeline using LayoutCache"
```

---

## Task 4: Smoke-test mouse interactions manually

**Files:** None modified.

- [ ] **Step 4.1: Build a release binary and verify it launches**

```bash
cargo build --release 2>&1 | tail -5
```

Expected: compiles without errors.

- [ ] **Step 4.2: Manual smoke test checklist**

Launch the app (`cargo run`) and verify each interaction:

| # | Action | Expected result |
|---|---|---|
| 1 | Scroll up/down on left pane | Selection moves |
| 2 | Scroll up/down on right pane | Selection moves |
| 3 | Left-click on right pane when left is focused | Focus switches to right |
| 4 | Left-click on left pane when right is focused | Focus switches to left |
| 5 | Left-click on " File " in menu bar | File menu opens |
| 6 | Left-click on " Navigate " in menu bar | Navigate menu opens |
| 7 | Scroll wheel when preview panel is open and visible | Preview scrolls |
| 8 | Left-click on preview panel area | Preview gets focus |
| 9 | Scroll wheel in editor area | Editor cursor moves |
| 10 | Click inside an open menu | No crash (modals absorb clicks) |

- [ ] **Step 4.3: Fix any regressions found during smoke test**

For each failing case, add a unit test that exposes the bug, then fix it:

```rust
// Example template for a regression test:
#[test]
fn right_pane_click_switches_focus_from_left() {
    use crate::state::FocusLayer;
    use crate::ui::layout_cache::LayoutCache;
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind, KeyModifiers};
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        left_pane: Rect { x: 0, y: 1, width: 40, height: 20 },
        right_pane: Rect { x: 40, y: 1, width: 40, height: 20 },
        menu_bar: Rect { x: 0, y: 0, width: 80, height: 1 },
        tools_panel: None,
        status_bar: Rect { x: 0, y: 21, width: 80, height: 1 },
    };
    let mouse = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 50, // inside right pane
        row: 5,
        modifiers: KeyModifiers::NONE,
    };
    let action = route_mouse_event(mouse, &cache, FocusLayer::Pane);
    assert!(action.is_some());
}
```

- [ ] **Step 4.4: Commit regression fixes**

```bash
git add src/app.rs
git commit -m "fix(mouse): correct pane focus routing for right-pane clicks"
```

---

## Task 5: Final verification

**Files:** None modified.

- [ ] **Step 5.1: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: zero warnings.

- [ ] **Step 5.2: Verify mouse capture is released on exit**

```bash
cargo run
# Press Ctrl+Q to quit
# Verify terminal cursor is visible and mouse capture is off
```

Expected: shell is usable immediately after exit; mouse events in the terminal are not captured.

- [ ] **Step 5.3: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: `test result: ok. N passed; 0 failed`.

- [ ] **Step 5.4: Final commit**

```bash
git commit -m "chore: Wave 2B complete — full mouse support (click-to-focus, scroll, menu bar)"
```

---

## Appendix: Menu bar click offset reference

The menu bar is rendered by `render_menu_bar` in `src/ui/menu_bar.rs`. The offsets used in `route_menu_bar_click` correspond to:

```
0         1         2         3
0123456789012345678901234567890123
 [Z]eta  File  Navigate  View  Help
         ^     ^         ^     ^
     offset 1  offset 8  19    26
```

If the menu bar text changes (more items, different labels), update the match ranges in `route_menu_bar_click` to match. The `render_menu_popup` `x` offsets in `src/ui/overlay.rs` must also stay in sync.
