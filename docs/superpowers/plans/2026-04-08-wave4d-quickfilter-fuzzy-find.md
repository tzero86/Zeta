# Wave 4D — In-Pane Quick Filter + Fuzzy File Find

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two fast-navigation features. Quick filter (`/`) filters the current pane's entries in real time as you type. Fuzzy file find (`Ctrl+P`) searches file names across the current directory tree and jumps to the result.

**Architecture:**

**Quick filter:**
- `PaneState` gains `filter_query: String` and `filter_active: bool`.
- `visible_entries()` in `pane.rs` returns only entries whose name contains `filter_query` (case-insensitive substring, not fuzzy) when `filter_active`.
- `FocusLayer::PaneFilter` — new layer variant. In this layer, printable keys append to `filter_query`; `Esc`/`Enter` exit filter mode.
- A one-line filter bar renders at the bottom of the active pane block.

**Fuzzy file find:**
- New `FinderWorker` — fifth background worker. Receives `FindRequest { pane: PaneId, root: PathBuf }`, walks the tree (bounded depth), sends `JobResult::FindResults { entries: Vec<PathBuf> }`.
- `FocusLayer::Modal(ModalKind::FileFinder)` — reuses the existing modal infrastructure.
- UI: a full-width modal with a search input and a scrollable list of matches, filtered client-side using a simple subsequence match (no external fuzzy crate needed).
- Selecting a result navigates the active pane to that file's parent directory and selects the file.

**No new dependencies.** Walk uses `std::fs::read_dir` recursively (bounded to 5 levels by default to avoid very deep trees).

**Jira:** ZTA-91 (ZTA-144 through ZTA-150)

**Wave dependency:** Starts AFTER Wave 4C. Requires `FocusLayer`, `WorkerChannels`, `ModalKind`.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/pane.rs` | `filter_query`, `filter_active`, updated `visible_entries()` |
| Modify | `src/state/types.rs` | `FocusLayer::PaneFilter`; `ModalKind::FileFinder` |
| Modify | `src/state/mod.rs` | Pane filter actions; finder state; `focus_layer()` update |
| Modify | `src/jobs.rs` | `FindRequest`, `JobResult::FindResults`, `FinderWorker`, `find_tx` on `WorkerChannels` |
| Modify | `src/action.rs` | New actions for filter and finder |
| Modify | `src/app.rs` | Dispatch `FindRequest`; handle `FindResults` result |
| Modify | `src/ui/pane.rs` | Render filter bar at pane bottom |
| Create | `src/ui/finder.rs` | Render finder modal (input + result list) |
| Modify | `src/ui/mod.rs` | `pub mod finder;` call site |

---

## Quick filter keybindings

| Key | Context | Action |
|---|---|---|
| `/` | Pane focused | `OpenPaneFilter` — enter filter mode |
| printable char | `PaneFilter` layer | `PaneFilterInput(ch)` — append to query |
| `Backspace` | `PaneFilter` layer | `PaneFilterBackspace` |
| `Esc` / `Enter` | `PaneFilter` layer | `ClosePaneFilter` — clear and exit |

## Fuzzy finder keybindings

| Key | Context | Action |
|---|---|---|
| `Ctrl+P` | Any pane context | `OpenFileFinder` |
| printable char | `FileFinder` modal | `FileFinderInput(ch)` |
| `Backspace` | `FileFinder` modal | `FileFinderBackspace` |
| `Up` / `Down` | `FileFinder` modal | `FileFinderMoveUp/Down` |
| `Enter` | `FileFinder` modal | `FileFinderConfirm` — navigate to selection |
| `Esc` | `FileFinder` modal | `CloseFileFinder` |

---

## Task 1: Quick filter in PaneState

**Files:** `src/pane.rs`

- [ ] **Step 1.1: Add fields to `PaneState`**

```rust
pub struct PaneState {
    // ... existing fields ...
    pub filter_query: String,
    pub filter_active: bool,
}
```

- [ ] **Step 1.2: Update `visible_entries()` to respect filter**

```rust
pub fn visible_entries(&self, height: usize) -> Vec<&EntryInfo> {
    let filtered: Vec<&EntryInfo> = if self.filter_active && !self.filter_query.is_empty() {
        let q = self.filter_query.to_lowercase();
        self.entries.iter().filter(|e| e.name.to_lowercase().contains(&q)).collect()
    } else {
        self.entries.iter().collect()
    };
    // ... existing scroll/selection logic on `filtered` ...
}
```

- [ ] **Step 1.3: Tests**

```rust
#[test]
fn filter_active_hides_non_matching_entries() { ... }

#[test]
fn filter_empty_query_shows_all_entries() { ... }

