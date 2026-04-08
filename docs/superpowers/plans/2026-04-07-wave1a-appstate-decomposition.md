# Wave 1A — AppState Decomposition + Modal Exclusivity

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Break the 2,169-line `state/mod.rs` god object into four focused sub-states (`PaneSetState`, `EditorState`, `PreviewState`, `OverlayState`), enforce modal exclusivity at the type level via a `ModalState` enum, and remove the scattered `needs_redraw` flag.

**Architecture:** `AppState` becomes a thin coordinator that fans `apply()` out to four sub-state structs, each owning its own reducer. `OverlayState` holds a single `Option<ModalState>` enum instead of six separate optional fields — the compiler prevents two modals coexisting. `needs_redraw` is deleted; the app redraws unconditionally after every event.

**Tech Stack:** Rust, ratatui 0.29, crossterm 0.28, anyhow, existing `src/state/` helpers.

**Jira:** ZTA-78, ZTA-82 (ZTA-87 through ZTA-94, ZTA-109, ZTA-110)

**Wave dependency:** This is a Wave 1 plan. Start from the current `main` branch. Wave 2A (input routing) depends on the interfaces defined here — specifically `OverlayState`, `ModalState`, and `AppState::focus_layer()`.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/state/pane_set.rs` | `PaneSetState` + `reduce_pane()` |
| Create | `src/state/editor_state.rs` | `EditorState` + `reduce_editor()` |
| Create | `src/state/preview_state.rs` | `PreviewState` + `reduce_preview()` |
| Create | `src/state/overlay.rs` | `OverlayState` + `ModalState` enum + all modal reducers |
| Modify | `src/state/mod.rs` | Thin coordinator: fan-out `apply()`, public accessors, `apply_view()`, `apply_job_result()` |
| Keep | `src/state/dialog.rs` | `DialogState`, `CollisionState` — imported by `overlay.rs` |
| Keep | `src/state/menu.rs` | `menu_items_for()` — imported by `overlay.rs` |
| Keep | `src/state/prompt.rs` | `PromptState`, `PromptKind` — imported by `overlay.rs` |
| Keep | `src/state/settings.rs` | `SettingsState`, `SettingsField`, `SettingsEntry` — imported by `overlay.rs` |
| Keep | `src/state/types.rs` | `MenuItem`, `PaneFocus`, `PaneLayout` |
| Modify | `src/app.rs` | Remove `needs_redraw` check; redraw unconditionally |
| Modify | `src/lib.rs` | Re-export any types that move |

---

## Task 1: Create PaneSetState

**Files:**
- Create: `src/state/pane_set.rs`

- [ ] **Step 1.1: Write the failing test**

Add at the bottom of `src/state/pane_set.rs` (the file doesn't exist yet — create it with only this content first):

```rust
use std::path::PathBuf;

use anyhow::Result;

use crate::action::{Action, Command, RefreshTarget};
use crate::fs::EntryKind;
use crate::pane::{PaneId, PaneState};
use crate::state::types::{PaneFocus, PaneLayout};

#[derive(Clone, Debug)]
pub struct PaneSetState {
    pub left: PaneState,
    pub right: PaneState,
    pub focus: PaneFocus,
    pub pane_layout: PaneLayout,
}

impl PaneSetState {
    pub fn new(left: PaneState, right: PaneState) -> Self {
        Self {
            left,
            right,
            focus: PaneFocus::Left,
            pane_layout: PaneLayout::default(),
        }
    }

    pub fn active_pane(&self) -> &PaneState {
        match self.focus {
            PaneFocus::Left | PaneFocus::Preview => &self.left,
            PaneFocus::Right => &self.right,
        }
    }

    pub fn active_pane_mut(&mut self) -> &mut PaneState {
        match self.focus {
            PaneFocus::Left | PaneFocus::Preview => &mut self.left,
            PaneFocus::Right => &mut self.right,
        }
    }

    pub fn inactive_pane(&self) -> &PaneState {
        match self.focus {
            PaneFocus::Left | PaneFocus::Preview => &self.right,
            PaneFocus::Right => &self.left,
        }
    }

    pub fn focused_pane_id(&self) -> PaneId {
        match self.focus {
            PaneFocus::Left | PaneFocus::Preview => PaneId::Left,
            PaneFocus::Right => PaneId::Right,
        }
    }

    pub fn pane(&self, id: PaneId) -> &PaneState {
        match id {
            PaneId::Left => &self.left,
            PaneId::Right => &self.right,
        }
    }

    pub fn pane_mut(&mut self, id: PaneId) -> &mut PaneState {
        match id {
            PaneId::Left => &mut self.left,
            PaneId::Right => &mut self.right,
        }
    }

