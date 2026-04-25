# Flyout Submenu Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `Themes` out of the top-level menu bar and make it a flyout submenu under `View → Themes...`, using `→`/`←` keyboard navigation.

**Architecture:** Extend `ModalState::Menu` with `flyout: Option<(MenuId, usize)>`. State handlers auto-open the flyout when `↑`/`↓` lands on a trigger item. The renderer draws a second popup to the right of the parent popup when the flyout is active.

**Tech Stack:** Rust stable, ratatui, crossterm — no new dependencies.

---

## File Map

| File | Change |
|------|--------|
| `src/action.rs` | Add `MenuEnterFlyout`, `MenuExitFlyout` variants; reroute `Right` and `Left` keys |
| `src/state/menu.rs` | Remove `MenuId::Themes` from both `menu_tabs()` branches |
| `src/state/overlay.rs` | Extend `ModalState::Menu`, add accessors, update all action handlers |
| `src/ui/overlay.rs` | Show `►` on trigger items; add `render_flyout_popup` |
| `src/ui/mod.rs` | Call `render_flyout_popup` when flyout state is present |

---

## Task 1: Add `MenuEnterFlyout` / `MenuExitFlyout` actions and reroute keys

**Files:**
- Modify: `src/action.rs`

### Background

`from_menu_key_event` currently maps:
```
Left  → MenuPrevious
Right → MenuNext     (also Tab → MenuNext)
```

New mappings:
```
Right → MenuEnterFlyout   (if flyout available, open it; else tab forward — handled in state)
Left  → MenuExitFlyout    (if flyout open, close it; else tab backward — handled in state)
Tab   → MenuNext          (unchanged)
```

There is also an existing test (line 1578-1581 in `src/action.rs`) that asserts `Right → MenuNext`. That test must be updated.

- [ ] **Step 1.1: Add the two new variants to the `Action` enum**

In `src/action.rs`, locate the `Action` enum. After `MenuMoveUp` (around line 91), add:

```rust
    MenuEnterFlyout,
    MenuExitFlyout,
```

- [ ] **Step 1.2: Reroute `Right` and `Left` in `from_menu_key_event`**

Locate `from_menu_key_event` (line ~1038). Change:

```rust
            KeyCode::Left => Some(Self::MenuPrevious),
            KeyCode::Right | KeyCode::Tab => Some(Self::MenuNext),
```

to:

```rust
            KeyCode::Left => Some(Self::MenuExitFlyout),
            KeyCode::Right => Some(Self::MenuEnterFlyout),
            KeyCode::Tab => Some(Self::MenuNext),
```

- [ ] **Step 1.3: Update the existing key-routing test**

Locate the test asserting `Right → MenuNext` (around line 1578-1581). Change the expected value:

```rust
        assert_eq!(
            Action::from_menu_key_event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), &keymap),
            Some(Action::MenuEnterFlyout)
        );
```

Add a new assertion immediately after for `Left → MenuExitFlyout`:

```rust
        assert_eq!(
            Action::from_menu_key_event(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), &keymap),
            Some(Action::MenuExitFlyout)
        );
```

Add a new assertion for `Tab → MenuNext` (confirm it still works):

```rust
        assert_eq!(
            Action::from_menu_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), &keymap),
            Some(Action::MenuNext)
        );
```

- [ ] **Step 1.4: Run tests to verify they compile and the key-routing tests pass**

```bash
cargo test --lib action -- --nocapture 2>&1 | tail -20
```

Expected: all action tests pass.

- [ ] **Step 1.5: Commit**

```bash
git add src/action.rs
git commit -m "feat(action): add MenuEnterFlyout/MenuExitFlyout; reroute Right/Left keys"
```

---

## Task 2: Remove `Themes` from top-level `menu_tabs()`

**Files:**
- Modify: `src/state/menu.rs`

### Background

`menu_tabs()` currently returns `Themes` as a top-level tab in both the editor and pane branches. It must be removed from both so the menu bar shows: `File | Navigate | View | Help` (pane) and `File | Edit | Search | View | Help` (editor).

`MenuId::Themes` itself must stay in `src/action.rs` — it is still used as a submenu target.

- [ ] **Step 2.1: Write failing tests**

In `src/state/menu.rs`, locate the `#[cfg(test)]` block. Add:

