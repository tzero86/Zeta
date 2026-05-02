# Phase 1 — State Decomposition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Token efficiency:** Use `rtk read`, `rtk tree`, `rtk git`, `rtk test` instead of raw bash equivalents. Use `grep`/`glob` tools instead of bash `find`/`cat`.

**Goal:** Break `apply_view()` in `src/state/mod.rs` from a 1,500-line monolith into focused private handler methods, reducing cognitive complexity without moving types or changing public APIs.

**Architecture:** `apply_view()` currently handles ~60 distinct `Action` variants in one match block. We extract those into 7 private `fn apply_*()` methods on `AppState`, each owning a logical domain (layout, git-diff, bookmarks, archive, open-with, file-ops, navigation). `apply_view()` becomes a ~50-line dispatcher. No public API changes. No type moves.

**Tech Stack:** Rust stable, `cargo fmt`, `cargo clippy`, `cargo test --workspace`

**Branch:** `feat/phase1-state-decomposition`

---

## File Map

| File | Change |
|------|--------|
| `src/state/mod.rs` | Extract 7 private handler methods; `apply_view()` becomes dispatcher |
| No other files change | All types and public APIs stay in place |

---

## Pre-flight

- [ ] **Confirm you are on the right branch**

```bash
rtk git branch --show-current
# Expected: feat/phase1-state-decomposition
```

- [ ] **Run baseline tests to confirm green**

```bash
rtk test cargo test --workspace 2>&1 | tail -5
# Expected: test result: ok. N passed; 0 failed
```

- [ ] **Record current line count of apply_view()**

```bash
awk '/^    fn apply_view/,/^    pub fn apply_job_result/' src/state/mod.rs | wc -l
# Record this number — we'll verify it shrinks at the end
```

---

## Task 1: Extract `apply_layout()` — layout/view/resize actions

**Files:**
- Modify: `src/state/mod.rs` (extract ~120 lines from `apply_view`)

These actions all control visual layout and should be together:
`SetPaneLayout`, `TogglePreviewPanel`, `ToggleEditorFullscreen`, `ToggleMarkdownPreview`,
`ToggleHiddenFiles`, `ShrinkLeftPane`, `GrowLeftPane`, `Resize`, `ToggleDebugPanel`,
`ToggleDetailsView`, `OpenAboutDialog`, `SetTheme`.

- [ ] **Find the line ranges to extract**

```bash
grep -n "Action::SetPaneLayout\|Action::TogglePreviewPanel\|Action::ToggleEditorFullscreen\|Action::ToggleMarkdownPreview\|Action::ToggleHiddenFiles\|Action::ShrinkLeftPane\|Action::GrowLeftPane\|Action::Resize\|Action::ToggleDebugPanel\|Action::ToggleDetailsView\|Action::OpenAboutDialog\|Action::SetTheme" src/state/mod.rs | grep -v test | head -20
```

- [ ] **Add the new private method signature immediately after `apply_view`'s closing brace**

Find the line after `apply_view` ends (search for `pub fn apply_job_result`) and insert above it:

```rust
/// Handles layout, theme, and view-toggle actions.
fn apply_layout(&mut self, action: &Action) -> Result<Vec<Command>> {
    let mut commands = Vec::new();
    match action {
        // ── paste the extracted match arms here verbatim ──
        _ => {}
    }
    Ok(commands)
}
```

- [ ] **Move the match arms** for the 12 actions listed above from `apply_view`'s match block into `apply_layout`'s match block. Cut from one, paste to other — no logic changes.

- [ ] **Replace the removed arms in `apply_view` with a delegation call**

At the top of `apply_view`'s match block, before the remaining arms, add:

```rust
_ if matches!(action,
    Action::SetPaneLayout(_)
    | Action::TogglePreviewPanel
    | Action::ToggleEditorFullscreen
    | Action::ToggleMarkdownPreview
    | Action::ToggleHiddenFiles
    | Action::ShrinkLeftPane
    | Action::GrowLeftPane
    | Action::Resize { .. }
    | Action::ToggleDebugPanel
    | Action::ToggleDetailsView
    | Action::OpenAboutDialog
    | Action::SetTheme(_)
) => {
    commands.extend(self.apply_layout(action)?);
}
```

- [ ] **Verify it compiles**