    pub fn apply(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        match action {
            Action::EnterSelection => {
                if self.active_pane().can_enter_selected() {
                    if let Some(path) = self.active_pane().selected_path() {
                        let pane = self.focused_pane_id();
                        let active = self.active_pane_mut();
                        active.clear_marks();
                        active.push_history();
                        commands.push(Command::ScanPane { pane, path });
                    }
                }
            }
            Action::NavigateToParent => {
                if let Some(path) = self.active_pane().parent_path() {
                    let pane = self.focused_pane_id();
                    let active = self.active_pane_mut();
                    active.clear_marks();
                    active.push_history();
                    commands.push(Command::ScanPane { pane, path });
                }
            }
            Action::NavigateBack => {
                if let Some(path) = self.active_pane_mut().pop_back() {
                    let pane_id = self.focused_pane_id();
                    self.active_pane_mut().clear_marks();
                    commands.push(Command::ScanPane { pane: pane_id, path });
                }
            }
            Action::NavigateForward => {
                if let Some(path) = self.active_pane_mut().pop_forward() {
                    let pane_id = self.focused_pane_id();
                    self.active_pane_mut().clear_marks();
                    commands.push(Command::ScanPane { pane: pane_id, path });
                }
            }
            Action::FocusNextPane => {
                self.focus = match self.focus {
                    PaneFocus::Left | PaneFocus::Preview => PaneFocus::Right,
                    PaneFocus::Right => PaneFocus::Left,
                };
            }
            Action::MoveSelectionDown => {
                self.active_pane_mut().move_selection_down();
            }
            Action::MoveSelectionUp => {
                self.active_pane_mut().move_selection_up();
            }
            Action::ToggleMark => {
                self.active_pane_mut().toggle_mark_selected();
            }
            Action::ClearMarks => {
                self.active_pane_mut().clear_marks();
            }
            Action::Refresh => {
                let pane = self.focused_pane_id();
                let path = self.active_pane().cwd.clone();
                commands.push(Command::ScanPane { pane, path });
            }
            Action::CycleSortMode => {
                let pane = self.active_pane_mut();
                pane.sort_mode = pane.sort_mode.next();
                pane.selection = 0;
                pane.scroll_offset = 0;
            }
            Action::ToggleHiddenFiles => {
                let new_value = !self.active_pane().show_hidden;
                self.active_pane_mut().set_show_hidden(new_value)?;
            }
            _ => {}
        }
        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane::PaneState;
    use std::path::PathBuf;

    fn make_state() -> PaneSetState {
        let cwd = PathBuf::from("/tmp");
        PaneSetState::new(
            PaneState::empty("Left", cwd.clone()),
            PaneState::empty("Right", cwd),
        )
    }

    #[test]
    fn focus_next_pane_cycles_left_to_right() {
        let mut s = make_state();
        assert_eq!(s.focus, PaneFocus::Left);
        s.apply(&Action::FocusNextPane).unwrap();
        assert_eq!(s.focus, PaneFocus::Right);
    }

    #[test]
    fn focus_next_pane_cycles_right_to_left() {
        let mut s = make_state();
        s.focus = PaneFocus::Right;
        s.apply(&Action::FocusNextPane).unwrap();
        assert_eq!(s.focus, PaneFocus::Left);
    }
}
```

- [ ] **Step 1.2: Declare the module in `src/state/mod.rs`**

Add near the top of `src/state/mod.rs` alongside the existing module declarations:

```rust
pub mod pane_set;
pub use pane_set::PaneSetState;
```

- [ ] **Step 1.3: Run tests to verify the new module compiles and tests pass**

```
cargo test state::pane_set
```

Expected: 2 tests pass. Fix any compilation errors before proceeding.

- [ ] **Step 1.4: Commit**

```
git add src/state/pane_set.rs src/state/mod.rs
git commit -m "feat(state): extract PaneSetState with reduce_pane logic (ZTA-87)"
```

---

## Task 2: Create EditorState

**Files:**
- Create: `src/state/editor_state.rs`

- [ ] **Step 2.1: Create `src/state/editor_state.rs`**

```rust
use std::path::PathBuf;

use anyhow::Result;

use crate::action::{Action, Command};
use crate::editor::EditorBuffer;

/// Owns the optional editor buffer and routes editor actions.
/// Wave 1C will replace EditorBuffer with tui-textarea::TextArea;
/// only this file needs updating at that point.
#[derive(Clone, Debug, Default)]
pub struct EditorState {
    pub buffer: Option<EditorBuffer>,
}

impl EditorState {
    pub fn is_open(&self) -> bool {
        self.buffer.is_some()
    }

    pub fn is_dirty(&self) -> bool {
        self.buffer.as_ref().is_some_and(|e| e.is_dirty)
    }

    pub fn open(&mut self, editor: EditorBuffer) {
        self.buffer = Some(editor);
    }

    pub fn close(&mut self) {
        self.buffer = None;
    }

    pub fn apply(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        match action {
            Action::CloseEditor => {
                if let Some(editor) = self.buffer.as_mut() {
                    if editor.search_active {
                        editor.search_active = false;
                        editor.search_query.clear();
                        return Ok(commands);
                    }
                }
                if let Some(editor) = &self.buffer {
                    if !editor.is_dirty {
                        self.buffer = None;
                    }
                }
            }
            Action::DiscardEditorChanges => {
                self.buffer = None;
            }
            Action::EditorOpenSearch => {
                if let Some(editor) = self.buffer.as_mut() {
                    editor.search_active = true;
                    editor.search_query.clear();
                    editor.search_match_idx = 0;
                }
            }
            Action::EditorCloseSearch => {
                if let Some(editor) = self.buffer.as_mut() {
                    editor.search_active = false;
                    editor.search_query.clear();
                }
            }
            Action::EditorBackspace => {
                if let Some(editor) = self.buffer.as_mut() {
                    if editor.search_active {
                        editor.search_query.pop();
                        if !editor.search_query.is_empty() {
                            editor.search_next();
                        }
                        return Ok(commands);
                    }
                    editor.backspace();
                }
            }
            Action::EditorInsert(ch) => {
                if let Some(editor) = self.buffer.as_mut() {
                    if editor.search_active {
                        editor.search_query.push(*ch);
                        editor.search_next();
                        return Ok(commands);
                    }
                    editor.insert_char(*ch);
                }
            }
            Action::EditorMoveDown => { if let Some(e) = self.buffer.as_mut() { e.move_down(); } }
            Action::EditorMoveLeft => { if let Some(e) = self.buffer.as_mut() { e.move_left(); } }
            Action::EditorMoveRight => { if let Some(e) = self.buffer.as_mut() { e.move_right(); } }
            Action::EditorMoveUp => { if let Some(e) = self.buffer.as_mut() { e.move_up(); } }
            Action::EditorNewline => {
                if let Some(editor) = self.buffer.as_mut() {
                    if editor.search_active {
                        editor.search_next();
                        return Ok(commands);
                    }
                    editor.insert_newline();
                }
            }
            Action::EditorSearchBackspace => {
                if let Some(editor) = self.buffer.as_mut() {
                    if editor.search_active {
                        editor.search_query.pop();
                        if !editor.search_query.is_empty() {
                            editor.search_next();
                        }
                    }
                }
            }
            Action::EditorSearchNext => { if let Some(e) = self.buffer.as_mut() { e.search_next(); } }
            Action::EditorSearchPrev => { if let Some(e) = self.buffer.as_mut() { e.search_prev(); } }
            Action::OpenSelectedInEditor => {
                // Handled in AppState::apply_view — needs access to active pane
            }
            Action::SaveEditor => {
                if let Some(editor) = &self.buffer {
                    if editor.is_dirty {
                        commands.push(Command::SaveEditor);
                    }
                }
            }
            _ => {}
        }
        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_state_starts_closed() {
        let s = EditorState::default();
        assert!(!s.is_open());
        assert!(!s.is_dirty());
    }

    #[test]
    fn discard_closes_buffer() {
        let mut s = EditorState::default();
        s.buffer = Some(EditorBuffer::default());
        s.apply(&Action::DiscardEditorChanges).unwrap();
        assert!(!s.is_open());
    }
}
```

- [ ] **Step 2.2: Declare the module**

In `src/state/mod.rs`, add:

```rust
pub mod editor_state;
pub use editor_state::EditorState;
```

- [ ] **Step 2.3: Run tests**

```
cargo test state::editor_state
```

Expected: 2 tests pass.

- [ ] **Step 2.4: Commit**

```
git add src/state/editor_state.rs src/state/mod.rs
git commit -m "feat(state): extract EditorState with reduce_editor logic (ZTA-88)"
```

---

## Task 3: Create PreviewState

**Files:**
- Create: `src/state/preview_state.rs`

- [ ] **Step 3.1: Create `src/state/preview_state.rs`**

```rust
use std::path::PathBuf;

use anyhow::Result;

use crate::action::{Action, Command};
use crate::fs::EntryKind;
use crate::preview::ViewBuffer;
use crate::state::types::PaneFocus;

#[derive(Clone, Debug, Default)]
pub struct PreviewState {
    pub view: Option<(PathBuf, ViewBuffer)>,
    pub panel_open: bool,
    pub preview_on_selection: bool,
}

impl PreviewState {
    pub fn new(panel_open: bool, preview_on_selection: bool) -> Self {
        Self {
            view: None,
            panel_open,
            preview_on_selection,
        }
    }

