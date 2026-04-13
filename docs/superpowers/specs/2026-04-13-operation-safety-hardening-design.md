# Operation Safety Hardening Design

**Date:** 2026-04-13
**Branch:** `hardening/operation-safety`

## Goal
Harden Zeta's file-operation behavior so copy, move, delete, trash, rename, and batch workflows remain truthful, deterministic, and resistant to operator mistakes without removing features or changing the product's basic interaction model.

## Non-goals
- No feature-loss work
- No plugin, remote, or architecture redesign unrelated to file-operation safety
- No broad editor/watcher refactors except where directly required by operation correctness
- No speculative abstractions beyond the affected workflow

## Scope
This branch covers:
- batch operation semantics
- destructive-operation prompt safety
- rename validation and inline-rename truthfulness
- archive-member copy correctness
- exact post-operation refresh targets
- partial-failure reporting
- regression tests for copy/move/delete/trash/rename edge cases

This branch does not cover:
- unrelated editor feature work
- large-scale async/job-system redesign
- replacing existing operation primitives with a new framework

## Design Invariants
1. Prompts must describe what the operation will actually do.
2. Marks must not be cleared optimistically before the operation outcome is known.
3. Partial success must never be reported as full success.
4. Refresh must target every actually affected pane/path.
5. Batch ordering must be deterministic.
6. Archive-member copy must behave as extraction, not a fake filesystem copy.
7. Validation belongs before dispatch when the error is knowable in advance.

## User-Facing Behavior

### 1. Prompt safety
- Copy/move prompts show item count when marks exist.
- Delete/trash prompts show item count and a short preview of target names.
- Permanent delete wording stays explicit about irreversibility.
- Rename rejects:
  - empty names
  - names containing path separators
  - obvious no-op submissions (`old == new`)
- Validation failures do not dispatch jobs; they update status text instead.

### 2. Batch semantics
- If marks exist, they are the source set even when only one item is marked.
- Batch source order is deterministic (`BTreeSet`/sorted path order).
- Marks are preserved until outcome is known.
- On full success, marks clear.
- On partial failure, failed items remain marked so the operator can retry or inspect.
- Status messages distinguish:
  - completed
  - partially completed
  - failed

### 3. Collision behavior
- Pre-dispatch warnings summarize likely collisions for copy/move.
- Runtime collision policy remains authoritative.
- Single-item and batch flows share the same collision logic.
- The branch keeps current collision policy choices, but removes mismatches between prompt expectations and job execution.

### 4. Refresh and state cleanup
- Refresh targets are computed per actual affected path, not from a shared guessed parent.
- Copy/move across panes refreshes both panes when required.
- Archive extraction refreshes the destination pane/path only.
- Overlay closure can still happen at dispatch time, but mark cleanup and completion messaging happen from results, not optimistic assumptions.

## Internal Design

### A. Operation result aggregation
Add a lightweight batch tracking layer in state, keyed by submitted batch operation group, so result handling can answer:
- how many operations were dispatched
- how many succeeded
- how many failed/collided
- which source paths failed

This tracking stays in `src/state/mod.rs` or a focused adjacent state module unless it becomes large enough to justify extraction.

### B. Shared copy-operation derivation
The logic that decides whether a copy is a normal filesystem copy or an archive extraction should live in one helper and be used by both:
- single-item prompt submit
- batch prompt submit

### C. Shared refresh derivation
Refresh target derivation should accept the actual affected target path for each operation. Batch copy/move must compute refresh per item, not once from a common directory input.

### D. Validation layer
Simple validation belongs before dispatch:
- rename target validity
- obviously invalid destination combinations
- destructive-operation prompt text derived from the real source set

This validation should produce user-visible status messages and skip dispatch when invalid.

## Testing Strategy

### Unit tests
Add/extend unit tests for:
- refresh target derivation from exact destination paths
- archive-member copy helper selection
- rename validation rules
- batch result aggregation behavior

### Integration tests
Add filesystem integration coverage for:
- batch copy refreshing destination pane inputs correctly
- batch move with mixed outcomes
- delete/trash with marked items
- rename invalid target rejection
- archive-member copy through marked/batch path
- same-name rename no-op behavior

### Regression focus
The branch should explicitly guard against:
- stale destination panes after batch copy/move
- marks clearing before failed jobs are visible
- archive copies failing because virtual paths were treated as real paths
- destructive prompts understating what will be deleted

## Acceptance Criteria
- Copy/move/delete/trash/rename behavior is deterministic for marked and unmarked flows.
- Batch copy from archive panes works through the same extract semantics as single-item copy.
- Destination panes refresh correctly after batch operations.
- Marks are not lost when part of a batch fails.
- Invalid rename submissions are blocked before dispatch.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` all pass.

## Recommended Branch Strategy
Use one focused branch:
- `hardening/operation-safety`

Keep the branch centered on operation truthfulness and operator safety. If unrelated correctness work appears during implementation, defer it unless it blocks one of the invariants above.
