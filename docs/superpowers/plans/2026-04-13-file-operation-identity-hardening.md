# File Operation Identity Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make local file-operation results carry canonical operation identity so batch settlement in state is driven by truthful result metadata instead of queue order or progress-path interpretation.

**Architecture:** Keep the change tightly scoped to the existing jobs/state boundary. Define a small identity type in `src/jobs.rs`, thread it through local file-operation result variants, then migrate `src/state/mod.rs` batch settlement to consume that identity directly. Do not introduce operation UUIDs, a new dispatcher, or SFTP behavior changes in this phase.

**Tech Stack:** Rust stable, existing `FileOperation`/`JobResult` model, `cargo test`, `cargo fmt`, `cargo clippy`.

---

## File Map

**Modify**
- `src/jobs.rs` — define `FileOperationKind` / `FileOperationIdentity`, derive identity from `FileOperation`, thread identity through local file-op `JobResult` variants, and add unit tests for identity mapping
- `src/state/mod.rs` — replace queue-order batch settlement with identity-based settlement and update state tests

**Do not modify unless a failing test proves it is required**
- `src/action.rs`
- `src/pane.rs`
- `src/session.rs`
- SFTP execution code in `src/jobs.rs` beyond type compatibility fixes
- prompt/UI modules

---

### Task 1: Add canonical file-operation identity in jobs

**Files:**
- Modify: `src/jobs.rs`
- Test: `src/jobs.rs` unit tests

- [ ] **Step 1: Write failing jobs-layer identity tests**

Add tests near the existing `src/jobs.rs` unit tests for the canonical identity mapping. Use names like:

```rust
#[test]
fn file_operation_identity_maps_copy_source_and_destination() {
    let identity = file_operation_identity(&FileOperation::Copy {
        source: PathBuf::from("/tmp/src.txt"),
        destination: PathBuf::from("/tmp/dst.txt"),
    });

    assert_eq!(identity.kind, FileOperationKind::Copy);
    assert_eq!(identity.source, PathBuf::from("/tmp/src.txt"));
    assert_eq!(identity.destination, Some(PathBuf::from("/tmp/dst.txt")));
}

#[test]
fn file_operation_identity_maps_extract_archive_member_as_source() {
    let identity = file_operation_identity(&FileOperation::ExtractArchive {
        archive: PathBuf::from("/tmp/bundle.zip"),
        inner_path: PathBuf::from("nested").join("note.txt"),
        destination: PathBuf::from("/tmp/out"),
    });

    assert_eq!(identity.kind, FileOperationKind::ExtractArchive);
    assert_eq!(
        identity.source,
        PathBuf::from("/tmp/bundle.zip").join("nested").join("note.txt"),
    );
    assert_eq!(identity.destination, Some(PathBuf::from("/tmp/out")));
}
```

Also add at least one no-destination case:

```rust
#[test]
fn file_operation_identity_maps_delete_without_destination() {
    let identity = file_operation_identity(&FileOperation::Delete {
        path: PathBuf::from("/tmp/old.txt"),
    });

    assert_eq!(identity.kind, FileOperationKind::Delete);
    assert_eq!(identity.source, PathBuf::from("/tmp/old.txt"));
    assert_eq!(identity.destination, None);
}
```

- [ ] **Step 2: Run focused jobs tests and confirm they fail for the right reason**

Run:

```bash
cargo test file_operation_identity_ -- --nocapture
```

Expected: compile failure or test failure because `FileOperationKind`, `FileOperationIdentity`, or `file_operation_identity()` do not exist yet.

- [ ] **Step 3: Add the minimal identity types and derivation helper in `src/jobs.rs`**

