# Wave 6B — Directory Diff Mode

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When both panes are loaded and showing different directories, toggle a diff mode (`F10`) that colour-codes each entry by its comparison status: only in left, only in right, in both with same size/date, or in both but different. A sync action (`Ctrl+D`) copies differing/missing files from the active pane to the inactive one.

**Architecture:**
- `DiffStatus` enum: `LeftOnly`, `RightOnly`, `Same`, `Different`.
- `AppState` gains `diff_mode: bool` and `diff_map: HashMap<String, DiffStatus>` — keyed by filename, computed from both panes' `entries` at the moment diff mode is toggled.
- The diff map is recomputed whenever either pane rescans while `diff_mode` is active.
- `ui/pane.rs` receives the diff map and colours entry names accordingly.
- No new background worker — diff computation is O(n log n) on the already-loaded entry lists, fast enough for the UI thread.

**No new dependencies.**

**Jira:** ZTA-157 (ZTA-164 through ZTA-167)

**Wave dependency:** Starts AFTER Wave 6A. No other waves depend on this.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/diff.rs` | `DiffStatus`, `compute_diff(left, right) -> HashMap<String, DiffStatus>` |
| Modify | `src/lib.rs` | `pub mod diff;` |
| Modify | `src/state/mod.rs` | `diff_mode`, `diff_map`; recompute on toggle and on scan results |
| Modify | `src/action.rs` | `ToggleDiffMode`, `DiffSyncToOther` |
| Modify | `src/ui/pane.rs` | Colour entries from `diff_map`; diff legend in title |

---

## DiffStatus colours

| Status | Meaning | Suggested colour |
|---|---|---|
| `LeftOnly` | Entry exists only in left pane | Green (left pane) / Red (right pane) |
| `RightOnly` | Entry exists only in right pane | Red (left pane) / Green (right pane) |
| `Same` | Name matches, size and modified date match | Dim (normal text) |
| `Different` | Name matches but size or date differs | Yellow |

The colour is applied to the entry name span in `render_item`. When diff mode is off, existing colours apply unchanged.

---

## Task 1: `src/diff.rs` — pure diff logic

**Files:** `src/diff.rs`, `src/lib.rs`

- [ ] **Step 1.1: Define types**

```rust
use std::collections::HashMap;
use crate::fs::EntryInfo;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffStatus {
    LeftOnly,
    RightOnly,
    Same,
    Different,
}

impl DiffStatus {
    pub fn colour(self, is_left_pane: bool) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Self::Same      => Color::Reset,
            Self::Different => Color::Yellow,
            Self::LeftOnly  => if is_left_pane { Color::Green } else { Color::Red },
            Self::RightOnly => if is_left_pane { Color::Red } else { Color::Green },
        }
    }
}
```

- [ ] **Step 1.2: Implement `compute_diff`**

```rust
/// Compare two directory listings by filename.
/// Comparison criteria: name (key), then size and modified timestamp.
pub fn compute_diff(
    left: &[EntryInfo],
    right: &[EntryInfo],
) -> HashMap<String, DiffStatus> {
    use std::collections::HashMap;

    let left_map: HashMap<&str, &EntryInfo> =
        left.iter().map(|e| (e.name.as_str(), e)).collect();
    let right_map: HashMap<&str, &EntryInfo> =
        right.iter().map(|e| (e.name.as_str(), e)).collect();

    let mut result = HashMap::new();

    for (name, l_entry) in &left_map {
        let status = match right_map.get(name) {
            None => DiffStatus::LeftOnly,
            Some(r_entry) => {
                if entries_match(l_entry, r_entry) {
                    DiffStatus::Same
                } else {
                    DiffStatus::Different
                }
            }
        };
        result.insert(name.to_string(), status);
    }

    for name in right_map.keys() {
        if !left_map.contains_key(name) {
            result.insert(name.to_string(), DiffStatus::RightOnly);
        }
    }

    result
}

fn entries_match(a: &EntryInfo, b: &EntryInfo) -> bool {
    // Directories always compare by name only.
    if a.kind == crate::fs::EntryKind::Directory {
        return b.kind == crate::fs::EntryKind::Directory;
    }
    // Files: compare size and modified time.
    a.size == b.size && a.modified == b.modified
}
```

- [ ] **Step 1.3: Tests**

```rust
#[test]
fn compute_diff_left_only() {
    let left = vec![make_entry("a.txt", 100)];
    let right = vec![];
    let diff = compute_diff(&left, &right);
    assert_eq!(diff["a.txt"], DiffStatus::LeftOnly);
}

#[test]
fn compute_diff_right_only() {
    let left = vec![];
    let right = vec![make_entry("b.txt", 200)];
    let diff = compute_diff(&left, &right);
    assert_eq!(diff["b.txt"], DiffStatus::RightOnly);
}