    pub fn apply(
        &mut self,
        action: &Action,
        focus: &PaneFocus,
    ) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        match action {
            Action::ClearPreview => {
                self.view = None;
            }
            Action::TogglePreviewPanel => {
                self.panel_open = !self.panel_open;
            }
            Action::PreviewFile { path } => {
                commands.push(Command::PreviewFile { path: path.clone() });
            }
            Action::FocusPreviewPanel => {
                // Focus toggling handled at AppState level (needs full focus context)
            }
            Action::ScrollPreviewDown => {
                if *focus == PaneFocus::Preview {
                    if let Some((_, v)) = self.view.as_mut() { v.scroll_down(1); }
                }
            }
            Action::ScrollPreviewUp => {
                if *focus == PaneFocus::Preview {
                    if let Some((_, v)) = self.view.as_mut() { v.scroll_up(1); }
                }
            }
            Action::ScrollPreviewPageDown => {
                if *focus == PaneFocus::Preview {
                    if let Some((_, v)) = self.view.as_mut() { v.scroll_down(20); }
                }
            }
            Action::ScrollPreviewPageUp => {
                if *focus == PaneFocus::Preview {
                    if let Some((_, v)) = self.view.as_mut() { v.scroll_up(20); }
                }
            }
            _ => {}
        }
        Ok(commands)
    }

    pub fn apply_job_loaded(&mut self, path: PathBuf, view: ViewBuffer) {
        if let Some((ref current, ref mut buf)) = self.view {
            if *current == path {
                buf.reset_scroll();
                return;
            }
        }
        self.view = Some((path, view));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::ViewBuffer;
    use std::path::PathBuf;

    #[test]
    fn toggle_panel_flips_state() {
        let mut s = PreviewState::new(false, true);
        s.apply(&Action::TogglePreviewPanel, &PaneFocus::Left).unwrap();
        assert!(s.panel_open);
        s.apply(&Action::TogglePreviewPanel, &PaneFocus::Left).unwrap();
        assert!(!s.panel_open);
    }

    #[test]
    fn clear_preview_removes_view() {
        let mut s = PreviewState::new(true, true);
        s.view = Some((PathBuf::from("/tmp/a.txt"), ViewBuffer::from_plain("hello")));
        s.apply(&Action::ClearPreview, &PaneFocus::Left).unwrap();
        assert!(s.view.is_none());
    }

    #[test]
    fn scroll_only_applies_when_preview_focused() {
        let mut s = PreviewState::new(true, true);
        s.view = Some((PathBuf::from("/tmp/a.txt"), ViewBuffer::from_plain("line1\nline2\nline3")));
        // scroll while pane is focused — should have no effect
        s.apply(&Action::ScrollPreviewDown, &PaneFocus::Left).unwrap();
        let row = s.view.as_ref().unwrap().1.scroll_row;
        assert_eq!(row, 0);
        // scroll while preview is focused — should move
        s.apply(&Action::ScrollPreviewDown, &PaneFocus::Preview).unwrap();
        let row = s.view.as_ref().unwrap().1.scroll_row;
        assert_eq!(row, 1);
    }
}
```

- [ ] **Step 3.2: Declare the module**

In `src/state/mod.rs`:

```rust
pub mod preview_state;
pub use preview_state::PreviewState;
```

- [ ] **Step 3.3: Run tests**

```
cargo test state::preview_state
```

Expected: 3 tests pass.

- [ ] **Step 3.4: Commit**

```
git add src/state/preview_state.rs src/state/mod.rs
git commit -m "feat(state): extract PreviewState with reduce_preview logic (ZTA-90)"
```

---

## Task 4: Create OverlayState with ModalState enum

This is the largest task. It implements modal exclusivity (ZTA-82/ZTA-109/ZTA-110) and absorbs all modal reducers.

**Files:**
- Create: `src/state/overlay.rs`

- [ ] **Step 4.1: Create `src/state/overlay.rs`**

```rust
use anyhow::Result;

use crate::action::{
    Action, CollisionPolicy, Command, FileOperation, MenuId, RefreshTarget,
};
use crate::config::{IconMode, ThemePreset};
use crate::state::dialog::{CollisionState, DialogState};
use crate::state::menu::menu_items_for;
use crate::state::prompt::{resolve_prompt_target, PromptKind, PromptState};
use crate::state::settings::SettingsState;
use crate::state::types::{PaneLayout, MenuItem};
use crate::palette::PaletteState;

/// All modal UI states, mutually exclusive by construction.
/// Only one variant can be active at a time — the type system enforces this.
#[derive(Clone, Debug)]
pub enum ModalState {
    Menu { id: MenuId, selection: usize },
    Prompt(PromptState),
    Dialog(DialogState),
    Collision(CollisionState),
    Palette(PaletteState),
    Settings(SettingsState),
}

#[derive(Clone, Debug, Default)]
pub struct OverlayState {
    pub modal: Option<ModalState>,
}

impl OverlayState {
    /// Close any open modal. All prompt-open arms call this once.
    pub fn close_all(&mut self) {
        self.modal = None;
    }

