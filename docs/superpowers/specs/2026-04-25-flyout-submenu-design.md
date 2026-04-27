# Flyout Submenu Design

## Problem

`Themes` currently appears as a standalone top-level tab in the menu bar, which looks out of place. It should live as a submenu under `View`, opening as a flyout panel beside the parent popup — consistent with how nested menus behave in GUI applications.

## Proposed Approach

Extend the existing `ModalState::Menu` with an optional `flyout` field. When the user navigates to a menu item whose action is `Action::OpenMenu(sub_id)`, the flyout opens automatically beside the parent popup. No second modal is created; both parent and flyout are rendered as a single composed UI.

## State

```rust
// in src/state/overlay.rs
Menu {
    id: MenuId,
    selection: usize,
    flyout: Option<(MenuId, usize)>,  // (submenu id, submenu selection)
}
```

`flyout` is `None` when no submenu is open. When `Some`, keyboard navigation targets the flyout, not the parent.

## Navigation Behaviour

| Key | Flyout closed | Flyout open |
|-----|---------------|-------------|
| `↑` / `↓` | Move parent selection; auto-open flyout if landing on trigger item | Move flyout selection |
| `→` / `Enter` on trigger | Enter flyout focus (same as auto-open) | Activate flyout item |
| `←` / `Esc` | Close entire menu | Collapse flyout; return focus to parent |
| `Esc` from parent | Close entire menu | — |
| `Tab` / `Shift+Tab` | Switch top-level menu tab | Switch top-level tab, close flyout |
| Mnemonic char | Match item in parent | Match item in flyout |

**Auto-open rule:** whenever `↑`/`↓` moves the parent selection and the newly selected item's action is `Action::OpenMenu(sub_id)`, the flyout opens immediately with `selection: 0`. Moving away closes it.

## Rendering

```
View popup open:         View popup + Themes flyout open:
┌─────────────────┐      ┌─────────────────┐ ┌──────────────────────────┐
│ Toggle Hidden   │      │ Toggle Hidden   │ │ Theme: Zeta (default)  Z │
│ Settings        │      │ Settings        │ │ Theme: Neon            E │
│ Layout: Side ▶  │      │ Layout: Side    │ │ Theme: Monochrome      O │
│ Themes...      ►│      │▶Themes...      ►│ │▶Theme: Matrix          M │
│ Toggle Details  │      │ Toggle Details  │ │ Theme: Norton          N │
└─────────────────┘      └─────────────────┘ │ Theme: Fjord           F │
                                              │ ...                      │
                                              └──────────────────────────┘
```

- Flyout-trigger items render `►` on the right instead of a shortcut string.
- Flyout popup x = parent_popup_x + parent_popup_width; y = menu_bar_y + trigger_row_index + 1.
- If the flyout would extend off the right edge of the terminal, render it to the left of the parent instead.
- When flyout is focused, parent items are not dimmed (both remain fully readable).

## Files Changed

| File | Change |
|------|--------|
| `src/state/overlay.rs` | Extend `ModalState::Menu` with `flyout`; update all menu action handlers; add `MenuEnterFlyout` / `MenuExitFlyout` handlers; add `menu_flyout()` accessor |
| `src/action.rs` | Add `MenuEnterFlyout`, `MenuExitFlyout` variants; map `→` / `←` in `from_menu_key_event` |
| `src/ui/overlay.rs` | Extend `render_menu_popup` to accept and render optional flyout popup; trigger items show `►` |
| `src/ui/mod.rs` | Pass `state.menu_flyout()` to `render_menu_popup` |
| `src/state/menu.rs` | Remove `MenuId::Themes` from `menu_tabs()` (both contexts); remove from `tab_is_relevant()` |

## Out of Scope

- More than one level of nesting (no flyout-of-flyout).
- Mouse hover triggering flyout (mouse click on a trigger item opens/closes the flyout).
- Adding other submenus beyond Themes for now.

## Testing

- Unit: `MenuMoveDown` into trigger item sets `flyout = Some(...)`.
- Unit: `MenuMoveDown` away from trigger item clears `flyout = None`.
- Unit: `MenuExitFlyout` when flyout open → flyout cleared, parent focus restored.
- Unit: `MenuActivate` when flyout focused → flyout item action dispatched, menu closed.
- Unit: `MenuNext`/`MenuPrevious` closes flyout and switches top-level tab.
- Render: flyout popup x positioned to right of parent; flips left when it would overflow.
