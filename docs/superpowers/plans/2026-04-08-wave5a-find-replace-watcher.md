# Wave 5A — Find & Replace in Editor + Directory Watcher

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two quality-of-life improvements. Find & Replace adds a replace bar below the existing search bar in the editor (`Ctrl+H`). Directory watcher auto-refreshes panes when the filesystem changes externally, using the `notify` crate.

**Jira:** ZTA-92 (ZTA-151 through ZTA-157)

**Wave dependency:** Starts AFTER Wave 4D.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `Cargo.toml` | Add `notify = "6"` |
| Modify | `src/action.rs` | `OpenEditorReplace`, `EditorReplaceInput`, `EditorReplaceNext`, `EditorReplaceAll` |
| Modify | `src/state/editor_state.rs` | `replace_query`, `replace_active` fields; replace logic |
| Modify | `src/editor.rs` | `replace_next()`, `replace_all()` methods on `EditorBuffer` |
| Modify | `src/ui/editor.rs` | Render replace bar below search bar |
| Modify | `src/jobs.rs` | `WatcherWorker` — spawns notify watcher, sends `JobResult::DirectoryChanged` |
| Modify | `src/state/mod.rs` | Handle `DirectoryChanged` → re-queue scan |
| Modify | `src/app.rs` | Pass watcher channel; start watching on scan |

---

## Part A: Find & Replace

### New actions

```rust
OpenEditorReplace,          // Ctrl+H — open replace bar
EditorReplaceInput(char),   // type into replace field
EditorReplaceBackspace,     // backspace in replace field
EditorReplaceNext,          // replace current match, advance
EditorReplaceAll,           // replace all matches
```

### EditorBuffer methods

```rust
/// Replace the match at `cursor_char_idx` (current search match) with `replacement`.
/// Returns true if a replacement was made.
pub fn replace_next(&mut self, replacement: &str) -> bool {
    let matches = self.find_matches(&self.search_query.clone());
    if matches.is_empty() { return false; }
    let (start, end) = matches[self.search_match_idx.min(matches.len() - 1)];
    // Remove matched range and insert replacement.
    let len = end - start;
    for _ in 0..len {
        self.text.remove(start..start + 1);
    }
    self.text.insert(start, replacement);
    self.edit_version += 1;
    self.is_dirty = true;
    self.search_next(); // advance to next match
    true
}

/// Replace ALL occurrences of search_query with replacement.
pub fn replace_all(&mut self, replacement: &str) -> usize {
    let mut count = 0;
    loop {
        let matches = self.find_matches(&self.search_query.clone());
        if matches.is_empty() { break; }
        let (start, end) = matches[0];
        let len = end - start;
        for _ in 0..len { self.text.remove(start..start + 1); }
        self.text.insert(start, replacement);
        self.edit_version += 1;
        count += 1;
    }
    self.is_dirty = count > 0;
    count
}
```

### UI — replace bar

When `replace_active`, show a second bar below the search bar:
```
 Find:    <query>  [N/M]   [Enter=next  Shift+Enter=prev  Esc=close]
 Replace: <replacement>    [Ctrl+H=replace  Ctrl+Shift+H=replace all]
```

The two bars are each `Constraint::Length(1)` below the content area.

---

## Part B: Directory Watcher

### New dependency

```toml
notify = { version = "6", default-features = false, features = ["macos_fsevent"] }
```

On Windows, `notify` uses `ReadDirectoryChangesW` automatically.

### New `JobResult` variant

```rust
DirectoryChanged {
    path: PathBuf,
},
```

### WatcherWorker design

The watcher worker is different from the other workers — it receives a `WatchRequest { paths: Vec<PathBuf> }` to set the watched paths, and emits `JobResult::DirectoryChanged` when a change is detected. It uses `notify::RecommendedWatcher` with a channel bridge:

```rust
// In spawn_workers():
let (watch_tx, watch_rx) = bounded::<WatchRequest>(8);
{
    let result_tx = result_tx.clone();
    thread::Builder::new()
        .name("zeta-watcher".into())
        .spawn(move || run_watcher_worker(watch_rx, result_tx))
        .expect("failed to spawn watcher worker");
}
```

```rust
fn run_watcher_worker(
    watch_rx: Receiver<WatchRequest>,
    result_tx: Sender<JobResult>,
) {
    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc;

    let (notify_tx, notify_rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(notify_tx, Config::default())
        .expect("failed to create filesystem watcher");
    let mut watched_paths: Vec<PathBuf> = Vec::new();

    loop {
        // Process new watch requests (non-blocking).
        while let Ok(req) = watch_rx.try_recv() {
            // Unwatch old paths.
            for p in &watched_paths {
                let _ = watcher.unwatch(p);
            }
            watched_paths = req.paths;
            for p in &watched_paths {
                let _ = watcher.watch(p, RecursiveMode::NonRecursive);
            }
        }

        // Check for filesystem events (non-blocking).
        while let Ok(Ok(event)) = notify_rx.try_recv() {
            for path in event.paths {
                if let Some(parent) = path.parent() {
                    if result_tx
                        .send(JobResult::DirectoryChanged { path: parent.to_path_buf() })
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}
```

### State handling

In `AppState::apply_job_result`:

```rust
JobResult::DirectoryChanged { path } => {
    // Re-scan whichever pane is showing this directory.
    if self.panes.left.cwd == path {
        // queue Command::ScanPane { pane: Left, path }
    }
    if self.panes.right.cwd == path {
        // queue Command::ScanPane { pane: Right, path }
    }
}
```

Since `apply_job_result` doesn't return commands, the simplest approach is to directly update the scan state or emit a status message and let the user trigger a manual refresh. A slightly more complex approach: `apply_job_result` returns `Vec<Command>` (requires a signature change). Evaluate which is cleaner at implementation time.

### Tests

```rust
#[test]
fn replace_next_replaces_current_match_and_advances() { ... }

#[test]
fn replace_all_replaces_every_occurrence() { ... }

#[test]
fn replace_all_returns_correct_count() { ... }

#[test]
fn watcher_worker_responds_to_watch_request() { ... }
```

---

## Final verification

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
git commit -m "chore: Wave 5A complete — Find & Replace + directory watcher"
```
