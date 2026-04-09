# Wave 4A — Git Integration

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show git status indicators next to file entries in both panes, display the current branch name in the status bar, and wire a dedicated `GitWorker` into the existing three-worker job system — all with zero new crate dependencies (uses `std::process::Command` to shell out to `git`).

**Architecture:**
- `src/git.rs` — new module: `RepoStatus`, `FileStatus`, `detect_repo()`, `fetch_status()`. Pure functions; no state.
- `src/jobs.rs` — add `GitStatusRequest`, `JobResult::GitStatusLoaded`, and a fourth `git_tx` sender on `WorkerChannels`. The git worker runs `detect_repo` then `fetch_status` in a loop, fanning results into the shared `Receiver<JobResult>`.
- `src/state/mod.rs` — add `git: [Option<RepoStatus>; 2]` (indexed by `PaneId`) and `apply_job_result` handling for `GitStatusLoaded`. Expose `git_status(pane: PaneId) -> Option<&RepoStatus>` and update `status_line()` to include the branch name.
- `src/app.rs` — trigger a `GitStatusRequest` alongside every `ScanPane` command.
- `src/ui/pane.rs` — add a one-character git indicator column to `render_entry_row`, looking up the entry path in `RepoStatus::file_statuses`.

**No new dependencies.** `git` must be on `PATH`; if it is not (or the directory is not a repo), `RepoStatus` is `None` and panes render exactly as today.

**Tech Stack:** Rust std only (`std::process::Command`, `std::collections::HashMap`). Touches `src/git.rs` (new), `src/jobs.rs`, `src/state/mod.rs`, `src/app.rs`, `src/ui/pane.rs`.

**Jira:** ZTA-88 (ZTA-125 through ZTA-130)

**Wave dependency:** Starts AFTER Wave 3A is merged. No other waves depend on this.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/git.rs` | `RepoStatus`, `FileStatus`, `detect_repo`, `fetch_status` |
| Modify | `src/lib.rs` | `pub mod git;` |
| Modify | `src/jobs.rs` | `GitStatusRequest`, `JobResult::GitStatusLoaded`, fourth worker |
| Modify | `src/state/mod.rs` | `git: [Option<RepoStatus>; 2]`, `apply_job_result`, `status_line`, accessor |
| Modify | `src/app.rs` | Dispatch `GitStatusRequest` on `ScanPane` |
| Modify | `src/ui/pane.rs` | Git indicator column in entry rows |

---

## Git status indicators

| Symbol | Meaning | Colour suggestion |
|---|---|---|
| `M` | Modified (tracked, changed in working tree) | Yellow |
| `A` | Added / staged new file | Green |
| `D` | Deleted | Red |
| `R` | Renamed | Cyan |
| `?` | Untracked | Dim white |
| `U` | Conflicted (both sides modified) | Red bold |
| ` ` | Clean / not in repo | — |

Priority when both index and working-tree flags are set: `U > A > M > D > R > ?`.

---

## `git status --porcelain=v1` output format (reference)

```
XY path
XY old_path -> new_path   (renames)
```

`X` = index status, `Y` = working-tree status. Both can be space, `M`, `A`, `D`, `R`, `C`, `U`, or `?`/`!`.

Examples:
```
 M src/main.rs        working-tree modified, not staged
M  src/lib.rs         staged modification
?? new_file.txt       untracked
A  added.rs           staged new file
D  deleted.rs         staged deletion
UU conflict.rs        merge conflict
R  old.rs -> new.rs   renamed
```

---

## Task 1: Create `src/git.rs`

**Files:**
- Create: `src/git.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1.1: Write the failing tests**