```rust
    #[test]
    fn themes_not_in_pane_menu_tabs() {
        let tabs = menu_tabs(MenuContext::Pane);
        assert!(
            !tabs.iter().any(|t| t.id == MenuId::Themes),
            "Themes must not appear as a top-level tab in pane context"
        );
    }

    #[test]
    fn themes_not_in_editor_menu_tabs() {
        let tabs = menu_tabs(MenuContext::Editor);
        assert!(
            !tabs.iter().any(|t| t.id == MenuId::Themes),
            "Themes must not appear as a top-level tab in editor context"
        );
    }
```

- [ ] **Step 2.2: Run tests to confirm they fail**

```bash
cargo test --lib menu::tests::themes_not_in_pane_menu_tabs -- --exact --nocapture 2>&1 | tail -10
```

Expected: FAILED.

- [ ] **Step 2.3: Remove the `Themes` entry from the editor branch of `menu_tabs()`**

In the editor branch (first `vec![...]`, around line 38-68), remove:

```rust
            MenuTab {
                id: MenuId::Themes,
                label: " Themes ",
                mnemonic: 't',
            },
```

- [ ] **Step 2.4: Remove the `Themes` entry from the pane branch of `menu_tabs()`**

In the pane branch (second `vec![...]`, around line 70-98), remove:

```rust
            MenuTab {
                id: MenuId::Themes,
                label: " Themes ",
                mnemonic: 't',
            },
```

- [ ] **Step 2.5: Run tests to confirm they pass**

```bash
cargo test --lib menu -- --nocapture 2>&1 | tail -20
```

Expected: all menu tests pass.

- [ ] **Step 2.6: Commit**

```bash
git add src/state/menu.rs
git commit -m "feat(menu): remove Themes from top-level menu_tabs; it is now a flyout submenu"
```

---

## Task 3: Extend `ModalState::Menu` with `flyout` field and add accessors

**Files:**
- Modify: `src/state/overlay.rs`

### Background

`ModalState::Menu` currently has `{ id: MenuId, selection: usize }`. We add `flyout: Option<(MenuId, usize)>` where the tuple is `(submenu_id, submenu_selection)`. When `flyout` is `Some`, keyboard focus is in the submenu.

All existing pattern matches on `Menu { id, selection }` must be updated to also destructure `flyout` (or use `..`).

- [ ] **Step 3.1: Write a failing test for the new accessor**

In `src/state/overlay.rs`, find the `#[cfg(test)]` block (or create one). Add:

```rust
    #[test]
    fn open_menu_has_no_flyout() {
        let mut s = OverlayState::default();
        s.apply(&Action::OpenMenu(MenuId::View)).unwrap();
        assert!(s.menu_flyout().is_none(), "newly opened menu must have no flyout");
    }
```

- [ ] **Step 3.2: Run test to confirm it fails (menu_flyout method does not exist)**

```bash
cargo test --lib overlay::tests::open_menu_has_no_flyout -- --exact --nocapture 2>&1 | tail -10
```

Expected: compile error — `method not found`.

- [ ] **Step 3.3: Extend `ModalState::Menu` with the `flyout` field**

In `src/state/overlay.rs`, change:

```rust
    Menu {
        id: MenuId,
        selection: usize,
    },
```

to:

```rust
    Menu {
        id: MenuId,
        selection: usize,
        /// Active flyout submenu: (submenu_id, submenu_selection).
        /// When Some, keyboard navigation targets the flyout.
        flyout: Option<(MenuId, usize)>,
    },
```

- [ ] **Step 3.4: Update `OpenMenu` handler to set `flyout: None`**

In `apply`, locate `Action::OpenMenu(menu_id)`:

```rust
            Action::OpenMenu(menu_id) => {
                self.close_all();
                self.modal = Some(ModalState::Menu {
                    id: *menu_id,
                    selection: 0,
                    flyout: None,
                });
            }
```

- [ ] **Step 3.5: Fix all other pattern matches that now fail to compile**

There are several `ModalState::Menu { id, selection }` matches. Add `flyout: _` or `flyout` to each one. Use grep to find them all:

```bash
grep -n "ModalState::Menu {" src/state/overlay.rs
```

For `is_menu_open`, `active_menu`, `menu_selection`, `menu_items`: add `..` to destructure remaining fields:

```rust
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
            Some(ModalState::Menu { selection, flyout, .. }) => {
                // When flyout is active, return flyout selection for the caller
                // (callers that need parent selection access raw state directly)
                if flyout.is_some() {
                    // Return parent selection for the menu bar highlight
                    *selection
                } else {
                    *selection
                }
            }
            _ => 0,
        }
    }

    pub fn menu_items(&self) -> Vec<MenuItem> {
        match &self.modal {
            Some(ModalState::Menu { id, .. }) => menu_items_for(*id, self.menu_context),
            _ => vec![],
        }
    }
```

Also update `MenuMoveDown`, `MenuMoveUp`, `MenuNext`, `MenuPrevious`, `MenuActivate`, `MenuClickItem`, `MenuSetSelection`, `MenuMnemonic` to include `flyout` in their destructure patterns. At this stage, just add `flyout: _` or `..` — behavior changes come in Task 4.

- [ ] **Step 3.6: Add new accessors**

After `menu_items()`, add:

```rust
    /// Returns the active flyout state as `(submenu_id, submenu_selection)` if open.
    pub fn menu_flyout(&self) -> Option<(MenuId, usize)> {
        match &self.modal {
            Some(ModalState::Menu { flyout, .. }) => *flyout,
            _ => None,
        }
    }

    /// Returns the flyout submenu items if a flyout is open.
    pub fn menu_flyout_items(&self) -> Vec<MenuItem> {
        match &self.modal {
            Some(ModalState::Menu { flyout, .. }) => {
                if let Some((flyout_id, _)) = flyout {
                    menu_items_for(*flyout_id, self.menu_context)
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    /// Returns the flyout submenu selection index if a flyout is open.
    pub fn menu_flyout_selection(&self) -> usize {
        match &self.modal {
            Some(ModalState::Menu { flyout, .. }) => {
                flyout.map(|(_, sel)| sel).unwrap_or(0)
            }
            _ => 0,
        }
    }
```

- [ ] **Step 3.7: Run tests to confirm `open_menu_has_no_flyout` passes and nothing regressed**

```bash
cargo test --workspace 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 3.8: Commit**

```bash
git add src/state/overlay.rs
git commit -m "feat(state): extend ModalState::Menu with flyout field; add flyout accessors"
```

---

## Task 4: Implement flyout state-transition handlers

**Files:**
- Modify: `src/state/overlay.rs`

### Background

Helper needed: check if a given `MenuItem` is a flyout trigger.

```rust
fn flyout_trigger(item: &MenuItem) -> Option<MenuId> {
    if let Action::OpenMenu(id) = item.action {
        Some(id)
    } else {
        None
    }
}
```

Key behaviors:
- `MenuMoveDown`/`MenuMoveUp`: move selection in flyout when open, else move parent and auto-open flyout if new item is trigger.
- `MenuActivate` / `MenuEnterFlyout`: activate flyout item when open; open flyout if on trigger; else activate parent item.
- `MenuExitFlyout`: close flyout if open; else do `MenuPrevious` (switch to previous tab).
- `MenuNext` / `MenuPrevious`: clear flyout then switch tab.
- `MenuMnemonic`: route to flyout when open, else parent.
- `MenuClickItem`: only operates on parent items (no mouse interaction with flyout for now).

- [ ] **Step 4.1: Write failing tests**

In `src/state/overlay.rs`, add to the tests module. First, a helper to open the View menu (which has the Themes trigger item at index 3 in pane context):

```rust
    fn open_view_menu() -> OverlayState {
        let mut s = OverlayState::default();
        s.apply(&Action::OpenMenu(MenuId::View)).unwrap();
        s
    }
