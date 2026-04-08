# Wave 2A — FocusLayer + Input Routing Redesign

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the ad-hoc `RouteContext` struct (9 untyped booleans) in `app.rs` and the 6-parameter `from_key_event_with_settings` function in `action.rs` with a compiler-enforced `FocusLayer` enum that makes illegal routing states unrepresentable. Remove all dead code exposed during this cleanup.

**Architecture:**
- `FocusLayer` and `ModalKind` enums live in `src/state/types.rs` (alongside `PaneFocus`, `PaneLayout` etc.).
- `AppState::focus_layer()` derives the current layer from the state's single source of truth, eliminating all the `is_*_open()` / `is_*_focused()` calls scattered across `app.rs`.
- `route_key_event` in `app.rs` is rewritten to `match focus_layer { … }` — each arm dispatches to a focused helper in `action.rs`. No booleans.
- `RouteContext` struct is deleted from `app.rs`.
- Dead code removed: `Action::from_key_event()` wrapper, `let _ = is_editor_focused;` in `action.rs`, the redundant `from_key_event_with_settings` boolean flags that are now unreachable.
- Existing tests in `app.rs` are updated to use `FocusLayer` instead of the old `RouteContext`.

**Tech Stack:** Rust, existing ratatui/crossterm stack. No new dependencies.

**Jira:** ZTA-80 (ZTA-102 through ZTA-108)

**Wave dependency:** Starts AFTER Wave 1 (1A + 1B + 1C) is merged. The `ModalState` enum introduced in Wave 1A is the basis for `ModalKind` — use it directly. Wave 2B (mouse support) depends on `AppState::focus_layer()` being stable.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/state/types.rs` | Add `FocusLayer`, `ModalKind` enums |
| Modify | `src/state/mod.rs` | Add `AppState::focus_layer()` method |
| Modify | `src/action.rs` | Add focused dispatch helpers; remove dead `from_key_event` wrapper; clean up `from_key_event_with_settings` |
| Modify | `src/app.rs` | Rewrite `route_key_event`; delete `RouteContext`; update tests |

---

## Task 1: Add FocusLayer and ModalKind to state/types.rs

**Files:**
- Modify: `src/state/types.rs`

- [ ] **Step 1.1: Write the failing test**

Add to the test module in `src/state/types.rs`:

```rust
#[test]
fn focus_layer_modal_wraps_kind() {
    let layer = FocusLayer::Modal(ModalKind::Palette);
    assert!(matches!(layer, FocusLayer::Modal(ModalKind::Palette)));
}

#[test]
fn focus_layer_pane_is_default() {
    assert!(matches!(FocusLayer::default(), FocusLayer::Pane));
}
```

- [ ] **Step 1.2: Confirm it fails**

```bash
cargo test focus_layer_modal_wraps_kind 2>&1 | head -5
```

Expected: compile error — `FocusLayer` not found.

- [ ] **Step 1.3: Add the enums to `src/state/types.rs`**

Append after the existing `PaneFocus` and `PaneLayout` definitions:

```rust
/// Which input layer currently has keyboard focus.
///
/// Derivable from `AppState::focus_layer()` — do not store separately.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusLayer {
    /// No overlay open; directional keys navigate pane entries.
    Pane,
    /// The editor tools panel is focused.
    Editor,
    /// The preview tools panel is focused.
    Preview,
    /// A modal overlay is open; only modal-specific keys are processed.
    Modal(ModalKind),
}

impl Default for FocusLayer {
    fn default() -> Self {
        Self::Pane
    }
}

/// Identifies which modal overlay is currently active.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModalKind {
    Menu,
    Prompt,
    Dialog,
    Collision,
    Palette,
    Settings,
}
```

- [ ] **Step 1.4: Export from `src/state/mod.rs`**

At the top of `src/state/mod.rs`, add to the `pub use types::…` line:

```rust
pub use types::{FocusLayer, MenuItem, ModalKind, PaneFocus, PaneLayout};
```

- [ ] **Step 1.5: Run tests**

```bash
cargo test focus_layer
```

Expected: both tests pass.

- [ ] **Step 1.6: Commit**

```bash
git add src/state/types.rs src/state/mod.rs
git commit -m "feat(state): add FocusLayer and ModalKind enums"
```

---

## Task 2: Implement AppState::focus_layer()

**Files:**
- Modify: `src/state/mod.rs`

- [ ] **Step 2.1: Write the failing test**

Add to the `AppState` test module in `src/state/mod.rs`:

```rust
#[test]
fn focus_layer_returns_palette_when_palette_open() {
    // Requires a bootstrapped AppState with palette open.
    // Use the minimal scaffold pattern already in the file, or construct directly.
    let mut state = make_test_state(); // helper used by existing tests
    state.apply(Action::OpenCommandPalette).unwrap();
    assert!(matches!(state.focus_layer(), FocusLayer::Modal(ModalKind::Palette)));
}