Create `src/git.rs` with the test module only (no implementation yet):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_porcelain_modified() {
        let output = " M src/main.rs\n";
        let statuses = parse_porcelain(output);
        assert_eq!(statuses.get(std::path::Path::new("src/main.rs")), Some(&FileStatus::Modified));
    }

    #[test]
    fn parse_porcelain_untracked() {
        let output = "?? new_file.txt\n";
        let statuses = parse_porcelain(output);
        assert_eq!(statuses.get(std::path::Path::new("new_file.txt")), Some(&FileStatus::Untracked));
    }

    #[test]
    fn parse_porcelain_staged_new() {
        let output = "A  staged.rs\n";
        let statuses = parse_porcelain(output);
        assert_eq!(statuses.get(std::path::Path::new("staged.rs")), Some(&FileStatus::Added));
    }

    #[test]
    fn parse_porcelain_rename() {
        let output = "R  old.rs -> new.rs\n";
        let statuses = parse_porcelain(output);
        assert_eq!(statuses.get(std::path::Path::new("new.rs")), Some(&FileStatus::Renamed));
    }

    #[test]
    fn parse_porcelain_conflict() {
        let output = "UU conflict.rs\n";
        let statuses = parse_porcelain(output);
        assert_eq!(statuses.get(std::path::Path::new("conflict.rs")), Some(&FileStatus::Conflicted));
    }

    #[test]
    fn parse_porcelain_empty_gives_no_entries() {
        let statuses = parse_porcelain("");
        assert!(statuses.is_empty());
    }

    #[test]
    fn file_status_indicator_and_colour() {
        assert_eq!(FileStatus::Modified.symbol(), 'M');
        assert_eq!(FileStatus::Untracked.symbol(), '?');
        assert_eq!(FileStatus::Added.symbol(), 'A');
        assert_eq!(FileStatus::Conflicted.symbol(), 'U');
    }
}
```

- [ ] **Step 1.2: Confirm they fail**

```bash
cargo test parse_porcelain 2>&1 | head -5
```

Expected: compile error — `git` module not found.

- [ ] **Step 1.3: Add `pub mod git;` to `src/lib.rs`**

- [ ] **Step 1.4: Implement `src/git.rs`**

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Per-file git status, collapsed to the most significant flag.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileStatus {
    /// Merge conflict (both sides modified).
    Conflicted,
    /// Staged new file.
    Added,
    /// Modified in working tree or index.
    Modified,
    /// Deleted.
    Deleted,
    /// Renamed.
    Renamed,
    /// Not tracked by git.
    Untracked,
}

impl FileStatus {
    /// Single-character indicator shown in the pane gutter.
    pub fn symbol(self) -> char {
        match self {
            Self::Conflicted => 'U',
            Self::Added      => 'A',
            Self::Modified   => 'M',
            Self::Deleted    => 'D',
            Self::Renamed    => 'R',
            Self::Untracked  => '?',
        }
    }

    /// Ratatui colour for the indicator.
    pub fn colour(self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Self::Conflicted => Color::Red,
            Self::Added      => Color::Green,
            Self::Modified   => Color::Yellow,
            Self::Deleted    => Color::Red,
            Self::Renamed    => Color::Cyan,
            Self::Untracked  => Color::DarkGray,
        }
    }
}

/// Snapshot of `git status` for one repository.
#[derive(Clone, Debug)]
pub struct RepoStatus {
    /// Absolute path to the repository root.
    pub root: PathBuf,
    /// Current branch name (e.g. `"main"`, `"HEAD"` when detached).
    pub branch: String,
    /// Status of every dirty file, keyed by path relative to `root`.
    pub file_statuses: HashMap<PathBuf, FileStatus>,
}

impl RepoStatus {
    /// Look up the status for an absolute path, returning `None` for clean files.
    pub fn status_for(&self, absolute_path: &Path) -> Option<FileStatus> {
        let relative = absolute_path.strip_prefix(&self.root).ok()?;
        self.file_statuses.get(relative).copied()
    }
}

// ---------------------------------------------------------------------------
// Detection and fetching
// ---------------------------------------------------------------------------

/// Returns the repo root if `path` is inside a git repository, else `None`.
///
/// Shells out to `git rev-parse --show-toplevel`. Returns `None` if `git` is
/// not on `PATH`, the path is not inside a repo, or any other error occurs.
pub fn detect_repo(path: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root = String::from_utf8(output.stdout).ok()?;
    Some(PathBuf::from(root.trim()))
}

/// Fetch a full `RepoStatus` for the repository that contains `path`.
///
/// Returns `None` when the path is not inside a repo or `git` is unavailable.
pub fn fetch_status(path: &Path) -> Option<RepoStatus> {
    let root = detect_repo(path)?;

    // Current branch / ref name.
    let branch = current_branch(&root).unwrap_or_else(|| String::from("HEAD"));

    // File statuses.
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "-u"])
        .current_dir(&root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let file_statuses = parse_porcelain(&text);

    Some(RepoStatus { root, branch, file_statuses })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Returns the current branch name from `git symbolic-ref --short HEAD`.
fn current_branch(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if output.status.success() {
        return Some(String::from_utf8(output.stdout).ok()?.trim().to_string());
    }
    // Detached HEAD — use the short commit hash instead.
    let hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if hash.status.success() {
        return Some(format!(
            "{}",
            String::from_utf8(hash.stdout).ok()?.trim()
        ));
    }
    None
}

/// Parse `git status --porcelain=v1` output into a status map.
///
/// This is pure and testable — no subprocess calls.
pub fn parse_porcelain(output: &str) -> HashMap<PathBuf, FileStatus> {
    let mut map = HashMap::new();
    for line in output.lines() {
        if line.len() < 4 {
            continue;
        }
        let x = line.chars().next().unwrap_or(' ');
        let y = line.chars().nth(1).unwrap_or(' ');
        let path_part = &line[3..]; // skip "XY " prefix

        // Renames: "R  old -> new" — track the new name.
        let path = if (x == 'R' || x == 'C') && path_part.contains(" -> ") {
            path_part
                .split(" -> ")
                .nth(1)
                .map(str::trim)
                .unwrap_or(path_part)
        } else {
            path_part.trim()
        };

        let status = classify(x, y);
        map.insert(PathBuf::from(path), status);
    }
    map
}

/// Collapse an XY pair into a single `FileStatus`.
///
/// Priority: Conflicted > Added > Modified > Deleted > Renamed > Untracked.
fn classify(x: char, y: char) -> FileStatus {
    // Conflict.
    if x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D') {
        return FileStatus::Conflicted;
    }
    // Untracked / ignored.
    if x == '?' || y == '?' {
        return FileStatus::Untracked;
    }
    // Staged new file.
    if x == 'A' {
        return FileStatus::Added;
    }
    // Rename (index or working tree).
    if x == 'R' || y == 'R' || x == 'C' || y == 'C' {
        return FileStatus::Renamed;
    }
    // Deletion.
    if x == 'D' || y == 'D' {
        return FileStatus::Deleted;
    }
    // Any remaining modification.
    FileStatus::Modified
}
```