```

Now add test cases:

```rust
    #[test]
    fn move_down_to_themes_trigger_opens_flyout() {
        let mut s = open_view_menu();
        // View menu pane items (0=Toggle Hidden, 1=Settings, 2=Layout:Side, 3=Layout:Stacked, 4=Themes..., 5=Toggle Details)
        // Navigate down 4 times to land on "Themes..."
        s.apply(&Action::MenuMoveDown).unwrap(); // sel=1
        s.apply(&Action::MenuMoveDown).unwrap(); // sel=2
        s.apply(&Action::MenuMoveDown).unwrap(); // sel=3
        s.apply(&Action::MenuMoveDown).unwrap(); // sel=4 (Themes...)
        let flyout = s.menu_flyout();
        assert!(flyout.is_some(), "flyout must open when landing on Themes trigger");
        assert_eq!(flyout.unwrap().0, MenuId::Themes);
        assert_eq!(flyout.unwrap().1, 0, "flyout selection starts at 0");
    }

    #[test]
    fn move_up_away_from_trigger_closes_flyout() {
        let mut s = open_view_menu();
        // Navigate to Themes... (index 4)
        for _ in 0..4 { s.apply(&Action::MenuMoveDown).unwrap(); }
        assert!(s.menu_flyout().is_some());
        // Move back up
        s.apply(&Action::MenuMoveUp).unwrap();
        assert!(s.menu_flyout().is_none(), "flyout must close when leaving trigger item");
    }

    #[test]
    fn move_down_while_flyout_open_moves_flyout_selection() {
        let mut s = open_view_menu();
        for _ in 0..4 { s.apply(&Action::MenuMoveDown).unwrap(); }
        assert_eq!(s.menu_flyout().unwrap().1, 0);
        s.apply(&Action::MenuMoveDown).unwrap();
        assert_eq!(s.menu_flyout().unwrap().1, 1, "down key should advance flyout selection");
    }

    #[test]
    fn enter_flyout_on_trigger_opens_flyout() {
        let mut s = open_view_menu();
        for _ in 0..4 { s.apply(&Action::MenuMoveDown).unwrap(); }
        let flyout_before = s.menu_flyout();
        s.apply(&Action::MenuEnterFlyout).unwrap();
        // flyout stays open, focus now in flyout
        assert_eq!(s.menu_flyout(), flyout_before, "flyout stays open after MenuEnterFlyout");
    }

    #[test]
    fn enter_flyout_not_on_trigger_switches_tab() {
        let mut s = open_view_menu();
        // selection=0, not a trigger item
        let initial_menu = s.active_menu();
        s.apply(&Action::MenuEnterFlyout).unwrap();
        assert_ne!(s.active_menu(), initial_menu, "MenuEnterFlyout on non-trigger must switch tab");
        assert!(s.menu_flyout().is_none());
    }

    #[test]
    fn exit_flyout_when_open_collapses_flyout() {
        let mut s = open_view_menu();
        for _ in 0..4 { s.apply(&Action::MenuMoveDown).unwrap(); }
        assert!(s.menu_flyout().is_some());
        s.apply(&Action::MenuExitFlyout).unwrap();
        assert!(s.menu_flyout().is_none(), "MenuExitFlyout must close flyout");
        assert!(s.is_menu_open(), "parent menu must remain open");
    }

    #[test]
    fn exit_flyout_when_closed_switches_prev_tab() {
        let mut s = open_view_menu();
        let initial_menu = s.active_menu();
        s.apply(&Action::MenuExitFlyout).unwrap();
        assert_ne!(s.active_menu(), initial_menu, "MenuExitFlyout with no flyout switches to prev tab");
    }

    #[test]
    fn menu_activate_on_flyout_item_dispatches_action() {
        let mut s = open_view_menu();
        for _ in 0..4 { s.apply(&Action::MenuMoveDown).unwrap(); }
        assert!(s.menu_flyout().is_some());
        let cmds = s.apply(&Action::MenuActivate).unwrap();
        assert!(!cmds.is_empty(), "MenuActivate on flyout item must dispatch action");
        assert!(s.modal.is_none(), "menu must close after activating flyout item");
    }

    #[test]
    fn menu_next_clears_flyout() {
        let mut s = open_view_menu();
        for _ in 0..4 { s.apply(&Action::MenuMoveDown).unwrap(); }
        assert!(s.menu_flyout().is_some());
        s.apply(&Action::MenuNext).unwrap();
        assert!(s.menu_flyout().is_none(), "MenuNext must clear flyout");
    }

    #[test]
    fn mnemonic_in_flyout_activates_flyout_item() {
        let mut s = open_view_menu();
        for _ in 0..4 { s.apply(&Action::MenuMoveDown).unwrap(); }
        assert!(s.menu_flyout().is_some());
        // 'z' is mnemonic for "Theme: Zeta (default)" in Themes submenu
        let cmds = s.apply(&Action::MenuMnemonic('z')).unwrap();
        assert!(!cmds.is_empty(), "mnemonic in flyout must dispatch theme action");
        assert!(s.modal.is_none(), "menu closes after mnemonic in flyout");
    }
