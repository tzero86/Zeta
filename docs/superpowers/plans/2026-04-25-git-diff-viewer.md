# Git Diff Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `Ctrl+D`-toggled git diff viewer that replaces the dual panes with a changed-files list (left) and scrollable unified diff (right), both independently focusable via `Tab`.

**Architecture:** New types and pure parser functions land in `src/git.rs`. New `GitDiffFileList` / `GitDiffContent` variants are added to `FocusLayer` in `src/state/types.rs`. Six git-diff fields are added to `WorkspaceState`; action handlers live in `src/state/mod.rs`. A new `src/ui/git_diff.rs` module renders both panes and is wired into the existing `src/ui/mod.rs` layout branch.

**Tech Stack:** Rust stable, ratatui 0.29, crossterm, existing `run_git` subprocess helper in `src/git.rs`.

**Spec:** `docs/superpowers/specs/2026-04-25-git-diff-viewer-design.md`

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/git.rs` | New types + parser functions |
| Modify | `src/state/types.rs` | Add `GitDiffFileList`, `GitDiffContent` to `FocusLayer` |
| Modify | `src/state/mod.rs` | New state fields + all action handlers |
| Modify | `src/action.rs` | New action variants + key routing |
| **Create** | `src/ui/git_diff.rs` | `render_git_diff_view`, file list, diff content renderers |
| Modify | `src/ui/mod.rs` | Wire `render_git_diff_view` into layout |
| Modify | `README.md` | Add Git Integration section |
| Modify | `CHANGELOG.md` | Add unreleased entry |
| Modify | `site/index.html` | Add git diff viewer feature card |

---

## Task 1: New types in `src/git.rs`

**Files:**
- Modify: `src/git.rs`

- [ ] **Step 1: Add the three new types after the existing `FileStatus` impl block**

Open `src/git.rs` and add after the closing `}` of the `impl FileStatus` block (around line 51):

```rust
/// A file that has pending git changes, shown in the diff viewer file list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitDiffFile {
    pub path: PathBuf,
    pub status: FileStatus,
    pub added: usize,
    pub removed: usize,
}

/// The semantic kind of a single line in a unified diff.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffLineKind {
    Added,
    Removed,
    Context,
    HunkHeader,
    FileHeader,
}

/// A single parsed line from a unified diff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}
```

- [ ] **Step 2: Write unit tests for the new types**

Add to the `#[cfg(test)] mod tests` block at the bottom of `src/git.rs`:

```rust
#[test]
fn diff_line_kind_variants_are_distinct() {
    assert_ne!(DiffLineKind::Added, DiffLineKind::Removed);
    assert_ne!(DiffLineKind::HunkHeader, DiffLineKind::Context);
}

#[test]
fn git_diff_file_fields_are_accessible() {
    let f = GitDiffFile {
        path: PathBuf::from("src/main.rs"),
        status: FileStatus::Modified,
        added: 5,
        removed: 2,
    };
    assert_eq!(f.added, 5);
    assert_eq!(f.removed, 2);
    assert_eq!(f.status, FileStatus::Modified);
}
```

- [ ] **Step 3: Verify tests compile and pass**

```bash
cargo test --lib git -- --nocapture
```

Expected: all git module tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/git.rs
git commit -m "feat(git): add GitDiffFile, DiffLineKind, DiffLine types"
```

---

## Task 2: Pure diff parsers in `src/git.rs`

**Files:**
- Modify: `src/git.rs`

- [ ] **Step 1: Write failing tests for `parse_numstat`**

Add to the tests block in `src/git.rs`:

```rust
#[test]
fn parse_numstat_parses_modified_file() {
    let out = "12\t3\tsrc/git.rs\n";
    let map = parse_numstat(out);
    assert_eq!(map.get("src/git.rs"), Some(&(12usize, 3usize)));
}

#[test]
fn parse_numstat_binary_file_gives_zeros() {
    // git outputs "-\t-\tfile.bin" for binary files
    let out = "-\t-\tassets/logo.png\n";
    let map = parse_numstat(out);
    assert_eq!(map.get("assets/logo.png"), Some(&(0usize, 0usize)));
}