```bash
cargo check 2>&1 | grep "^error" | head -10
# Expected: no errors
```

- [ ] **Run tests**

```bash
rtk test cargo test --workspace 2>&1 | tail -5
# Expected: 0 failed
```

- [ ] **Commit**

```bash
git add src/state/mod.rs
git commit -m "refactor(state): extract apply_layout() from apply_view()"
```

---

## Task 2: Extract `apply_git_diff()` — git diff viewer actions

**Files:**
- Modify: `src/state/mod.rs` (extract ~90 lines)

Actions: `ToggleGitDiff`, `GitDiffSelectPrev`, `GitDiffSelectNext`, `GitDiffPageUp`,
`GitDiffPageDown`, `GitDiffScrollUp`, `GitDiffScrollDown`, `GitDiffToggleFocus`,
`GitDiffContentPageUp`, `GitDiffContentPageDown`, `GitDiffSetViewport`.

- [ ] **Find line ranges**

```bash
grep -n "Action::ToggleGitDiff\|Action::GitDiff" src/state/mod.rs | grep -v test | head -20
```

- [ ] **Add private method**

```rust
/// Handles all git-diff viewer actions.
fn apply_git_diff(&mut self, action: &Action) -> Result<Vec<Command>> {
    let mut commands = Vec::new();
    match action {
        // ── paste extracted arms here ──
        _ => {}
    }
    Ok(commands)
}
```

- [ ] **Move arms, add delegation in `apply_view`**

```rust
_ if matches!(action,
    Action::ToggleGitDiff
    | Action::GitDiffSelectPrev
    | Action::GitDiffSelectNext
    | Action::GitDiffPageUp
    | Action::GitDiffPageDown
    | Action::GitDiffScrollUp
    | Action::GitDiffScrollDown
    | Action::GitDiffToggleFocus
    | Action::GitDiffContentPageUp
    | Action::GitDiffContentPageDown
    | Action::GitDiffSetViewport(_)
) => {
    commands.extend(self.apply_git_diff(action)?);
}
```

- [ ] **Compile check + tests**

```bash
cargo check 2>&1 | grep "^error" | head -5
rtk test cargo test --workspace 2>&1 | tail -5
```

- [ ] **Commit**

```bash
git add src/state/mod.rs
git commit -m "refactor(state): extract apply_git_diff() from apply_view()"
```

---

## Task 3: Extract `apply_open_with()` — open-with menu actions

**Files:**
- Modify: `src/state/mod.rs` (extract ~90 lines)

Actions: `OpenOpenWithMenu`, `OpenWithMoveUp`, `OpenWithMoveDown`, `OpenWithConfirm`, `CloseOpenWithMenu`.

- [ ] **Find line ranges**

```bash
grep -n "Action::OpenOpenWithMenu\|Action::OpenWithMove\|Action::OpenWithConfirm\|Action::CloseOpenWithMenu" src/state/mod.rs | grep -v test
```

- [ ] **Add private method**

```rust
/// Handles the "open with" application menu.
fn apply_open_with(&mut self, action: &Action) -> Result<Vec<Command>> {
    let mut commands = Vec::new();
    match action {
        // ── paste extracted arms here ──
        _ => {}
    }
    Ok(commands)
}
```

- [ ] **Move arms, add delegation in `apply_view`**

```rust
_ if matches!(action,
    Action::OpenOpenWithMenu
    | Action::OpenWithMoveUp
    | Action::OpenWithMoveDown
    | Action::OpenWithConfirm
    | Action::CloseOpenWithMenu
) => {
    commands.extend(self.apply_open_with(action)?);
}
```

- [ ] **Compile + test + commit**

```bash
cargo check 2>&1 | grep "^error" | head -5
rtk test cargo test --workspace 2>&1 | tail -5
git add src/state/mod.rs
git commit -m "refactor(state): extract apply_open_with() from apply_view()"
```

---

## Task 4: Extract `apply_archive()` — archive browsing actions

**Files:**
- Modify: `src/state/mod.rs` (extract ~15 lines)

Actions: `OpenArchive`, `ExitArchive`.

- [ ] **Find line ranges**

```bash
grep -n "Action::OpenArchive\|Action::ExitArchive" src/state/mod.rs | grep -v test
```

- [ ] **Add private method**