```

- [ ] **Step 4.2: Run tests to confirm they fail (expected before implementation)**

```bash
cargo test --lib overlay::tests -- --nocapture 2>&1 | tail -30
```

Expected: several failures.

- [ ] **Step 4.3: Add the `flyout_trigger` helper at module scope (outside `impl`)**

At the top of the `apply` method section in `src/state/overlay.rs`, before the `impl OverlayState` block or as a private free function inside the module:

```rust
/// Returns the submenu `MenuId` if the given item is a flyout trigger, else `None`.
fn flyout_trigger(item: &MenuItem) -> Option<MenuId> {
    if let Action::OpenMenu(id) = item.action {
        Some(id)
    } else {
        None
    }
}
```

- [ ] **Step 4.4: Update `MenuMoveDown` handler**

Replace:

```rust
            Action::MenuMoveDown => {
                if let Some(ModalState::Menu { id, selection }) = self.modal.as_mut() {
                    let len = menu_items_for(*id, self.menu_context).len();
                    if len > 0 {
                        *selection = (*selection + 1).min(len.saturating_sub(1));
                    }
                }
            }
```

With:

```rust
            Action::MenuMoveDown => {
                // Snapshot Copy fields to avoid borrow issues when mutating later.
                let snapshot = if let Some(ModalState::Menu { id, selection, flyout }) = &self.modal {
                    Some((*id, *selection, *flyout))
                } else {
                    None
                };
                if let Some((id, selection, flyout_opt)) = snapshot {
                    if let Some((flyout_id, flyout_sel)) = flyout_opt {
                        let flyout_len = menu_items_for(flyout_id, self.menu_context).len();
                        if flyout_len > 0 {
                            let new_sel = (flyout_sel + 1).min(flyout_len.saturating_sub(1));
                            if let Some(ModalState::Menu { flyout, .. }) = self.modal.as_mut() {
                                if let Some((_, s)) = flyout.as_mut() {
                                    *s = new_sel;
                                }
                            }
                        }
                    } else {
                        let items = menu_items_for(id, self.menu_context);
                        let len = items.len();
                        if len > 0 {
                            let new_sel = (selection + 1).min(len.saturating_sub(1));
                            let trigger = items.get(new_sel).and_then(flyout_trigger);
                            if let Some(ModalState::Menu { selection, flyout, .. }) = self.modal.as_mut() {
                                *selection = new_sel;
                                *flyout = trigger.map(|sub_id| (sub_id, 0));
                            }
                        }
                    }
                }
            }
```

- [ ] **Step 4.5: Update `MenuMoveUp` handler**

Replace:

```rust
            Action::MenuMoveUp => {
                if let Some(ModalState::Menu { selection, .. }) = self.modal.as_mut() {
                    *selection = selection.saturating_sub(1);
                }
            }
```

With:

```rust
            Action::MenuMoveUp => {
                let snapshot = if let Some(ModalState::Menu { id, selection, flyout }) = &self.modal {
                    Some((*id, *selection, *flyout))
                } else {
                    None
                };
                if let Some((id, selection, flyout_opt)) = snapshot {
                    if let Some((_, flyout_sel)) = flyout_opt {
                        if flyout_sel > 0 {
                            if let Some(ModalState::Menu { flyout, .. }) = self.modal.as_mut() {
                                if let Some((_, s)) = flyout.as_mut() {
                                    *s -= 1;
                                }
                            }
                        } else {
                            // At top of flyout — collapse back to parent
                            if let Some(ModalState::Menu { flyout, .. }) = self.modal.as_mut() {
                                *flyout = None;
                            }
                        }
                    } else {
                        let new_sel = selection.saturating_sub(1);
                        let items = menu_items_for(id, self.menu_context);
                        let trigger = items.get(new_sel).and_then(flyout_trigger);
                        if let Some(ModalState::Menu { selection, flyout, .. }) = self.modal.as_mut() {
                            *selection = new_sel;
                            *flyout = trigger.map(|sub_id| (sub_id, 0));
                        }
                    }
                }
            }