#[test]
fn focus_layer_returns_pane_when_nothing_open() {
    let state = make_test_state();
    assert!(matches!(state.focus_layer(), FocusLayer::Pane));
}
```

> If `make_test_state()` does not exist, use `AppState::bootstrap(LoadedConfig::default_in_memory(), Instant::now()).unwrap()` or the equivalent test scaffold already used in the file.

- [ ] **Step 2.2: Confirm it fails**

```bash
cargo test focus_layer_returns_palette 2>&1 | head -5
```

Expected: compile error — `focus_layer()` method not found.

- [ ] **Step 2.3: Implement `focus_layer()` in `AppState`**

Add the method to the `impl AppState` block in `src/state/mod.rs`:

```rust
/// Derive the current input focus layer from state.
/// Priority (highest to lowest): Palette > Collision > Prompt > Dialog > Menu > Settings > Editor > Preview > Pane.
pub fn focus_layer(&self) -> FocusLayer {
    use crate::state::types::{FocusLayer, ModalKind};

    if self.command_palette.is_some() {
        return FocusLayer::Modal(ModalKind::Palette);
    }
    if self.collision.is_some() {
        return FocusLayer::Modal(ModalKind::Collision);
    }
    if self.prompt.is_some() {
        return FocusLayer::Modal(ModalKind::Prompt);
    }
    if self.dialog.is_some() {
        return FocusLayer::Modal(ModalKind::Dialog);
    }
    if self.active_menu.is_some() {
        return FocusLayer::Modal(ModalKind::Menu);
    }
    if self.settings.is_some() {
        return FocusLayer::Modal(ModalKind::Settings);
    }
    if self.is_editor_focused() {
        return FocusLayer::Editor;
    }
    if self.is_preview_focused() {
        return FocusLayer::Preview;
    }
    FocusLayer::Pane
}
```

> **If Wave 1A has merged:** `self.command_palette`, `self.collision`, etc. are accessed via the sub-state structs (`self.overlay.modal`, etc.). Adapt the field references to match the Wave 1A refactored `AppState`. The priority chain remains identical.

- [ ] **Step 2.4: Run tests**

```bash
cargo test focus_layer_returns
```

Expected: both pass.

- [ ] **Step 2.5: Commit**

```bash
git add src/state/mod.rs
git commit -m "feat(state): implement AppState::focus_layer() — derives focus from single source of truth"
```

---

## Task 3: Add focused dispatch helpers to action.rs

**Files:**
- Modify: `src/action.rs`

- [ ] **Step 3.1: Write the failing test**

Add to `src/action.rs` test module:

```rust
#[test]
fn from_palette_key_event_handles_esc() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(
        Action::from_palette_key_event(key),
        Some(Action::CloseCommandPalette)
    );
}

#[test]
fn from_pane_key_event_handles_quit() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let keymap = RuntimeKeymap::default();
    let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
    assert_eq!(
        Action::from_pane_key_event(key, &keymap),
        Some(Action::Quit)
    );
}
```

- [ ] **Step 3.2: Confirm they fail**

```bash
cargo test from_palette_key_event_handles_esc from_pane_key_event_handles_quit 2>&1 | head -5
```

Expected: compile errors — helpers not yet added.

- [ ] **Step 3.3: Add helpers to `src/action.rs`**

Add the following `impl Action` methods. Each corresponds to one `FocusLayer` arm in the new `route_key_event`:

```rust
impl Action {
    // -----------------------------------------------------------------------
    // Focused dispatch helpers (used by route_key_event in app.rs)
    // -----------------------------------------------------------------------

