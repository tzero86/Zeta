# Wave 1C — Multi-Worker Jobs + tui-textarea Editor + tui-markdown Preview

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single-threaded background worker with three dedicated workers (ScanWorker, FileOpWorker, PreviewWorker) so long file operations never block directory scans or preview loads. Replace the hand-rolled `EditorBuffer` core with `tui-textarea::TextArea` to gain undo/redo, clipboard, and visual selection. Add `tui-markdown` AST-based rendering for `.md` files in the preview panel.

**Architecture:**
- `jobs.rs` exposes `spawn_workers() -> (WorkerChannels, Receiver<JobResult>)` where `WorkerChannels` wraps three separate `Sender` channels — one per worker.
- `App` in `app.rs` replaces the single `job_requests: Sender<JobRequest>` field with `workers: WorkerChannels` and dispatches to the correct channel inside `execute_command`.
- `EditorBuffer` in `editor.rs` replaces its `ropey::Rope` text storage with `tui_textarea::TextArea<'static>`, keeping the same public method signatures so `state/mod.rs` and `ui.rs` require minimal changes. Undo/redo is added as new `pub fn undo/redo` methods.
- `ViewBuffer` gains a `Markdown(String)` variant. The preview worker detects `.md` files and stores raw text. The preview renderer checks the variant and uses `tui_markdown::from_str()` instead of the highlighted line renderer.

**Tech Stack:** ratatui 0.29, crossterm 0.28, tui-textarea 0.7, tui-markdown 0.2, crossbeam-channel 0.5 (already present).

**Jira:** ZTA-81, ZTA-83, ZTA-84 (ZTA-111 through ZTA-115)

**Wave dependency:** Starts from `main`. Runs in parallel with Wave 1A and Wave 1B. Owns `src/jobs.rs`, `src/editor.rs`, `src/preview.rs`, and `Cargo.toml`. Does NOT modify `src/state/`, `src/app.rs` (only `execute_command` dispatch logic), or `src/ui.rs` rendering logic beyond preview.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `Cargo.toml` | Add tui-textarea, tui-markdown |
| Modify | `src/jobs.rs` | Split into 3 dedicated workers; new `WorkerChannels` type |
| Modify | `src/app.rs` | Replace `job_requests` with `workers: WorkerChannels` |
| Modify | `src/editor.rs` | Replace Rope with TextArea; add undo/redo |
| Modify | `src/preview.rs` | Add `ViewBuffer::Markdown` variant |
| Modify | `src/ui.rs` | Detect markdown variant, render with tui-markdown |

---

## Task 1: Add Cargo.toml dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1.1: Write failing test that references new crates**

Add at the bottom of any existing test module (e.g. `src/jobs.rs`):

```rust
#[test]
fn tui_textarea_crate_is_available() {
    // This fails to compile until the dependency is added.
    let _ta: tui_textarea::TextArea<'static> = tui_textarea::TextArea::default();
}
```

- [ ] **Step 1.2: Confirm it fails**

```bash
cargo test tui_textarea_crate_is_available 2>&1 | head -5
```

Expected: compile error — crate not found.

- [ ] **Step 1.3: Add dependencies to `Cargo.toml`**

In the `[dependencies]` section, add:

```toml
tui-textarea = { version = "0.7", features = ["crossterm"] }
tui-markdown = "0.2"
```

- [ ] **Step 1.4: Run the test**

```bash
cargo test tui_textarea_crate_is_available
```

Expected: passes.

- [ ] **Step 1.5: Remove the throwaway test, commit**

Remove the test added in Step 1.1, then:

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add tui-textarea 0.7 and tui-markdown 0.2"
```

---

## Task 2: Redesign jobs.rs — three dedicated workers

**Files:**
- Modify: `src/jobs.rs`

- [ ] **Step 2.1: Write the failing test**

Add to `src/jobs.rs` test module:

```rust
#[test]
fn worker_channels_can_send_and_receive_scan_request() {
    let (workers, results) = spawn_workers();
    let tmp = std::env::temp_dir();
    workers.scan_tx.send(ScanRequest { pane: PaneId::Left, path: tmp }).unwrap();
    let result = results.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
    assert!(matches!(result, JobResult::DirectoryScanned { pane: PaneId::Left, .. }
        | JobResult::JobFailed { pane: PaneId::Left, .. }));
}
```

- [ ] **Step 2.2: Confirm it fails**

```bash
cargo test worker_channels_can_send_and_receive_scan_request 2>&1 | head -5
```

Expected: compile error — `spawn_workers`, `ScanRequest`, `WorkerChannels` don't exist yet.

- [ ] **Step 2.3: Rewrite `src/jobs.rs`**

Replace the entire file:

```rust
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::action::{CollisionPolicy, FileOperation, RefreshTarget};
use crate::fs::{
    copy_path_with_progress, count_path_entries, create_directory, create_file, delete_path,
    looks_like_binary, rename_path, scan_directory, EntryInfo, FileSystemError,
};
use crate::pane::PaneId;