```

- [ ] **Step 4.6: Update `MenuActivate` handler**

Replace:

```rust
            Action::MenuActivate => {
                if let Some(ModalState::Menu { id, selection }) = &self.modal {
                    let id = *id;
                    let sel = *selection;
                    if let Some(item) = menu_items_for(id, self.menu_context).get(sel).cloned() {
                        self.close_all();
                        commands.push(Command::DispatchAction(item.action.clone()));
                    }
                }
            }
```

With:

```rust
            Action::MenuActivate => {
                // Snapshot Copy fields first to avoid simultaneous borrow issues.
                let snapshot = if let Some(ModalState::Menu { id, selection, flyout }) = &self.modal {
                    Some((*id, *selection, *flyout))
                } else {
                    None
                };
                if let Some((id, sel, flyout_opt)) = snapshot {
                    if let Some((flyout_id, flyout_sel)) = flyout_opt {
                        if let Some(item) = menu_items_for(flyout_id, self.menu_context)
                            .get(flyout_sel)
                            .cloned()
                        {
                            self.close_all();
                            commands.push(Command::DispatchAction(item.action.clone()));
                        }
                    } else {
                        let items = menu_items_for(id, self.menu_context);
                        if let Some(item) = items.get(sel).cloned() {
                            if let Some(sub_id) = flyout_trigger(&item) {
                                if let Some(ModalState::Menu { flyout, .. }) = self.modal.as_mut() {
                                    *flyout = Some((sub_id, 0));
                                }
                            } else {
                                self.close_all();
                                commands.push(Command::DispatchAction(item.action.clone()));
                            }
                        }
                    }
                }
            }
```

- [ ] **Step 4.7: Add `MenuEnterFlyout` handler**

After the `MenuActivate` handler, add:

```rust
            Action::MenuEnterFlyout => {
                let snapshot = if let Some(ModalState::Menu { id, selection, flyout }) = &self.modal {
                    Some((*id, *selection, flyout.is_some()))
                } else {
                    None
                };
                if let Some((id, sel, flyout_open)) = snapshot {
                    if flyout_open {
                        // Already in flyout — act as Activate on flyout item
                        commands.push(Command::DispatchAction(Action::MenuActivate));
                    } else {
                        let items = menu_items_for(id, self.menu_context);
                        if let Some(sub_id) = items.get(sel).and_then(flyout_trigger) {
                            if let Some(ModalState::Menu { flyout, .. }) = self.modal.as_mut() {
                                *flyout = Some((sub_id, 0));
                            }
                        } else {
                            // Not a trigger — forward as MenuNext (tab switch)
                            commands.push(Command::DispatchAction(Action::MenuNext));
                        }
                    }
                }
            }
```

- [ ] **Step 4.8: Add `MenuExitFlyout` handler**

After the `MenuEnterFlyout` handler, add:

```rust
            Action::MenuExitFlyout => {
                if let Some(ModalState::Menu { flyout, .. }) = self.modal.as_mut() {
                    if flyout.is_some() {
                        *flyout = None;
                    } else {
                        // No flyout open — fall through to MenuPrevious (switch prev tab)
                        commands.push(Command::DispatchAction(Action::MenuPrevious));
                    }
                }
            }
```

- [ ] **Step 4.9: Update `MenuNext` handler to clear flyout**

Replace:

```rust
            Action::MenuNext => {
                if let Some(ModalState::Menu { id, selection }) = self.modal.as_mut() {
                    let tabs = crate::state::menu::menu_tabs(self.menu_context);
                    if let Some(pos) = tabs.iter().position(|tab| tab.id == *id) {
                        *id = tabs[(pos + 1) % tabs.len()].id;
                    }
                    *selection = 0;
                }
            }
```

With:

```rust
            Action::MenuNext => {
                if let Some(ModalState::Menu { id, selection, flyout }) = self.modal.as_mut() {
                    *flyout = None;
                    let tabs = crate::state::menu::menu_tabs(self.menu_context);
                    if let Some(pos) = tabs.iter().position(|tab| tab.id == *id) {
                        *id = tabs[(pos + 1) % tabs.len()].id;
                    }
                    *selection = 0;
                }
            }
```

- [ ] **Step 4.10: Update `MenuPrevious` handler to clear flyout**

Replace:

```rust
            Action::MenuPrevious => {
                if let Some(ModalState::Menu { id, selection }) = self.modal.as_mut() {
                    let tabs = crate::state::menu::menu_tabs(self.menu_context);
                    if let Some(pos) = tabs.iter().position(|tab| tab.id == *id) {
                        *id = tabs[(pos + tabs.len() - 1) % tabs.len()].id;
                    }
                    *selection = 0;
                }
            }
