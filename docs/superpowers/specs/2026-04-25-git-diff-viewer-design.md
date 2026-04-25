# Git Diff Viewer — Design Spec

**Status:** Approved  
**Branch:** `feature/git-diff-viewer`  
**Scope:** Core only — file list + scrollable diff pane (no staging, no side-by-side, no log browser)

---

## Problem & Goal

Zeta already displays per-file git status badges (M/A/D/R/?) in the pane gutter via `src/git.rs`. Users have no way to inspect the actual diff content without leaving the app. The goal is a lightweight, integrated git diff viewer that fits Zeta's existing dual-pane model and keyboard-first UX.

---

## Approach

**Pane Content Type (Approach B).** When the user presses `Ctrl+D`, the workspace enters `git_diff_active` mode. The left pane is replaced by a list of changed files (all staged + unstaged + untracked changes vs HEAD). The right pane shows the unified diff for the selected file. Both panes are independently focusable via `Tab`. `Ctrl+D` toggles the mode off and restores normal file manager state.

F9 (directory diff mode) is unchanged — it serves a different purpose (left pane vs right pane filesystem comparison) and the two modes coexist independently.

---

## Data Model

New types added to `src/git.rs`:

```rust
/// A file with pending git changes, shown in the diff viewer file list.
pub struct GitDiffFile {
    pub path: PathBuf,
    pub status: FileStatus,   // reuses existing enum
    pub added: usize,
    pub removed: usize,
}

/// The semantic kind of a line in a unified diff.
pub enum DiffLineKind {
    Added,
    Removed,
    Context,
    HunkHeader,
    FileHeader,
}

/// A single line of parsed unified diff output.
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}
```

New functions in `src/git.rs` (shell-out pattern, same as `run_git`):

- `fetch_diff_files(repo_root: &Path) -> Vec<GitDiffFile>`  
  Runs `git diff HEAD --numstat` (for ±counts) combined with `git status --porcelain=v1` (for status flags). Merges the two to produce the file list. Untracked files show `Added` status with no line counts.

- `fetch_file_diff(repo_root: &Path, path: &Path) -> Vec<DiffLine>`  
  Runs `git diff HEAD -- <path>` and parses the unified diff output line-by-line. For untracked files, runs `git diff --no-index /dev/null <path>` (Linux/macOS) or equivalent.

No new Cargo dependencies required.

---

## State

New fields on `WorkspaceState` in `src/state/mod.rs`:

```rust
pub git_diff_active: bool,
pub git_diff_files: Vec<crate::git::GitDiffFile>,
pub git_diff_selected: usize,
pub git_diff_lines: Vec<crate::git::DiffLine>,
pub git_diff_scroll: usize,
pub git_diff_focus: GitDiffFocus,
```

New enum in `src/state/types.rs`:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GitDiffFocus {
    #[default]
    FileList,
    DiffContent,
}
```

On `ToggleGitDiff` (entering): populate `git_diff_files` from `git::fetch_diff_files`, select index 0, load `git_diff_lines` for that file, reset scroll to 0, focus = `FileList`.  
On `ToggleGitDiff` (exiting): clear all fields, restore normal pane rendering.

---

## Actions

New variants added to `Action` in `src/action.rs`:

| Action | Binding | Context |
|---|---|---|
| `ToggleGitDiff` | `Ctrl+D` | Pane mode (enter/exit) |
| `GitDiffSelectNext` | `↓` / `j` | FileList focused |
| `GitDiffSelectPrev` | `↑` / `k` | FileList focused |
| `GitDiffScrollDown` | `↓` / `j` | DiffContent focused |
| `GitDiffScrollUp` | `↑` / `k` | DiffContent focused |
| `GitDiffPageDown` | `PgDn` / `d` | DiffContent focused |
| `GitDiffPageUp` | `PgUp` / `u` | DiffContent focused |
| `GitDiffTop` | `g` | DiffContent focused |
| `GitDiffBottom` | `G` | DiffContent focused |
| `GitDiffToggleFocus` | `Tab` | Either focus |

Key routing: when `git_diff_active` is true, key events are dispatched to git diff actions before the normal pane handlers. `Ctrl+D` is checked first in `from_pane_key_event`.

---

## UI

New module: `src/ui/git_diff.rs`

```
render_git_diff_view(f, area, state)
  ├── split area into left (38%) / right (62%)
  ├── render_diff_file_list(f, left_area, state)   — scrollable list with status badge + ±counts
  └── render_diff_content(f, right_area, state)    — scrollable diff lines with colour coding
```

Called from the workspace layout renderer (`src/ui/layout_cache.rs` or equivalent) when `git_diff_active` is true, replacing the normal dual-pane area.

**Colour coding for diff lines:**

| Kind | Colour |
|---|---|
| `Added` | Green (`#a6e3a1`) |
| `Removed` | Red (`#f38ba8`) |
| `HunkHeader` | Blue (`#89b4fa`), dimmed background |
| `FileHeader` | Cyan (`#89dceb`) |
| `Context` | Dim (`Color::DarkGray`) |

**Pane titles:**
- Left: `◉ Changed Files (N)` when focused, `○ Changed Files (N)` when unfocused
- Right: `◉ Diff · <filename>  [line X/Y]` when focused, `○ Diff · <filename>` when unfocused

**Status bar** shows: `git diff — N files changed · +A −R`

**Footer hint bar** (below layout):
- FileList focus: `↑↓ navigate · Tab focus diff · Ctrl+D exit`
- DiffContent focus: `↑↓ scroll · PgUp/PgDn page · g/G top/bottom · Tab focus files · Ctrl+D exit`

---

## Error Handling

| Condition | Behaviour |
|---|---|
| Not in a git repo | Status bar: "not a git repository" — mode does not activate |
| No changes in working tree | Left pane shows "No changes" placeholder, right pane empty |
| `git` not on PATH | Status bar error: "git not found" — same no-op pattern as `detect_repo` |
| File has no diff (e.g. untracked binary) | Right pane: "no diff available for this file" |
| Git subprocess fails | Status bar shows error message, diff lines are empty |

---

## README & Site Updates

Alongside the feature implementation:

- **`README.md`** — add a "Git Integration" section documenting: (1) per-file status badges already shown in panes, (2) the new `Ctrl+D` git diff viewer with key reference table
- **`CHANGELOG.md`** — add entry under `[Unreleased]` describing the git diff viewer
- **`site/`** — update the GitHub Pages site to reflect v0.4.3 features and the new diff viewer

---

## Out of Scope (v1)

- Stage / unstage individual files
- Side-by-side split diff view
- Commit log browser
- Compare against arbitrary ref / branch
- AI commit message generation
- Push / pull / branch operations
- `git2` / libgit2 dependency
