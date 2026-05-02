# Phase 2 — UI Polish: Contextual Hints & Error Severity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Token efficiency:** Use `rtk read`, `rtk tree`, `rtk git`, `rtk test` instead of raw bash equivalents.

**Goal:** Two focused UI quality improvements — contextual hint bar that changes with pane state (marks, entry type), and status bar messages that carry severity (error = red + persists, warning = yellow, info = default).

**Architecture:**
- **2A — Status severity:** Add `MessageKind` enum + `StatusMessage` struct to `src/state/types.rs`. Change `AppState.status_message: String` → `StatusMessage`. Add `set_status()` / `set_status_error()` helpers. Propagate kind through `StatusZones` to `render_status_bar()`.
- **2B — Contextual hints:** Extend `render_key_hints()` in `src/ui/mod.rs` — add `FocusLayer::Pane` arm that reads mark count and entry kind from `AppState` to show relevant hints.

**Tech Stack:** Rust stable, ratatui `Span`/`Style`, `cargo fmt`, `cargo clippy`, `cargo test --workspace`

**Branch:** `feat/phase2-ui-polish`

---

## File Map

| File | Change |
|------|--------|
| `src/state/types.rs` | Add `MessageKind` enum, `StatusMessage` struct |
| `src/state/mod.rs` | Change field type, add helper methods, update ~7 error-class call sites |
| `src/ui/mod.rs` | Update `render_status_bar()` for colored messages; extend `render_key_hints()` with Pane arm |

---

## Pre-flight

- [ ] **Confirm branch**

```bash
rtk git branch --show-current
# Expected: feat/phase2-ui-polish
```

- [ ] **Baseline tests**

```bash
rtk test cargo test --workspace 2>&1 | tail -5
# Expected: 0 failed
```

---

## Task 1: Define `MessageKind` and `StatusMessage` types

**Files:**
- Modify: `src/state/types.rs`

- [ ] **Write tests first**

Add at the bottom of the `#[cfg(test)]` block in `src/state/types.rs` (around line 116):

```rust
#[test]
fn status_message_default_is_empty_info() {
    let m = StatusMessage::default();
    assert_eq!(m.text, "");
    assert!(matches!(m.kind, MessageKind::Info));
}

#[test]
fn status_message_error_constructor() {
    let m = StatusMessage::error("disk full");
    assert_eq!(m.text, "disk full");
    assert!(matches!(m.kind, MessageKind::Error));
}

#[test]
fn status_message_warning_constructor() {
    let m = StatusMessage::warning("read-only");
    assert!(matches!(m.kind, MessageKind::Warning));
}
```