```

With:

```rust
            Action::MenuPrevious => {
                if let Some(ModalState::Menu { id, selection, flyout }) = self.modal.as_mut() {
                    *flyout = None;
                    let tabs = crate::state::menu::menu_tabs(self.menu_context);
                    if let Some(pos) = tabs.iter().position(|tab| tab.id == *id) {
                        *id = tabs[(pos + tabs.len() - 1) % tabs.len()].id;
                    }
                    *selection = 0;
                }
            }
```

- [ ] **Step 4.11: Update `MenuMnemonic` handler to route to flyout when open**

Replace:

```rust
            Action::MenuMnemonic(ch) => {
                if let Some(ModalState::Menu { id, .. }) = &self.modal {
                    let id = *id;
                    if let Some(item) = menu_items_for(id, self.menu_context)
                        .into_iter()
                        .find(|item| item.mnemonic.eq_ignore_ascii_case(ch))
                    {
                        self.close_all();
                        commands.push(Command::DispatchAction(item.action.clone()));
                    }
                }
            }
```

With:

```rust
            Action::MenuMnemonic(ch) => {
                let search_id = if let Some(ModalState::Menu { id, flyout, .. }) = &self.modal {
                    Some(flyout.map(|(fid, _)| fid).unwrap_or(*id))
                } else {
                    None
                };
                if let Some(search_id) = search_id {
                    if let Some(item) = menu_items_for(search_id, self.menu_context)
                        .into_iter()
                        .find(|item| item.mnemonic.eq_ignore_ascii_case(ch))
                    {
                        self.close_all();
                        commands.push(Command::DispatchAction(item.action.clone()));
                    }
                }
            }
```

- [ ] **Step 4.12: Update `MenuSetSelection` and `MenuClickItem` to include `flyout: _` in patterns**

For `MenuSetSelection`:
```rust
            Action::MenuSetSelection(index) => {
                if let Some(ModalState::Menu { selection, .. }) = &mut self.modal {
                    *selection = *index;
                }
            }
```

For `MenuClickItem` (no changes to behavior, just fix pattern):
```rust
            Action::MenuClickItem(index) => {
                if let Some(ModalState::Menu { id, .. }) = &self.modal {
                    let id = *id;
                    let items = menu_items_for(id, self.menu_context);
                    if *index < items.len() {
                        let item = items[*index].clone();
                        self.close_all();
                        commands.push(Command::DispatchAction(item.action.clone()));
                    }
                }
            }
```

(Both already use `..` so no structural change needed — verify they compile.)

- [ ] **Step 4.13: Run all tests**

```bash
cargo test --workspace 2>&1 | tail -30
```

Expected: all tests pass including the new flyout state tests.

- [ ] **Step 4.14: Commit**

```bash
git add src/state/overlay.rs
git commit -m "feat(state): implement flyout submenu state transitions"
```

---

## Task 5: Update renderer — `►` on trigger items and flyout popup

**Files:**
- Modify: `src/ui/overlay.rs`
- Modify: `src/ui/mod.rs`

### Background

Two renderer changes:
1. In `render_menu_popup`, trigger items (those with `Action::OpenMenu(_)`) show `►` instead of their shortcut string.
2. A new `render_flyout_popup` function renders the submenu popup to the right of the parent popup.

In `src/ui/mod.rs`, after `render_menu_popup` is called, call `render_flyout_popup` when `state.menu_flyout()` is `Some`.

- [ ] **Step 5.1: Update `render_menu_popup` to show `►` on trigger items**

In `src/ui/overlay.rs`, locate `render_menu_popup`. Find the row-building closure:

```rust
            let row = format!(" {:<label_width$} {}", item.label, item.shortcut);
```

Change to:

```rust
            let shortcut_display = if matches!(item.action, Action::OpenMenu(_)) {
                "►"
            } else {
                item.shortcut
            };
            let row = format!(" {:<label_width$} {}", item.label, shortcut_display);
