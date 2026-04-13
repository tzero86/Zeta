# Workspaces Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 4 Linux-desktop-style workspaces to Zeta so each workspace preserves its own pane, preview, editor, terminal, and local transient runtime state while async jobs continue to settle correctly in the workspace that launched them.

**Architecture:** Introduce a `WorkspaceState` that owns the current single-desktop working context, then make `AppState` the global shell that tracks an active workspace and shared app-wide state. Thread `workspace_id` through async requests/results at the app/jobs boundary so background work updates the correct workspace even when it is not visible.

**Tech Stack:** Rust stable, existing `AppState`/`JobResult` architecture, `crossterm`, `ratatui`, Cargo fmt/clippy/test.

---

## File Map

**Modify**
- `src/state/mod.rs` — introduce `WorkspaceState`, move workspace-local fields under it, add active workspace handling, update reducers and tests
- `src/state/pane_set.rs` — reuse under `WorkspaceState`, adjust helpers/tests only if needed
- `src/state/editor_state.rs` — remain behaviorally same but move under workspace ownership
- `src/state/preview_state.rs` — remain behaviorally same but move under workspace ownership
- `src/state/terminal.rs` — remain behaviorally same but move under workspace ownership
- `src/action.rs` — add workspace switch actions and key routing for `Alt+1..Alt+4`
- `src/app.rs` — route workspace-scoped commands/results, persist multi-workspace session
- `src/jobs.rs` — add `workspace_id` to request/result types that mutate workspace-local state
- `src/session.rs` — persist multiple workspaces plus active workspace index
- `src/ui/mod.rs` and `src/ui/menu_bar.rs` — add minimal workspace indicator

**Maybe modify**
- `src/config.rs` — only if current keymap compilation strongly prefers configurable workspace bindings now; otherwise keep fixed bindings in code for v1

**Do not modify unless a failing test proves it is required**
- remote/SFTP semantics beyond carrying `workspace_id` through shared request/result shapes
- archive behavior unrelated to workspace routing
- preview/editor rendering behavior unrelated to switching and indication

---

### Task 1: Introduce `WorkspaceState` and move local runtime state under it

**Files:**
- Modify: `src/state/mod.rs`
- Maybe modify: `src/state/pane_set.rs`, `src/state/editor_state.rs`, `src/state/preview_state.rs`, `src/state/terminal.rs`
- Test: `src/state/mod.rs` tests

- [ ] **Step 1: Write failing state tests for independent workspace context**

Add focused tests in `src/state/mod.rs` proving the current single-state model is insufficient. Use names like:

```rust
#[test]
fn switching_workspace_preserves_independent_pane_directories() {
    let mut state = test_state();

    state.active_workspace_mut().panes.left.cwd = PathBuf::from("/repo-a");
    state.apply(Action::SwitchToWorkspace(1)).unwrap();
    state.active_workspace_mut().panes.left.cwd = PathBuf::from("/repo-b");
    state.apply(Action::SwitchToWorkspace(0)).unwrap();

    assert_eq!(state.active_workspace().panes.left.cwd, PathBuf::from("/repo-a"));
}

#[test]
fn switching_workspace_preserves_independent_editor_state() {
    let mut state = test_state();

    state.active_workspace_mut().editor.buffer = Some(EditorBuffer::from_text(
        PathBuf::from("/repo-a/a.txt"),
        String::from("alpha"),
    ));
    state.apply(Action::SwitchToWorkspace(1)).unwrap();
    assert!(state.active_workspace().editor.buffer.is_none());
}
```

- [ ] **Step 2: Run focused workspace-state tests and confirm failure**

Run:

```bash
cargo test switching_workspace_preserves_independent_ -- --nocapture
```

Expected: compile failure because workspace actions/helpers/state do not exist yet.

- [ ] **Step 3: Introduce `WorkspaceState` and migrate local fields**

In `src/state/mod.rs`, define a focused container for one desktop context. The shape should be close to:

```rust
#[derive(Clone, Debug)]
pub struct WorkspaceState {
    pub panes: PaneSetState,
    pub preview: PreviewState,
    pub editor: EditorState,
    pub terminal: crate::state::terminal::TerminalState,
    pending_reveal: Option<(PaneId, PathBuf)>,
    pending_batch: Option<PendingBatchOperation>,
    file_operation_status: Option<FileOperationStatus>,
    diff_mode: bool,
    diff_map: std::collections::HashMap<String, crate::diff::DiffStatus>,
    status_message: String,
    last_scan_time_ms: Option<u128>,
}
```

Then make `AppState` own:

```rust
pub struct AppState {
    workspaces: [WorkspaceState; 4],
    active_workspace: usize,
    overlay: OverlayState,
    config_path: String,
    config: AppConfig,
    icon_mode: IconMode,
    theme: ResolvedTheme,
    last_size: Option<(u16, u16)>,
    redraw_count: u64,
    startup_time_ms: u128,
    should_quit: bool,
    editor_fullscreen: bool,
    git: [Option<crate::git::RepoStatus>; 4 * 2],
}
```

You do not need to use this exact storage layout, but the design must make workspace-local state truly local and keep shell-global state global.

Add convenience helpers immediately so later edits stay readable:

```rust
pub fn active_workspace(&self) -> &WorkspaceState {
    &self.workspaces[self.active_workspace]
}
pub fn active_workspace_mut(&mut self) -> &mut WorkspaceState {
    &mut self.workspaces[self.active_workspace]
}
pub fn workspace(&self, idx: usize) -> &WorkspaceState {
    &self.workspaces[idx]
}
pub fn workspace_mut(&mut self, idx: usize) -> &mut WorkspaceState {
    &mut self.workspaces[idx]
}
```

Update existing `AppState` methods to delegate through the active workspace instead of directly touching flat fields.

- [ ] **Step 4: Re-run the focused workspace-state tests and confirm they pass**

Run:

```bash
cargo test switching_workspace_preserves_independent_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit the workspace-state split**

```bash
git add src/state/mod.rs src/state/pane_set.rs src/state/editor_state.rs src/state/preview_state.rs src/state/terminal.rs
git commit -m "refactor: introduce per-workspace runtime state"
```

---

### Task 2: Add workspace switching actions and workspace-aware key routing

**Files:**
- Modify: `src/action.rs`
- Modify: `src/state/mod.rs`
- Test: `src/action.rs` tests, `src/state/mod.rs` tests

- [ ] **Step 1: Write failing tests for direct workspace switching**

Add routing and reducer tests such as:

```rust
#[test]
fn alt_number_shortcuts_switch_workspaces() {
    let keymap = RuntimeKeymap::default();

    assert_eq!(
        Action::from_pane_key_event(
            KeyEvent::new(KeyCode::Char('1'), KeyModifiers::ALT),
            &keymap,
        ),
        Some(Action::SwitchToWorkspace(0))
    );
    assert_eq!(
        Action::from_pane_key_event(
            KeyEvent::new(KeyCode::Char('4'), KeyModifiers::ALT),
            &keymap,
        ),
        Some(Action::SwitchToWorkspace(3))
    );
}

#[test]
fn switching_workspace_changes_active_workspace_index() {
    let mut state = test_state();
    state.apply(Action::SwitchToWorkspace(2)).unwrap();
    assert_eq!(state.active_workspace_index(), 2);
}
```

- [ ] **Step 2: Run focused routing/switch tests and confirm failure**

Run:

```bash
cargo test workspace_switch -- --nocapture
```

Expected: compile failure because `SwitchToWorkspace` and related helpers do not exist yet.

- [ ] **Step 3: Implement fixed 4-workspace switching**

Add to `src/action.rs`:

```rust
SwitchToWorkspace(usize),
```

Handle `Alt+1..Alt+4` in pane/global key routing before other Alt-menu fallbacks intercept them.

In `src/state/mod.rs`, add a reducer branch that:
- validates the index is within `0..4`
- swaps the active workspace index
- leaves all other workspaces untouched
- updates status text to reflect the active workspace, e.g. `workspace 3 active`

Also add helper accessors used by UI/tests:

```rust
pub fn active_workspace_index(&self) -> usize {
    self.active_workspace
}
pub fn workspace_count(&self) -> usize {
    self.workspaces.len()
}
```

- [ ] **Step 4: Re-run focused workspace-switch tests and confirm they pass**

Run:

```bash
cargo test workspace_switch -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit workspace switching**