- [ ] **Run tests to verify they fail (types don't exist yet)**

```bash
cargo test -p zeta --lib types 2>&1 | grep "error\[E"
# Expected: errors about missing StatusMessage, MessageKind
```

- [ ] **Add types** — in `src/state/types.rs`, before the `#[cfg(test)]` block:

```rust
/// Severity of a status bar message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageKind {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

/// A status bar message with optional severity for colored display.
#[derive(Debug, Clone, Default)]
pub struct StatusMessage {
    pub text: String,
    pub kind: MessageKind,
}

impl StatusMessage {
    pub fn info(text: impl Into<String>) -> Self {
        Self { text: text.into(), kind: MessageKind::Info }
    }

    pub fn success(text: impl Into<String>) -> Self {
        Self { text: text.into(), kind: MessageKind::Success }
    }

    pub fn warning(text: impl Into<String>) -> Self {
        Self { text: text.into(), kind: MessageKind::Warning }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self { text: text.into(), kind: MessageKind::Error }
    }
}
```

- [ ] **Run tests**

```bash
cargo test -p zeta --lib types 2>&1 | tail -5
# Expected: 3 new tests pass
```

- [ ] **Commit**

```bash
git add src/state/types.rs
git commit -m "feat(state): add MessageKind and StatusMessage types"
```

---

## Task 2: Update `AppState` to use `StatusMessage`

**Files:**
- Modify: `src/state/mod.rs`

This task changes the field type and adds helper methods. The compiler will then guide us to all broken call sites.

- [ ] **Change the field in the `AppState` struct** (around line 149)

Find:
```rust
    status_message: String,
```

Replace with:
```rust
    status_message: StatusMessage,
```

- [ ] **Update the `new()` constructor** (around line 183) — the `status_message` field in `TestAppState::new`:

Find:
```rust
            status_message,
```

The `status_message` parameter in `new()` is `String`. Change parameter type and construction:

```rust
// In fn new signature, change:
//   status_message: String
// to:
//   status_message: impl Into<String>
// And the field assignment:
            status_message: StatusMessage::info(status_message),
```

Add the import at the top of `src/state/mod.rs` with other `types` imports:
```rust
use crate::state::types::{MessageKind, StatusMessage};
```
(or wherever the existing `use crate::state::types::...` line lives — check with `grep -n "use crate::state::types" src/state/mod.rs`)

- [ ] **Add helper methods on `AppState`** — find the `impl AppState {` block and add these near the other utility methods (search for `fn status_zones` at line 2957, add before it):

```rust
    /// Sets a plain informational status message.
    pub fn set_status(&mut self, text: impl Into<String>) {
        self.status_message = StatusMessage::info(text);
    }

    /// Sets a success status message (green).
    pub fn set_status_success(&mut self, text: impl Into<String>) {
        self.status_message = StatusMessage::success(text);
    }

    /// Sets a warning status message (yellow).
    pub fn set_status_warning(&mut self, text: impl Into<String>) {
        self.status_message = StatusMessage::warning(text);
    }

    /// Sets an error status message (red).
    pub fn set_status_error(&mut self, text: impl Into<String>) {
        self.status_message = StatusMessage::error(text);
    }
```

- [ ] **Run `cargo check` to see all broken call sites**

```bash
cargo check 2>&1 | grep "error\[E" | grep "status_message" | wc -l
# This tells you how many assignments need updating
```

- [ ] **Bulk replace all info-class assignments** using sed

```bash
# Replace: self.status_message = String::from("...")
sed -i 's/self\.status_message = String::from(/self.set_status(String::from(/g' src/state/mod.rs
# Fix the trailing ); — the sed above leaves an extra ) we need to close:
# String::from("X"); → becomes set_status(String::from("X"); — broken
# Better approach: use a more precise pattern
```

> **Note:** The sed approach for closing parens is fragile. Instead, use your editor's multi-cursor or search-replace:
> - Find: `self.status_message = `
> - Replace: `self.set_status(`
> Then for each line, move the trailing `;` to after an added `)`:
>   `self.set_status(format!("thing")` → `self.set_status(format!("thing"))` (add one `)` before `;`)
> There are ~130 call sites total but most are single-line assignments with simple values.

- [ ] **Run `cargo check` iteratively** after batch edit to find remaining errors

```bash
cargo check 2>&1 | grep "error" | head -20
```

- [ ] **Find and update the error-class call sites** (these should use `set_status_error`):

```bash
grep -n "set_status.*error\|set_status.*fail\|set_status.*cannot\|set_status.*denied\|set_status.*clipboard error\|set_status.*invalid" src/state/mod.rs | head -20
```

Change the following from `set_status()` to `set_status_error()`:
- Any site that was `self.status_message = format!("clipboard error: {e}")` (around line 1875, 1887)
- Any site that was `self.status_message = failure_status` (around line 2252-2258) — failure_status is a String from a failed job result
- Any site that was `self.status_message = String::from("name cannot be empty")` (around line 1414)
- Any site that uses a `ZetaError` or `failure_status` variable

- [ ] **Update `status_zones()` to propagate `MessageKind`** (around line 3036)

Find:
```rust
        StatusZones {
            git_branch,
            entry_detail,
            message: format!(" {} ", self.status_message),
```

Replace with:
```rust
        StatusZones {
            git_branch,
            entry_detail,
            message: format!(" {} ", self.status_message.text),
            message_kind: self.status_message.kind,
```

- [ ] **Update `StatusZones` struct** — find it (search `struct StatusZones` in `src/state/mod.rs`) and add the new field:

```rust
pub struct StatusZones {
    pub git_branch: Option<String>,
    pub entry_detail: Option<String>,
    pub message: String,
    pub message_kind: MessageKind,   // ← add this
    pub marks: Option<MarksInfo>,
    pub progress: Option<FileOpProgress>,
    pub workspace: String,
    pub clock: String,
}
```

Also update the `StatusZones::default()` or any `StatusZones { .. }` construction in tests to include `message_kind: MessageKind::Info`.

- [ ] **Compile + test**

```bash
cargo check 2>&1 | grep "^error" | head -10
rtk test cargo test --workspace 2>&1 | tail -5
```

- [ ] **Commit**

```bash
git add src/state/mod.rs
git commit -m "feat(state): use StatusMessage for typed status severity"
```

---

## Task 3: Render message with severity color in status bar

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Write test** — add to `src/state/mod.rs` tests section:

```rust
#[test]
fn status_zones_error_kind_propagates() {
    let mut state = test_state();
    state.set_status_error("permission denied");
    let zones = state.status_zones();
    assert!(matches!(zones.message_kind, crate::state::types::MessageKind::Error));
    assert!(zones.message.contains("permission denied"));
}
```

- [ ] **Run to verify it passes** (relies on Task 2 work)

```bash
cargo test status_zones_error_kind_propagates -- --nocapture
```

- [ ] **Update `render_status_bar()`** in `src/ui/mod.rs` (around line 460)

Find:
```rust
        spans.push(Span::styled(
            zones.message.clone(),
            Style::default()
                .fg(palette.text_subtext)
                .bg(palette.status_bg),
        ));
```

Replace with:
```rust
        let message_fg = match zones.message_kind {
            crate::state::types::MessageKind::Error => palette.accent_red,
            crate::state::types::MessageKind::Warning => palette.accent_yellow,
            crate::state::types::MessageKind::Success => palette.accent_green,
            crate::state::types::MessageKind::Info => palette.text_subtext,
        };
        spans.push(Span::styled(
            zones.message.clone(),
            Style::default().fg(message_fg).bg(palette.status_bg),
        ));
```

> **Note:** `accent_red`, `accent_yellow`, `accent_green` must exist on `ThemePalette`. Verify:
> ```bash
> grep -n "accent_red\|accent_yellow\|accent_green" src/config/mod.rs | head -10
> ```
> If any are missing, check what the palette does have and use the nearest equivalent (e.g., `accent_orange` for warning if `accent_yellow` is absent).

- [ ] **Compile + test**

```bash
cargo check 2>&1 | grep "^error" | head -10
rtk test cargo test --workspace 2>&1 | tail -5
```

- [ ] **Manual smoke test** — run zeta and trigger an error message

```bash
cargo run -- 2>/dev/null &
# Navigate to a file, trigger a failed operation (e.g., try to delete a read-only file)
# Status bar should show the message in red
```

- [ ] **Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat(ui): render status messages with severity color"
```

---

## Task 4: Contextual hint bar for Pane mode

**Files:**
- Modify: `src/ui/mod.rs`

The hint bar at `render_key_hints()` currently falls through to the generic NC-style hints for `FocusLayer::Pane`. We add a dedicated arm that shows context-relevant hints.

Three states to handle:
1. **Marks active** → `[F5] Copy  [F6] Move  [F8] Delete  [M] Clear marks`
2. **Entry is directory** → `[Enter] Open  [F5] Copy  [F8] Delete  [F7] Mkdir`
3. **Default / file / symlink** → `[F3] View  [F4] Edit  [F5] Copy  [F6] Rename  [F8] Delete`

- [ ] **Verify how to detect marks and entry kind from `AppState`**

```bash
grep -n "fn marked_count\|fn selected_entry\|EntryKind\|is_dir\|kind:" src/state/mod.rs | head -10
grep -n "EntryKind\|Directory\|File\|Symlink" src/state/types.rs | head -15
```

- [ ] **Write test** — add to `src/state/mod.rs` tests:

```rust
#[test]
fn focus_layer_is_pane_by_default() {
    let state = test_state();
    assert!(matches!(state.focus_layer(), crate::state::FocusLayer::Pane));
}
```

```bash
cargo test focus_layer_is_pane_by_default -- --nocapture
```

- [ ] **Add the `FocusLayer::Pane` arm to `render_key_hints()`** in `src/ui/mod.rs`

Find (around line 656):
```rust
        _ => &[
            ("Alt+1..4", "Workspace"),
            ("F1", "Help"),
            ("F3", "View"),
            ("F4", "Edit"),
            ("F5", "Copy"),
            ("F6", "Rename"),
            ("F7", "Mkdir"),
            ("F8", "Delete"),
            ("F10", "Quit"),
        ],
```

The problem: `hints` is `&[(&str, &str)]` — a static slice — so we can't construct dynamic ones inline.
We need to change the pattern slightly. Replace the entire `render_key_hints()` function body to use an owned `Vec` instead:

Find the `let hints: &[(&str, &str)] = match state.focus_layer()` pattern and refactor to return `Vec`:

```rust
fn render_key_hints(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    palette: crate::config::ThemePalette,
) {
    use crate::state::{FocusLayer, ModalKind};

    // Build owned hints vec to allow context-driven dynamic hints for Pane mode.
    let hints: Vec<(&str, &str)> = match state.focus_layer() {
        FocusLayer::Modal(ModalKind::Dialog) => vec![
            ("\u{2191}\u{2193}", "Scroll"),
            ("PgUp/Dn", "Page"),
            ("Esc", "Close"),
        ],
        FocusLayer::Modal(ModalKind::Collision) => vec![
            ("O", "Overwrite"),
            ("R", "Rename"),
            ("S", "Skip"),
            ("Esc", "Cancel"),
        ],
        FocusLayer::Modal(ModalKind::Prompt) => {
            vec![("Enter", "Confirm"), ("Esc", "Cancel")]
        }
        FocusLayer::Modal(ModalKind::Settings) => vec![
            ("\u{2191}\u{2193}", "Navigate"),
            ("Space", "Toggle"),
            ("Esc", "Close"),
        ],
        FocusLayer::Modal(ModalKind::Bookmarks) => {
            vec![("Enter", "Go"), ("Del", "Remove"), ("Esc", "Close")]
        }
        FocusLayer::Modal(ModalKind::Palette) | FocusLayer::Modal(ModalKind::FileFinder) => vec![
            ("\u{2191}\u{2193}", "Navigate"),
            ("Enter", "Open"),
            ("Esc", "Cancel"),
        ],
        FocusLayer::GitDiffFileList => vec![
            ("\u{2191}\u{2193}/j/k", "Navigate"),
            ("PgUp/PgDn", "Page"),
            ("Tab", "Switch Pane"),
            ("Ctrl+D", "Close Diff"),
        ],
        FocusLayer::GitDiffContent => vec![
            ("\u{2191}\u{2193}/j/k", "Scroll"),
            ("PgUp/PgDn", "Page"),
            ("d", "Page Down"),
            ("Tab", "Switch Pane"),
            ("Ctrl+D", "Close Diff"),
        ],
        FocusLayer::Editor => vec![
            ("Ctrl+S", "Save"),
            ("Ctrl+F", "Find"),
            ("F3", "Next"),
            ("Esc", "Close"),
        ],
        FocusLayer::Preview | FocusLayer::MarkdownPreview => {
            vec![("Ctrl+W", "Cycle"), ("PgUp/Dn", "Scroll"), ("Esc", "Close")]
        }
        FocusLayer::Pane => {
            let pane = state.panes.active_pane();
            if pane.marked_count() > 0 {
                vec![
                    ("F5", "Copy marked"),
                    ("F6", "Move marked"),
                    ("F8", "Delete marked"),
                    ("M", "Clear marks"),
                    ("Esc", "Deselect"),
                ]
            } else {
                use crate::state::types::EntryKind;
                let is_dir = pane
                    .selected_entry()
                    .map(|e| matches!(e.kind, EntryKind::Directory))
                    .unwrap_or(false);
                if is_dir {
                    vec![
                        ("Enter", "Open"),
                        ("F5", "Copy"),
                        ("F7", "Mkdir"),
                        ("F8", "Delete"),
                        ("F10", "Quit"),
                    ]
                } else {
                    vec![
                        ("F3", "View"),
                        ("F4", "Edit"),
                        ("F5", "Copy"),
                        ("F6", "Rename"),
                        ("F8", "Delete"),
                        ("F10", "Quit"),
                    ]
                }
            }
        }
        _ => vec![
            ("Alt+1..4", "Workspace"),
            ("F1", "Help"),
            ("F3", "View"),
            ("F4", "Edit"),
            ("F5", "Copy"),
            ("F6", "Rename"),
            ("F7", "Mkdir"),
            ("F8", "Delete"),
            ("F10", "Quit"),
        ],
    };

    let key_style = Style::default()
        .fg(palette.surface_bg)
        .bg(palette.key_hint_fg)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default()
        .fg(palette.text_primary)
        .bg(palette.surface_bg);
    let sep_style = Style::default().bg(palette.surface_bg);

    let mut spans: Vec<Span> = Vec::new();
    let mut used_width = 0u16;

    for (key, label) in &hints {
        let key_text = format!(" {} ", key);
        let label_text = format!(" {} ", label);
        let segment_width = (key_text.chars().count() + label_text.chars().count()) as u16;
        if used_width + segment_width > area.width {
            break;
        }
        spans.push(Span::styled(key_text, key_style));
        spans.push(Span::styled(label_text, label_style));
        used_width += segment_width;
    }

    if used_width < area.width {
        spans.push(Span::styled(
            " ".repeat((area.width - used_width) as usize),
            sep_style,
        ));
    }

    // ... rest of the function (frame.render_widget call) stays the same
}
```

> **Note on `state.panes`:** `panes` may be private. Check with:
> ```bash
> grep -n "pub panes\|panes:" src/state/mod.rs | head -5
> ```
> If `panes` is private, use the existing public accessor: `state.active_pane()` or similar.
> Check what public pane accessors exist:
> ```bash
> grep -n "pub fn.*pane" src/state/mod.rs | head -10
> ```
> Use whichever accessor returns the active pane.

> **Note on `EntryKind`:** Verify the actual variant names:
> ```bash
> grep -n "enum EntryKind\|Directory\|File\|Symlink" src/state/types.rs | head -10
> ```

- [ ] **Compile check**

```bash
cargo check 2>&1 | grep "^error" | head -10
```

- [ ] **Run tests**

```bash
rtk test cargo test --workspace 2>&1 | tail -5
```

- [ ] **Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat(ui): contextual hint bar for pane mode (marks, dir, file)"
```

---

## Task 5: Full validation + PR

- [ ] **Run full validation sequence**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | grep "^error\|^warning.*-D warnings"
rtk test cargo test --workspace 2>&1 | tail -10
```

- [ ] **Fix any remaining clippy warnings**

- [ ] **Push branch**

```bash
git push -u origin feat/phase2-ui-polish
```

- [ ] **Create PR**

```bash
gh pr create \
  --title "feat(ui): contextual hint bar + status message severity" \
  --body "## Summary

Two focused UI quality improvements:

### 2A — Status message severity
- New \`MessageKind\` enum (Info/Success/Warning/Error) and \`StatusMessage\` struct in \`src/state/types.rs\`
- \`AppState.status_message\` changed from \`String\` to \`StatusMessage\`
- Helper methods: \`set_status()\`, \`set_status_success()\`, \`set_status_warning()\`, \`set_status_error()\`
- Status bar renders errors in red, warnings in yellow, success in green
- Error sites (clipboard error, name validation, failed operations) upgraded to \`set_status_error()\`

### 2B — Contextual pane hint bar
- \`render_key_hints()\` now has a \`FocusLayer::Pane\` arm
- Marks active → shows \`[F5]Copy marked [F6]Move marked [F8]Delete marked [M]Clear\`
- Cursor on directory → shows \`[Enter]Open [F5]Copy [F7]Mkdir [F8]Delete\`
- Default (file/symlink) → shows \`[F3]View [F4]Edit [F5]Copy [F6]Rename [F8]Delete\`

## Why
The hint bar previously showed the same F-key hints regardless of context. Users with marks active had no visual indication of what F5/F8 would operate on. Error messages were styled identically to informational messages — silent failures looked the same as success.

## Testing
All existing tests pass. Two new unit tests added." \
  --base main
```

---

## Success Criteria

- `cargo test --workspace` passes with 0 failures
- `cargo clippy -- -D warnings` passes clean
- Status bar shows red text when `set_status_error()` is called
- Hint bar shows `F5 Copy marked` when entries are marked in pane
- Hint bar shows `Enter Open` when cursor is on a directory
- `MessageKind` and `StatusMessage` are exported from `src/state/types.rs`