Add a focused enum + struct near the existing `JobResult` definitions:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileOperationKind {
    Copy,
    Move,
    Rename,
    Trash,
    Delete,
    CreateDirectory,
    CreateFile,
    ExtractArchive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileOperationIdentity {
    pub kind: FileOperationKind,
    pub source: PathBuf,
    pub destination: Option<PathBuf>,
}
```

Add a helper that derives the identity once from the submitted operation:

```rust
fn file_operation_identity(operation: &FileOperation) -> FileOperationIdentity {
    match operation {
        FileOperation::Copy { source, destination } => FileOperationIdentity {
            kind: FileOperationKind::Copy,
            source: source.clone(),
            destination: Some(destination.clone()),
        },
        FileOperation::Move { source, destination } => FileOperationIdentity {
            kind: FileOperationKind::Move,
            source: source.clone(),
            destination: Some(destination.clone()),
        },
        FileOperation::Rename { source, destination } => FileOperationIdentity {
            kind: FileOperationKind::Rename,
            source: source.clone(),
            destination: Some(destination.clone()),
        },
        FileOperation::Trash { path } => FileOperationIdentity {
            kind: FileOperationKind::Trash,
            source: path.clone(),
            destination: None,
        },
        FileOperation::Delete { path } => FileOperationIdentity {
            kind: FileOperationKind::Delete,
            source: path.clone(),
            destination: None,
        },
        FileOperation::CreateDirectory { path } => FileOperationIdentity {
            kind: FileOperationKind::CreateDirectory,
            source: path.clone(),
            destination: Some(path.clone()),
        },
        FileOperation::CreateFile { path } => FileOperationIdentity {
            kind: FileOperationKind::CreateFile,
            source: path.clone(),
            destination: Some(path.clone()),
        },
        FileOperation::ExtractArchive {
            archive,
            inner_path,
            destination,
        } => FileOperationIdentity {
            kind: FileOperationKind::ExtractArchive,
            source: archive.join(inner_path),
            destination: Some(destination.clone()),
        },
    }
}
```

- [ ] **Step 4: Re-run the focused jobs tests and confirm they pass**

Run:

```bash
cargo test file_operation_identity_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit the identity-type addition**

```bash
git add src/jobs.rs
git commit -m "refactor: add canonical file operation identity"
```

---

### Task 2: Thread identity through local file-operation job results

**Files:**
- Modify: `src/jobs.rs`
- Test: `src/jobs.rs` unit tests

- [ ] **Step 1: Write failing result-shape tests for local file operations**

Add tests that prove local file-op results carry canonical identity:

```rust
#[test]
fn file_op_extract_result_carries_archive_member_identity() {
    use std::io::Write;
    use zip::write::FileOptions;

    let root = std::env::temp_dir().join("zeta-jobs-identity-test");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let archive_path = root.join("bundle.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive_path).unwrap());
    zip.start_file::<_, ()>("a.txt", FileOptions::default()).unwrap();
    zip.write_all(b"hello").unwrap();
    zip.finish().unwrap();

    let (workers, results) = spawn_workers();
    workers.file_op_tx.send(FileOpRequest {
        backend: BackendRef::Local,
        operation: FileOperation::ExtractArchive {
            archive: archive_path.clone(),
            inner_path: PathBuf::from("a.txt"),
            destination: root.join("out"),
        },
        refresh: vec![],
        collision: CollisionPolicy::Fail,
        src_session: None,
        dst_session: None,
    }).unwrap();

    loop {
        let result = results.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        if let JobResult::FileOperationCompleted { identity, .. } = result {
            assert_eq!(identity.kind, FileOperationKind::ExtractArchive);
            assert_eq!(identity.source, archive_path.join("a.txt"));
            break;
        }
    }
}
```

Also add a direct unit test for local failures using `run_file_operation()` if needed, asserting:

```rust
match result {
    Err(JobResult::JobFailed { file_op: Some(identity), .. }) => {
        assert_eq!(identity.kind, FileOperationKind::Copy);
        assert_eq!(identity.source, PathBuf::from("/tmp/missing.txt"));
    }
    other => panic!("unexpected result: {other:?}"),
}
```

- [ ] **Step 2: Run focused local job-result tests and confirm failure**

Run:

```bash
cargo test file_op_extract_result_carries_archive_member_identity -- --nocapture
```

Expected: compile failure because `JobResult::FileOperationCompleted` does not yet expose `identity`, or assertion failure because the result lacks canonical identity.

- [ ] **Step 3: Thread identity through local file-operation results in `src/jobs.rs`**

At the top of `run_file_operation()`, derive identity once:

```rust
let identity = file_operation_identity(&operation);
```

Then thread it into result variants:

```rust
Ok(JobResult::FileOperationCompleted {
    identity,
    message: operation_label,
    refreshed,
    elapsed_ms: started_at.elapsed().as_millis(),
})
```

```rust
Err(JobResult::FileOperationCollision {
    identity,
    operation,
    refresh,
    path: PathBuf::from(path),
    elapsed_ms: started_at.elapsed().as_millis(),
})
```

```rust
Err(JobResult::JobFailed {
    file_op: Some(identity),
    pane: failure_pane,
    path: primary_path,
    message: other.to_string(),
    elapsed_ms: started_at.elapsed().as_millis(),
})
```

For non-file-op worker failures already emitted elsewhere, keep:

```rust
file_op: None,
```

Do not widen the SFTP logic beyond making the result type compile; if the SFTP branch must construct `FileOperationCompleted` / `JobFailed`, keep behavior unchanged but fill `file_op`/`identity` only if it is trivial and local to the type change.

- [ ] **Step 4: Re-run the focused local job-result tests and confirm they pass**

Run:

```bash
cargo test file_op_extract_result_carries_archive_member_identity -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit the jobs boundary change**

```bash
git add src/jobs.rs
git commit -m "refactor: attach identity to local file operation results"
```

---

### Task 3: Migrate state batch settlement to identity-based tracking

**Files:**
- Modify: `src/state/mod.rs`
- Test: `src/state/mod.rs` tests

- [ ] **Step 1: Write failing state tests that stop depending on queue order or path shape**

Add tests near the existing batch-settlement tests.

First, add a test that proves completed results settle by identity even when the display path would not help:

```rust
#[test]
fn batch_move_completion_settles_by_identity_not_display_path() {
    let mut state = test_state();
    state.panes.right.cwd = PathBuf::from("/tmp/target");
    state.panes.left.marked.insert(PathBuf::from("./note.txt"));

    let mut prompt = PromptState::with_value(
        PromptKind::Move,
        "Move Marked Items",
        PathBuf::from("/tmp/target"),
        None,
        String::from("/tmp/target"),
    );
    prompt.source_paths = vec![PathBuf::from("./note.txt")];
    state.overlay.open_prompt(prompt);
    state.apply(Action::PromptSubmit).unwrap();

    state.apply_job_result(JobResult::FileOperationCompleted {
        identity: FileOperationIdentity {
            kind: FileOperationKind::Move,
            source: PathBuf::from("./note.txt"),
            destination: Some(PathBuf::from("/tmp/target/note.txt")),
        },
        message: String::from("moved"),
        refreshed: Vec::new(),
        elapsed_ms: 1,
    });

    assert_eq!(state.panes.left.marked_count(), 0);
    assert!(state.pending_batch.is_none());
}
```

Also add a non-file-op failure test:

```rust
#[test]
fn non_file_op_job_failure_does_not_settle_pending_batch() {
    let mut state = test_state();
    state.panes.right.cwd = PathBuf::from("/tmp/target");
    state.panes.left.marked.insert(PathBuf::from("./note.txt"));

    let mut prompt = PromptState::with_value(
        PromptKind::Copy,
        "Copy Marked Items",
        PathBuf::from("/tmp/target"),
        None,
        String::from("/tmp/target"),
    );
    prompt.source_paths = vec![PathBuf::from("./note.txt")];
    state.overlay.open_prompt(prompt);
    state.apply(Action::PromptSubmit).unwrap();

    state.apply_job_result(JobResult::JobFailed {
        file_op: None,
        pane: PaneId::Left,
        path: PathBuf::from("/tmp/unrelated"),
        message: String::from("watcher failed"),
        elapsed_ms: 1,
    });

    assert!(state.pending_batch.is_some());
    assert_eq!(state.panes.left.marked_count(), 1);
}
```

- [ ] **Step 2: Run focused state tests and confirm failure**

Run:

```bash
cargo test batch_move_completion_settles_by_identity_not_display_path -- --nocapture
cargo test non_file_op_job_failure_does_not_settle_pending_batch -- --nocapture
```

Expected: compile failure because state still expects old `JobResult` shapes, or logic failure because settlement still depends on queue-order bookkeeping.

- [ ] **Step 3: Replace queue-order batch bookkeeping with identity-based settlement**

Update `PendingBatchOperation` to hold sets keyed by canonical source path, not `VecDeque<PathBuf>`.

The target shape should be closer to:

```rust
struct PendingBatchOperation {
    pane: PaneId,
    original_sources: BTreeSet<PathBuf>,
    failed_sources: BTreeSet<PathBuf>,
    settled_sources: BTreeSet<PathBuf>,
    total_count: usize,
}
```

Update `PromptSubmit` batch initialization to build `original_sources` from the submitted source paths and drop queue-order state.

Replace `note_batch_settled()` with an identity-driven helper, for example:

```rust
fn note_batch_settled(&mut self, identity: &FileOperationIdentity, failed: bool) {
    let mut finalize: Option<(PaneId, BTreeSet<PathBuf>, BTreeSet<PathBuf>, usize)> = None;

    if let Some(batch) = self.pending_batch.as_mut() {
        if batch.original_sources.contains(&identity.source)
            && batch.settled_sources.insert(identity.source.clone())
        {
            if failed {
                batch.failed_sources.insert(identity.source.clone());
            }
            if batch.settled_sources.len() >= batch.total_count {
                finalize = Some((
                    batch.pane,
                    batch.original_sources.clone(),
                    batch.failed_sources.clone(),
                    batch.total_count,
                ));
            }
        }
    }

    if let Some((pane_id, originals, failed_sources, total_count)) = finalize {
        let succeeded = total_count.saturating_sub(failed_sources.len());
        let pane = self.panes.pane_mut(pane_id);
        for source in originals {
            if !failed_sources.contains(&source) {
                pane.marked.remove(&source);
            }
        }
        self.status_message = if failed_sources.is_empty() {
            format!("completed {succeeded} items")
        } else if succeeded == 0 {
            format!("failed {total_count} items")
        } else {
            format!(
                "partially completed: {succeeded} succeeded, {} failed",
                failed_sources.len()
            )
        };
        self.pending_batch = None;
    }
}
```

Then update result handling:

```rust
JobResult::FileOperationCompleted { identity, message, refreshed, elapsed_ms } => {
    // refresh handling stays intact
    if self.pending_batch.is_some() {
        self.note_batch_settled(&identity, false);
        if self.pending_batch.is_some() {
            self.status_message = format!("{message} in {elapsed_ms} ms");
        }
    } else {
        self.status_message = format!("{message} in {elapsed_ms} ms");
    }
}
```

```rust
JobResult::FileOperationCollision { identity, operation, refresh, path, elapsed_ms } => {
    self.note_batch_settled(&identity, true);
    // collision UI behavior stays the same
}
```

```rust
JobResult::JobFailed { file_op, path, message, elapsed_ms, .. } => {
    if let Some(identity) = file_op.as_ref() {
        self.note_batch_settled(identity, true);
        if self.pending_batch.is_some() {
            self.status_message = format!(
                "job failed for {} after {elapsed_ms} ms: {message}",
                path.display()
            );
        }
    } else {
        self.status_message = format!(
            "job failed for {} after {elapsed_ms} ms: {message}",
            path.display()
        );
    }
}
```

Remove any helper whose only purpose was reconstructing source identity from `FileOperation` or queued order if it becomes dead code.

- [ ] **Step 4: Re-run focused state tests and existing batch tests**

Run:

```bash
cargo test batch_move_completion_settles_by_identity_not_display_path -- --nocapture
cargo test non_file_op_job_failure_does_not_settle_pending_batch -- --nocapture
cargo test batch_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit the state migration**

```bash
git add src/state/mod.rs
git commit -m "fix: settle batches from file operation identity"
```

---

### Task 4: Full verification and drift check

**Files:**
- Modify only what verification exposes
- Test: workspace checks

- [ ] **Step 1: Run formatter check**

Run:

```bash
cargo fmt --all -- --check
```

Expected: PASS.

- [ ] **Step 2: Run clippy**

Run:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: PASS.

- [ ] **Step 3: Run full test suite**

Run:

```bash
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 4: Inspect diff for scope drift**

Run:

```bash
git diff --stat origin/main...HEAD
```

Expected: only `src/jobs.rs`, `src/state/mod.rs`, and the new plan/spec docs unless verification forced a tightly related change.

- [ ] **Step 5: Commit the verification pass**

```bash
git add -A
git commit -m "test: finalize file operation identity hardening"
```

---

## Self-Check Against Spec
- Canonical identity type in jobs: Task 1
- Result payloads carry identity: Task 2
- State batch settlement no longer depends on queue order: Task 3
- Progress path remains display-only: Task 3
- Archive-member extraction identity preserved: Tasks 1 and 2
- Local-first design with later SFTP adoption possible: Tasks 1 and 2
- Full verification: Task 4

## Execution Notes
- Prefer narrow `cargo test <name>` runs while iterating.
- Do not widen scope into operation IDs, UI polish, or SFTP behavior changes unless a failing test proves the type contract cannot compile cleanly without it.
- If the SFTP result constructors break due to the new `JobResult` shape, make the smallest compatibility edit there and stop.
- Delete obsolete queue-order helpers in the same change that replaces them; do not leave both systems live.