#[test]
fn parse_numstat_empty_gives_empty_map() {
    assert!(parse_numstat("").is_empty());
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test --lib git::tests::parse_numstat -- --nocapture
```

Expected: compile error — `parse_numstat` not yet defined.

- [ ] **Step 3: Implement `parse_numstat`**

Add in the `// Internal helpers` section of `src/git.rs` (after the existing `classify` fn):

```rust
/// Parse `git diff HEAD --numstat` output into a map of path → (added, removed).
/// Pure function — no subprocess calls.
pub(crate) fn parse_numstat(output: &str) -> std::collections::HashMap<String, (usize, usize)> {
    let mut map = std::collections::HashMap::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let added = parts[0].parse::<usize>().unwrap_or(0);
        let removed = parts[1].parse::<usize>().unwrap_or(0);
        let path = parts[2].trim().to_string();
        if !path.is_empty() {
            map.insert(path, (added, removed));
        }
    }
    map
}
```

- [ ] **Step 4: Write failing tests for `parse_unified_diff`**

```rust
#[test]
fn parse_unified_diff_added_line() {
    let out = "+    let x = 1;\n";
    let lines = parse_unified_diff(out);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].kind, DiffLineKind::Added);
}

#[test]
fn parse_unified_diff_removed_line() {
    let out = "-    let x = 0;\n";
    let lines = parse_unified_diff(out);
    assert_eq!(lines[0].kind, DiffLineKind::Removed);
}

#[test]
fn parse_unified_diff_hunk_header() {
    let out = "@@ -1,4 +1,5 @@ fn main() {\n";
    let lines = parse_unified_diff(out);
    assert_eq!(lines[0].kind, DiffLineKind::HunkHeader);
}

#[test]
fn parse_unified_diff_file_header() {
    let out = "diff --git a/src/main.rs b/src/main.rs\n";
    let lines = parse_unified_diff(out);
    assert_eq!(lines[0].kind, DiffLineKind::FileHeader);
}

#[test]
fn parse_unified_diff_context_line() {
    let out = " fn existing_function() {\n";
    let lines = parse_unified_diff(out);
    assert_eq!(lines[0].kind, DiffLineKind::Context);
}

#[test]
fn parse_unified_diff_triple_plus_is_file_header() {
    let out = "+++ b/src/main.rs\n";
    let lines = parse_unified_diff(out);
    assert_eq!(lines[0].kind, DiffLineKind::FileHeader);
}

#[test]
fn parse_unified_diff_triple_minus_is_file_header() {
    let out = "--- a/src/main.rs\n";
    let lines = parse_unified_diff(out);
    assert_eq!(lines[0].kind, DiffLineKind::FileHeader);
}
```

- [ ] **Step 5: Run tests to confirm they fail**

```bash
cargo test --lib git::tests::parse_unified_diff -- --nocapture
```

Expected: compile error — `parse_unified_diff` not yet defined.

- [ ] **Step 6: Implement `parse_unified_diff`**

Add after `parse_numstat` in `src/git.rs`:

```rust
/// Parse the stdout of `git diff` (unified format) into a vec of typed lines.
/// Pure function — no subprocess calls.
pub(crate) fn parse_unified_diff(output: &str) -> Vec<DiffLine> {
    output.lines().map(|line| {
        let kind = if line.starts_with("diff ")
            || line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
        {
            DiffLineKind::FileHeader
        } else if line.starts_with("@@") {
            DiffLineKind::HunkHeader
        } else if line.starts_with('+') {
            DiffLineKind::Added
        } else if line.starts_with('-') {
            DiffLineKind::Removed
        } else {
            DiffLineKind::Context
        };
        DiffLine { kind, content: line.to_string() }
    }).collect()
}
```

- [ ] **Step 7: Run all parser tests**

```bash
cargo test --lib git -- --nocapture
```

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add src/git.rs
git commit -m "feat(git): add parse_numstat and parse_unified_diff pure parsers"
```

---

## Task 3: Subprocess functions in `src/git.rs`

**Files:**
- Modify: `src/git.rs`

- [ ] **Step 1: Add `fetch_diff_files` after `fetch_status`**

```rust
/// Fetch all files with pending changes (staged + unstaged + untracked).
///
/// Combines `git diff HEAD --numstat` (tracked changes) with
/// `git status --porcelain=v1 -u` (untracked files).
/// Returns `None` when `path` is not inside a repo or `git` is unavailable.
pub fn fetch_diff_files(path: &Path) -> Option<Vec<GitDiffFile>> {
    let root = detect_repo(path)?;

    // Get +/- counts for tracked changes
    let numstat_out = run_git(&root, &["diff", "HEAD", "--numstat"])?;
    let counts = parse_numstat(&String::from_utf8_lossy(&numstat_out.stdout));

    // Get status + untracked from porcelain
    let status_out = run_git(&root, &["status", "--porcelain=v1", "-u", "--no-optional-locks"])
        .or_else(|| run_git(&root, &["status", "--porcelain=v1", "-u"]))?;
    let statuses = parse_porcelain(&String::from_utf8_lossy(&status_out.stdout));

    let mut files: Vec<GitDiffFile> = statuses
        .into_iter()
        .map(|(rel_path, status)| {
            let key = rel_path.display().to_string().replace('\\', "/");
            let (added, removed) = counts.get(&key).copied().unwrap_or((0, 0));
            GitDiffFile { path: rel_path, status, added, removed }
        })
        .collect();

    // Sort: modified/added first, then deleted, then untracked; alpha within groups
    files.sort_by(|a, b| {
        status_sort_key(a.status).cmp(&status_sort_key(b.status))
            .then(a.path.cmp(&b.path))
    });

    Some(files)
}

fn status_sort_key(s: FileStatus) -> u8 {
    match s {
        FileStatus::Conflicted => 0,
        FileStatus::Modified   => 1,
        FileStatus::Added      => 2,
        FileStatus::Renamed    => 3,
        FileStatus::Deleted    => 4,
        FileStatus::Untracked  => 5,
    }
}
```

- [ ] **Step 2: Add `fetch_file_diff` after `fetch_diff_files`**

```rust
/// Fetch the unified diff for a single file.
///
/// For tracked files: runs `git diff HEAD -- <path>`.
/// For untracked files: reads the file directly and marks all lines as Added.
pub fn fetch_file_diff(path: &Path, rel_path: &Path, is_untracked: bool) -> Vec<DiffLine> {
    if is_untracked {
        let full_path = match detect_repo(path) {
            Some(root) => root.join(rel_path),
            None => rel_path.to_path_buf(),
        };
        return match std::fs::read_to_string(&full_path) {
            Ok(content) => content
                .lines()
                .map(|l| DiffLine { kind: DiffLineKind::Added, content: format!("+{l}") })
                .collect(),
            Err(_) => vec![DiffLine {
                kind: DiffLineKind::Context,
                content: String::from("(binary or unreadable file)"),
            }],
        };
    }

    let root = match detect_repo(path) {
        Some(r) => r,
        None => return Vec::new(),
    };

    let output = run_git(
        &root,
        &["diff", "HEAD", "--", &rel_path.display().to_string()],
    );

    match output {
        Some(o) if o.status.success() => {
            parse_unified_diff(&String::from_utf8_lossy(&o.stdout))
        }
        _ => Vec::new(),
    }
}
```

- [ ] **Step 3: Run cargo check to confirm it compiles**

```bash
cargo check 2>&1
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/git.rs
git commit -m "feat(git): add fetch_diff_files and fetch_file_diff subprocess functions"
```

---

## Task 4: FocusLayer variants and WorkspaceState fields

**Files:**
- Modify: `src/state/types.rs`
- Modify: `src/state/mod.rs`

- [ ] **Step 1: Add `GitDiffFileList` and `GitDiffContent` to `FocusLayer`**

In `src/state/types.rs`, add two variants to the `FocusLayer` enum (after the `Terminal` variant):

```rust
/// The git diff viewer file list pane has focus.
GitDiffFileList,
/// The git diff viewer diff content pane has focus.
GitDiffContent,
```

- [ ] **Step 2: Add git diff fields to `WorkspaceState`**

In `src/state/mod.rs`, add to the `WorkspaceState` struct (after the `diff_map` field):

```rust
pub git_diff_active: bool,
pub git_diff_files: Vec<crate::git::GitDiffFile>,
pub git_diff_selected: usize,
pub git_diff_lines: Vec<crate::git::DiffLine>,
pub git_diff_scroll: usize,
pub git_diff_focus_content: bool,   // false = FileList focused, true = DiffContent focused
```

- [ ] **Step 3: Initialise the new fields in `WorkspaceState::new`**

In `WorkspaceState::new`, add after `diff_map: ...,`:

```rust
git_diff_active: false,
git_diff_files: Vec::new(),
git_diff_selected: 0,
git_diff_lines: Vec::new(),
git_diff_scroll: 0,
git_diff_focus_content: false,
```

- [ ] **Step 4: Add `focus_layer` branch for git diff mode**

In `AppState::focus_layer()` in `src/state/mod.rs`, add **before** the final `FocusLayer::Pane` return at the bottom:

```rust
if self.git_diff_active {
    return if self.git_diff_focus_content {
        FocusLayer::GitDiffContent
    } else {
        FocusLayer::GitDiffFileList
    };
}
```

- [ ] **Step 5: Write a unit test for the new focus_layer branches**

Add to the `#[cfg(test)] mod tests` block in `src/state/mod.rs`:

```rust
#[test]
fn focus_layer_returns_git_diff_file_list_when_active() {
    let mut state = AppState::default();
    state.git_diff_active = true;
    state.git_diff_focus_content = false;
    assert_eq!(state.focus_layer(), FocusLayer::GitDiffFileList);
}

#[test]
fn focus_layer_returns_git_diff_content_when_content_focused() {
    let mut state = AppState::default();
    state.git_diff_active = true;
    state.git_diff_focus_content = true;
    assert_eq!(state.focus_layer(), FocusLayer::GitDiffContent);
}
```

- [ ] **Step 6: Run the tests**

```bash
cargo test --lib state -- --nocapture
```

Expected: all pass including the two new tests.

- [ ] **Step 7: Commit**

```bash
git add src/state/types.rs src/state/mod.rs
git commit -m "feat(state): add git diff focus layers and WorkspaceState fields"
```

---

## Task 5: New actions and key routing

**Files:**
- Modify: `src/action.rs`

- [ ] **Step 1: Add new action variants to the `Action` enum**

In `src/action.rs`, add these variants to the `Action` enum (after `ToggleDiffMode`):

```rust
ToggleGitDiff,
GitDiffSelectNext,
GitDiffSelectPrev,
GitDiffScrollDown,
GitDiffScrollUp,
GitDiffPageDown,
GitDiffPageUp,
GitDiffTop,
GitDiffBottom,
GitDiffToggleFocus,
```

- [ ] **Step 2: Add `Ctrl+D` routing for `ToggleGitDiff`**

In `src/action.rs`, find the `from_pane_key_event` function (the one handling normal pane keys). Add **near the top**, before the existing F-key matches:

```rust
// Ctrl+D — toggle git diff viewer
if key_event.code == KeyCode::Char('d')
    && key_event.modifiers == KeyModifiers::CONTROL
{
    return Some(Self::ToggleGitDiff);
}
```

- [ ] **Step 3: Add a key routing function for `GitDiffFileList` focus**

Add a new public function after `from_pane_key_event`:

```rust
/// Key routing when the git diff file list pane has focus.
pub fn from_git_diff_file_list_key_event(key_event: KeyEvent) -> Option<Self> {
    // Ctrl+D exits the mode from either pane
    if key_event.code == KeyCode::Char('d')
        && key_event.modifiers == KeyModifiers::CONTROL
    {
        return Some(Self::ToggleGitDiff);
    }
    match key_event.code {
        KeyCode::Down | KeyCode::Char('j') => Some(Self::GitDiffSelectNext),
        KeyCode::Up   | KeyCode::Char('k') => Some(Self::GitDiffSelectPrev),
        KeyCode::Tab                        => Some(Self::GitDiffToggleFocus),
        _                                   => None,
    }
}
```

- [ ] **Step 4: Add a key routing function for `GitDiffContent` focus**

```rust
/// Key routing when the git diff content pane has focus.
pub fn from_git_diff_content_key_event(key_event: KeyEvent) -> Option<Self> {
    if key_event.code == KeyCode::Char('d')
        && key_event.modifiers == KeyModifiers::CONTROL
    {
        return Some(Self::ToggleGitDiff);
    }
    match key_event.code {
        KeyCode::Down  | KeyCode::Char('j') => Some(Self::GitDiffScrollDown),
        KeyCode::Up    | KeyCode::Char('k') => Some(Self::GitDiffScrollUp),
        KeyCode::PageDown => Some(Self::GitDiffPageDown),
        KeyCode::Char('d') => Some(Self::GitDiffPageDown),  // 'd' only reachable when modifiers == NONE (Ctrl+D caught above)
        KeyCode::PageUp => Some(Self::GitDiffPageUp),
        KeyCode::Char('u') => Some(Self::GitDiffPageUp),
        KeyCode::Char('g') => Some(Self::GitDiffTop),
        KeyCode::Char('G') => Some(Self::GitDiffBottom),
        KeyCode::Tab       => Some(Self::GitDiffToggleFocus),
        _                  => None,
    }
}
```

- [ ] **Step 5: Wire the new routing functions into `src/app.rs`**

In `src/app.rs`, find where `Action::from_pane_key_event` is called for key dispatch. Add branches for the two new focus layers before the existing pane dispatch:

```rust
FocusLayer::GitDiffFileList => {
    Action::from_git_diff_file_list_key_event(key_event)
}
FocusLayer::GitDiffContent => {
    Action::from_git_diff_content_key_event(key_event)
}
```

- [ ] **Step 6: Write unit tests for key routing**

Add to the tests block in `src/action.rs`:

```rust
#[test]
fn ctrl_d_maps_to_toggle_git_diff() {
    let ev = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_eq!(
        Action::from_git_diff_file_list_key_event(ev),
        Some(Action::ToggleGitDiff)
    );
}

#[test]
fn j_maps_to_select_next_in_file_list() {
    let ev = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        Action::from_git_diff_file_list_key_event(ev),
        Some(Action::GitDiffSelectNext)
    );
}

#[test]
fn tab_maps_to_toggle_focus_from_file_list() {
    let ev = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(
        Action::from_git_diff_file_list_key_event(ev),
        Some(Action::GitDiffToggleFocus)
    );
}

#[test]
fn j_maps_to_scroll_down_in_content() {
    let ev = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        Action::from_git_diff_content_key_event(ev),
        Some(Action::GitDiffScrollDown)
    );
}
```

- [ ] **Step 7: Run all action tests**

```bash
cargo test --lib action -- --nocapture
```

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add src/action.rs src/app.rs
git commit -m "feat(action): add git diff actions and key routing functions"
```

---

## Task 6: Action handlers in `src/state/mod.rs`

**Files:**
- Modify: `src/state/mod.rs`

- [ ] **Step 1: Write a failing test for `ToggleGitDiff`**

Add to the tests block in `src/state/mod.rs`:

```rust
#[test]
fn toggle_git_diff_sets_active_flag() {
    let mut state = AppState::default();
    assert!(!state.git_diff_active);
    state.handle_action(Action::ToggleGitDiff);
    assert!(state.git_diff_active);
    state.handle_action(Action::ToggleGitDiff);
    assert!(!state.git_diff_active);
}

#[test]
fn toggle_git_diff_off_clears_lines_and_scroll() {
    let mut state = AppState::default();
    state.git_diff_active = true;
    state.git_diff_lines = vec![crate::git::DiffLine {
        kind: crate::git::DiffLineKind::Added,
        content: String::from("+foo"),
    }];
    state.git_diff_scroll = 5;
    state.handle_action(Action::ToggleGitDiff);
    assert!(!state.git_diff_active);
    assert!(state.git_diff_lines.is_empty());
    assert_eq!(state.git_diff_scroll, 0);
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test --lib state::tests::toggle_git_diff -- --nocapture
```

Expected: compile error or test failure — handler not yet written.

- [ ] **Step 3: Implement `ToggleGitDiff` handler**

In the main `match action` block in `WorkspaceState::handle_action` (or its equivalent in `AppState`), add after the `ToggleDiffMode` arm:

```rust
Action::ToggleGitDiff => {
    if self.git_diff_active {
        // Exit mode — clear all diff state
        self.git_diff_active = false;
        self.git_diff_files.clear();
        self.git_diff_selected = 0;
        self.git_diff_lines.clear();
        self.git_diff_scroll = 0;
        self.git_diff_focus_content = false;
        self.status_message = String::from("git diff closed");
    } else {
        // Enter mode — load changed files
        let cwd = self.panes.active_pane().cwd.clone();
        let files = crate::git::fetch_diff_files(&cwd).unwrap_or_default();
        if files.is_empty() {
            self.status_message = String::from("no changes in working tree");
            return;
        }
        // Load diff for first file
        let first = &files[0];
        let is_untracked = first.status == crate::git::FileStatus::Untracked;
        let lines = crate::git::fetch_file_diff(&cwd, &first.path, is_untracked);
        let count = files.len();
        self.git_diff_files = files;
        self.git_diff_selected = 0;
        self.git_diff_lines = lines;
        self.git_diff_scroll = 0;
        self.git_diff_focus_content = false;
        self.git_diff_active = true;
        self.status_message = format!("git diff — {count} file(s) changed");
    }
}
```

- [ ] **Step 4: Implement `GitDiffSelectNext` and `GitDiffSelectPrev`**

```rust
Action::GitDiffSelectNext => {
    if self.git_diff_active && !self.git_diff_files.is_empty() {
        let max = self.git_diff_files.len() - 1;
        if self.git_diff_selected < max {
            self.git_diff_selected += 1;
            self.load_selected_diff();
        }
    }
}
Action::GitDiffSelectPrev => {
    if self.git_diff_active && self.git_diff_selected > 0 {
        self.git_diff_selected -= 1;
        self.load_selected_diff();
    }
}
```

Add the helper method on `WorkspaceState`:

```rust
fn load_selected_diff(&mut self) {
    let cwd = self.panes.active_pane().cwd.clone();
    if let Some(file) = self.git_diff_files.get(self.git_diff_selected) {
        let is_untracked = file.status == crate::git::FileStatus::Untracked;
        self.git_diff_lines = crate::git::fetch_file_diff(&cwd, &file.path, is_untracked);
        self.git_diff_scroll = 0;
    }
}
```

- [ ] **Step 5: Implement scroll and focus actions**

```rust
Action::GitDiffScrollDown => {
    if self.git_diff_active {
        let max = self.git_diff_lines.len().saturating_sub(1);
        if self.git_diff_scroll < max {
            self.git_diff_scroll += 1;
        }
    }
}
Action::GitDiffScrollUp => {
    if self.git_diff_active && self.git_diff_scroll > 0 {
        self.git_diff_scroll -= 1;
    }
}
Action::GitDiffPageDown => {
    if self.git_diff_active {
        let max = self.git_diff_lines.len().saturating_sub(1);
        self.git_diff_scroll = (self.git_diff_scroll + 20).min(max);
    }
}
Action::GitDiffPageUp => {
    if self.git_diff_active {
        self.git_diff_scroll = self.git_diff_scroll.saturating_sub(20);
    }
}
Action::GitDiffTop => {
    if self.git_diff_active {
        self.git_diff_scroll = 0;
    }
}
Action::GitDiffBottom => {
    if self.git_diff_active {
        let max = self.git_diff_lines.len().saturating_sub(1);
        self.git_diff_scroll = max;
    }
}
Action::GitDiffToggleFocus => {
    if self.git_diff_active {
        self.git_diff_focus_content = !self.git_diff_focus_content;
    }
}
```

- [ ] **Step 6: Write tests for select, scroll, and focus handlers**

```rust
#[test]
fn git_diff_select_next_advances_selection_and_resets_scroll() {
    let mut state = AppState::default();
    state.git_diff_active = true;
    state.git_diff_files = vec![
        crate::git::GitDiffFile { path: PathBuf::from("a.rs"), status: crate::git::FileStatus::Modified, added: 1, removed: 0 },
        crate::git::GitDiffFile { path: PathBuf::from("b.rs"), status: crate::git::FileStatus::Modified, added: 2, removed: 1 },
    ];
    state.git_diff_scroll = 10;
    // Note: load_selected_diff will call fetch_file_diff which shells out;
    // in a unit test context with no git repo the result is an empty vec — that's fine.
    state.handle_action(Action::GitDiffSelectNext);
    assert_eq!(state.git_diff_selected, 1);
    assert_eq!(state.git_diff_scroll, 0);
}

#[test]
fn git_diff_select_prev_does_not_underflow() {
    let mut state = AppState::default();
    state.git_diff_active = true;
    state.git_diff_selected = 0;
    state.git_diff_files = vec![
        crate::git::GitDiffFile { path: PathBuf::from("a.rs"), status: crate::git::FileStatus::Modified, added: 1, removed: 0 },
    ];
    state.handle_action(Action::GitDiffSelectPrev);
    assert_eq!(state.git_diff_selected, 0);
}

#[test]
fn git_diff_scroll_down_does_not_exceed_line_count() {
    let mut state = AppState::default();
    state.git_diff_active = true;
    state.git_diff_lines = vec![
        crate::git::DiffLine { kind: crate::git::DiffLineKind::Added, content: String::from("+a") },
        crate::git::DiffLine { kind: crate::git::DiffLineKind::Added, content: String::from("+b") },
    ];
    state.git_diff_scroll = 1;
    state.handle_action(Action::GitDiffScrollDown);
    assert_eq!(state.git_diff_scroll, 1); // already at max (len-1)
}

#[test]
fn git_diff_toggle_focus_flips_flag() {
    let mut state = AppState::default();
    state.git_diff_active = true;
    state.git_diff_focus_content = false;
    state.handle_action(Action::GitDiffToggleFocus);
    assert!(state.git_diff_focus_content);
    state.handle_action(Action::GitDiffToggleFocus);
    assert!(!state.git_diff_focus_content);
}
```

- [ ] **Step 7: Run all state tests**

```bash
cargo test --lib state -- --nocapture
```

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add src/state/mod.rs
git commit -m "feat(state): implement git diff action handlers"
```

---

## Task 7: UI renderer `src/ui/git_diff.rs`

**Files:**
- Create: `src/ui/git_diff.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Create `src/ui/git_diff.rs` with the file list renderer**

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::git::{DiffLineKind, FileStatus};
use crate::state::AppState;

/// Render the full git diff view, splitting `area` into a file-list pane (left)
/// and a diff-content pane (right).
pub fn render_git_diff_view(frame: &mut Frame<'_>, area: Rect, state: &AppState, palette: ThemePalette) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    render_diff_file_list(frame, chunks[0], state, palette);
    render_diff_content(frame, chunks[1], state, palette);
}

fn render_diff_file_list(frame: &mut Frame<'_>, area: Rect, state: &AppState, palette: ThemePalette) {
    let focused = !state.git_diff_focus_content;
    let border_style = if focused {
        Style::default().fg(palette.border_focus).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_muted)
    };
    let title_prefix = if focused { "◉" } else { "○" };
    let count = state.git_diff_files.len();
    let title = format!(" {title_prefix} Changed Files ({count}) ");

    let block = Block::default()
        .title(title)
        .borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = state
        .git_diff_files
        .iter()
        .map(|f| {
            let (status_char, status_colour) = status_display(f.status);
            let name = f.path.display().to_string();
            let counts = if f.added > 0 || f.removed > 0 {
                format!(" +{} -{}", f.added, f.removed)
            } else {
                String::new()
            };
            Line::from(vec![
                Span::styled(
                    format!("{status_char} "),
                    Style::default().fg(status_colour),
                ),
                Span::raw(name),
                Span::styled(counts, Style::default().fg(Color::DarkGray)),
            ])
            .into()
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.git_diff_selected));

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(palette.selection_bg)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_stateful_widget(list, inner, &mut list_state);
}

fn render_diff_content(frame: &mut Frame<'_>, area: Rect, state: &AppState, palette: ThemePalette) {
    let focused = state.git_diff_focus_content;
    let border_style = if focused {
        Style::default().fg(palette.border_focus).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_muted)
    };
    let title_prefix = if focused { "◉" } else { "○" };

    let filename = state
        .git_diff_files
        .get(state.git_diff_selected)
        .map(|f| f.path.display().to_string())
        .unwrap_or_default();

    let total = state.git_diff_lines.len();
    let scroll = state.git_diff_scroll;
    let line_info = if total > 0 {
        format!("  [line {}/{total}]", scroll + 1)
    } else {
        String::new()
    };
    let title = format!(" {title_prefix} Diff · {filename}{line_info} ");

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.git_diff_lines.is_empty() {
        let msg = if state.git_diff_files.is_empty() {
            "No changes in working tree"
        } else {
            "No diff available for this file"
        };
        let para = Paragraph::new(msg).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(para, inner);
        return;
    }

    let visible_height = inner.height as usize;
    let lines: Vec<Line> = state
        .git_diff_lines
        .iter()
        .skip(scroll)
        .take(visible_height)
        .map(|dl| diff_line_to_ratatui(dl))
        .collect();

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn diff_line_to_ratatui(dl: &crate::git::DiffLine) -> Line<'static> {
    let (colour, bold) = match dl.kind {
        DiffLineKind::Added      => (Color::Green,    true),
        DiffLineKind::Removed    => (Color::Red,      true),
        DiffLineKind::HunkHeader => (Color::Blue,     false),
        DiffLineKind::FileHeader => (Color::Cyan,     false),
        DiffLineKind::Context    => (Color::DarkGray, false),
    };
    let mut style = Style::default().fg(colour);
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    Line::from(Span::styled(dl.content.clone(), style))
}

fn status_display(status: FileStatus) -> (&'static str, Color) {
    match status {
        FileStatus::Modified   => ("M", Color::Yellow),
        FileStatus::Added      => ("A", Color::Green),
        FileStatus::Deleted    => ("D", Color::Red),
        FileStatus::Renamed    => ("R", Color::Cyan),
        FileStatus::Conflicted => ("U", Color::Red),
        FileStatus::Untracked  => ("?", Color::DarkGray),
    }
}
```

- [ ] **Step 2: Register the module in `src/ui/mod.rs`**

At the top of `src/ui/mod.rs`, add alongside the other module declarations:

```rust
mod git_diff;
pub(crate) use git_diff::render_git_diff_view;
```

- [ ] **Step 3: Wire the renderer into the layout in `src/ui/mod.rs`**

In the `render` function of `src/ui/mod.rs`, find the block beginning with `if !editor_fullscreen {` (around line 122). Replace the opening of that block with:

```rust
if state.git_diff_active {
    let palette = state.theme().palette;
    render_git_diff_view(frame, pane_area, state, palette);
} else if !editor_fullscreen {
```

Make sure the existing `}` closes the `else if` block correctly, keeping all the existing pane / tools rendering inside the `else if` branch.

- [ ] **Step 4: Add hint bar hints for git diff focus layers**

In `src/ui/mod.rs`, find the `render_key_hints` function (around line 533). In the `match state.focus_layer()` block, add two new arms **before** the `_ =>` fallback arm:

```rust
        crate::state::FocusLayer::GitDiffFileList => &[
            ("\u{2191}\u{2193}", "Navigate"),
            ("Tab", "Focus diff"),
            ("Ctrl+D", "Exit"),
        ],
        crate::state::FocusLayer::GitDiffContent => &[
            ("\u{2191}\u{2193}", "Scroll"),
            ("PgUp/Dn", "Page"),
            ("g/G", "Top/Bottom"),
            ("Tab", "Focus files"),
            ("Ctrl+D", "Exit"),
        ],

- [ ] **Step 5: Run cargo check**

```bash
cargo check 2>&1
```

Expected: no errors.

- [ ] **Step 6: Build and smoke-test manually**

```bash
cargo build 2>&1
./target/debug/zeta
```

Press `Ctrl+D` inside a git repo — the diff viewer should appear. `Tab` should switch focus. `↑↓` should scroll the file list and diff content. `Ctrl+D` should exit.

- [ ] **Step 7: Run the full test suite**

```bash
cargo test --workspace 2>&1
```

Expected: all pass (or pre-existing failures only).

- [ ] **Step 8: Commit**

```bash
git add src/ui/git_diff.rs src/ui/mod.rs
git commit -m "feat(ui): add git diff viewer rendering — file list + diff content panes"
```

---

## Task 8: README and CHANGELOG

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add a Git Integration section to README.md**

Find the `### SSH/SFTP Remote Filesystems` section in `README.md`. Insert the following **before** it:

```markdown
### Git Integration

- **Per-file status badges** in both panes: `M` (modified), `A` (added), `D` (deleted), `R` (renamed), `U` (conflict), `?` (untracked) — colour-coded in the file name gutter
- **Git branch indicator** shown in the status bar when the active pane is inside a repository
- **Git diff viewer** (`Ctrl+D`) — press `Ctrl+D` to enter an inline diff view:
  - Left pane: list of changed files with status badge and `+N −N` line counts
  - Right pane: scrollable unified diff with syntax-coloured `+`/`−` lines and hunk headers
  - `↑`/`↓` or `j`/`k` to navigate files (left pane) or scroll diff lines (right pane)
  - `Tab` to toggle focus between file list and diff content
  - `PgUp` / `PgDn` / `g` / `G` for fast diff navigation
  - `Ctrl+D` again to close and return to the file manager

> The existing **directory diff mode** (`F9`) compares files between the left and right panes — it remains unchanged.
```

- [ ] **Step 2: Add an Unreleased entry to CHANGELOG.md**

Find the `## [Unreleased]` section and add under `### Added`:

```markdown
- **Git diff viewer** (`Ctrl+D`): inline diff mode showing changed files (left pane) with scrollable unified diff (right pane). Navigate files with `↑↓`/`jk`, scroll diff with `↑↓`/`jk`/`PgUp`/`PgDn`/`g`/`G`, toggle pane focus with `Tab`, exit with `Ctrl+D`. Works with all staged, unstaged, and untracked changes. No new dependencies — shells out to `git diff HEAD`.
```

- [ ] **Step 3: Commit**

```bash
git add README.md CHANGELOG.md
git commit -m "docs: add git diff viewer to README and CHANGELOG"
```

---

## Task 9: GitHub Pages site update

**Files:**
- Modify: `site/index.html`

- [ ] **Step 1: Add a git diff feature card**

In `site/index.html`, find the `<div class="feature-grid">` block. Add a new feature card for git diff (add it after the existing git status card if one exists, or as the next card):

```html
<div class="feature-card animate">
  <div class="feature-icon">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <line x1="12" y1="5" x2="12" y2="19"/>
      <polyline points="19 12 12 5 5 12"/>
    </svg>
  </div>
  <h3>Git Diff Viewer</h3>
  <p>Press <code>Ctrl+D</code> to open an inline diff view — changed files on the left, scrollable unified diff on the right. Navigate with <code>j/k</code>, switch panes with <code>Tab</code>.</p>
</div>
```

- [ ] **Step 2: Update the transition-delay CSS if there is now a 9th or 10th card**

Check how many `.feature-card` elements exist after adding the new one. If needed, the CSS already handles up to 10 cards (`.feature-card:nth-child(9)` and `:nth-child(10)` are defined). No change needed unless the count exceeds 10.

- [ ] **Step 3: Run cargo clippy and fmt before final commit**

```bash
cargo fmt --all -- --check 2>&1
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1
cargo test --workspace 2>&1
```

Expected: all clean.

- [ ] **Step 4: Final commit**

```bash
git add site/index.html
git commit -m "site: add git diff viewer feature card"
```

---

## Pre-PR Checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] Manual smoke test: `Ctrl+D` opens diff viewer, `Tab` switches panes, `↑↓` scrolls, `Ctrl+D` closes
- [ ] Manual smoke test: outside a git repo, `Ctrl+D` shows "not a git repository" in status bar and does not activate
- [ ] `F9` (directory diff) still works normally