```rust
/// Handles archive open and exit.
fn apply_archive(&mut self, action: &Action) -> Result<Vec<Command>> {
    let mut commands = Vec::new();
    match action {
        // ── paste extracted arms here ──
        _ => {}
    }
    Ok(commands)
}
```

- [ ] **Move arms, add delegation**

```rust
_ if matches!(action, Action::OpenArchive { .. } | Action::ExitArchive) => {
    commands.extend(self.apply_archive(action)?);
}
```

- [ ] **Compile + test + commit**

```bash
cargo check 2>&1 | grep "^error" | head -5
rtk test cargo test --workspace 2>&1 | tail -5
git add src/state/mod.rs
git commit -m "refactor(state): extract apply_archive() from apply_view()"
```

---

## Task 5: Extract `apply_bookmarks()` — bookmarks actions

**Files:**
- Modify: `src/state/mod.rs` (extract ~45 lines)

Actions: `AddBookmark`, `OpenBookmarks`, `BookmarkSelect`, `DeleteBookmark`.

- [ ] **Find line ranges**

```bash
grep -n "Action::AddBookmark\|Action::OpenBookmarks\|Action::BookmarkSelect\|Action::DeleteBookmark" src/state/mod.rs | grep -v test
```

- [ ] **Add private method**

```rust
/// Handles bookmark add, open, select, and delete.
fn apply_bookmarks(&mut self, action: &Action) -> Result<Vec<Command>> {
    let mut commands = Vec::new();
    match action {
        // ── paste extracted arms here ──
        _ => {}
    }
    Ok(commands)
}
```

- [ ] **Move arms, add delegation**

```rust
_ if matches!(action,
    Action::AddBookmark
    | Action::OpenBookmarks
    | Action::BookmarkSelect(_)
    | Action::DeleteBookmark(_)
) => {
    commands.extend(self.apply_bookmarks(action)?);
}
```

- [ ] **Compile + test + commit**

```bash
cargo check 2>&1 | grep "^error" | head -5
rtk test cargo test --workspace 2>&1 | tail -5
git add src/state/mod.rs
git commit -m "refactor(state): extract apply_bookmarks() from apply_view()"
```

---

## Task 6: Extract `apply_diff_mode()` — directory diff mode actions

**Files:**
- Modify: `src/state/mod.rs` (extract ~60 lines)

Actions: `ToggleDiffMode`, `DiffSyncToOther`.

- [ ] **Find line ranges**

```bash
grep -n "Action::ToggleDiffMode\|Action::DiffSyncToOther" src/state/mod.rs | grep -v test
```

- [ ] **Add private method**

```rust
/// Handles directory diff mode toggle and sync.
fn apply_diff_mode(&mut self, action: &Action) -> Result<Vec<Command>> {
    let mut commands = Vec::new();
    match action {
        // ── paste extracted arms here ──
        _ => {}
    }
    Ok(commands)
}
```

- [ ] **Move arms, add delegation**

```rust
_ if matches!(action, Action::ToggleDiffMode | Action::DiffSyncToOther) => {
    commands.extend(self.apply_diff_mode(action)?);
}
```

- [ ] **Compile + test + commit**

```bash
cargo check 2>&1 | grep "^error" | head -5
rtk test cargo test --workspace 2>&1 | tail -5
git add src/state/mod.rs
git commit -m "refactor(state): extract apply_diff_mode() from apply_view()"
```

---

## Task 7: Extract `apply_file_ops()` — file operation prompts

**Files:**
- Modify: `src/state/mod.rs` (extract ~300 lines)

Actions: `OpenCopyPrompt`, `OpenMovePrompt`, `OpenDeletePrompt`, `OpenPermanentDeletePrompt`,
`OpenNewFilePrompt`, `OpenNewDirPrompt`, `OpenRenamePrompt`, `OpenBulkRenamePrompt`,
`StartInlineRename`, `ConfirmInlineRename`, `CancelInlineRename`, and all collision actions
(`CollisionOverwrite`, `CollisionRename`, `CollisionSkip`).

- [ ] **Find line ranges**

```bash
grep -n "Action::Open.*Prompt\|Action::Confirm.*Rename\|Action::Cancel.*Rename\|Action::Start.*Rename\|Action::Collision\|Action::OpenBulkRename" src/state/mod.rs | grep -v test | head -30
```

- [ ] **Add private method**