- [ ] **Step 1.5: Run tests**

```bash
cargo test parse_porcelain file_status_indicator
```

Expected: all pass.

- [ ] **Step 1.6: Commit**

```bash
git add src/git.rs src/lib.rs
git commit -m "feat(git): add git module — RepoStatus, FileStatus, parse_porcelain"
```

---

## Task 2: Add GitWorker to `src/jobs.rs`

**Files:**
- Modify: `src/jobs.rs`

- [ ] **Step 2.1: Write the failing test**

Add to `src/jobs.rs` test module:

```rust
#[test]
fn git_worker_responds_to_request() {
    let (workers, results) = spawn_workers();
    let tmp = std::env::temp_dir();
    workers
        .git_tx
        .send(GitStatusRequest { pane: PaneId::Left, path: tmp })
        .unwrap();
    // Either GitStatusLoaded (inside a repo) or GitStatusAbsent (not a repo) arrives.
    let result = results
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    assert!(
        matches!(
            result,
            JobResult::GitStatusLoaded { pane: PaneId::Left, .. }
                | JobResult::GitStatusAbsent { pane: PaneId::Left }
        ),
        "unexpected result: {result:?}"
    );
}
```

- [ ] **Step 2.2: Confirm it fails**

```bash
cargo test git_worker_responds_to_request 2>&1 | head -5
```