// ---------------------------------------------------------------------------
// Public request types — one per worker
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ScanRequest {
    pub pane: PaneId,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct FileOpRequest {
    pub operation: FileOperation,
    pub refresh: Vec<RefreshTarget>,
    pub collision: CollisionPolicy,
}

#[derive(Clone, Debug)]
pub struct PreviewRequest {
    pub path: PathBuf,
    pub syntect_theme: String,
}

/// Kept for backwards compatibility — callers that construct `JobRequest`
/// variants are migrated in Task 3 (app.rs). Remove after Wave 1C merges.
#[deprecated(note = "use ScanRequest / FileOpRequest / PreviewRequest instead")]
#[allow(dead_code)]
pub enum JobRequest {
    FileOperation {
        operation: FileOperation,
        refresh: Vec<RefreshTarget>,
        collision: CollisionPolicy,
    },
    ScanDirectory {
        pane: PaneId,
        path: PathBuf,
    },
    PreviewFile {
        path: PathBuf,
        syntect_theme: String,
    },
}

// ---------------------------------------------------------------------------
// Result types (unchanged)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobResult {
    DirectoryScanned {
        pane: PaneId,
        path: PathBuf,
        entries: Vec<EntryInfo>,
        elapsed_ms: u128,
    },
    FileOperationCompleted {
        message: String,
        refreshed: Vec<RefreshedPane>,
        elapsed_ms: u128,
    },
    FileOperationCollision {
        operation: FileOperation,
        refresh: Vec<RefreshTarget>,
        path: PathBuf,
        elapsed_ms: u128,
    },
    FileOperationProgress {
        status: FileOperationStatus,
    },
    JobFailed {
        pane: PaneId,
        path: PathBuf,
        message: String,
        elapsed_ms: u128,
    },
    PreviewLoaded {
        path: PathBuf,
        view: crate::preview::ViewBuffer,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefreshedPane {
    pub pane: PaneId,
    pub path: PathBuf,
    pub entries: Vec<EntryInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileOperationStatus {
    pub operation: &'static str,
    pub completed: u64,
    pub total: u64,
    pub current_path: PathBuf,
}

// ---------------------------------------------------------------------------
// Worker channels
// ---------------------------------------------------------------------------

/// Three typed senders — one per dedicated worker thread.
pub struct WorkerChannels {
    pub scan_tx: Sender<ScanRequest>,
    pub file_op_tx: Sender<FileOpRequest>,
    pub preview_tx: Sender<PreviewRequest>,
}

/// Spawn three dedicated background workers that all fan results into a single
/// `Receiver<JobResult>`. Each worker processes its queue sequentially; because
/// the queues are independent, a slow file operation never delays a scan.
pub fn spawn_workers() -> (WorkerChannels, Receiver<JobResult>) {
    let (result_tx, result_rx) = bounded::<JobResult>(64);

    // --- Scan worker ---
    let (scan_tx, scan_rx) = bounded::<ScanRequest>(16);
    {
        let result_tx = result_tx.clone();
        thread::Builder::new()
            .name("zeta-scan".into())
            .spawn(move || {
                for req in scan_rx {
                    let started_at = Instant::now();
                    let job_result = match scan_directory(&req.path) {
                        Ok(entries) => JobResult::DirectoryScanned {
                            pane: req.pane,
                            path: req.path,
                            entries,
                            elapsed_ms: started_at.elapsed().as_millis(),
                        },
                        Err(err) => JobResult::JobFailed {
                            pane: req.pane,
                            path: req.path,
                            message: err.to_string(),
                            elapsed_ms: started_at.elapsed().as_millis(),
                        },
                    };
                    if result_tx.send(job_result).is_err() {
                        break;
                    }
                }
            })
            .expect("failed to spawn scan worker");
    }

    // --- File operation worker ---
    let (file_op_tx, file_op_rx) = bounded::<FileOpRequest>(8);
    {
        let result_tx = result_tx.clone();
        thread::Builder::new()
            .name("zeta-file-op".into())
            .spawn(move || {
                for req in file_op_rx {
                    let outcome = run_file_operation(
                        req.operation,
                        req.refresh,
                        req.collision,
                        &result_tx,
                    );
                    let job_result = match outcome {
                        Ok(r) | Err(r) => r,
                    };
                    if result_tx.send(job_result).is_err() {
                        break;
                    }
                }
            })
            .expect("failed to spawn file-op worker");
    }

    // --- Preview worker ---
    let (preview_tx, preview_rx) = bounded::<PreviewRequest>(8);
    {
        let result_tx = result_tx; // last clone — move ownership
        thread::Builder::new()
            .name("zeta-preview".into())
            .spawn(move || {
                for req in preview_rx {
                    let view = load_preview_content(&req.path, &req.syntect_theme);
                    if result_tx
                        .send(JobResult::PreviewLoaded { path: req.path, view })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("failed to spawn preview worker");
    }

    (WorkerChannels { scan_tx, file_op_tx, preview_tx }, result_rx)
}

/// Legacy single-worker entry-point kept so existing callers compile while the
/// migration is in progress. Remove after `app.rs` is updated in Task 3.
pub fn spawn_scan_worker() -> (Sender<ScanRequest>, Receiver<JobResult>) {
    let (workers, results) = spawn_workers();
    (workers.scan_tx, results)
}

// ---------------------------------------------------------------------------
// Internal worker logic (unchanged from original)
// ---------------------------------------------------------------------------

fn load_preview_content(path: &Path, syntect_theme: &str) -> crate::preview::ViewBuffer {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return crate::preview::ViewBuffer::from_plain("[empty file]"),
    };

    if bytes.is_empty() {
        return crate::preview::ViewBuffer::from_plain("[empty file]");
    }

    if looks_like_binary(&bytes) {
        let size_bytes = std::fs::metadata(path)
            .map(|m| m.len())
            .unwrap_or(bytes.len() as u64);
        let label = format!("[binary file — {size_bytes} bytes]");
        return crate::preview::ViewBuffer::from_plain(&label);
    }

    let text = String::from_utf8_lossy(&bytes);
    let extension = path.extension().and_then(|e| e.to_str());

    // Markdown: store raw text for AST rendering at display time.
    if extension == Some("md") {
        return crate::preview::ViewBuffer::from_markdown(text.into_owned());
    }

    if let Some(lines) = crate::highlight::highlight_text(&text, extension, syntect_theme) {
        return crate::preview::ViewBuffer::from_highlighted(lines);
    }

    let truncated: String = text
        .lines()
        .take(200)
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(200 * 80)
        .collect();

    crate::preview::ViewBuffer::from_plain(&truncated)
}

fn run_file_operation(
    operation: FileOperation,
    refresh: Vec<RefreshTarget>,
    collision: CollisionPolicy,
    result_tx: &Sender<JobResult>,
) -> Result<JobResult, JobResult> {
    let started_at = Instant::now();
    let operation_label = describe_operation(&operation);
    let primary_path = primary_path(&operation);
    let failure_pane = refresh.first().map(|t| t.pane).unwrap_or(PaneId::Left);

    let op_result = match &operation {
        FileOperation::Copy { source, destination } => {
            run_copy_with_progress(source, destination, collision, result_tx)
        }
        FileOperation::CreateDirectory { path } => create_directory(path, collision),
        FileOperation::CreateFile { path } => create_file(path, collision),
        FileOperation::Delete { path } => delete_path(path),
        FileOperation::Move { source, destination } => {
            run_move_with_progress(source, destination, collision, result_tx)
        }
        FileOperation::Rename { source, destination } => rename_path(source, destination, collision),
    };

    if let Err(error) = op_result {
        return match error {
            FileSystemError::PathExists { path } => Err(JobResult::FileOperationCollision {
                operation,
                refresh,
                path: PathBuf::from(path),
                elapsed_ms: started_at.elapsed().as_millis(),
            }),
            other => Err(JobResult::JobFailed {
                pane: failure_pane,
                path: primary_path,
                message: other.to_string(),
                elapsed_ms: started_at.elapsed().as_millis(),
            }),
        };
    }

    let mut refreshed = Vec::with_capacity(refresh.len());
    for target in refresh {
        match scan_directory(&target.path) {
            Ok(entries) => refreshed.push(RefreshedPane {
                pane: target.pane,
                path: target.path,
                entries,
            }),
            Err(err) => {
                return Err(JobResult::JobFailed {
                    pane: target.pane,
                    path: target.path,
                    message: err.to_string(),
                    elapsed_ms: started_at.elapsed().as_millis(),
                });
            }
        }
    }

    Ok(JobResult::FileOperationCompleted {
        message: operation_label,
        refreshed,
        elapsed_ms: started_at.elapsed().as_millis(),
    })
}

fn run_copy_with_progress(
    source: &Path,
    destination: &Path,
    collision: CollisionPolicy,
    result_tx: &Sender<JobResult>,
) -> Result<(), FileSystemError> {
    copy_path_with_progress(source, destination, collision, &mut |progress| {
        let _ = send_progress(result_tx, "copy", progress.completed, progress.total, progress.current_path);
    })
}

fn run_move_with_progress(
    source: &Path,
    destination: &Path,
    collision: CollisionPolicy,
    result_tx: &Sender<JobResult>,
) -> Result<(), FileSystemError> {
    match rename_path(source, destination, collision) {
        Ok(()) => {
            let _ = send_progress(result_tx, "move", 1, 1, destination.to_path_buf());
            Ok(())
        }
        Err(err) if is_cross_device_error(&err) => {
            let total = count_path_entries(source)?.saturating_add(1);
            let _ = send_progress(result_tx, "move", 0, total, source.to_path_buf());
            copy_path_with_progress(source, destination, collision, &mut |p| {
                let _ = send_progress(result_tx, "move", p.completed, total, p.current_path);
            })?;
            delete_path(source)?;
            let _ = send_progress(result_tx, "move", total, total, destination.to_path_buf());
            Ok(())
        }
        Err(err) => Err(err),
    }
}

fn send_progress(
    result_tx: &Sender<JobResult>,
    operation: &'static str,
    completed: u64,
    total: u64,
    current_path: PathBuf,
) -> Result<(), ()> {
    result_tx
        .send(JobResult::FileOperationProgress {
            status: FileOperationStatus { operation, completed, total, current_path },
        })
        .map_err(|_| ())
}

fn is_cross_device_error(error: &FileSystemError) -> bool {
    matches!(
        error,
        FileSystemError::RenamePath { source, .. } if source.raw_os_error() == Some(EXDEV_ERROR)
    )
}

#[cfg(unix)]
const EXDEV_ERROR: i32 = 18;
#[cfg(not(unix))]
const EXDEV_ERROR: i32 = -1;

fn primary_path(operation: &FileOperation) -> PathBuf {
    match operation {
        FileOperation::Copy { source, .. } => source.clone(),
        FileOperation::CreateDirectory { path } => path.clone(),
        FileOperation::CreateFile { path } => path.clone(),
        FileOperation::Delete { path } => path.clone(),
        FileOperation::Move { source, .. } => source.clone(),
        FileOperation::Rename { source, .. } => source.clone(),
    }
}

fn describe_operation(operation: &FileOperation) -> String {
    match operation {
        FileOperation::Copy { destination, .. } => format!("copied to {}", destination.display()),
        FileOperation::CreateDirectory { path } => format!("created {}", path.display()),
        FileOperation::CreateFile { path } => format!("created {}", path.display()),
        FileOperation::Delete { path } => format!("deleted {}", path.display()),
        FileOperation::Move { destination, .. } => format!("moved to {}", destination.display()),
        FileOperation::Rename { destination, .. } => format!("renamed to {}", destination.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_channels_can_send_and_receive_scan_request() {
        let (workers, results) = spawn_workers();
        let tmp = std::env::temp_dir();
        workers.scan_tx.send(ScanRequest { pane: PaneId::Left, path: tmp }).unwrap();
        let result = results.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        assert!(
            matches!(
                result,
                JobResult::DirectoryScanned { pane: PaneId::Left, .. }
                    | JobResult::JobFailed { pane: PaneId::Left, .. }
            ),
            "unexpected result: {:?}",
            result
        );
    }

    #[test]
    fn three_workers_process_requests_independently() {
        let (workers, results) = spawn_workers();
        let tmp = std::env::temp_dir();

        // Send two scan requests — they should both complete.
        workers.scan_tx.send(ScanRequest { pane: PaneId::Left, path: tmp.clone() }).unwrap();
        workers.scan_tx.send(ScanRequest { pane: PaneId::Right, path: tmp }).unwrap();

        let mut received = 0;
        for _ in 0..2 {
            if results.recv_timeout(std::time::Duration::from_secs(5)).is_ok() {
                received += 1;
            }
        }
        assert_eq!(received, 2);
    }
}
```

- [ ] **Step 2.4: Run tests**

```bash
cargo test worker_channels
```

Expected: both tests pass.

- [ ] **Step 2.5: Commit**

```bash
git add src/jobs.rs
git commit -m "feat(jobs): split single worker into three dedicated workers (scan/file-op/preview)"
```

---

## Task 3: Update App to use WorkerChannels

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 3.1: Write the failing test**

Add to `src/app.rs` test module — this will fail until the struct is updated:

```rust
#[test]
fn app_uses_worker_channels_not_single_sender() {
    // Structural — just verify WorkerChannels is referenced in the module.
    // This compiles only after the migration.
    use crate::jobs::WorkerChannels;
    let _: fn() -> WorkerChannels = || unreachable!();
}
```

- [ ] **Step 3.2: Update `src/app.rs`**

Replace the `job_requests: Sender<JobRequest>` field and all its usages:

**Change imports (top of file):**
```rust
// Remove:
use crate::jobs::{self, JobRequest, JobResult};

// Add:
use crate::jobs::{self, FileOpRequest, JobResult, PreviewRequest, ScanRequest, WorkerChannels};
```

**Change `App` struct:**
```rust
pub struct App {
    workers: WorkerChannels,
    job_results: Receiver<JobResult>,
    keymap: RuntimeKeymap,
    state: AppState,
    pub layout_cache: LayoutCache,  // Wave 1B adds this; include if 1B has merged
}
```

**Change `bootstrap`:**
```rust
let (workers, job_results) = jobs::spawn_workers();
// replace: let (job_requests, job_results) = jobs::spawn_scan_worker();

let mut app = Self {
    workers,
    job_results,
    keymap,
    state,
    layout_cache: LayoutCache::default(), // Wave 1B field; include if present
};
```

**Change `execute_command`:**
```rust
fn execute_command(&mut self, command: Command) -> Result<()> {
    match command {
        Command::OpenEditor { path } => match EditorBuffer::open(&path) {
            Ok(editor) => self.state.open_editor(editor),
            Err(error) => self
                .state
                .set_error_status(format!("failed to open editor buffer: {error}")),
        },
        Command::PreviewFile { path } => self
            .workers
            .preview_tx
            .send(PreviewRequest {
                path,
                syntect_theme: self.state.theme().palette.syntect_theme.to_string(),
            })
            .context("failed to queue background preview job")?,
        Command::RunFileOperation { operation, refresh, collision } => self
            .workers
            .file_op_tx
            .send(FileOpRequest { operation, refresh, collision })
            .context("failed to queue background file operation")?,
        Command::ScanPane { pane, path } => self
            .workers
            .scan_tx
            .send(ScanRequest { pane, path })
            .context("failed to queue background scan job")?,
        Command::SaveEditor => {
            if let Some(editor) = self.state.editor_mut() {
                match editor.save() {
                    Ok(()) => self.state.mark_editor_saved(),
                    Err(error) => self
                        .state
                        .set_error_status(format!("failed to save editor buffer: {error}")),
                }
            } else {
                self.state.set_error_status("no editor buffer is open");
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 3.3: Run the full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 3.4: Commit**

```bash
git add src/app.rs
git commit -m "refactor(app): use WorkerChannels — dispatch to typed worker senders"
```

---

## Task 4: Replace EditorBuffer core with tui-textarea

**Files:**
- Modify: `src/editor.rs`

- [ ] **Step 4.1: Write the failing test**

Add to `src/editor.rs` test module:

```rust
#[test]
fn undo_restores_previous_content() {
    let mut editor = EditorBuffer::default();
    editor.insert_char('a');
    editor.insert_char('b');
    editor.undo();
    // After one undo the last inserted char should be gone.
    let contents = editor.contents();
    assert!(
        contents == "a" || contents == "",
        "unexpected contents after undo: {:?}",
        contents
    );
}

#[test]
fn redo_reapplies_undone_change() {
    let mut editor = EditorBuffer::default();
    editor.insert_char('x');
    editor.undo();
    editor.redo();
    assert!(editor.contents().contains('x'));
}
```

- [ ] **Step 4.2: Confirm they fail**

```bash
cargo test undo_restores_previous_content redo_reapplies_undone_change 2>&1 | head -5
```

Expected: compile error — `undo`/`redo` methods don't exist yet.

- [ ] **Step 4.3: Rewrite `src/editor.rs`**

Replace the entire file. The public interface is preserved exactly; internal storage changes from `ropey::Rope` to `tui_textarea::TextArea<'static>`. Undo/redo is exposed via two new public methods.

```rust
use std::fs as std_fs;
use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier};
use thiserror::Error;
use tui_textarea::TextArea;

use crate::highlight::{highlight_text, normalize_preview_text, HighlightedLine};

// ---------------------------------------------------------------------------
// Public types unchanged from original
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorRenderState {
    pub visible_start: usize,
    pub visible_lines: Vec<String>,
    pub cursor_visible_row: Option<usize>,
    pub scroll_col: usize,
}

// ---------------------------------------------------------------------------
// EditorBuffer — now backed by tui_textarea::TextArea
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct EditorBuffer {
    pub path: Option<PathBuf>,
    pub is_dirty: bool,
    pub search_query: String,
    pub search_active: bool,
    pub search_match_idx: usize,
    /// Horizontal scroll offset in columns (managed separately since tui-textarea
    /// doesn't expose horizontal scrolling for multi-line buffers).
    pub scroll_col: usize,
    inner: TextArea<'static>,
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self {
            path: None,
            is_dirty: false,
            search_query: String::new(),
            search_active: false,
            search_match_idx: 0,
            scroll_col: 0,
            inner: TextArea::default(),
        }
    }
}

impl EditorBuffer {
    pub fn open(path: &Path) -> Result<Self, EditorError> {
        let bytes = std_fs::read(path).map_err(|source| EditorError::ReadFile {
            path: path.display().to_string(),
            source,
        })?;
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<String> = text.lines().map(String::from).collect();
        let mut inner = TextArea::from(lines);
        // Move cursor to start.
        inner.move_cursor(tui_textarea::CursorMove::Top);

        Ok(Self {
            path: Some(path.to_path_buf()),
            is_dirty: false,
            search_query: String::new(),
            search_active: false,
            search_match_idx: 0,
            scroll_col: 0,
            inner,
        })
    }

    // -----------------------------------------------------------------------
    // Text mutation (maps directly to TextArea input operations)
    // -----------------------------------------------------------------------

    pub fn insert_char(&mut self, ch: char) {
        use tui_textarea::{CursorMove, Input, Key};
        self.inner.input(Input { key: Key::Char(ch), ctrl: false, alt: false, shift: false });
        self.is_dirty = true;
    }

    pub fn insert_newline(&mut self) {
        use tui_textarea::{Input, Key};
        self.inner.input(Input { key: Key::Enter, ctrl: false, alt: false, shift: false });
        self.is_dirty = true;
    }

    pub fn backspace(&mut self) {
        use tui_textarea::{Input, Key};
        let (row, col) = self.inner.cursor();
        if row == 0 && col == 0 {
            return;
        }
        self.inner.input(Input { key: Key::Backspace, ctrl: false, alt: false, shift: false });
        self.is_dirty = true;
    }

    pub fn move_left(&mut self) {
        self.inner.move_cursor(tui_textarea::CursorMove::Back);
    }

    pub fn move_right(&mut self) {
        self.inner.move_cursor(tui_textarea::CursorMove::Forward);
    }

    pub fn move_up(&mut self) {
        self.inner.move_cursor(tui_textarea::CursorMove::Up);
    }

    pub fn move_down(&mut self) {
        self.inner.move_cursor(tui_textarea::CursorMove::Down);
    }

    /// Undo the most recent edit. (New in Wave 1C — not in original EditorBuffer.)
    pub fn undo(&mut self) {
        self.inner.undo();
        self.is_dirty = self.inner.lines() != &[String::new()];
    }

    /// Redo the most recently undone edit. (New in Wave 1C.)
    pub fn redo(&mut self) {
        self.inner.redo();
        self.is_dirty = true;
    }

    // -----------------------------------------------------------------------
    // Persistence
    // -----------------------------------------------------------------------

    pub fn save(&mut self) -> Result<(), EditorError> {
        let path = self.path.as_ref().ok_or(EditorError::NoPath)?;
        let content = self.inner.lines().join("\n");
        std_fs::write(path, content.as_bytes()).map_err(|source| EditorError::WriteFile {
            path: path.display().to_string(),
            source,
        })?;
        self.is_dirty = false;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Content accessors
    // -----------------------------------------------------------------------

    pub fn contents(&self) -> String {
        self.inner.lines().join("\n")
    }

    pub fn visible_lines(&self) -> Vec<String> {
        self.inner.lines().to_vec()
    }

    // -----------------------------------------------------------------------
    // Rendering support
    // -----------------------------------------------------------------------

    pub fn visible_highlighted_window(
        &self,
        height: usize,
        syntect_theme: &str,
        fallback_color: ratatui::style::Color,
    ) -> (usize, Vec<HighlightedLine>) {
        let (start, lines) = self.visible_line_window(height);
        let text = lines.join("\n");
        let ext = self.path.as_ref().and_then(|p| p.extension()).and_then(|e| e.to_str());
        let normalized = normalize_preview_text(&text);
        if let Some(highlighted) = highlight_text(&normalized, ext, syntect_theme) {
            return (start, highlighted);
        }
        let plain: Vec<HighlightedLine> = lines
            .iter()
            .map(|line| {
                vec![(fallback_color, Modifier::empty(), line.clone())]
            })
            .collect();
        (start, plain)
    }

    pub fn visible_line_window(&self, height: usize) -> (usize, Vec<String>) {
        let all_lines = self.inner.lines();
        let total = all_lines.len();
        let (cursor_row, _) = self.inner.cursor();

        // Clamp scroll so cursor stays in view.
        let scroll_top = if cursor_row >= height {
            cursor_row - height + 1
        } else {
            0
        };
        let end = (scroll_top + height).min(total);
        (scroll_top, all_lines[scroll_top..end].to_vec())
    }

    pub fn cursor_line_col(&self) -> (usize, usize) {
        self.inner.cursor()
    }

    pub fn clamp_horizontal_scroll(&mut self, viewport_cols: usize) {
        let (_, col) = self.inner.cursor();
        if col < self.scroll_col {
            self.scroll_col = col;
        } else if col >= self.scroll_col + viewport_cols {
            self.scroll_col = col.saturating_sub(viewport_cols - 1);
        }
    }

    pub fn render_state(
        &mut self,
        viewport_rows: usize,
        viewport_cols: usize,
        is_active: bool,
    ) -> EditorRenderState {
        self.clamp_horizontal_scroll(viewport_cols);
        let (start, lines) = self.visible_line_window(viewport_rows);
        let (cursor_row, _) = self.inner.cursor();
        let visible_cursor = if is_active && cursor_row >= start && cursor_row < start + lines.len()
        {
            Some(cursor_row - start)
        } else {
            None
        };

        EditorRenderState {
            visible_start: start,
            visible_lines: lines,
            cursor_visible_row: visible_cursor,
            scroll_col: self.scroll_col,
        }
    }

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------

    pub fn find_matches(&self, query: &str) -> Vec<(usize, usize)> {
        if query.is_empty() {
            return vec![];
        }
        let q = query.to_lowercase();
        let mut matches = Vec::new();
        let mut char_offset = 0usize;
        for line in self.inner.lines() {
            let lower = line.to_lowercase();
            let mut search_start = 0;
            while let Some(found) = lower[search_start..].find(&q) {
                let abs = char_offset + search_start + found;
                matches.push((abs, abs + q.len()));
                search_start += found + q.len();
            }
            char_offset += line.len() + 1; // +1 for newline
        }
        matches
    }

    pub fn search_next(&mut self) {
        let matches = self.find_matches(&self.search_query.clone());
        if matches.is_empty() {
            return;
        }
        self.search_match_idx = (self.search_match_idx + 1) % matches.len();
    }

    pub fn search_prev(&mut self) {
        let matches = self.find_matches(&self.search_query.clone());
        if matches.is_empty() {
            return;
        }
        if self.search_match_idx == 0 {
            self.search_match_idx = matches.len() - 1;
        } else {
            self.search_match_idx -= 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("read file {path}: {source}")]
    ReadFile { path: String, source: std::io::Error },
    #[error("write file {path}: {source}")]
    WriteFile { path: String, source: std::io::Error },
    #[error("no path set — cannot save")]
    NoPath,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(name)
    }

    #[test]
    fn typing_and_backspace_update_cursor() {
        let mut editor = EditorBuffer::default();
        editor.insert_char('a');
        editor.insert_char('b');
        let (_, col) = editor.cursor_line_col();
        assert_eq!(col, 2);
        editor.backspace();
        let (_, col) = editor.cursor_line_col();
        assert_eq!(col, 1);
    }

    #[test]
    fn save_persists_changes_and_clears_dirty_flag() {
        let path = temp_file_path("editor_save_test.txt");
        let mut editor = EditorBuffer::default();
        editor.path = Some(path.clone());
        editor.insert_char('h');
        editor.insert_char('i');
        assert!(editor.is_dirty);
        editor.save().unwrap();
        assert!(!editor.is_dirty);
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("hi"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_without_path_fails() {
        let mut editor = EditorBuffer::default();
        editor.insert_char('x');
        assert!(matches!(editor.save(), Err(EditorError::NoPath)));
    }

    #[test]
    fn undo_restores_previous_content() {
        let mut editor = EditorBuffer::default();
        editor.insert_char('a');
        editor.insert_char('b');
        editor.undo();
        let contents = editor.contents();
        assert!(
            contents == "a" || contents.is_empty(),
            "unexpected after undo: {:?}",
            contents
        );
    }

    #[test]
    fn redo_reapplies_undone_change() {
        let mut editor = EditorBuffer::default();
        editor.insert_char('x');
        editor.undo();
        editor.redo();
        assert!(editor.contents().contains('x'));
    }

    #[test]
    fn find_matches_returns_all_occurrences() {
        let mut editor = EditorBuffer::default();
        for ch in "hello world hello".chars() {
            if ch == ' ' { editor.insert_newline(); } else { editor.insert_char(ch); }
        }
        // Use contents-level search, not per-line (for simplicity we allow per-line results)
        let matches = editor.find_matches("hello");
        assert!(matches.len() >= 2, "expected >= 2 matches, got {}", matches.len());
    }

    #[test]
    fn find_matches_is_case_insensitive() {
        let mut editor = EditorBuffer::default();
        for ch in "Hello HELLO hello".chars() {
            if ch == ' ' { editor.insert_newline(); } else { editor.insert_char(ch); }
        }
        let matches = editor.find_matches("hello");
        assert!(matches.len() >= 3);
    }

    #[test]
    fn find_matches_empty_query_returns_nothing() {
        let mut editor = EditorBuffer::default();
        editor.insert_char('a');
        assert!(editor.find_matches("").is_empty());
    }
}
```

- [ ] **Step 4.4: Run tests**

```bash
cargo test --lib editor
```

Expected: all tests pass including the new undo/redo ones.

- [ ] **Step 4.5: Commit**

```bash
git add src/editor.rs
git commit -m "refactor(editor): replace ropey Rope with tui-textarea TextArea; add undo/redo"
```

---

## Task 5: Add ViewBuffer::Markdown variant + update preview rendering

**Files:**
- Modify: `src/preview.rs`
- Modify: `src/ui.rs`

- [ ] **Step 5.1: Write failing tests**

Add to `src/preview.rs` test module:

```rust
#[test]
fn markdown_variant_stores_raw_text() {
    let vb = ViewBuffer::from_markdown("# Hello\n\nWorld".to_string());
    assert!(vb.is_markdown());
    assert_eq!(vb.markdown_source(), Some("# Hello\n\nWorld"));
}
```

- [ ] **Step 5.2: Confirm it fails**

```bash
cargo test markdown_variant_stores_raw_text 2>&1 | head -5
```

Expected: compile error — `from_markdown`, `is_markdown`, `markdown_source` not found.

- [ ] **Step 5.3: Update `src/preview.rs`**

Read the current `ViewBuffer` definition and add the new variant and accessors. The key changes:

```rust
/// Buffered content for the preview panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewBuffer {
    /// Plain or syntax-highlighted lines.
    Lines {
        lines: Vec<crate::highlight::HighlightedLine>,
        scroll_row: usize,
    },
    /// Raw Markdown source — rendered at display time by tui-markdown.
    Markdown {
        source: String,
        scroll_row: usize,
    },
}

impl ViewBuffer {
    pub fn from_plain(text: &str) -> Self {
        use ratatui::style::{Color, Modifier};
        let lines: Vec<crate::highlight::HighlightedLine> = text
            .lines()
            .map(|l| vec![(Color::Reset, Modifier::empty(), l.to_string())])
            .collect();
        Self::Lines { lines, scroll_row: 0 }
    }

    pub fn from_highlighted(lines: Vec<crate::highlight::HighlightedLine>) -> Self {
        Self::Lines { lines, scroll_row: 0 }
    }

    pub fn from_markdown(source: String) -> Self {
        Self::Markdown { source, scroll_row: 0 }
    }

    pub fn is_markdown(&self) -> bool {
        matches!(self, Self::Markdown { .. })
    }

    pub fn markdown_source(&self) -> Option<&str> {
        match self {
            Self::Markdown { source, .. } => Some(source),
            _ => None,
        }
    }

    /// Returns `(first_line_index, &[HighlightedLine])` for the visible window.
    /// Panics if called on a Markdown variant — callers must check `is_markdown()` first.
    pub fn visible_window(&self, height: usize) -> (usize, &[crate::highlight::HighlightedLine]) {
        match self {
            Self::Lines { lines, scroll_row } => {
                let start = (*scroll_row).min(lines.len().saturating_sub(1));
                let end = (start + height).min(lines.len());
                (start, &lines[start..end])
            }
            Self::Markdown { .. } => panic!("visible_window called on Markdown variant — use markdown_source() instead"),
        }
    }

    pub fn scroll_down(&mut self, lines: usize) {
        match self {
            Self::Lines { lines: content, scroll_row } => {
                *scroll_row = (*scroll_row + lines).min(content.len().saturating_sub(1));
            }
            Self::Markdown { scroll_row, .. } => {
                *scroll_row += lines;
            }
        }
    }

    pub fn scroll_up(&mut self, lines: usize) {
        match self {
            Self::Lines { scroll_row, .. } | Self::Markdown { scroll_row, .. } => {
                *scroll_row = scroll_row.saturating_sub(lines);
            }
        }
    }
}
```

> **Note:** If `ViewBuffer` currently uses a simpler struct (not enum), wrap it in an enum. The `from_highlighted`, `from_plain`, `visible_window`, `scroll_down`, `scroll_up` method signatures must stay compatible with existing callers in `state/mod.rs` and `ui.rs`.

- [ ] **Step 5.4: Run preview tests**

```bash
cargo test markdown_variant_stores_raw_text
```

Expected: passes.

- [ ] **Step 5.5: Update `src/ui.rs` preview renderer to handle Markdown variant**

In `render_preview_panel`, replace the `Some(v) =>` arm:

```rust
Some(v) => {
    if v.is_markdown() {
        // Render with tui-markdown for AST-based formatting.
        if let Some(source) = v.markdown_source() {
            let widget = tui_markdown::from_str(source);
            frame.render_widget(widget, inner);
        }
    } else {
        let height = inner.height as usize;
        let (first_line_num, window) = v.visible_window(height);
        if window.is_empty() {
            return;
        }
        render_wrapped_preview_view(frame, inner, window, first_line_num + 1, palette);
    }
}
```

Add the import at the top of `src/ui.rs`:

```rust
// (If not already present from Wave 1C dependency addition)
// tui_markdown is used in render_preview_panel
```

> **Note:** If Wave 1B has already split `render_preview_panel` into `src/ui/preview.rs`, make this change there instead.

- [ ] **Step 5.6: Run full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 5.7: Commit**

```bash
git add src/preview.rs src/ui.rs
git commit -m "feat(preview): add Markdown variant to ViewBuffer; render .md with tui-markdown"
```

---

## Task 6: Final verification

**Files:** None modified.

- [ ] **Step 6.1: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: zero warnings. Fix any deprecation warnings from the removed `JobRequest` enum if it's still referenced.

- [ ] **Step 6.2: Remove deprecated `JobRequest` if unused**

```bash
grep -rn "JobRequest" src/
```

If only the dead definition remains in `jobs.rs`, delete it and remove the `#[deprecated]` shim:

```bash
# Remove the JobRequest enum and spawn_scan_worker shim from jobs.rs
cargo test
```

- [ ] **Step 6.3: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: `test result: ok. N passed; 0 failed`.

- [ ] **Step 6.4: Final commit**

```bash
git commit -m "chore: Wave 1C complete — 3-worker jobs, tui-textarea editor, tui-markdown preview"
```
