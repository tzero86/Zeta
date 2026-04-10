# Wave 4C — Editor Fullscreen Mode + Scroll Sync + Preview Focus/Toggle

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the embedded editor comfortable for real editing sessions. Three connected improvements:
1. **Full-window editor** — toggle that hides both file panes and expands the editor + markdown preview to the entire terminal. Press the same key to return.
2. **Scroll sync** — when the editor cursor moves vertically, the markdown preview scrolls proportionally so the visible content stays in context.
3. **Markdown preview focus and toggle** — Tab (within editor context) shifts keyboard focus to the preview panel so you can scroll it independently; Ctrl+M toggles the preview panel on/off when a markdown file is open.

**Architecture:**
- `AppState` gains `editor_fullscreen: bool`. When `true`, `ui::render` skips the pane split and gives the entire content area to the tools panel.
- `EditorState` gains `markdown_preview_scroll: usize` and `markdown_preview_focused: bool`. The scroll is updated proportionally every time the editor cursor moves.
- `Action::ToggleEditorFullscreen`, `Action::ToggleMarkdownPreview`, `Action::FocusMarkdownPreview` added to `action.rs`.
- `FocusLayer::MarkdownPreview` added to `state/types.rs` and `AppState::focus_layer()`. Arrow keys in this layer emit `ScrollMarkdownPreview{Up,Down}` instead of editor movement.
- `ui/mod.rs` reads `editor_fullscreen` and `markdown_preview_scroll`; passes scroll to `render_markdown_preview`.
- `ui/markdown.rs` `render_markdown_preview` gains a `scroll: usize` parameter.

**No new dependencies.**

**Jira:** ZTA-128 (ZTA-133 through ZTA-137)

**Wave dependency:** Starts AFTER Wave 4B. Requires `is_markdown_file`, `render_markdown_preview`, `EditorState`.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/state/types.rs` | Add `FocusLayer::MarkdownPreview` |
| Modify | `src/state/mod.rs` | `editor_fullscreen`, `focus_layer()`, new action handlers |
| Modify | `src/state/editor_state.rs` | `markdown_preview_scroll`, `markdown_preview_focused`, scroll update logic |
| Modify | `src/action.rs` | Three new actions; arrow key routing in `from_editor_key_event` |
| Modify | `src/ui/mod.rs` | Fullscreen layout branch; pass scroll to renderer |
| Modify | `src/ui/markdown.rs` | `scroll: usize` param on `render_markdown_preview` |

---

## Keybindings

| Key | Context | Action |
|---|---|---|
| `F11` | Editor focused | `ToggleEditorFullscreen` |
| `Ctrl+M` | Editor focused, `.md` file | `ToggleMarkdownPreview` |
| `Tab` | Editor focused, markdown preview visible | `FocusMarkdownPreview` |
| `Esc` | `MarkdownPreview` layer | Return focus to editor |
| `Up` / `Down` | `MarkdownPreview` layer | `ScrollMarkdownPreviewUp/Down` |
| `PgUp` / `PgDn` | `MarkdownPreview` layer | `ScrollMarkdownPreviewPageUp/Down` |

---

## Task 1: Add FocusLayer::MarkdownPreview

**Files:** `src/state/types.rs`, `src/state/mod.rs`

- [ ] **Step 1.1: Add to FocusLayer enum**

```rust
pub enum FocusLayer {
    Pane,
    Editor,
    Preview,
    MarkdownPreview,  // ← new
    Modal(ModalKind),
}
```

- [ ] **Step 1.2: Update `AppState::focus_layer()`**

After the `Editor` arm, add:

```rust
if self.editor.markdown_preview_focused {
    return FocusLayer::MarkdownPreview;
}
```

- [ ] **Step 1.3: Update `route_key_event` in `app.rs`**

Add a `FocusLayer::MarkdownPreview` arm:

```rust
FocusLayer::MarkdownPreview => {
    Action::from_markdown_preview_key_event(key_event)
}
```

And add `from_markdown_preview_key_event` to `action.rs`:

```rust
pub fn from_markdown_preview_key_event(key_event: KeyEvent) -> Option<Self> {
    match key_event.code {
        KeyCode::Esc        => Some(Self::FocusMarkdownPreview), // toggle back
        KeyCode::Up         => Some(Self::ScrollMarkdownPreviewUp),
        KeyCode::Down       => Some(Self::ScrollMarkdownPreviewDown),
        KeyCode::PageUp     => Some(Self::ScrollMarkdownPreviewPageUp),
        KeyCode::PageDown   => Some(Self::ScrollMarkdownPreviewPageDown),
        _ => None,
    }
}
```

- [ ] **Step 1.4: Tests**

```rust
#[test]
fn focus_layer_returns_markdown_preview_when_focused() {
    // Set editor.markdown_preview_focused = true, editor open
    // assert focus_layer() == FocusLayer::MarkdownPreview
}
```

- [ ] **Step 1.5: Commit**

```bash
git commit -m "feat(state): add FocusLayer::MarkdownPreview"
```

---

## Task 2: Add new actions

**Files:** `src/action.rs`

- [ ] **Step 2.1: Add to `Action` enum**

```rust
ToggleEditorFullscreen,
ToggleMarkdownPreview,
FocusMarkdownPreview,
ScrollMarkdownPreviewUp,
ScrollMarkdownPreviewDown,
ScrollMarkdownPreviewPageUp,
ScrollMarkdownPreviewPageDown,
```

- [ ] **Step 2.2: Wire `ToggleEditorFullscreen` into `from_editor_key_event`**

```rust
KeyCode::F(11) => Some(Self::ToggleEditorFullscreen),
```

- [ ] **Step 2.3: Wire `ToggleMarkdownPreview` into `from_editor_key_event`**

```rust
KeyCode::Char('m') if key_event.modifiers == KeyModifiers::CONTROL
    => Some(Self::ToggleMarkdownPreview),