    /// Keys when the command palette is open. Consumes ALL input.
    pub fn from_palette_key_event(key_event: crossterm::event::KeyEvent) -> Option<Self> {
        use crossterm::event::{KeyCode, KeyModifiers};
        match key_event.code {
            KeyCode::Esc => Some(Self::CloseCommandPalette),
            KeyCode::Enter => Some(Self::PaletteConfirm),
            KeyCode::Up => Some(Self::PaletteMoveUp),
            KeyCode::Down => Some(Self::PaletteMoveDown),
            KeyCode::Backspace => Some(Self::PaletteBackspace),
            KeyCode::Char(c)
                if key_event.modifiers == KeyModifiers::NONE
                    || key_event.modifiers == KeyModifiers::SHIFT =>
            {
                Some(Self::PaletteInput(c))
            }
            _ => None,
        }
    }

    /// Keys when the settings panel is open. Consumes ALL input.
    pub fn from_settings_key_event(key_event: crossterm::event::KeyEvent) -> Option<Self> {
        use crossterm::event::KeyCode;
        match key_event.code {
            KeyCode::Esc => Some(Self::CloseSettingsPanel),
            KeyCode::Enter | KeyCode::Char(' ') => Some(Self::SettingsToggleCurrent),
            KeyCode::Up => Some(Self::SettingsMoveUp),
            KeyCode::Down => Some(Self::SettingsMoveDown),
            _ => None,
        }
    }

    /// Keys when the preview panel has focus. Consumes ALL input.
    pub fn from_preview_key_event(key_event: crossterm::event::KeyEvent) -> Option<Self> {
        use crossterm::event::{KeyCode, KeyModifiers};
        match key_event.code {
            KeyCode::Up => Some(Self::ScrollPreviewUp),
            KeyCode::Down => Some(Self::ScrollPreviewDown),
            KeyCode::PageUp => Some(Self::ScrollPreviewPageUp),
            KeyCode::PageDown => Some(Self::ScrollPreviewPageDown),
            KeyCode::Esc => Some(Self::FocusPreviewPanel),
            KeyCode::Char('w') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::CycleFocus)
            }
            _ => None,
        }
    }

    /// Global keys available in Pane context (and as fallback from Editor context).
    pub fn from_pane_key_event(
        key_event: crossterm::event::KeyEvent,
        keymap: &crate::config::RuntimeKeymap,
    ) -> Option<Self> {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Alt-modified keys open menus or navigate.
        if key_event.modifiers == KeyModifiers::ALT {
            return match key_event.code {
                KeyCode::Char('f') | KeyCode::Char('F') => Some(Self::OpenMenu(MenuId::File)),
                KeyCode::Char('n') | KeyCode::Char('N') => Some(Self::OpenMenu(MenuId::Navigate)),
                KeyCode::Char('v') | KeyCode::Char('V') => Some(Self::OpenMenu(MenuId::View)),
                KeyCode::Char('h') | KeyCode::Char('H') => Some(Self::OpenMenu(MenuId::Help)),
                KeyCode::Left => Some(Self::NavigateBack),
                KeyCode::Right => Some(Self::NavigateForward),
                _ => None,
            };
        }

        if key_event.code == KeyCode::Char('q') && key_event.modifiers == KeyModifiers::CONTROL {
            return Some(Self::Quit);
        }
        if key_event.code == KeyCode::Char('w') && key_event.modifiers == KeyModifiers::CONTROL {
            return Some(Self::CycleFocus);
        }
        if keymap.switch_pane.matches(&key_event) {
            return Some(Self::FocusNextPane);
        }

        // Delegate to the existing comprehensive handler for the remaining keys.
        Self::from_key_event_with_settings(key_event, keymap, false, false, false, false)
    }
}
```

> **Note:** `from_menu_key_event`, `from_collision_key_event`, `from_prompt_key_event`, and `from_dialog_key_event` are already defined in `action.rs` (they were referenced in the old `route_key_event`). Verify they exist; add them if they were split out separately.

- [ ] **Step 3.4: Run tests**

```bash
cargo test from_palette_key_event_handles_esc from_pane_key_event_handles_quit
```

Expected: both pass.

- [ ] **Step 3.5: Commit**

```bash
git add src/action.rs
git commit -m "feat(action): add focused dispatch helpers — from_palette/settings/preview/pane_key_event"
```

---

## Task 4: Rewrite route_key_event — delete RouteContext

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 4.1: Write the failing tests**

Add to `src/app.rs` test module:

```rust
#[test]
fn palette_layer_routes_esc_to_close_palette() {
    use crate::state::FocusLayer;
    let keymap = RuntimeKeymap::default();
    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    );
    let action = route_key_event(key, &keymap, FocusLayer::Modal(ModalKind::Palette), false);
    assert_eq!(action, Some(Action::CloseCommandPalette));
}