```bash
git add src/action.rs src/state/mod.rs
git commit -m "feat: add fixed workspace switching"
```

---

### Task 3: Make async requests and results workspace-aware

**Files:**
- Modify: `src/app.rs`
- Modify: `src/jobs.rs`
- Modify: `src/state/mod.rs`
- Test: `src/state/mod.rs` tests, `src/jobs.rs` tests if needed

- [ ] **Step 1: Write failing tests for job/result isolation between workspaces**

Add reducer tests in `src/state/mod.rs` showing that results mutate only the launching workspace:

```rust
#[test]
fn directory_scan_result_updates_only_matching_workspace() {
    let mut state = test_state();
    state.apply(Action::SwitchToWorkspace(1)).unwrap();

    state.apply_job_result(JobResult::DirectoryScanned {
        workspace_id: 1,
        pane: PaneId::Left,
        path: PathBuf::from("/repo-b"),
        entries: vec![],
        elapsed_ms: 1,
    });

    assert_eq!(state.workspace(1).panes.left.cwd, PathBuf::from("/repo-b"));
    assert_ne!(state.workspace(0).panes.left.cwd, PathBuf::from("/repo-b"));
}

#[test]
fn file_operation_progress_does_not_leak_across_workspaces() {
    let mut state = test_state();
    state.apply_job_result(JobResult::FileOperationProgress {
        workspace_id: 2,
        status: FileOperationStatus {
            operation: "copy",
            completed: 1,
            total: 3,
            current_path: PathBuf::from("/tmp/a"),
        },
    });

    assert!(state.workspace(2).file_operation_status.is_some());
    assert!(state.workspace(0).file_operation_status.is_none());
}
```

- [ ] **Step 2: Run focused isolation tests and confirm failure**

Run:

```bash
cargo test matching_workspace -- --nocapture
cargo test leak_across_workspaces -- --nocapture
```

Expected: compile failure because request/result types do not yet carry `workspace_id`.

- [ ] **Step 3: Thread `workspace_id` through async commands/results**

In `src/jobs.rs`, add `workspace_id: usize` to request/result types that are workspace-scoped. At minimum this should include:
- scan requests/results
- preview requests/results
- editor load/save results where applicable
- file operations
- terminal-related async output/state if terminal is workspace-local
- finder results if finder is workspace-local

Use a simple `usize` in v1.

In `src/app.rs`, when dispatching a workspace-scoped command, attach the current active workspace index.

When receiving a result, apply it to the matching workspace state, not implicitly to the currently visible one.

- [ ] **Step 4: Re-run focused isolation tests and existing jobs/state tests**

Run:

```bash
cargo test matching_workspace -- --nocapture
cargo test leak_across_workspaces -- --nocapture
cargo test batch_ -- --nocapture
cargo test file_operation_identity_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit workspace-aware async routing**

```bash
git add src/app.rs src/jobs.rs src/state/mod.rs
git commit -m "fix: route async results by workspace"
```

---

### Task 4: Persist lightweight per-workspace sessions

**Files:**
- Modify: `src/session.rs`
- Modify: `src/app.rs`
- Modify: `src/state/mod.rs`
- Test: `src/session.rs` tests, `src/state/mod.rs` tests

- [ ] **Step 1: Write failing tests for multi-workspace session persistence**

Add tests such as:

```rust
#[test]
fn session_round_trips_multiple_workspaces_and_active_index() {
    let session = SessionState {
        active_workspace: Some(2),
        workspaces: vec![
            WorkspaceSessionState {
                left_cwd: Some(PathBuf::from("/repo-a")),
                right_cwd: Some(PathBuf::from("/repo-b")),
                left_sort: None,
                right_sort: None,
                left_hidden: false,
                right_hidden: true,
                layout: Some(PaneLayout::SideBySide),
            },
            WorkspaceSessionState::default(),
            WorkspaceSessionState::default(),
            WorkspaceSessionState::default(),
        ],
    };

    let text = toml::to_string(&session).unwrap();
    let round_trip: SessionState = toml::from_str(&text).unwrap();
    assert_eq!(round_trip.active_workspace, Some(2));
    assert_eq!(round_trip.workspaces[0].left_cwd, Some(PathBuf::from("/repo-a")));
}
```

- [ ] **Step 2: Run focused session tests and confirm failure**

Run:

```bash
cargo test session_round_trips_multiple_workspaces -- --nocapture
```

Expected: compile failure because session types are still single-workspace.

- [ ] **Step 3: Extend session persistence to 4 workspaces**

In `src/session.rs`, replace the flat left/right model with a multi-workspace model such as:

```rust
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct WorkspaceSessionState {
    pub left_cwd: Option<PathBuf>,
    pub right_cwd: Option<PathBuf>,
    pub left_sort: Option<SortMode>,
    pub right_sort: Option<SortMode>,
    pub left_hidden: bool,
    pub right_hidden: bool,
    pub layout: Option<PaneLayout>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SessionState {
    pub active_workspace: Option<usize>,
    pub workspaces: Vec<WorkspaceSessionState>,
}
```

In bootstrap/save logic:
- restore up to 4 workspaces
- validate cwd paths as today (`is_dir()`)
- fall back safely for missing/malformed workspace entries
- persist active workspace index

Do not persist editor buffers or live terminal sessions in this phase.

- [ ] **Step 4: Re-run focused session tests and relevant state tests**

Run:

```bash
cargo test session_round_trips_multiple_workspaces -- --nocapture
cargo test switching_workspace_preserves_independent_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit workspace session persistence**

```bash
git add src/session.rs src/app.rs src/state/mod.rs
git commit -m "feat: persist lightweight workspace sessions"
```

---

### Task 5: Add minimal workspace indicator and final verification

**Files:**
- Modify: `src/ui/menu_bar.rs` and/or `src/ui/mod.rs`
- Test: relevant UI/state tests

- [ ] **Step 1: Write failing UI/state test for workspace indication**

Add a focused test around whatever helper renders the workspace indicator. Keep it minimal, e.g. if using the menu bar:

```rust
#[test]
fn workspace_indicator_marks_active_workspace() {
    let mut state = test_state();
    state.apply(Action::SwitchToWorkspace(2)).unwrap();

    let status = state.status_line();
    assert!(status.contains("ws:3/4"));
}
```

If the indicator goes in the menu bar instead, test the extracted helper directly.

- [ ] **Step 2: Run focused workspace-indicator test and confirm failure**

Run:

```bash
cargo test workspace_indicator_ -- --nocapture
```

Expected: fail because the indicator is not rendered yet.

- [ ] **Step 3: Implement the minimal indicator**

Use the least disruptive location — preferably status line or hint/menu bar helper — and render the active workspace clearly. Keep v1 simple:
- active workspace shown as `ws:1/4`, `ws:2/4`, etc.
- no naming system yet
- no extra layout churn beyond what is required to fit the text cleanly

- [ ] **Step 4: Run full verification**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 5: Inspect diff for drift and commit**

Run:

```bash
git diff --stat origin/main...HEAD
```

Expected: only workspace-related state/app/jobs/session/ui files changed.

Then commit:

```bash
git add -A
git commit -m "feat: add isolated workspaces"
```

---

## Self-Check Against Spec
- Fixed 4 workspaces: Tasks 1 and 2
- Full runtime isolation for pane/preview/editor/terminal/local transient state: Tasks 1 and 3
- Workspace-aware async job routing: Task 3
- Lightweight per-workspace session persistence: Task 4
- Keybindings and minimal workspace indication: Tasks 2 and 5
- No deep restart restore of editor buffers/terminals: Task 4 scope
- Final verification: Task 5

## Execution Notes
- Keep `WorkspaceState` focused: one desktop context, no extra abstractions.
- Prefer moving existing fields under workspace ownership over inventing new wrappers around old APIs.
- When a helper or accessor is updated, propagate all callers in the same task; do not leave mixed global/workspace state access alive.
- If a request/result is clearly app-global, do not force `workspace_id` onto it.
- Do not add configurable workspace counts in this phase.