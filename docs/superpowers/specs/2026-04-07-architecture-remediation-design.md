# Zeta — Architecture Remediation & Feature Foundation Design

**Date:** 2026-04-07
**Status:** Approved
**Jira Epic:** ZTA-77 (Architecture Remediation), ZTA-75 (Editor & Markdown), ZTA-76 (Mouse Support)
**Approach:** Option C — Full structural refactor executed in two parallel-agent waves before further feature expansion.

---

## Background & Motivation

Zeta is a dual-pane TUI file manager written in Rust using ratatui 0.29 + crossterm 0.28. A full review of the codebase identified the following blockers to the planned feature roadmap (fully-fledged text editor with markdown support, full mouse interaction, advanced settings panels):

| File | Lines | Problem |
|---|---|---|
| `state/mod.rs` | 2,169 | God object: 30+ fields, 10+ reducers, owns all app state |
| `ui.rs` | 1,763 | Monolith: every widget rendered flat in one file |
| `action.rs` | 761 | Input routing mixed with domain type definitions |

Six structural defects were identified that will actively resist the planned features:

1. **Single background worker thread** — scan, file-op, and preview jobs share one sequential thread; a large copy blocks everything.
2. **Modal priority duplicated** — `app.rs:route_key_event()` checks context flags; `state/mod.rs:apply()` checks `command_palette.is_some()` again independently.
3. **Recursive `self.apply()`** — `MenuActivate` and `PaletteConfirm` call `apply()` recursively, re-running all 10 reducers.
4. **Config I/O on main thread** — `SetTheme`, `SetPaneLayout`, `TogglePreviewPanel` call `config.save()` synchronously in reducers, blocking the render loop.
5. **`needs_redraw` scattered across ~60 sites** — noise; a file manager redraws on every meaningful state change regardless.
6. **Modal exclusivity unenforced** — six separate `Option<T>` modal fields; each prompt-open arm manually clears the others with 4 copy-pasted lines; no compiler guarantee.

Additional minor defects: dead code (`let _ = is_editor_focused`), hardcoded menu popup X offsets, public fields on otherwise-private `AppState`, theme preset string round-trip, `EXDEV_ERROR = -1` on Windows.

The TUI stack (ratatui + crossterm) is retained. It is the correct foundation. The ecosystem gaps (proper text editor, markdown rendering, mouse support) are addressed via `tui-textarea` and `tui-markdown` crates.

---

## Execution Strategy

Two waves of parallel agent work in isolated git worktrees. Each wave runs up to three agents simultaneously on non-overlapping source files, then merges before the next wave begins.

### Wave 1 — Three parallel agents

| Agent | Jira Stories | Primary files owned |
|---|---|---|
| A | ZTA-78, ZTA-82 | `src/state/mod.rs` and new `src/state/*.rs` modules |
| B | ZTA-79 | `src/ui.rs` → `src/ui/*.rs` modules |
| C | ZTA-81, ZTA-83, ZTA-84 | `src/jobs.rs`, `src/editor.rs`, preview pipeline |

**Merge risk:** Agent B (ui split) imports state types that Agent A renames. Mitigation: Agent A publishes new sub-state public interfaces as the first commit of its branch; Agent B codes against them and adapts at merge time.

### Wave 2 — Two parallel agents (after Wave 1 merges)

| Agent | Jira Stories | Depends on |
|---|---|---|
| A | ZTA-80 | Wave 1 A — new sub-state structure and OverlayState |
| B | ZTA-85, ZTA-86 | Wave 1 B — `LayoutCache` in ui modules; Wave 1 C — stable multi-worker |

---

## Design Section 1 — AppState Decomposition

### Structure

`AppState` becomes a thin coordinator owning four focused sub-states plus shared config/theme/status fields:

```
AppState
├── panes:   PaneSetState    — left/right PaneState, focus, pane_layout, nav history
├── overlay: OverlayState    — single Option<ModalState> enum
├── preview: PreviewState    — preview_view, panel_open, preview_on_selection
├── editor:  EditorState     — Option<TextArea>, dirty flag, path
└── (direct) config, theme, icon_mode, status_message, startup_time, last_scan_time
```

### File layout

```
src/state/
├── mod.rs           — AppState coordinator, apply() fan-out, public accessors
├── pane_set.rs      — PaneSetState + reduce_pane()
├── editor_state.rs  — EditorState + reduce_editor()
├── overlay.rs       — OverlayState + ModalState enum + all modal reducers
└── preview_state.rs — PreviewState + reduce_preview()
```

Existing `src/state/dialog.rs`, `prompt.rs`, `settings.rs`, `menu.rs`, `types.rs` are absorbed into `overlay.rs` or kept as focused helper modules imported by it.

### apply() fan-out