```rust
/// Handles all file operation prompt actions (copy, move, delete, rename, collision).
fn apply_file_ops(&mut self, action: &Action) -> Result<Vec<Command>> {
    let mut commands = Vec::new();
    match action {
        // ── paste extracted arms here ──
        _ => {}
    }
    Ok(commands)
}
```

- [ ] **Move arms, add delegation**

```rust
_ if matches!(action,
    Action::OpenCopyPrompt
    | Action::OpenMovePrompt
    | Action::OpenDeletePrompt
    | Action::OpenPermanentDeletePrompt
    | Action::OpenNewFilePrompt
    | Action::OpenNewDirPrompt
    | Action::OpenRenamePrompt
    | Action::OpenBulkRenamePrompt
    | Action::StartInlineRename
    | Action::ConfirmInlineRename
    | Action::CancelInlineRename
    | Action::CollisionOverwrite
    | Action::CollisionRename
    | Action::CollisionSkip
) => {
    commands.extend(self.apply_file_ops(action)?);
}
```

- [ ] **Compile check — this is the largest extraction, watch for borrow checker issues**

```bash
cargo check 2>&1 | grep "^error"
# If borrow errors appear, check if the extracted code borrows self fields
# that are also used in the outer method. Add local variable bindings before
# the delegation call if needed:
#   let cwd = self.panes.active_pane().cwd.clone();
#   commands.extend(self.apply_file_ops(action)?);
```

- [ ] **Run full test suite**

```bash
rtk test cargo test --workspace 2>&1 | tail -10
# Expected: 0 failed
```

- [ ] **Commit**

```bash
git add src/state/mod.rs
git commit -m "refactor(state): extract apply_file_ops() from apply_view()"
```

---

## Task 8: Verify and measure the improvement

- [ ] **Measure new `apply_view()` size**

```bash
awk '/^    fn apply_view/,/^    fn apply_layout/' src/state/mod.rs | wc -l
# Should be significantly smaller than the baseline recorded in Pre-flight
```

- [ ] **Confirm all 7 handler methods exist**

```bash
grep -n "fn apply_layout\|fn apply_git_diff\|fn apply_open_with\|fn apply_archive\|fn apply_bookmarks\|fn apply_diff_mode\|fn apply_file_ops" src/state/mod.rs
# Expected: 7 lines, each with a line number
```

- [ ] **Run full validation sequence**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
rtk test cargo test --workspace 2>&1 | tail -5
```

- [ ] **Fix any clippy warnings** (do not suppress with `#[allow]` — fix them)

- [ ] **Final commit**

```bash
git add -A
git commit -m "refactor(state): apply_view() decomposition complete — 7 focused handlers"
```

---

## Task 9: Open PR

- [ ] **Push branch**

```bash
git push -u origin feat/phase1-state-decomposition
```

- [ ] **Create PR**

```bash
gh pr create \
  --title "refactor(state): decompose apply_view() into 7 focused handlers" \
  --body "## Summary
Breaks the 1,500-line \`apply_view()\` monolith in \`src/state/mod.rs\` into 7 private handler methods grouped by domain:

- \`apply_layout()\` — pane layout, theme, view toggles
- \`apply_git_diff()\` — git diff viewer navigation
- \`apply_open_with()\` — open-with app menu
- \`apply_archive()\` — archive open/exit
- \`apply_bookmarks()\` — bookmark CRUD
- \`apply_diff_mode()\` — directory diff mode
- \`apply_file_ops()\` — copy/move/delete/rename prompts + collision handling

**No public API changes. No type moves. Zero behaviour change.**

## Why
- \`apply_view()\` had betweenness centrality 0.108 in the codebase graph — the single highest-scored real bottleneck. Every new feature that touches state had to navigate 1,500 lines to find its place.
- Each new domain (e.g. tags, shell hooks in later phases) now adds a new \`apply_*()\` method rather than extending the monolith.

## Testing
All existing tests pass. No new tests needed — this is a pure structural refactor with no behaviour change." \
  --base main
```

---

## Success Criteria

- `apply_view()` body is under 100 lines
- 7 new private methods visible in `grep "fn apply_" src/state/mod.rs`
- `cargo test --workspace` passes with 0 failures
- `cargo clippy -- -D warnings` passes clean
- No public API changes (no changes to `pub fn`, `pub struct`, `pub enum`)