#[test]
fn compute_diff_same_entry() {
    let entry = make_entry("c.txt", 300);
    let diff = compute_diff(&[entry.clone()], &[entry]);
    assert_eq!(diff["c.txt"], DiffStatus::Same);
}

#[test]
fn compute_diff_different_size() {
    let left = vec![make_entry_with_size("d.txt", 100)];
    let right = vec![make_entry_with_size("d.txt", 200)];
    let diff = compute_diff(&left, &right);
    assert_eq!(diff["d.txt"], DiffStatus::Different);
}

#[test]
fn compute_diff_symmetric_count() {
    // Total keys = unique names across both panes.
    let left  = vec![make_entry("a.txt", 0), make_entry("b.txt", 0)];
    let right = vec![make_entry("b.txt", 0), make_entry("c.txt", 0)];
    let diff = compute_diff(&left, &right);
    assert_eq!(diff.len(), 3); // a, b, c
}
```

- [ ] **Step 1.4: Commit**

```bash
git commit -m "feat(diff): add compute_diff — pure directory comparison logic"
```

---

## Task 2: AppState integration

**Files:** `src/state/mod.rs`

- [ ] **Step 2.1: Add fields to `AppState`**

```rust
pub struct AppState {
    // ...
    pub diff_mode: bool,
    pub diff_map: std::collections::HashMap<String, crate::diff::DiffStatus>,
}
```

- [ ] **Step 2.2: Add `ToggleDiffMode` to `action.rs`**

```rust
ToggleDiffMode,
DiffSyncToOther,  // copy LeftOnly or Different entries to the other pane
```

- [ ] **Step 2.3: Handle `ToggleDiffMode`**

```rust
Action::ToggleDiffMode => {
    self.diff_mode = !self.diff_mode;
    if self.diff_mode {
        self.diff_map = crate::diff::compute_diff(
            &self.panes.left.entries,
            &self.panes.right.entries,
        );
        self.status_message = String::from("diff mode — green=unique, yellow=different");
    } else {
        self.diff_map.clear();
        self.status_message = String::from("diff mode off");
    }
}
```

- [ ] **Step 2.4: Recompute diff on `DirectoryScanned` when diff mode is active**

In `apply_job_result`, after updating a pane's entries:

```rust
if self.diff_mode {
    self.diff_map = crate::diff::compute_diff(
        &self.panes.left.entries,
        &self.panes.right.entries,
    );
}
```

- [ ] **Step 2.5: Handle `DiffSyncToOther`**

Queue `FileOperation::Copy` for each entry with status `LeftOnly` or `Different` (if left is active) or `RightOnly` / `Different` (if right is active). Use the inactive pane's cwd as the destination. Show a count in the status message: `"queued 3 files to sync"`.

- [ ] **Step 2.6: Wire `F10` → `ToggleDiffMode` in `from_pane_key_event`**

- [ ] **Step 2.7: Tests**

```rust
#[test]
fn toggle_diff_mode_computes_diff_map() { ... }

#[test]
fn diff_mode_recomputes_on_scan() { ... }

#[test]
fn toggle_diff_mode_off_clears_map() { ... }
```

- [ ] **Step 2.8: Commit**

```bash
git commit -m "feat(state): diff_mode flag, diff_map, ToggleDiffMode action"
```

---

## Task 3: Render diff colours in pane

**Files:** `src/ui/pane.rs`, `src/ui/mod.rs`

- [ ] **Step 3.1: Pass diff state to `render_pane`**

```rust
pub fn render_pane(
    // ...
    diff: Option<(&HashMap<String, DiffStatus>, bool /* is_left */)>,
)
```

- [ ] **Step 3.2: Apply diff colour to entry name span**

In `render_item`, look up the entry name in the diff map:

```rust
let diff_colour = diff
    .and_then(|(map, is_left)| map.get(&entry.name))
    .map(|status| status.colour(is_left))
    .unwrap_or(row_styles.name.fg.unwrap_or(palette.text_primary));

// Use diff_colour instead of row_styles.name for the name span.
```

- [ ] **Step 3.3: Show diff legend in pane title when active**

```
Left [42]  /home/zero/project-v1  (diff: 12 same, 5 diff, 3 left-only)
```

- [ ] **Step 3.4: Update `ui/mod.rs` call sites**

Pass `Some((&state.diff_map, true))` for left pane and `Some((&state.diff_map, false))` for right pane when `state.diff_mode`.

- [ ] **Step 3.5: Commit**

```bash
git commit -m "feat(ui): diff colour coding in pane entry rows + diff legend in title"
```

---

## Task 4: Final verification

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Manual smoke test:
- Load two different directories in left and right pane
- Press `F10` — entries colour-code immediately
- Copy a file in one pane → press `F5` (refresh both) → diff updates
- Press `F10` again → colours disappear

```bash
git commit -m "chore: Wave 6B complete — directory diff mode"
```