```rust
impl AppState {
    pub fn apply(&mut self, action: Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        commands.extend(self.overlay.apply(&action)?);
        commands.extend(self.panes.apply(&action)?);
        commands.extend(self.editor.apply(&action)?);
        commands.extend(self.preview.apply(&action)?);
        commands.extend(self.apply_view(&action)?);  // theme, layout, quit, resize
        Ok(commands)
    }
}
```

No sub-state calls `apply()` on another. Commands bubble up to `App` in `app.rs` which is the sole executor.

Job results (`AppEvent::Job`) are handled separately via `AppState::apply_job_result(result)` — a direct state mutation path that does not go through `apply()`. This keeps the two event paths distinct: user actions flow through `apply()`, background worker results flow through `apply_job_result()`.

### Modal exclusivity — ModalState enum

Replaces six separate `Option<T>` fields:

```rust
pub enum ModalState {
    Menu(MenuState),
    Prompt(PromptState),
    Dialog(DialogState),
    Collision(CollisionState),
    Palette(PaletteState),
    Settings(SettingsState),
}
```

`OverlayState` holds `modal: Option<ModalState>`. Opening any modal is a single assignment. The compiler prevents two modals coexisting — no manual clearing required.

### needs_redraw removal

The flag is deleted. `App::run()` redraws unconditionally after every `apply()` call:

```rust
while !self.state.should_quit() {
    terminal.draw(|frame| ui::render(frame, &mut self.state, &mut self.layout))?;
    self.process_next_event()?;
}
```

A file manager responds to every event; the flag saved nothing and added ~60 maintenance points.

### close_all_modals() helper

```rust
impl OverlayState {
    fn close_all(&mut self) {
        self.modal = None;
    }
}
```

All prompt-open reducer arms call `self.overlay.close_all()` once instead of manually clearing four fields.

### Minor fixes in this pass

- `preview_view` and `preview_panel_open` made private; public accessor methods added.
- `ThemePreset` enum stored directly in `AppState`; string round-trip removed from `settings_entries()`.

---

## Design Section 2 — ui.rs Module Split

### File layout

```
src/ui/
├── mod.rs       — render() compositor + LayoutCache computation, ~60 lines
├── pane.rs      — render_pane(), render_item(), pane_chrome_style(), PaneChrome
├── editor.rs    — render_editor(), render_code_view(), CodeViewRenderArgs
├── preview.rs   — render_preview_panel(), render_wrapped_preview_view(), wrap_preview_line()
├── menu.rs      — render_menu_bar(), render_menu_popup(), menu_spans(), top_bar_logo_spans()
├── overlay.rs   — render_prompt(), render_dialog(), render_collision_dialog(), render_command_palette()
└── settings.rs  — render_settings_panel()
```

### LayoutCache

Computed once per frame in `mod.rs` and stored on `App` via a mutable reference passed into `render()`:

```rust
// app.rs
terminal.draw(|frame| ui::render(frame, &mut self.state, &mut self.layout))?;
```

`render()` overwrites `self.layout` each frame before submodules use it, so `App` always holds the layout that matches the last drawn frame. Mouse hit-testing reads from `self.layout` between frames.

`LayoutCache` is passed into every submodule render call:

```rust
pub struct LayoutCache {
    pub menu_bar:    Rect,
    pub pane_left:   Rect,
    pub pane_right:  Rect,
    pub tools_area:  Option<Rect>,
    pub status_bar:  Rect,
    pub menu_labels: [(MenuId, u16, u16); 4],  // (id, x_start, x_end) per menu label
}
```

Submodules stop recomputing layouts independently. Wave 2 mouse routing reads from `LayoutCache` directly.

### Menu popup position fix

`render_menu_popup()` currently uses hardcoded offsets (`x+1`, `x+8`, `x+19`, `x+26`). Replace with:

```rust
let x = layout.menu_labels.iter()
    .find(|(id, _, _)| *id == menu)
    .map(|(_, x_start, _)| *x_start)
    .unwrap_or(area.x + 1);
```

Label x-ranges are measured during `render_menu_bar()` and stored in `LayoutCache.menu_labels`.

---

## Design Section 3 — Input Routing Redesign

### FocusLayer enum

Replaces `RouteContext` with 9 booleans:

```rust
pub enum FocusLayer {
    Pane,
    Editor,
    Preview,
    Modal(ModalKind),
}

pub enum ModalKind {
    Menu,
    Prompt,
    Dialog,
    Collision,
    Palette,
    Settings,
}
```

`AppState` exposes:

```rust
pub fn focus_layer(&self) -> FocusLayer {
    match &self.overlay.modal {
        Some(ModalState::Palette(_))   => FocusLayer::Modal(ModalKind::Palette),
        Some(ModalState::Collision(_)) => FocusLayer::Modal(ModalKind::Collision),
        Some(ModalState::Prompt(_))    => FocusLayer::Modal(ModalKind::Prompt),
        Some(ModalState::Dialog(_))    => FocusLayer::Modal(ModalKind::Dialog),
        Some(ModalState::Menu(_))      => FocusLayer::Modal(ModalKind::Menu),
        Some(ModalState::Settings(_))  => FocusLayer::Modal(ModalKind::Settings),
        None => match self.panes.focus {
            PaneFocus::Preview => FocusLayer::Preview,
            _ if self.editor.is_open() => FocusLayer::Editor,
            _ => FocusLayer::Pane,
        },
    }
}
```

Modal priority is encoded once, in this method. `app.rs` and `state/mod.rs` both consume it — no duplication.

### route_key_event() redesign

```rust
fn route_key_event(event: KeyEvent, keymap: &RuntimeKeymap, layer: FocusLayer) -> Option<Action> {
    match layer {
        FocusLayer::Modal(ModalKind::Palette)   => route_palette(event),
        FocusLayer::Modal(ModalKind::Collision) => route_collision(event),
        FocusLayer::Modal(ModalKind::Prompt)    => route_prompt(event),
        FocusLayer::Modal(ModalKind::Dialog)    => route_dialog(event),
        FocusLayer::Modal(ModalKind::Menu)      => route_menu(event),
        FocusLayer::Modal(ModalKind::Settings)  => route_settings(event),
        FocusLayer::Editor                      => route_editor(event, keymap),
        FocusLayer::Preview                     => route_preview(event, keymap),
        FocusLayer::Pane                        => route_pane(event, keymap),
    }
}
```

Each `route_*` function is a small focused match block. The monolithic `from_key_event_with_settings()` is deleted.

### Recursive apply() elimination

`MenuActivate` and `PaletteConfirm` currently call `self.apply(item.action)` recursively. Replace with:

```rust
// In OverlayState::apply():
Action::MenuActivate => {
    if let Some(item) = self.active_menu_item() {
        self.modal = None;
        return Ok(vec![Command::DispatchAction(item.action.clone())]);
    }
}
```

`App::execute_command()` handles `Command::DispatchAction(action)` by calling `self.dispatch(action)` — one level of indirection, no re-entrant reducer calls.

### Dead code removed in this pass

- `from_key_event()` wrapper deleted; call site updated to use `from_key_event_with_settings()` directly (or the new `route_key_event()`).
- `let _ = is_editor_focused;` removed along with its parameter.
- `EXDEV_ERROR` on Windows fixed: `cfg(windows)` branch uses `ERROR_NOT_SAME_DEVICE = 17i32`.

---

## Design Section 4 — Multi-Worker Job System

### Architecture

Three dedicated worker threads replace the single sequential worker:

```
App
├── scan_tx/rx     → ScanWorker     — directory listing, lightweight, frequent
├── file_op_tx/rx  → FileOpWorker   — copy/move/delete/rename/config-save
└── preview_tx/rx  → PreviewWorker  — file read + syntect/tui-markdown highlight
```

`next_event()` uses `crossbeam_channel::select!` to drain whichever result channel is ready before falling through to terminal event polling:

```rust
fn next_event(&mut self) -> Result<Option<AppEvent>> {
    select! {
        recv(self.scan_results)    -> r => return Ok(Some(AppEvent::Job(r?))),
        recv(self.file_op_results) -> r => return Ok(Some(AppEvent::Job(r?))),
        recv(self.preview_results) -> r => return Ok(Some(AppEvent::Job(r?))),
        default => {}
    }
    // fall through to crossterm event polling...
}
```

### Config save — fire and forget

New `JobRequest::SaveConfig { config: AppConfig, path: PathBuf }` variant on the file-op worker. Reducers send it and return immediately. Failures surface as `JobResult::ConfigSaveFailed { message }` which sets the status bar.

### Future extensibility

The preview worker's channel boundary makes it straightforward to upgrade to a thread pool (rayon) for large file highlighting without touching the rest of the architecture.

---

## Design Section 5 — Editor & Markdown Integration

### tui-textarea replaces EditorBuffer

`src/editor.rs` is deleted. `EditorState` holds:

```rust
pub struct EditorState {
    pub textarea: Option<TextArea<'static>>,
    pub path:     Option<PathBuf>,
    pub is_dirty: bool,
}
```

Editor action variants collapse from ~15 down to 3:

```rust
Action::EditorInput(KeyEvent)   // raw event passed to TextArea::input()
Action::EditorSave
Action::EditorClose
```

`tui-textarea` handles all cursor movement, word-wise navigation, undo/redo, visual selection, clipboard, and search internally. Syntax highlighting bridges through a syntect `Highlighter` impl on `TextArea`.

### tui-markdown in the preview pipeline

`jobs.rs::load_preview_content()` gains a markdown branch:

```rust
if extension == Some("md") {
    let styled = tui_markdown::from_str(&text);   // → ratatui::text::Text
    return ViewBuffer::from_ratatui_text(styled);
}
```

`ViewBuffer` gains `from_ratatui_text(text: Text)` — stores pre-rendered styled lines, bypasses syntect. The preview panel renders them without any new render path.

Split edit/preview pane for live markdown authoring is a follow-on feature, not part of this remediation.

---

## Design Section 6 — Mouse Support

### Infrastructure

`TerminalSession::enter()`:
```rust
execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
```

`TerminalSession::drop()`:
```rust
let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture);
```

`AppEvent` gains:
```rust
AppEvent::Mouse(crossterm::event::MouseEvent)
```

`next_event()` matches `Event::Mouse(m)` and returns `AppEvent::Mouse(m)`. `handle_event()` routes it through `dispatch_mouse()`.

### Hit testing

`App` stores the last computed `LayoutCache`. `dispatch_mouse()` calls:

```rust
fn mouse_event_to_action(event: MouseEvent, layout: &LayoutCache, state: &AppState) -> Option<Action> {
    let (col, row) = (event.column, event.row);

    // ratatui Rect has no contains() method; use this helper throughout:
    // fn rect_contains(r: Rect, col: u16, row: u16) -> bool {
    //     col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height
    // }

    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if rect_contains(layout.pane_left, col, row)  { return pane_click(PaneId::Left, row, layout, state); }
            if rect_contains(layout.pane_right, col, row) { return pane_click(PaneId::Right, row, layout, state); }
            if rect_contains(layout.menu_bar, col, row)   { return menu_bar_click(col, layout); }
            if layout.tools_area.map_or(false, |a| rect_contains(a, col, row)) { return tools_click(col, row, state); }
            None
        }
        MouseEventKind::ScrollDown => scroll_action(col, row, layout, ScrollDir::Down),
        MouseEventKind::ScrollUp   => scroll_action(col, row, layout, ScrollDir::Up),
        _ => None,
    }
}
```

### Interactions

| Mouse event | Action produced |
|---|---|
| Left click in pane area | `FocusPane(id)` + `SetSelection(row - pane_rect.y + scroll_offset)` |
| Double click file/dir | `EnterSelection` |
| Left click menu bar label | `OpenMenu(id)` — from `layout.menu_labels` x-ranges |
| Scroll in preview panel | `ScrollPreviewDown/Up` |
| Scroll in editor panel | passed to `TextArea::scroll()` |

---

## Non-Goals (explicit scope boundary)

The following are **not** part of this remediation and are deferred to subsequent epics:

- Split edit/preview pane for live markdown authoring
- Image/media preview (binary preview shows size label as today)
- Drag-and-drop file operations
- Multiple editor tabs
- Plugin/extension system

---

## Jira Ticket Map

### Epic ZTA-77 — Architecture Remediation
| Story | Tasks |
|---|---|
| ZTA-78 Decompose AppState | ZTA-87, ZTA-88, ZTA-89, ZTA-90, ZTA-91, ZTA-92, ZTA-93, ZTA-94 |
| ZTA-79 Split ui.rs | ZTA-95, ZTA-96, ZTA-97, ZTA-98, ZTA-99, ZTA-100, ZTA-101 |
| ZTA-80 Input routing redesign | ZTA-102, ZTA-103, ZTA-104, ZTA-105, ZTA-106, ZTA-107, ZTA-108 |
| ZTA-81 Multi-worker jobs | (tasks inline in Wave 1 Agent C) |
| ZTA-82 Modal exclusivity | ZTA-109, ZTA-110 |

### Epic ZTA-75 — Editor & Markdown
| Story | Tasks |
|---|---|
| ZTA-83 tui-textarea | ZTA-111, ZTA-112, ZTA-113 |
| ZTA-84 tui-markdown | ZTA-114, ZTA-115 |

### Epic ZTA-76 — Mouse Support
| Story | Tasks |
|---|---|
| ZTA-85 Mouse infrastructure | ZTA-116, ZTA-117, ZTA-118 |
| ZTA-86 Mouse interactions | ZTA-119, ZTA-120, ZTA-121 |

---

## Success Criteria

- `state/mod.rs` is under 300 lines; no single file in `src/state/` exceeds 500 lines
- `ui.rs` is deleted; `src/ui/mod.rs` is under 80 lines
- `route_key_event()` is a single clean `match` on `FocusLayer` with no boolean flags
- A large file copy does not block directory scan or preview loading
- Two modals cannot be open simultaneously (compiler-enforced)
- The editor supports undo/redo, visual selection, word-wise movement, and clipboard
- `.md` files render with heading/bold/italic/code formatting in the preview panel
- Left click focuses panes and selects list items; scroll wheel works in preview and editor
- All existing tests pass; key routing tests updated to use `FocusLayer`