Expected: compile error — `git_tx`, `GitStatusRequest`, `GitStatusLoaded`, `GitStatusAbsent` don't exist.

- [ ] **Step 2.3: Update `src/jobs.rs`**

**Add request type** (after existing request types):

```rust
#[derive(Clone, Debug)]
pub struct GitStatusRequest {
    pub pane: PaneId,
    pub path: PathBuf,
}
```

**Add two result variants** to `JobResult`:

```rust
/// Git status successfully fetched for a pane's working directory.
GitStatusLoaded {
    pane: PaneId,
    status: crate::git::RepoStatus,
},
/// The path is not inside a git repository (or git is not on PATH).
GitStatusAbsent {
    pane: PaneId,
},
```

**Add `git_tx` to `WorkerChannels`**:

```rust
pub struct WorkerChannels {
    pub scan_tx:    Sender<ScanRequest>,
    pub file_op_tx: Sender<FileOpRequest>,
    pub preview_tx: Sender<PreviewRequest>,
    pub git_tx:     Sender<GitStatusRequest>,
}
```

**Spawn the git worker** inside `spawn_workers()`, before the `(WorkerChannels { … }, result_rx)` return:

```rust
// --- Git status worker ---
let (git_tx, git_rx) = bounded::<GitStatusRequest>(16);
{
    let result_tx = result_tx.clone(); // or move if this is the last
    thread::Builder::new()
        .name("zeta-git".into())
        .spawn(move || {
            for req in git_rx {
                let result = match crate::git::fetch_status(&req.path) {
                    Some(status) => JobResult::GitStatusLoaded {
                        pane: req.pane,
                        status,
                    },
                    None => JobResult::GitStatusAbsent { pane: req.pane },
                };
                if result_tx.send(result).is_err() {
                    break;
                }
            }
        })
        .expect("failed to spawn git worker");
}
```

Update the return to include `git_tx`:

```rust
(WorkerChannels { scan_tx, file_op_tx, preview_tx, git_tx }, result_rx)
```

- [ ] **Step 2.4: Run test**

```bash
cargo test git_worker_responds_to_request
```

Expected: passes (may take a moment while git runs).

- [ ] **Step 2.5: Commit**

```bash
git add src/jobs.rs
git commit -m "feat(jobs): add GitWorker — fourth dedicated worker for git status"
```

---

## Task 3: Update `AppState` to store and expose git status

**Files:**
- Modify: `src/state/mod.rs`

- [ ] **Step 3.1: Write the failing tests**

Add to the `AppState` test module in `src/state/mod.rs`:

```rust
#[test]
fn git_status_defaults_to_none_for_both_panes() {
    let state = test_state();
    assert!(state.git_status(PaneId::Left).is_none());
    assert!(state.git_status(PaneId::Right).is_none());
}

#[test]
fn git_status_loaded_result_stores_status_for_correct_pane() {
    use crate::git::{FileStatus, RepoStatus};
    use std::collections::HashMap;

    let mut state = test_state();
    let status = RepoStatus {
        root: PathBuf::from("/tmp/repo"),
        branch: String::from("main"),
        file_statuses: HashMap::new(),
    };
    state.apply_job_result(JobResult::GitStatusLoaded {
        pane: PaneId::Left,
        status,
    });
    assert!(state.git_status(PaneId::Left).is_some());
    assert_eq!(state.git_status(PaneId::Left).unwrap().branch, "main");
    assert!(state.git_status(PaneId::Right).is_none());
}

#[test]
fn git_status_absent_clears_status() {
    use crate::git::{FileStatus, RepoStatus};
    use std::collections::HashMap;

    let mut state = test_state();
    // Load then clear.
    state.apply_job_result(JobResult::GitStatusLoaded {
        pane: PaneId::Left,
        status: RepoStatus {
            root: PathBuf::from("/tmp/repo"),
            branch: String::from("main"),
            file_statuses: HashMap::new(),
        },
    });
    assert!(state.git_status(PaneId::Left).is_some());
    state.apply_job_result(JobResult::GitStatusAbsent { pane: PaneId::Left });
    assert!(state.git_status(PaneId::Left).is_none());
}

#[test]
fn status_line_includes_branch_name_when_git_loaded() {
    use crate::git::{FileStatus, RepoStatus};
    use std::collections::HashMap;

    let mut state = test_state();
    state.apply_job_result(JobResult::GitStatusLoaded {
        pane: PaneId::Left,
        status: RepoStatus {
            root: PathBuf::from("/tmp/repo"),
            branch: String::from("feature/cool"),
            file_statuses: HashMap::new(),
        },
    });
    assert!(
        state.status_line().contains("feature/cool"),
        "status line should contain branch name"
    );
}
```