#[test]
fn pane_layer_ctrl_q_quits() {
    use crate::state::FocusLayer;
    let keymap = RuntimeKeymap::default();
    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('q'),
        crossterm::event::KeyModifiers::CONTROL,
    );
    let action = route_key_event(key, &keymap, FocusLayer::Pane, false);
    assert_eq!(action, Some(Action::Quit));
}

#[test]
fn editor_layer_ctrl_f_opens_search() {
    use crate::state::FocusLayer;
    let keymap = RuntimeKeymap::default();
    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('f'),
        crossterm::event::KeyModifiers::CONTROL,
    );
    let action = route_key_event(key, &keymap, FocusLayer::Editor, false);
    assert_eq!(action, Some(Action::EditorOpenSearch));
}
```

- [ ] **Step 4.2: Confirm they fail**

```bash
cargo test palette_layer_routes pane_layer_ctrl_q editor_layer_ctrl_f 2>&1 | head -5
```

Expected: compile errors — `route_key_event` still takes `RouteContext`, not `FocusLayer`.

- [ ] **Step 4.3: Rewrite `route_key_event` and delete `RouteContext` in `src/app.rs`**

**Remove** the `RouteContext` struct entirely.

**Add** imports at the top:

```rust
use crate::state::{FocusLayer, ModalKind};
```

**Replace** `route_key_event` with:

```rust
fn route_key_event(
    key_event: crossterm::event::KeyEvent,
    keymap: &RuntimeKeymap,
    focus: FocusLayer,
    is_preview_open: bool,
) -> Option<Action> {
    use crossterm::event::{KeyCode, KeyModifiers};

    // Alt-F3 focuses the preview from any non-modal context if preview is open.
    let alt_f3 = key_event.code == KeyCode::F(3)
        && key_event.modifiers == KeyModifiers::ALT;

    match focus {
        FocusLayer::Modal(ModalKind::Palette) => {
            Action::from_palette_key_event(key_event)
        }
        FocusLayer::Modal(ModalKind::Collision) => {
            Action::from_collision_key_event(key_event)
        }
        FocusLayer::Modal(ModalKind::Prompt) => {
            Action::from_prompt_key_event(key_event)
        }
        FocusLayer::Modal(ModalKind::Dialog) => {
            Action::from_dialog_key_event(key_event)
        }
        FocusLayer::Modal(ModalKind::Menu) => {
            Action::from_menu_key_event(key_event)
        }
        FocusLayer::Modal(ModalKind::Settings) => {
            Action::from_settings_key_event(key_event)
        }
        FocusLayer::Preview => {
            Action::from_preview_key_event(key_event)
        }
        FocusLayer::Editor => {
            if is_preview_open && alt_f3 {
                return Some(Action::FocusPreviewPanel);
            }
            // Editor-specific bindings take priority; fall back to pane bindings.
            Action::from_editor_key_event(key_event)
                .or_else(|| Action::from_pane_key_event(key_event, keymap))
        }
        FocusLayer::Pane => {
            if is_preview_open && alt_f3 {
                return Some(Action::FocusPreviewPanel);
            }
            Action::from_pane_key_event(key_event, keymap)
        }
    }
}
```

**Update `handle_event`** to use `focus_layer()` instead of `RouteContext`:

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

- [ ] **Step 4.4: Update existing tests in `src/app.rs`**

The old tests used `RouteContext { … }`. Replace them:

```rust
// BEFORE:
route_key_event(key, &keymap, RouteContext {
    is_palette_open: true,
    is_editor_focused: true,
    // ...
})