```

Note: `Action` is already imported in this file via `use crate::action::MenuId;`. You need to also import `Action` or use the full path. Add to the imports:

```rust
use crate::action::{Action, MenuId};
```

(Replace the existing `use crate::action::MenuId;` with this.)

- [ ] **Step 5.2: Add the `render_flyout_popup` function to `src/ui/overlay.rs`**

After `render_menu_popup`, add:

```rust
/// Render a flyout submenu popup to the right of `parent_area`.
/// If the flyout would overflow the right edge of `area`, it flips to the left of the parent.
pub fn render_flyout_popup(
    frame: &mut Frame<'_>,
    area: Rect,
    parent_area: Rect,
    items: &[MenuItem],
    selection: usize,
    palette: ThemePalette,
) {
    if items.is_empty() {
        return;
    }

    let width = menu_popup_width(items);
    let height = items.len() as u16 + 2;

    let flyout_x = if parent_area.x + parent_area.width + width <= area.x + area.width {
        parent_area.x + parent_area.width
    } else {
        parent_area.x.saturating_sub(width)
    };

    let flyout_area = Rect {
        x: flyout_x,
        y: parent_area.y,
        width,
        height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.prompt_border))
        .style(Style::default().bg(palette.surface_bg));
    let inner = block.inner(flyout_area);
    frame.render_widget(Clear, flyout_area);
    frame.render_widget(block, flyout_area);

    let rows = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let selected = index == selection;
            let base_style = if selected {
                Style::default()
                    .fg(palette.menu_fg)
                    .bg(palette.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(palette.text_primary)
                    .bg(palette.surface_bg)
            };
            let content_width = inner.width as usize;
            let shortcut_width = item.shortcut.chars().count();
            let label_width = content_width.saturating_sub(shortcut_width + 2).max(1);
            let row = format!(" {:<label_width$} {}", item.label, item.shortcut);
            let pad = content_width.saturating_sub(row.chars().count());
            ListItem::new(Line::from(vec![Span::styled(
                format!("{}{}", row, " ".repeat(pad)),
                base_style,
            )]))
        })
        .collect::<Vec<_>>();

    let list = List::new(rows);
    let mut state = ListState::default();
    state.select(Some(selection.min(items.len().saturating_sub(1))));
    frame.render_stateful_widget(list, inner, &mut state);
}
```

- [ ] **Step 5.3: Update `src/ui/mod.rs` to call `render_flyout_popup`**

In `src/ui/mod.rs`, locate the `render_menu_popup` call block (around line 291-320). After the existing call:

```rust
        render_menu_popup(
            frame,
            areas[1],
            menu,
            &state.menu_items(),
            state.menu_selection(),
            palette,
            menu_ctx,
        );
```

Add:

```rust
        // Render flyout submenu if open
        if let Some((_, flyout_sel)) = state.menu_flyout() {
            let flyout_items = state.menu_flyout_items();
            if !flyout_items.is_empty() {
                crate::ui::overlay::render_flyout_popup(
                    frame,
                    areas[1],
                    rect,
                    &flyout_items,
                    flyout_sel,
                    palette,
                );
            }
        }
```

Note: `rect` is the `menu_popup_rect` value (`Rect { x: popup_x, y: areas[1].y, width, height }`), which is already computed in this block. Ensure `rect` is in scope here (it's assigned as `menu_popup_rect = Some(rect)` at line ~310 — use `rect` directly since it's a `Copy` type).

- [ ] **Step 5.4: Add imports to `src/ui/overlay.rs` if needed**

Verify `Clear` is imported. The current imports include:
```rust
use ratatui::widgets::{
    block::Title, Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ...
```

`Clear` is already present. ✓

- [ ] **Step 5.5: Build to verify compilation**

```bash
cargo check 2>&1 | tail -20
```

Expected: no errors.

- [ ] **Step 5.6: Run full test suite**

```bash
cargo test --workspace 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 5.7: Commit**

```bash
git add src/ui/overlay.rs src/ui/mod.rs
git commit -m "feat(ui): render flyout popup and show ► on trigger menu items"
```

---

## Task 6: Pre-PR Validation

- [ ] **Step 6.1: Format check**

```bash
cargo fmt --all -- --check
```

If failures: run `cargo fmt --all` then re-check.

- [ ] **Step 6.2: Clippy**

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -30
```

Expected: no warnings.

- [ ] **Step 6.3: Full test suite**

```bash
cargo test --workspace 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 6.4: Commit formatting fixes if needed**

```bash
git add -A && git commit -m "style: apply rustfmt and clippy fixes"
```