- [ ] **Step 3.2: Implement changes in `src/state/mod.rs`**

**Add import** at the top of the file:

```rust
use crate::git::RepoStatus;
use crate::pane::PaneId;
```

**Add field to `AppState`**:

```rust
pub struct AppState {
    // ... existing fields ...
    /// Git status for the left [0] and right [1] pane working directories.
    git: [Option<RepoStatus>; 2],
}
```

**Initialise to `[None, None]`** in `bootstrap()`.

**Add accessor**:

```rust
pub fn git_status(&self, pane: PaneId) -> Option<&RepoStatus> {
    self.git[pane as usize].as_ref()
}
```

**Handle new `JobResult` variants** in `apply_job_result`:

```rust
JobResult::GitStatusLoaded { pane, status } => {
    self.git[pane as usize] = Some(status);
}
JobResult::GitStatusAbsent { pane } => {
    self.git[pane as usize] = None;
}
```

**Update `status_line()`** to include branch:

```rust
// After existing scan/marks/progress, append the active pane's branch.
let branch = self
    .git_status(self.panes.focus.into())  // PaneFocus → PaneId (add From impl if needed)
    .map(|g| format!(" | {}", g.branch))
    .unwrap_or_default();
// Append `branch` to the returned string.
```

> **Note:** `PaneFocus` and `PaneId` may need a `From<PaneFocus> for PaneId` conversion. Check `src/pane.rs` and add it there if not present.

- [ ] **Step 3.3: Run tests**

```bash
cargo test git_status
```

Expected: all four new tests pass.

- [ ] **Step 3.4: Commit**

```bash
git add src/state/mod.rs
git commit -m "feat(state): store RepoStatus per pane; expose git_status() accessor; branch in status line"
```

---

## Task 4: Trigger git scan from `app.rs`

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 4.1: Update `execute_command`**

After the `Command::ScanPane` arm dispatches a `ScanRequest`, also dispatch a `GitStatusRequest` for the same pane:

```rust
Command::ScanPane { pane, path } => {
    self.workers
        .scan_tx
        .send(ScanRequest { pane, path: path.clone() })
        .context("failed to queue background scan job")?;
    // Trigger git status update alongside every directory scan.
    self.workers
        .git_tx
        .send(GitStatusRequest { pane, path })
        .context("failed to queue git status job")?;
}
```

Add the import:

```rust
use crate::jobs::{self, FileOpRequest, GitStatusRequest, JobResult, PreviewRequest, ScanRequest, WorkerChannels};
```

- [ ] **Step 4.2: Run the full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 4.3: Commit**

```bash
git add src/app.rs
git commit -m "feat(app): dispatch GitStatusRequest alongside every ScanPane command"
```

---

## Task 5: Render git indicator in the pane

**Files:**
- Modify: `src/ui/pane.rs`

- [ ] **Step 5.1: Write the failing test**

Add to `src/ui/pane.rs` test module (or `src/ui/mod.rs`):

```rust
#[test]
fn git_indicator_symbol_matches_file_status() {
    use crate::git::FileStatus;
    assert_eq!(git_indicator_char(Some(FileStatus::Modified)), 'M');
    assert_eq!(git_indicator_char(Some(FileStatus::Untracked)), '?');
    assert_eq!(git_indicator_char(None), ' ');
}
```

- [ ] **Step 5.2: Add `git_indicator_char` helper**