    pub fn is_menu_open(&self) -> bool {
        matches!(self.modal, Some(ModalState::Menu { .. }))
    }

    pub fn active_menu(&self) -> Option<MenuId> {
        match &self.modal {
            Some(ModalState::Menu { id, .. }) => Some(*id),
            _ => None,
        }
    }

    pub fn menu_selection(&self) -> usize {
        match &self.modal {
            Some(ModalState::Menu { selection, .. }) => *selection,
            _ => 0,
        }
    }

    pub fn menu_items(&self) -> Vec<MenuItem> {
        match &self.modal {
            Some(ModalState::Menu { id, .. }) => menu_items_for(*id),
            _ => vec![],
        }
    }

    pub fn prompt(&self) -> Option<&PromptState> {
        match &self.modal {
            Some(ModalState::Prompt(p)) => Some(p),
            _ => None,
        }
    }

    pub fn dialog(&self) -> Option<&DialogState> {
        match &self.modal {
            Some(ModalState::Dialog(d)) => Some(d),
            _ => None,
        }
    }

    pub fn collision(&self) -> Option<&CollisionState> {
        match &self.modal {
            Some(ModalState::Collision(c)) => Some(c),
            _ => None,
        }
    }

    pub fn palette(&self) -> Option<&PaletteState> {
        match &self.modal {
            Some(ModalState::Palette(p)) => Some(p),
            _ => None,
        }
    }

    pub fn settings(&self) -> Option<&SettingsState> {
        match &self.modal {
            Some(ModalState::Settings(s)) => Some(s),
            _ => None,
        }
    }

    pub fn apply(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        match action {
            // ── Palette ──────────────────────────────────────────────────────
            Action::OpenCommandPalette => {
                if self.modal.is_none() {
                    self.modal = Some(ModalState::Palette(PaletteState::new()));
                }
            }
            Action::CloseCommandPalette => { self.close_all(); }
            Action::PaletteInput(c) => {
                if let Some(ModalState::Palette(p)) = self.modal.as_mut() {
                    p.query.push(*c);
                    p.selection = 0;
                }
            }
            Action::PaletteBackspace => {
                if let Some(ModalState::Palette(p)) = self.modal.as_mut() {
                    p.query.pop();
                    p.selection = 0;
                }
            }
            Action::PaletteMoveDown => {
                if let Some(ModalState::Palette(p)) = self.modal.as_mut() {
                    let entries = crate::palette::all_entries();
                    let matches = crate::palette::filter_entries(&entries, &p.query);
                    if !matches.is_empty() {
                        p.selection = (p.selection + 1).min(matches.len() - 1);
                    }
                }
            }
            Action::PaletteMoveUp => {
                if let Some(ModalState::Palette(p)) = self.modal.as_mut() {
                    p.selection = p.selection.saturating_sub(1);
                }
            }
            Action::PaletteConfirm => {
                if let Some(ModalState::Palette(p)) = self.modal.take() {
                    let entries = crate::palette::all_entries();
                    let matches = crate::palette::filter_entries(&entries, &p.query);
                    if let Some(entry) = matches.get(p.selection) {
                        // Emit DispatchAction so App re-dispatches without
                        // recursive apply() calls.
                        commands.push(Command::DispatchAction(entry.action.clone()));
                    }
                }
            }

            // ── Collision ────────────────────────────────────────────────────
            Action::CollisionCancel => { self.close_all(); }
            Action::CollisionOverwrite => {
                if let Some(ModalState::Collision(c)) = self.modal.take() {
                    commands.push(Command::RunFileOperation {
                        operation: c.operation,
                        refresh: c.refresh,
                        collision: CollisionPolicy::Overwrite,
                    });
                }
            }
            Action::CollisionRename => {
                if let Some(ModalState::Collision(c)) = self.modal.take() {
                    self.modal = Some(ModalState::Prompt(c.rename_prompt()));
                }
            }
            Action::CollisionSkip => { self.close_all(); }

            // ── Dialog ───────────────────────────────────────────────────────
            Action::CloseDialog => { self.close_all(); }
            Action::OpenAboutDialog => {
                // Note: needs theme preset + config_path — passed from AppState::apply_view
            }
            Action::OpenHelpDialog => {
                self.close_all();
                self.modal = Some(ModalState::Dialog(DialogState::help()));
            }

            // ── Menu ─────────────────────────────────────────────────────────
            Action::CloseMenu => { self.close_all(); }
            Action::OpenMenu(menu_id) => {
                self.close_all();
                self.modal = Some(ModalState::Menu { id: *menu_id, selection: 0 });
            }
            Action::MenuActivate => {
                if let Some(ModalState::Menu { id, selection }) = &self.modal {
                    let id = *id;
                    let sel = *selection;
                    if let Some(item) = menu_items_for(id).get(sel).cloned() {
                        self.close_all();
                        commands.push(Command::DispatchAction(item.action.clone()));
                    }
                }
            }
            Action::MenuMnemonic(ch) => {
                if let Some(ModalState::Menu { id, .. }) = &self.modal {
                    let id = *id;
                    if let Some(item) = menu_items_for(id)
                        .into_iter()
                        .find(|item| item.mnemonic.eq_ignore_ascii_case(ch))
                    {
                        self.close_all();
                        commands.push(Command::DispatchAction(item.action.clone()));
                    }
                }
            }
            Action::MenuMoveDown => {
                if let Some(ModalState::Menu { id, selection }) = self.modal.as_mut() {
                    let len = menu_items_for(*id).len();
                    if len > 0 {
                        *selection = (*selection + 1).min(len.saturating_sub(1));
                    }
                }
            }
            Action::MenuMoveUp => {
                if let Some(ModalState::Menu { selection, .. }) = self.modal.as_mut() {
                    *selection = selection.saturating_sub(1);
                }
            }
            Action::MenuNext => {
                if let Some(ModalState::Menu { id, selection }) = self.modal.as_mut() {
                    *id = match *id {
                        MenuId::File => MenuId::Navigate,
                        MenuId::Navigate => MenuId::View,
                        MenuId::View => MenuId::Help,
                        MenuId::Help => MenuId::File,
                    };
                    *selection = 0;
                }
            }
            Action::MenuPrevious => {
                if let Some(ModalState::Menu { id, selection }) = self.modal.as_mut() {
                    *id = match *id {
                        MenuId::File => MenuId::Help,
                        MenuId::Navigate => MenuId::File,
                        MenuId::View => MenuId::Navigate,
                        MenuId::Help => MenuId::View,
                    };
                    *selection = 0;
                }
            }

            // ── File op prompts ──────────────────────────────────────────────
            Action::OpenCopyPrompt
            | Action::OpenDeletePrompt
            | Action::OpenMovePrompt
            | Action::OpenNewDirectoryPrompt
            | Action::OpenNewFilePrompt
            | Action::OpenRenamePrompt => {
                // Building these prompts requires active pane context.
                // AppState::apply_view handles them after calling overlay.close_all().
                self.close_all();
            }

            // ── Prompt input ─────────────────────────────────────────────────
            Action::PromptBackspace => {
                if let Some(ModalState::Prompt(p)) = self.modal.as_mut() {
                    if p.kind != PromptKind::Delete { p.value.pop(); }
                }
            }
            Action::PromptCancel => { self.close_all(); }
            Action::PromptInput(ch) => {
                if let Some(ModalState::Prompt(p)) = self.modal.as_mut() {
                    if p.kind != PromptKind::Delete { p.value.push(*ch); }
                }
            }
            Action::PromptSubmit => {
                // Full submission logic needs pane context — handled in AppState::apply_view
            }

            // ── Settings ─────────────────────────────────────────────────────
            Action::OpenSettingsPanel => {
                self.close_all();
                self.modal = Some(ModalState::Settings(SettingsState::new()));
            }
            Action::CloseSettingsPanel => { self.close_all(); }
            Action::SettingsMoveDown => {
                if let Some(ModalState::Settings(s)) = self.modal.as_mut() {
                    s.selection = s.selection.saturating_add(1);
                }
            }
            Action::SettingsMoveUp => {
                if let Some(ModalState::Settings(s)) = self.modal.as_mut() {
                    s.selection = s.selection.saturating_sub(1);
                }
            }
            Action::SettingsToggleCurrent => {
                // Toggle logic needs full config context — handled in AppState::apply_view
            }
            _ => {}
        }
        Ok(commands)
    }

