# Operation Safety Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make file operations truthful, deterministic, and safer by hardening prompt behavior, batch semantics, refresh targeting, archive copy handling, and result reporting.

**Architecture:** Keep the existing modular-monolith structure. Concentrate behavior changes in `src/state/mod.rs` with small targeted helpers, reuse existing `FileOperation` variants rather than introducing a new operation framework, and verify correctness primarily through unit and integration tests.

**Tech Stack:** Rust stable, `crossterm`, `ratatui`, existing `FileOperation`/job system, Cargo test/clippy/fmt.

---

## File Map

**Modify**
- `src/state/mod.rs` — prompt submit behavior, batch tracking, refresh helpers, validation, post-op state cleanup
- `src/action.rs` — only if new status/reporting actions are required; avoid unless needed
- `src/pane.rs` — mark retention helpers only if current API is insufficient
- `src/session.rs` — already safe; no further changes expected unless plan drifts
- `tests/fs_integration.rs` — filesystem-level regression tests
- `src/state/mod.rs` tests — prompt/refresh/unit coverage

**Maybe modify**
- `src/jobs.rs` — only if result payloads are insufficient for truthful batch completion tracking
- `src/state/prompt.rs` — only if prompt summaries need additional explicit fields

---

### Task 1: Fix refresh truthfulness and copy semantics

**Files:**
- Modify: `src/state/mod.rs`
- Test: `src/state/mod.rs` tests, `tests/fs_integration.rs`

- [ ] **Step 1: Add failing unit tests for refresh derivation and archive copy helper**
  - Add a unit test proving batch copy/move refreshes the actual destination directory, not its parent.
  - Add a unit test proving archive-member copy selection returns `FileOperation::ExtractArchive` for archive paths and `FileOperation::Copy` otherwise.

- [ ] **Step 2: Run focused tests to verify failure**
  - Run: `cargo test refresh_targets_for_prompt cargo test copy_operation_for_source`
  - Expected: at least one failure or missing-test compile failure before implementation is complete.

- [ ] **Step 3: Implement exact-target refresh and shared copy derivation**
  - Keep `copy_operation_for_source()` as the single source of truth for copy-vs-extract.
  - Compute batch refresh targets per item from the real destination path.
  - Preserve archive extraction semantics for both single-item and marked-item copy.

- [ ] **Step 4: Re-run focused tests**
  - Run: `cargo test refresh_targets_for_prompt cargo test copy_operation_for_source`
  - Expected: pass.

- [ ] **Step 5: Commit**
  - `git add src/state/mod.rs tests/fs_integration.rs`
  - `git commit -m "fix: harden refresh targeting and archive copy semantics"`

### Task 2: Harden batch completion semantics and mark handling

**Files:**
- Modify: `src/state/mod.rs`
- Maybe modify: `src/pane.rs`
- Test: `src/state/mod.rs` tests, `tests/fs_integration.rs`

- [ ] **Step 1: Add failing tests for mark lifecycle and partial failures**
  - Add a unit test proving marks are not cleared at dispatch time.
  - Add a unit test proving partial batch failure leaves failed items marked.
  - Add an integration test for a mixed batch outcome (one success, one collision/failure).

- [ ] **Step 2: Run focused tests to verify failure**
  - Run: `cargo test marked_items cargo test partial_batch`
  - Expected: fail before implementation.

- [ ] **Step 3: Implement lightweight batch result aggregation**
  - Track submitted batch count and failing source paths in state.
  - Clear marks only on full success.
  - Retain failed items on partial failure.
  - Update status messages to distinguish success, partial success, and failure.

- [ ] **Step 4: Re-run focused tests**
  - Run: `cargo test marked_items cargo test partial_batch`
  - Expected: pass.

- [ ] **Step 5: Commit**
  - `git add src/state/mod.rs src/pane.rs tests/fs_integration.rs`
  - `git commit -m "fix: make batch completion and mark cleanup truthful"`

### Task 3: Add prompt and rename safety validation

**Files:**
- Modify: `src/state/mod.rs`
- Maybe modify: `src/state/prompt.rs`
- Test: `src/state/mod.rs` tests

- [ ] **Step 1: Add failing tests for rename validation and destructive prompt summaries**
  - Reject empty rename.
  - Reject path separators in rename target.
  - Treat same-name rename as no-op with explicit status.
  - Verify delete/trash prompt summaries include count and representative names when marks exist.

- [ ] **Step 2: Run focused tests to verify failure**
  - Run: `cargo test rename_validation cargo test delete_prompt`
  - Expected: fail before implementation.

- [ ] **Step 3: Implement pre-dispatch validation and prompt summary formatting**
  - Validate rename target before queuing `RunFileOperation`.
  - Improve prompt strings for marked destructive operations.
  - Keep UI concise; no new modal system.

- [ ] **Step 4: Re-run focused tests**
  - Run: `cargo test rename_validation cargo test delete_prompt`
  - Expected: pass.

- [ ] **Step 5: Commit**
  - `git add src/state/mod.rs src/state/prompt.rs`
  - `git commit -m "fix: add prompt and rename safety validation"`

### Task 4: End-to-end verification and cleanup

**Files:**
- Modify only what verification exposes
- Test: existing suite plus any new targeted tests

- [ ] **Step 1: Run formatter check**
  - Run: `cargo fmt --all -- --check`
  - Expected: pass.

- [ ] **Step 2: Run clippy**
  - Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - Expected: pass.

- [ ] **Step 3: Run full test suite**
  - Run: `cargo test --workspace`
  - Expected: pass.

- [ ] **Step 4: Inspect diff for drift**
  - Run: `git diff --stat origin/main...HEAD`
  - Expected: only operation-safety hardening files and tests changed.

- [ ] **Step 5: Final commit**
  - `git add -A`
  - `git commit -m "test: finalize operation safety hardening coverage"`

---

## Self-Check Against Spec
- Prompt truthfulness: Tasks 2 and 3
- Batch semantics and mark retention: Task 2
- Archive-member copy correctness: Task 1
- Exact refresh targeting: Task 1
- Partial-failure reporting: Task 2
- Validation before dispatch: Task 3
- Full verification: Task 4

## Execution Notes
- Prefer narrow test runs while iterating; run the full workspace only in Task 4.
- Do not widen scope into watcher/editor/general state cleanup unless a failing test proves it is required for operation safety.
- Keep helpers local unless duplication appears twice.