// AFTER:
use crate::state::FocusLayer;
route_key_event(key, &keymap, FocusLayer::Modal(ModalKind::Palette), false)
```

Existing tests to migrate:
- `command_palette_remains_available_while_editor_is_open` → `FocusLayer::Modal(ModalKind::Palette)`, `is_preview_open: false`
- `editor_shortcuts_still_take_priority_over_global_fallbacks` → `FocusLayer::Editor`, `is_preview_open: false`
- `palette_open_state_blocks_lower_priority_input_paths` → `FocusLayer::Modal(ModalKind::Palette)`, `is_preview_open: false`

- [ ] **Step 4.5: Run all tests**

```bash
cargo test
```

Expected: all pass including the new and migrated tests.

- [ ] **Step 4.6: Commit**

```bash
git add src/app.rs
git commit -m "refactor(app): replace RouteContext with FocusLayer — route_key_event uses match on enum"
```

---

## Task 5: Remove dead code from action.rs

**Files:**
- Modify: `src/action.rs`

- [ ] **Step 5.1: Identify dead code**

```bash
cargo clippy -- -D warnings 2>&1 | grep "action.rs"
```

Look for:
- `from_key_event` — public wrapper that just calls `from_key_event_with_settings` with all-false booleans. Now only `from_pane_key_event` calls `from_key_event_with_settings`.
- `let _ = is_editor_focused;` at line ~237.
- `is_palette_open` / `is_settings_open` branches inside `from_key_event_with_settings` (now handled by dedicated helpers — these branches are dead when routed through `from_pane_key_event`).

- [ ] **Step 5.2: Remove `from_key_event` wrapper**

Find and delete:

```rust
pub fn from_key_event(
    key_event: KeyEvent,
    keymap: &RuntimeKeymap,
    is_editor_focused: bool,
    is_preview_focused: bool,
    is_palette_open: bool,
) -> Option<Self> {
    Self::from_key_event_with_settings(
        key_event,
        keymap,
        is_editor_focused,
        is_preview_focused,
        is_palette_open,
        false,
    )
}
```

- [ ] **Step 5.3: Remove `let _ = is_editor_focused;` dead assignment**

In `from_key_event_with_settings`, find and delete the line:

```rust
let _ = is_editor_focused;
```

- [ ] **Step 5.4: Simplify `from_key_event_with_settings` signature**

Since `is_palette_open` and `is_settings_open` branches are now dead (palette and settings get dedicated helpers), strip those parameters:

**Before:**
```rust
pub fn from_key_event_with_settings(
    key_event: KeyEvent,
    keymap: &RuntimeKeymap,
    is_editor_focused: bool,
    is_preview_focused: bool,
    is_palette_open: bool,
    is_settings_open: bool,
) -> Option<Self>
```

**After:**
```rust
/// Low-priority fallback key handler for Pane and Editor contexts.
/// Palette, settings, preview, collision, prompt, dialog, and menu
/// are handled by their dedicated `from_*_key_event` helpers.
pub fn from_key_event_with_settings(
    key_event: KeyEvent,
    keymap: &RuntimeKeymap,
) -> Option<Self>
```

Remove the `is_palette_open { return … }` and `is_settings_open { return … }` blocks from the function body. Remove the `is_preview_focused { return … }` block (now in `from_preview_key_event`). Update the single remaining call in `from_pane_key_event`:

```rust
// In from_pane_key_event, replace:
Self::from_key_event_with_settings(key_event, keymap, false, false, false, false)
// With:
Self::from_key_event_with_settings(key_event, keymap)
```

- [ ] **Step 5.5: Run tests**

```bash
cargo test
```

Expected: all tests still pass. Fix any compile errors from the signature change.

- [ ] **Step 5.6: Commit**

```bash
git add src/action.rs
git commit -m "refactor(action): remove dead from_key_event wrapper and stale boolean flags"
```

---

## Task 6: Final verification

**Files:** None modified.

- [ ] **Step 6.1: Run clippy with -D warnings**

```bash
cargo clippy -- -D warnings
```

Expected: zero warnings.

- [ ] **Step 6.2: Verify RouteContext is fully gone**

```bash
grep -rn "RouteContext" src/
```

Expected: no output.

- [ ] **Step 6.3: Verify is_editor_focused dead code is gone**

```bash
grep -n "let _ = is_editor" src/action.rs
```

Expected: no output.

- [ ] **Step 6.4: Run full test suite with verbose output**

```bash
cargo test -- --nocapture 2>&1 | tail -5
```

Expected: `test result: ok. N passed; 0 failed`.

- [ ] **Step 6.5: Final commit**

```bash
git commit -m "chore: Wave 2A complete — FocusLayer routing, dead code removed"
```