```

- [ ] **Step 2.4: Wire `FocusMarkdownPreview` into `from_editor_key_event`**

```rust
KeyCode::Tab => Some(Self::FocusMarkdownPreview),
```

> **Note:** Tab currently emits `FocusNextPane` in editor context. Override it here with `FocusMarkdownPreview` when the markdown preview is visible; otherwise fall through to existing behaviour.

- [ ] **Step 2.5: Commit**

```bash
git commit -m "feat(action): add fullscreen/markdown-preview/scroll actions"
```

---

## Task 3: EditorState — scroll tracking + fullscreen flag

**Files:** `src/state/editor_state.rs`, `src/state/mod.rs`

- [ ] **Step 3.1: Add fields to `EditorState`**

```rust
pub struct EditorState {
    pub buffer: Option<EditorBuffer>,
    /// Scroll offset for the markdown preview panel (lines from top).
    pub markdown_preview_scroll: usize,
    /// Whether keyboard focus is on the markdown preview, not the editor.
    pub markdown_preview_focused: bool,
    /// Whether the markdown preview split is visible.
    pub markdown_preview_visible: bool,
}
```

- [ ] **Step 3.2: Add `editor_fullscreen` to `AppState`**

```rust
pub struct AppState {
    // ...
    pub editor_fullscreen: bool,
}
```

- [ ] **Step 3.3: Handle new actions in `EditorState::apply`**

```rust
Action::FocusMarkdownPreview => {
    // Toggle: if already focused on preview, return to editor.
    self.markdown_preview_focused = !self.markdown_preview_focused;
}
Action::ToggleMarkdownPreview => {
    self.markdown_preview_visible = !self.markdown_preview_visible;
    if !self.markdown_preview_visible {
        self.markdown_preview_focused = false;
    }
}
Action::ScrollMarkdownPreviewUp => {
    self.markdown_preview_scroll = self.markdown_preview_scroll.saturating_sub(1);
}
Action::ScrollMarkdownPreviewDown => {
    self.markdown_preview_scroll = self.markdown_preview_scroll.saturating_add(1);
}
Action::ScrollMarkdownPreviewPageUp => {
    self.markdown_preview_scroll = self.markdown_preview_scroll.saturating_sub(20);
}
Action::ScrollMarkdownPreviewPageDown => {
    self.markdown_preview_scroll = self.markdown_preview_scroll.saturating_add(20);
}
```

- [ ] **Step 3.4: Handle `ToggleEditorFullscreen` in `AppState::apply_view`**

```rust
Action::ToggleEditorFullscreen => {
    self.editor_fullscreen = !self.editor_fullscreen;
}
```

- [ ] **Step 3.5: Scroll sync — update preview scroll on cursor move**

In `EditorState::apply`, when `EditorMoveUp` or `EditorMoveDown` is processed and the buffer exists, compute the proportional preview scroll:

```rust
Action::EditorMoveUp | Action::EditorMoveDown | Action::EditorMoveLeft | Action::EditorMoveRight => {
    // ... existing movement code ...
    // After movement, sync markdown preview scroll.
    if self.markdown_preview_visible {
        if let Some(buf) = &self.buffer {
            let (cursor_line, _) = buf.cursor_line_col();
            let total_lines = buf.visible_lines().len().max(1);
            // Proportional: preview scrolls at the same fractional position.
            self.markdown_preview_scroll =
                cursor_line * 100 / total_lines; // rough line estimate
        }
    }
}
```

> **Note:** The proportional scroll is an approximation. The rendered markdown line count differs from the source line count (headings produce 2 lines, blank lines are kept, etc.). A future improvement can refine this with a rendered-line count passed from the UI layer.

- [ ] **Step 3.6: Auto-show preview when opening a .md file**

In `EditorState::open`, set `markdown_preview_visible = true` when the path has a `.md` extension:

```rust
pub fn open(&mut self, editor: EditorBuffer) {
    let is_md = editor.path.as_ref()
        .and_then(|p| p.extension())
        .map_or(false, |e| e.eq_ignore_ascii_case("md"));
    self.markdown_preview_visible = is_md;
    self.markdown_preview_focused = false;
    self.markdown_preview_scroll = 0;
    self.buffer = Some(editor);
}
```

- [ ] **Step 3.7: Tests**

```rust
#[test]
fn toggle_markdown_preview_flips_visibility() { ... }

