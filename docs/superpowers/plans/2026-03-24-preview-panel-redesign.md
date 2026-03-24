# Preview Panel Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make code preview and editor rendering reliable for all source files, with preview focus available immediately when the panel is open.

**Architecture:** Keep shared text preparation, but split preview and editor into separate render surfaces. Introduce a terminal-safe render model that normalizes raw file text before any Ratatui painting, so control characters and width issues cannot corrupt borders or backgrounds. Focus should be driven by open panel state, not by whether preview content has loaded.

**Normalization rules:** Convert CRLF/CR to `\n` at ingestion, preserve visible Unicode characters, replace tabs with spaces using the current tab width policy, and drop any other non-printable control characters before rendering.

**Tech Stack:** Rust, `ratatui`, `crossterm`, `unicode-width`, existing `highlight` and `state` modules

---

### Task 1: Add terminal-safe render preparation

**Dependency:** This is the foundation for Task 2 and Task 4.

**Files:**
- Modify: `src/highlight.rs`
- Modify: `src/preview.rs`
- Test: `src/preview.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn preview_prep_strips_control_chars_and_preserves_visible_width() {
    let buf = ViewBuffer::from_plain("alpha\r\nbeta\nchar\tlie\nwide: 測試");
    assert_eq!(buf.lines.len(), 4);
    assert!(buf.lines.iter().all(|line| line.iter().all(|token| !token.2.contains('\r'))));
    assert!(buf.lines.iter().any(|line| line.iter().any(|token| token.2.contains("wide: 測試"))));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test preview_prep_strips_control_chars_and_preserves_visible_width -- --exact --nocapture`
Expected: FAIL because `\r` is still preserved in one of the prepared lines.

- [ ] **Step 3: Write minimal implementation**

Implement a sanitize step in the preview/highlight path that removes or normalizes terminal control characters before tokens reach the renderer. Keep raw text parsing and syntax highlighting separate from terminal-safe output.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test preview_prep_strips_control_chars_and_preserves_visible_width -- --exact --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/preview.rs src/highlight.rs
git commit -m "fix: sanitize preview text before rendering"
```

### Task 2: Split preview and editor surfaces

**Dependency:** This should build on Task 1 so the render surfaces consume sanitized content.

**Files:**
- Modify: `src/ui.rs`
- Modify: `src/editor.rs`
- Modify: `src/preview.rs`
- Test: `src/ui.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn preview_and_editor_render_paths_accept_different_view_models() {
    let preview = ViewBuffer::from_plain("alpha\nbeta");
    let editor = EditorBuffer::default();

    assert_eq!(preview.total_lines, 2);
    assert!(!editor.is_dirty);
    assert_eq!(editor.cursor_char_idx, 0);
    // Compile-time proof: preview rendering accepts preview buffers, editor rendering accepts editor buffers.
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test preview_and_editor_render_paths_accept_different_view_models -- --exact --nocapture`
Expected: FAIL until the surfaces are separated.

- [ ] **Step 3: Write minimal implementation**

Refactor `src/ui.rs` so preview rendering only consumes a sanitized view model, while editor rendering consumes the editable buffer and cursor state. Shared prep can remain, but painting should be specialized per surface.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test preview_and_editor_render_paths_accept_different_view_models -- --exact --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui.rs src/editor.rs src/preview.rs
git commit -m "refactor: separate code preview and editor rendering"
```

### Task 3: Fix preview focus startup behavior

**Dependency:** Independent of Task 1, but it should land alongside Task 2 because both touch input/state routing.

**Files:**
- Modify: `src/state/mod.rs`
- Modify: `src/action.rs`
- Test: `src/state/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn preview_focus_is_available_on_startup_when_panel_is_open() {
    let mut state = test_state();
    state.preview_panel_open = true;
    state.preview_view = None;
    assert!(state.is_preview_panel_open());
    assert!(!state.is_preview_focused());
    assert!(state.apply(Action::FocusPreviewPanel).is_ok());
    assert!(state.is_preview_focused());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test preview_focus_is_available_on_startup_when_panel_is_open -- --exact --nocapture`
Expected: FAIL until focus is based on panel state.

- [ ] **Step 3: Write minimal implementation**

Update focus initialization and `CycleFocus` handling so preview is focusable whenever the panel is open, even before content loads. Remove any dependency on `preview_view.is_some()` for focus eligibility.

Focus rules:
- If the preview panel is open at startup, initialize focus so preview can be reached immediately.
- `FocusPreviewPanel` must set preview focus whenever the panel is open, even if `preview_view` is `None`.
- `CycleFocus` must cycle `Left -> Right -> Preview -> Left` when preview is open, and `Left -> Right -> Left` when it is closed.
- If the preview panel is closed, preview focus must never be selected.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test preview_focus_is_available_on_startup_when_panel_is_open -- --exact --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/state/mod.rs src/action.rs
git commit -m "fix: allow preview focus whenever the panel is open"
```

### Task 4: Verify rendering regression coverage

**Dependency:** This validates Tasks 1-3 together.

**Files:**
- Modify: `src/highlight.rs`
- Modify: `src/preview.rs`
- Modify: `src/ui.rs`
- Modify: `src/state/mod.rs`
- Test: `cargo test`, `cargo fmt`, `cargo clippy`

- [ ] **Step 1: Add regression tests for CRLF and wide text**

```rust
#[test]
fn preview_render_boundary_never_receives_control_chars() {
    let buf = ViewBuffer::from_plain("alpha\r\nbeta\twide 測試");
    assert!(buf.lines.iter().all(|line| line.iter().all(|token| !token.2.contains('\r'))));
    assert_eq!(buf.total_lines, 2);
}
```

- [ ] **Step 2: Run targeted verification**

Run:
`cargo fmt --all -- --check`
`cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: PASS.

- [ ] **Step 3: Run the focused regression tests**

Run:
`cargo test preview_prep_strips_control_chars_and_preserves_visible_width -- --exact --nocapture`
`cargo test preview_and_editor_render_paths_accept_different_view_models -- --exact --nocapture`
`cargo test preview_focus_is_available_on_startup_when_panel_is_open -- --exact --nocapture`
Expected: PASS.

- [ ] **Step 4: Run workspace tests**

Run:
`cargo test --workspace`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/preview.rs src/ui.rs src/state/mod.rs
git commit -m "test: cover preview rendering regressions"
```