#[test]
fn filter_is_case_insensitive() { ... }
```

- [ ] **Step 1.4: Commit**

```bash
git commit -m "feat(pane): add filter_query + filter_active to PaneState; filter visible_entries"
```

---

## Task 2: FocusLayer::PaneFilter + actions

**Files:** `src/state/types.rs`, `src/action.rs`, `src/state/mod.rs`

- [ ] **Step 2.1: Add `FocusLayer::PaneFilter` and `ModalKind::FileFinder`**

- [ ] **Step 2.2: Add actions to `action.rs`**

```rust
OpenPaneFilter,
PaneFilterInput(char),
PaneFilterBackspace,
ClosePaneFilter,
OpenFileFinder,
FileFinderInput(char),
FileFinderBackspace,
FileFinderMoveUp,
FileFinderMoveDown,
FileFinderConfirm,
CloseFileFinder,
```

- [ ] **Step 2.3: Wire `/` to `OpenPaneFilter` in `from_pane_key_event`**

- [ ] **Step 2.4: Handle filter actions in `AppState::apply_view`**

```rust
Action::OpenPaneFilter => {
    self.panes.active_pane_mut().filter_active = true;
    self.panes.active_pane_mut().filter_query.clear();
}
Action::PaneFilterInput(ch) => {
    self.panes.active_pane_mut().filter_query.push(*ch);
}
Action::PaneFilterBackspace => {
    self.panes.active_pane_mut().filter_query.pop();
}
Action::ClosePaneFilter => {
    self.panes.active_pane_mut().filter_active = false;
    self.panes.active_pane_mut().filter_query.clear();
}
```

- [ ] **Step 2.5: Update `focus_layer()` to return `PaneFilter` when filter is active**

```rust
if self.panes.active_pane().filter_active {
    return FocusLayer::PaneFilter;
}
```

- [ ] **Step 2.6: Add `PaneFilter` arm to `route_key_event` in `app.rs`**

```rust
FocusLayer::PaneFilter => {
    Action::from_pane_filter_key_event(key_event)
}
```

- [ ] **Step 2.7: Commit**

```bash
git commit -m "feat(state): pane filter layer — OpenPaneFilter, input routing, focus_layer update"
```

---

## Task 3: FinderWorker + JobResult::FindResults

**Files:** `src/jobs.rs`

- [ ] **Step 3.1: Add `FindRequest` and `JobResult::FindResults`**

```rust
#[derive(Clone, Debug)]
pub struct FindRequest {
    pub root: PathBuf,
    pub max_depth: usize,
}

// In JobResult:
FindResults {
    root: PathBuf,
    entries: Vec<PathBuf>,
},
```

- [ ] **Step 3.2: Implement `walk_for_files(root, max_depth)`**

Pure function (no subprocess). Returns `Vec<PathBuf>` of all file paths within `max_depth` levels. Skips hidden directories and common noise directories (`target`, `node_modules`, `.git`).

```rust
fn walk_for_files(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut results = Vec::new();
    walk_recursive(root, root, max_depth, &mut results);
    results
}

fn walk_recursive(root: &Path, dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth == 0 { return; }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || matches!(name, "target" | "node_modules" | "__pycache__") {
            continue;
        }
        if path.is_dir() {
            walk_recursive(root, &path, depth - 1, out);
        } else {
            out.push(path);
        }
    }
}
```

- [ ] **Step 3.3: Add `find_tx: Sender<FindRequest>` to `WorkerChannels` and spawn FinderWorker**

- [ ] **Step 3.4: Tests**

```rust
#[test]
fn finder_worker_returns_find_results() {
    let (workers, results) = spawn_workers();
    workers.find_tx.send(FindRequest { root: temp_dir(), max_depth: 3 }).unwrap();
    let result = results.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(result, JobResult::FindResults { .. }));
}
```

- [ ] **Step 3.5: Commit**

```bash
git commit -m "feat(jobs): add FinderWorker and JobResult::FindResults"
```

---

## Task 4: File finder state + UI

**Files:** `src/state/mod.rs`, `src/ui/finder.rs`, `src/ui/mod.rs`

- [ ] **Step 4.1: Add finder state to `AppState`**

```rust
pub struct FinderState {
    pub query: String,
    pub all_entries: Vec<PathBuf>,   // full result set from worker
    pub filtered: Vec<PathBuf>,      // client-side filtered by query
    pub selection: usize,
    pub root: PathBuf,
}
```

Store as `finder: Option<FinderState>` in `AppState`.

- [ ] **Step 4.2: Client-side filtering — subsequence match**

When `FileFinderInput` or `FileFinderBackspace` fires, re-filter `all_entries` using a subsequence match:

```rust
fn subsequence_match(query: &str, candidate: &str) -> bool {
    let q = query.to_lowercase();
    let c = candidate.to_lowercase();
    let mut qi = q.chars();
    let mut current = qi.next();
    for ch in c.chars() {
        if Some(ch) == current {
            current = qi.next();
        }
        if current.is_none() { return true; }
    }
    false
}
```

- [ ] **Step 4.3: Handle `OpenFileFinder` — dispatch `FindRequest` + open modal**

- [ ] **Step 4.4: Handle `FindResults` in `apply_job_result`**

When results arrive, store in `finder.all_entries` and re-filter with current query.

- [ ] **Step 4.5: Handle `FileFinderConfirm` — navigate pane to selection**

Extract parent directory from selected path, dispatch `Command::ScanPane` and set selected entry.

- [ ] **Step 4.6: Create `src/ui/finder.rs`**

Render a centred modal with:
- Title: `Find: <query>`
- Scrollable list of `filtered` paths (relative to root, file name bold)
- Selected entry highlighted

- [ ] **Step 4.7: Render filter bar in pane**

In `src/ui/pane.rs`, when `pane.filter_active`, render a one-line bar at the pane bottom:
```
  Filter: <query>█
```

- [ ] **Step 4.8: Commit**

```bash
git commit -m "feat: in-pane quick filter + Ctrl+P fuzzy file finder"
```

---

## Task 5: Final verification

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Manual smoke test:
- Press `/` in a pane → filter bar appears, typing narrows entries
- Press `Esc` → filter clears, all entries return
- Press `Ctrl+P` → finder modal opens
- Type part of a filename → list narrows
- Press `Enter` → pane navigates to that file's directory

```bash
git commit -m "chore: Wave 4D complete — quick filter + fuzzy file find"
```