#[test]
fn focus_markdown_preview_toggles_focused_flag() { ... }

#[test]
fn toggle_editor_fullscreen_flips_flag() { ... }

#[test]
fn editor_fullscreen_defaults_to_false() { ... }
```

- [ ] **Step 3.8: Commit**

```bash
git commit -m "feat(state): editor fullscreen flag, markdown preview scroll/focus/toggle"
```

---

## Task 4: Update rendering

**Files:** `src/ui/mod.rs`, `src/ui/markdown.rs`

- [ ] **Step 4.1: Add `scroll` parameter to `render_markdown_preview`**

```rust
pub fn render_markdown_preview(
    frame: &mut Frame<'_>,
    area: Rect,
    source: &str,
    scroll: usize,    // ← new
    palette: ThemePalette,
) {
    // ...
    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(palette.tools_bg))
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));  // ← use scroll offset
    frame.render_widget(paragraph, inner);
}
```

- [ ] **Step 4.2: Update `ui/mod.rs` — fullscreen layout branch**

At the top of the layout block in `render()`:

```rust
// Fullscreen editor: give entire content area to tools, skip panes.
if state.editor_fullscreen && state.editor().is_some() {
    let tools_area = areas[1];
    // render editor (+ optional markdown preview split)
    // ... same rendering logic as the normal tools_area_opt path ...
    // skip pane rendering entirely
    return build_layout_cache(areas, None, None, menu_popup_rect);
}
```

- [ ] **Step 4.3: Pass preview visibility and scroll from state**

In the markdown preview rendering call:

```rust
if state.editor.markdown_preview_visible {
    let source = editor.contents();
    let scroll = state.editor.markdown_preview_scroll;
    render_markdown_preview(frame, md_area, &source, scroll, palette);
} 
```

- [ ] **Step 4.4: Update `show_md_preview` check in tools panel**

```rust
let show_md_preview = is_markdown_file(editor)
    && state.editor.markdown_preview_visible;
```

- [ ] **Step 4.5: Visual focus indicator on markdown preview border**

When `state.editor.markdown_preview_focused`, render the preview block with `palette.border_focus` instead of `palette.text_muted`.

- [ ] **Step 4.6: Run full test suite**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

- [ ] **Step 4.7: Commit**

```bash
git commit -m "feat(ui): fullscreen editor layout, scroll sync, markdown preview focus indicator"
```

---

## Task 5: Final verification

- [ ] Manual smoke test checklist:
  - Open a `.md` file → preview auto-appears
  - Type in editor → preview updates live
  - Scroll editor cursor → preview scrolls in sync
  - Press Tab → preview border highlights, arrows scroll preview
  - Press Esc → focus returns to editor
  - Press Ctrl+M → preview disappears / reappears
  - Press F11 → file panes hide, editor fills screen
  - Press F11 again → file panes return

- [ ] **Final commit**

```bash
git commit -m "chore: Wave 4C complete — fullscreen editor, scroll sync, preview focus/toggle"
```