    /// Called by AppState when a collision arrives from a job result.
    pub fn set_collision(&mut self, collision: CollisionState) {
        self.modal = Some(ModalState::Collision(collision));
    }

    /// Called by AppState::apply_view to open About dialog (needs config context).
    pub fn open_about(&mut self, dialog: DialogState) {
        self.close_all();
        self.modal = Some(ModalState::Dialog(dialog));
    }

    /// Called by AppState::apply_view to open a prompt (needs pane context).
    pub fn open_prompt(&mut self, prompt: PromptState) {
        self.modal = Some(ModalState::Prompt(prompt));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_menu_closes_any_existing_modal() {
        let mut s = OverlayState::default();
        s.modal = Some(ModalState::Palette(PaletteState::new()));
        s.apply(&Action::OpenMenu(MenuId::File)).unwrap();
        assert!(matches!(s.modal, Some(ModalState::Menu { id: MenuId::File, selection: 0 })));
    }

    #[test]
    fn open_settings_closes_any_existing_modal() {
        let mut s = OverlayState::default();
        s.modal = Some(ModalState::Menu { id: MenuId::File, selection: 0 });
        s.apply(&Action::OpenSettingsPanel).unwrap();
        assert!(matches!(s.modal, Some(ModalState::Settings(_))));
    }

    #[test]
    fn close_all_removes_modal() {
        let mut s = OverlayState::default();
        s.modal = Some(ModalState::Settings(SettingsState::new()));
        s.close_all();
        assert!(s.modal.is_none());
    }

    #[test]
    fn menu_activate_emits_dispatch_action() {
        let mut s = OverlayState::default();
        s.modal = Some(ModalState::Menu { id: MenuId::File, selection: 0 });
        let cmds = s.apply(&Action::MenuActivate).unwrap();
        assert!(s.modal.is_none(), "menu should close after activation");
        assert!(!cmds.is_empty(), "should emit DispatchAction command");
        assert!(matches!(cmds[0], Command::DispatchAction(_)));
    }

    #[test]
    fn palette_confirm_emits_dispatch_action_and_closes() {
        let mut s = OverlayState::default();
        s.modal = Some(ModalState::Palette(PaletteState::new()));
        // Empty query with selection 0 — may or may not match, but modal should close
        s.apply(&Action::PaletteConfirm).unwrap();
        assert!(s.modal.is_none());
    }
}
```

- [ ] **Step 4.2: Add `Command::DispatchAction` variant to `src/action.rs`**

Find the `Command` enum in `src/action.rs` and add:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    DispatchAction(Action),   // ADD THIS LINE
    OpenEditor { path: PathBuf },
    PreviewFile { path: PathBuf },
    RunFileOperation {
        operation: FileOperation,
        refresh: Vec<RefreshTarget>,
        collision: CollisionPolicy,
    },
    ScanPane { pane: PaneId, path: PathBuf },
    SaveEditor,
}
```

- [ ] **Step 4.3: Declare the module**

In `src/state/mod.rs`:

```rust
pub mod overlay;
pub use overlay::{ModalState, OverlayState};
```

- [ ] **Step 4.4: Run tests**

```
cargo test state::overlay
```

Expected: 5 tests pass.

- [ ] **Step 4.5: Commit**

```
git add src/state/overlay.rs src/state/mod.rs src/action.rs
git commit -m "feat(state): extract OverlayState with ModalState enum, add DispatchAction (ZTA-89, ZTA-109)"
```

---

## Task 5: Wire sub-states into AppState

Now replace the god struct's fields with the four sub-states and update `apply()` to fan out.

**Files:**
- Modify: `src/state/mod.rs`

- [ ] **Step 5.1: Replace AppState fields**

In `src/state/mod.rs`, replace the `AppState` struct definition. The existing 30+ field struct becomes:

```rust
#[derive(Clone, Debug)]
pub struct AppState {
    pub panes: PaneSetState,
    pub overlay: OverlayState,
    pub preview: PreviewState,
    pub editor: EditorState,
    // Shared config/theme/status — not owned by any single sub-state
    config_path: String,
    config: AppConfig,
    icon_mode: IconMode,
    theme: ResolvedTheme,
    status_message: String,
    last_size: Option<(u16, u16)>,
    redraw_count: u64,
    startup_time_ms: u128,
    last_scan_time_ms: Option<u128>,
    file_operation_status: Option<FileOperationStatus>,
    should_quit: bool,
}
```

Note: `needs_redraw` is intentionally removed.

- [ ] **Step 5.2: Update `bootstrap()`**

```rust
impl AppState {
    pub fn bootstrap(loaded_config: LoadedConfig, started_at: Instant) -> Result<Self> {
        let cwd = fs::current_dir()?;
        let secondary = cwd.parent().map(Path::to_path_buf).unwrap_or_else(|| cwd.clone());
        let resolved_theme = loaded_config.config.resolve_theme();
        let status_bar_label = loaded_config.config.theme.status_bar_label.clone();

        Ok(Self {
            panes: PaneSetState::new(
                PaneState::empty("Left", cwd.clone()),
                PaneState::empty("Right", secondary.clone()),
            ),
            overlay: OverlayState::default(),
            preview: PreviewState::new(
                loaded_config.config.preview_panel_open,
                loaded_config.config.preview_on_selection,
            ),
            editor: EditorState::default(),
            config_path: loaded_config.path.display().to_string(),
            config: loaded_config.config.clone(),
            icon_mode: loaded_config.config.icon_mode,
            theme: resolved_theme.clone(),
            status_message: resolved_theme.warning.unwrap_or_else(|| {
                format!("loading panes | config {}", loaded_config.path.display())
            }),
            last_size: None,
            redraw_count: 0,
            startup_time_ms: started_at.elapsed().as_millis(),
            last_scan_time_ms: None,
            file_operation_status: None,
            should_quit: false,
        })
    }
}
```

- [ ] **Step 5.3: Update `apply()` to fan out**

```rust
pub fn apply(&mut self, action: Action) -> Result<Vec<Command>> {
    let mut commands = Vec::new();
    commands.extend(self.overlay.apply(&action)?);
    commands.extend(self.panes.apply(&action)?);
    commands.extend(self.editor.apply(&action)?);
    commands.extend(self.preview.apply(&action, &self.panes.focus)?);
    commands.extend(self.apply_view(&action)?);
    Ok(commands)
}
```

- [ ] **Step 5.4: Update `apply_view()` to handle remaining actions**

`apply_view()` now handles: theme/layout/quit/resize, `OpenAboutDialog`, all prompt-open arms (which need pane context), `PromptSubmit`, `SettingsToggleCurrent`, `OpenSelectedInEditor`, `CycleFocus`, `FocusPreviewPanel`. Keep these in `mod.rs` as they require cross-sub-state access.

```rust
fn apply_view(&mut self, action: &Action) -> Result<Vec<Command>> {
    let mut commands = Vec::new();
    match action {
        Action::OpenAboutDialog => {
            self.overlay.open_about(DialogState::about(
                self.theme.preset.clone(),
                self.config_path.clone(),
            ));
            self.status_message = String::from("opened about");
        }
        Action::SetPaneLayout(layout) => {
            self.panes.pane_layout = *layout;
            self.config.pane_layout = *layout;
            self.status_message = match layout {
                PaneLayout::SideBySide => String::from("layout set to side-by-side"),
                PaneLayout::Stacked => String::from("layout set to stacked"),
            };
            commands.push(Command::SaveConfig {
                config: self.config.clone(),
                path: std::path::PathBuf::from(&self.config_path),
            });
        }
        Action::SetTheme(preset) => {
            self.theme = ThemePalette::from_preset(*preset);
            self.config.theme.preset = preset.as_str().to_string();
            self.status_message = format!("theme set to {}", preset.as_str());
            commands.push(Command::SaveConfig {
                config: self.config.clone(),
                path: std::path::PathBuf::from(&self.config_path),
            });
        }
        Action::TogglePreviewPanel => {
            self.config.preview_panel_open = self.preview.panel_open;
            commands.push(Command::SaveConfig {
                config: self.config.clone(),
                path: std::path::PathBuf::from(&self.config_path),
            });
        }
        Action::CycleFocus => {
            let preview_available = self.preview.panel_open && self.preview.view.is_some();
            self.panes.focus = match self.panes.focus {
                PaneFocus::Left => {
                    if preview_available {
                        self.status_message = String::from("preview panel focused");
                        PaneFocus::Preview
                    } else {
                        PaneFocus::Right
                    }
                }
                PaneFocus::Right => PaneFocus::Left,
                PaneFocus::Preview => {
                    self.status_message = String::from("focus returned to left pane");
                    PaneFocus::Left
                }
            };
        }
        Action::FocusPreviewPanel => {
            if self.preview.panel_open {
                self.panes.focus = if self.panes.focus == PaneFocus::Preview {
                    self.status_message = String::from("preview focus returned to file pane");
                    PaneFocus::Left
                } else {
                    self.status_message = String::from("preview panel focused");
                    PaneFocus::Preview
                };
            }
        }
        Action::Quit => {
            if self.editor.is_dirty() {
                self.status_message = String::from("unsaved changes: Ctrl+S save, Ctrl+D discard");
            } else {
                self.should_quit = true;
            }
        }
        Action::Resize { width, height } => {
            self.last_size = Some((*width, *height));
            self.status_message = format!("resized to {width}x{height}");
        }
        Action::OpenSelectedInEditor => {
            if let Some(entry) = self.panes.active_pane().selected_entry() {
                if entry.kind == EntryKind::File {
                    commands.push(Command::OpenEditor { path: entry.path.clone() });
                    self.status_message = format!("opening {}", entry.path.display());
                }
            }
        }
        Action::OpenCopyPrompt => {
            if let Some(entry) = self.panes.active_pane().selected_entry() {
                let suggested = self.panes.inactive_pane().cwd.join(&entry.name);
                self.overlay.open_prompt(PromptState::with_value(
                    PromptKind::Copy, "Copy",
                    self.panes.inactive_pane().cwd.clone(),
                    Some(entry.path.clone()),
                    suggested.display().to_string(),
                ));
                self.status_message = String::from("enter copy destination");
            }
        }
        Action::OpenDeletePrompt => {
            if let Some(entry) = self.panes.active_pane().selected_entry() {
                self.overlay.open_prompt(PromptState::with_value(
                    PromptKind::Delete, "Delete",
                    self.panes.active_pane().cwd.clone(),
                    Some(entry.path.clone()),
                    String::new(),
                ));
                self.status_message = format!("confirm delete for {}", entry.name);
            }
        }
        Action::OpenMovePrompt => {
            if let Some(entry) = self.panes.active_pane().selected_entry() {
                let suggested = self.panes.inactive_pane().cwd.join(&entry.name);
                self.overlay.open_prompt(PromptState::with_value(
                    PromptKind::Move, "Move",
                    self.panes.inactive_pane().cwd.clone(),
                    Some(entry.path.clone()),
                    suggested.display().to_string(),
                ));
                self.status_message = String::from("enter move destination");
            }
        }
        Action::OpenNewDirectoryPrompt => {
            self.overlay.open_prompt(PromptState::new(
                PromptKind::NewDirectory, "New Directory",
                self.panes.active_pane().cwd.clone(),
            ));
            self.status_message = String::from("enter directory name");
        }
        Action::OpenNewFilePrompt => {
            self.overlay.open_prompt(PromptState::new(
                PromptKind::NewFile, "New File",
                self.panes.active_pane().cwd.clone(),
            ));
            self.status_message = String::from("enter file name");
        }
        Action::OpenRenamePrompt => {
            if let Some(entry) = self.panes.active_pane().selected_entry() {
                self.overlay.open_prompt(PromptState::with_value(
                    PromptKind::Rename, "Rename",
                    self.panes.active_pane().cwd.clone(),
                    Some(entry.path.clone()),
                    entry.name.clone(),
                ));
                self.status_message = String::from("edit the new name");
            }
        }
        Action::PromptSubmit => {
            if let Some(ModalState::Prompt(prompt)) = &self.overlay.modal {
                let prompt = prompt.clone();
                if prompt.kind != PromptKind::Delete && prompt.value.trim().is_empty() {
                    self.status_message = String::from("name cannot be empty");
                } else {
                    let value = prompt.value.trim().to_string();
                    let target_path = resolve_prompt_target(&prompt, &value);
                    let refresh = self.refresh_targets_for_prompt(prompt.kind, &target_path);
                    let operation = self.build_file_operation(&prompt, &target_path);
                    if let Some(operation) = operation {
                        commands.push(Command::RunFileOperation {
                            operation,
                            refresh,
                            collision: CollisionPolicy::Fail,
                        });
                    }
                    self.overlay.close_all();
                }
            }
        }
        Action::CloseEditor => {
            if self.editor.is_dirty() {
                self.status_message = String::from("unsaved changes: Ctrl+S save, Ctrl+D discard");
            }
        }
        _ => {}
    }
    Ok(commands)
}
```

- [ ] **Step 5.5: Update `apply_job_result()`**

```rust
pub fn apply_job_result(&mut self, result: JobResult) {
    match result {
        JobResult::DirectoryScanned { pane, path, entries, elapsed_ms } => {
            self.panes.pane_mut(pane).cwd = path.clone();
            self.panes.pane_mut(pane).set_entries(entries);
            self.status_message = format!("refreshed {} in {elapsed_ms} ms", path.display());
            self.last_scan_time_ms = Some(elapsed_ms);
        }
        JobResult::FileOperationCompleted { message, refreshed, elapsed_ms } => {
            self.overlay.close_all();
            self.file_operation_status = None;
            for pane in refreshed {
                self.panes.pane_mut(pane.pane).cwd = pane.path;
                self.panes.pane_mut(pane.pane).set_entries(pane.entries);
            }
            self.status_message = format!("{message} in {elapsed_ms} ms");
            self.last_scan_time_ms = Some(elapsed_ms);
        }
        JobResult::FileOperationCollision { operation, refresh, path, elapsed_ms } => {
            self.file_operation_status = None;
            self.overlay.set_collision(CollisionState { operation, refresh, path: path.clone() });
            self.status_message = format!("destination exists: {}", path.display());
            self.last_scan_time_ms = Some(elapsed_ms);
        }
        JobResult::FileOperationProgress { status } => {
            self.file_operation_status = Some(status);
        }
        JobResult::JobFailed { path, message, elapsed_ms, .. } => {
            self.file_operation_status = None;
            self.status_message = format!("job failed for {}: {message}", path.display());
            self.last_scan_time_ms = Some(elapsed_ms);
        }
        JobResult::PreviewLoaded { path, view } => {
            self.preview.apply_job_loaded(path, view);
        }
    }
}
```

- [ ] **Step 5.6: Update public accessors to delegate to sub-states**

Replace the existing accessors at the bottom of `mod.rs`:

```rust
// Pane accessors — delegate to PaneSetState
pub fn left_pane(&self) -> &PaneState { &self.panes.left }
pub fn right_pane(&self) -> &PaneState { &self.panes.right }
pub fn focus(&self) -> PaneId { self.panes.focused_pane_id() }
pub fn active_pane(&self) -> &PaneState { self.panes.active_pane() }
pub fn active_pane_mut(&mut self) -> &mut PaneState { self.panes.active_pane_mut() }
pub fn inactive_pane(&self) -> &PaneState { self.panes.inactive_pane() }
pub fn pane_layout(&self) -> PaneLayout { self.panes.pane_layout }
pub fn is_editor_focused(&self) -> bool { self.editor.is_open() && self.panes.focus != PaneFocus::Preview }
pub fn active_pane_title(&self) -> &str {
    self.panes.active_pane().selected_entry().map(|e| e.name.as_str()).unwrap_or("")
}

// Overlay accessors — delegate to OverlayState
pub fn active_menu(&self) -> Option<MenuId> { self.overlay.active_menu() }
pub fn menu_items(&self) -> Vec<MenuItem> { self.overlay.menu_items() }
pub fn menu_selection(&self) -> usize { self.overlay.menu_selection() }
pub fn prompt(&self) -> Option<&PromptState> { self.overlay.prompt() }
pub fn dialog(&self) -> Option<&DialogState> { self.overlay.dialog() }
pub fn collision(&self) -> Option<&CollisionState> { self.overlay.collision() }
pub fn palette(&self) -> Option<&PaletteState> { self.overlay.palette() }
pub fn settings(&self) -> Option<&SettingsState> { self.overlay.settings() }
pub fn is_menu_open(&self) -> bool { self.overlay.is_menu_open() }
pub fn is_prompt_open(&self) -> bool { self.overlay.prompt().is_some() }
pub fn is_dialog_open(&self) -> bool { self.overlay.dialog().is_some() }
pub fn is_collision_open(&self) -> bool { self.overlay.collision().is_some() }
pub fn is_palette_open(&self) -> bool { self.overlay.palette().is_some() }
pub fn is_settings_open(&self) -> bool { self.overlay.settings().is_some() }

// Preview accessors — delegate to PreviewState
pub fn preview_view(&self) -> Option<&(std::path::PathBuf, crate::preview::ViewBuffer)> {
    self.preview.view.as_ref()
}
pub fn is_preview_panel_open(&self) -> bool { self.preview.panel_open }
pub fn is_preview_focused(&self) -> bool { self.panes.focus == PaneFocus::Preview }

// Editor accessor — delegate to EditorState
pub fn editor(&self) -> Option<&EditorBuffer> { self.editor.buffer.as_ref() }
pub fn editor_mut(&mut self) -> Option<&mut EditorBuffer> { self.editor.buffer.as_mut() }
pub fn open_editor(&mut self, buffer: EditorBuffer) { self.editor.open(buffer); }
pub fn mark_editor_saved(&mut self) {
    if let Some(e) = self.editor.buffer.as_mut() { e.is_dirty = false; }
}

// Theme/config
pub fn theme(&self) -> &ResolvedTheme { &self.theme }
pub fn icon_mode(&self) -> IconMode { self.icon_mode }
pub fn status_line(&self) -> &str { &self.status_message }
pub fn should_quit(&self) -> bool { self.should_quit }
pub fn set_error_status(&mut self, msg: impl Into<String>) { self.status_message = msg.into(); }
```

- [ ] **Step 5.7: Update `app.rs` — remove `needs_redraw` check**

In `src/app.rs`, find `App::run()` and replace:

```rust
// OLD:
while !self.state.should_quit() {
    if self.state.needs_redraw() {
        terminal.draw(|frame| ui::render(frame, &mut self.state))?;
        self.state.mark_drawn();
    }
    self.process_next_event()?;
}

// NEW:
while !self.state.should_quit() {
    terminal.draw(|frame| ui::render(frame, &mut self.state))?;
    self.process_next_event()?;
}
```

- [ ] **Step 5.8: Handle `Command::DispatchAction` in `app.rs`**

In `App::execute_command()`, add the new variant:

```rust
fn execute_command(&mut self, command: Command) -> Result<()> {
    match command {
        Command::DispatchAction(action) => {
            self.dispatch(action)?;
        }
        Command::OpenEditor { path } => { /* existing */ }
        // ... rest of existing arms
    }
    Ok(())
}
```

- [ ] **Step 5.9: Build to verify compilation**

```
cargo build 2>&1 | head -50
```

Fix any compilation errors. Common ones at this stage:
- Missing imports — add `use crate::state::{PaneSetState, EditorState, PreviewState, OverlayState};` to `state/mod.rs`
- Missing `PromptState`/`DialogState` imports in `overlay.rs`
- `needs_redraw()` and `mark_drawn()` method calls in `app.rs` — remove them

- [ ] **Step 5.10: Run all tests**

```
cargo test
```

Expected: All existing tests pass. The key routing tests in `app.rs` still use the old `RouteContext` — they will be updated in Wave 2A. They should still compile since `RouteContext` still exists in `app.rs`.

- [ ] **Step 5.11: Commit**

```
git add src/state/mod.rs src/app.rs
git commit -m "feat(state): wire sub-states into AppState, remove needs_redraw, add DispatchAction handler (ZTA-91, ZTA-92, ZTA-110)"
```

---

## Task 6: Minor fixes

**Files:**
- Modify: `src/state/mod.rs`
- Modify: `src/state/overlay.rs`

- [ ] **Step 6.1: Fix theme preset string round-trip**

In `src/state/mod.rs`, change `settings_entries()` to use the enum directly:

```rust
pub fn settings_entries(&self) -> Vec<SettingsEntry> {
    vec![
        SettingsEntry {
            label: "Theme",
            value: self.theme.preset.as_str().to_string(),
            hint: "Enter",
            field: SettingsField::Theme(self.theme.preset),  // use enum, not string parse
        },
        // ... rest unchanged
    ]
}
```

The `ResolvedTheme` struct needs `pub preset: ThemePreset` (enum) rather than `pub preset: String`. Update `config.rs` accordingly:

In `src/config.rs`, find `ResolvedTheme` and change its `preset` field type from `String` to `ThemePreset`. Update `ThemePalette::from_preset()` and callers as needed.

- [ ] **Step 6.2: Build and test**

```
cargo test
```

Expected: All tests pass.

- [ ] **Step 6.3: Commit**

```
git add src/state/mod.rs src/config.rs
git commit -m "fix(state): store ThemePreset enum directly, remove string round-trip (ZTA-93, ZTA-94)"
```

---

## Task 7: Final verification

- [ ] **Step 7.1: Run full test suite**

```
cargo test
```

Expected: All tests pass.

- [ ] **Step 7.2: Verify `state/mod.rs` line count**

```
wc -l src/state/mod.rs
```

Expected: Under 500 lines (down from 2,169).

- [ ] **Step 7.3: Verify sub-state files exist**

```
ls src/state/
```

Expected: `mod.rs`, `pane_set.rs`, `editor_state.rs`, `preview_state.rs`, `overlay.rs`, `dialog.rs`, `menu.rs`, `prompt.rs`, `settings.rs`, `types.rs`

- [ ] **Step 7.4: Final commit**

```
git add -A
git commit -m "chore(state): Wave 1A complete — AppState decomposed, ModalState enforces exclusivity"
```

---

## Merge Notes for Wave 2A

After this plan is merged, Wave 2A (input routing) can begin. The key interfaces it will use:
- `OverlayState::modal: Option<ModalState>` — for computing `FocusLayer`
- `AppState::focus_layer()` — to be added in Wave 2A based on `self.overlay.modal`
- `Command::DispatchAction(Action)` — already added in Task 4
- `ModalState` variants — must match exactly what Wave 2A pattern-matches on