In `src/ui/pane.rs`, add:

```rust
use crate::git::{FileStatus, RepoStatus};

pub fn git_indicator_char(status: Option<FileStatus>) -> char {
    status.map_or(' ', FileStatus::symbol)
}
```

- [ ] **Step 5.3: Update `render_pane` signature to accept git status**

```rust
pub fn render_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    pane: &PaneState,
    is_focused: bool,
    icon_mode: IconMode,
    palette: ThemePalette,
    git: Option<&RepoStatus>,   // ← new parameter
)
```

- [ ] **Step 5.4: Pass git indicator into the entry row**

In the entry-rendering loop inside `render_pane`, look up each entry's git status and pass it to the row renderer:

```rust
let git_status = git.and_then(|g| g.status_for(&entry.path));
render_entry_row(frame, row_area, entry, is_selected, is_marked, git_status, icon_mode, palette);
```

- [ ] **Step 5.5: Update `render_entry_row` to show the indicator**

Add a leading `git_status: Option<FileStatus>` parameter. Render a one-character coloured prefix before the existing icon:

```rust
// Git indicator — one char wide, space for clean files.
let (git_char, git_colour) = match git_status {
    Some(s) => (s.symbol(), s.colour()),
    None    => (' ', palette.text_muted),
};
let git_span = Span::styled(
    git_char.to_string(),
    Style::default().fg(git_colour),
);
// Prepend git_span to the row spans before rendering.
```

> The indicator slot is always 1 character wide so column alignment is preserved regardless of git availability. A clean file or non-repo pane shows a space.

- [ ] **Step 5.6: Update call sites in `src/ui/mod.rs`**

The two `render_pane` calls in `ui::render` need to pass the git status:

```rust
render_pane(
    frame, panes[0], left_pane_state, left_focused,
    icon_mode, palette,
    state.git_status(PaneId::Left),      // ← pass git status
);
render_pane(
    frame, panes[1], right_pane_state, right_focused,
    icon_mode, palette,
    state.git_status(PaneId::Right),     // ← pass git status
);
```

- [ ] **Step 5.7: Run all tests**

```bash
cargo test --workspace
```

Expected: all pass (165+ passing, same 2 pre-existing path-separator failures).

- [ ] **Step 5.8: Commit**

```bash
git add src/ui/pane.rs src/ui/mod.rs
git commit -m "feat(ui): git status indicator column in pane entry rows"
```

---

## Task 6: Final verification

- [ ] **Step 6.1: clippy**

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: zero warnings.

- [ ] **Step 6.2: Manual smoke test**

```
cargo run
# Navigate to a git repository directory
# Expected: M/A/?/D indicators appear next to modified/untracked/deleted files
# Expected: branch name (e.g. "main") appears in the status bar
# Navigate out of repo → indicators and branch name disappear
# Make a file change in the shell → F5 (refresh) → indicator updates
```

- [ ] **Step 6.3: Final commit**

```bash
git commit -m "chore: Wave 4A complete — git integration (status indicators + branch name)"
```

---

## Performance notes

- The git worker runs in its own thread and never blocks pane scans or file operations.
- `git status --porcelain=v1 -u` on a typical repo (thousands of files) completes in < 100 ms. On very large monorepos, consider adding `--no-optional-locks` to avoid write-lock contention.
- The result is a `HashMap<PathBuf, FileStatus>` so indicator lookup in the render loop is O(1) per entry.
- Git status is only refreshed when the directory scan is triggered — not on every frame. No polling.

## Jira

**ZTA-88** — Git integration: status indicators + branch name in status bar

Sub-tasks:
- ZTA-125 — `src/git.rs`: `RepoStatus`, `FileStatus`, `parse_porcelain`
- ZTA-126 — `GitWorker`: fourth dedicated background worker
- ZTA-127 — `AppState`: store and expose `RepoStatus` per pane
- ZTA-128 — Trigger `GitStatusRequest` on pane scan
- ZTA-129 — Pane renderer: git indicator column
- ZTA-130 — Status bar: branch name
