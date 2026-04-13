# File Operation Identity Hardening Design

**Date:** 2026-04-13
**Branch:** `main` (planning only)

## Goal
Make the jobs-to-state boundary tell the truth about which submitted file operation each result belongs to, so batch settlement no longer depends on worker queue order or progress-path interpretation.

## Non-goals
- No operation UUID framework in this phase
- No UI redesign
- No broad worker-pool or concurrency refactor
- No full SFTP implementation change in this phase
- No speculative replacement of the existing `FileOperation` model

## Scope
This phase covers:
- canonical file-operation identity for local file-operation results
- identity-based batch settlement in `src/state/mod.rs`
- removal of queue-order dependence from current batch tracking
- tests that prove settlement does not depend on progress-path meaning
- API shape chosen so SFTP can adopt the same contract later

This phase does not cover:
- changing file-operation UX
- redesigning prompts or status bar rendering
- adding new operation kinds
- implementing identity propagation across the SFTP worker unless needed to keep the type design sound

## Problem Statement
The merged operation-safety work fixed a real bug where successful batch moves and batch archive extracts did not settle correctly. The root cause was not in prompt logic or mark cleanup itself; it was the contract between the jobs layer and the state layer.

Today:
- `src/jobs.rs` emits `FileOperationCompleted` without canonical operation identity
- `JobFailed` carries only a `path`
- `FileOperationProgress.current_path` is display-oriented and varies by operation kind
- `src/state/mod.rs` must reconstruct settlement truth heuristically

That heuristic recently had to fall back to queue order in `PendingBatchOperation.queued_sources`. This is acceptable as a tactical repair under the current single local worker, but it is the wrong long-term contract. The jobs layer knows which submitted operation it is executing; state should consume that truth directly.

## Design Invariants
1. File-operation results must identify the submitted operation canonically.
2. Batch settlement must not depend on `current_path` semantics.
3. Batch settlement must not depend on worker queue order.
4. Progress output remains informational only.
5. Archive-member extraction identity must preserve the member path, not collapse to archive root.
6. The result contract should be adoptable by SFTP later without redesign.

## Proposed Design

### 1. Canonical identity type
Add a small `FileOperationIdentity` type in `src/jobs.rs` and derive it once from the submitted `FileOperation`.

Proposed fields:
- `kind: FileOperationKind`
- `source: PathBuf`
- `destination: Option<PathBuf>`

Where:
- `kind` distinguishes copy, move, rename, trash, delete, create directory, create file, and extract archive
- `source` is the canonical settlement identity
- `destination` is present only when the operation meaningfully targets a destination path

Canonical `source` mapping:
- `Copy { source, .. }` -> original source path
- `Move { source, .. }` -> original source path
- `Rename { source, .. }` -> original source path
- `Trash { path }` -> path
- `Delete { path }` -> path
- `CreateDirectory { path }` -> path
- `CreateFile { path }` -> path
- `ExtractArchive { archive, inner_path, .. }` -> `archive.join(inner_path)`

This keeps archive-member identity aligned with how archive entries are already represented elsewhere in the app.

### 2. Result payload changes
Extend local file-operation result variants to carry identity:
- `FileOperationCompleted { identity, message, refreshed, elapsed_ms }`
- `FileOperationCollision { identity, operation, refresh, path, elapsed_ms }`
- `JobFailed { file_op: Option<FileOperationIdentity>, pane, path, message, elapsed_ms }`
- optionally `FileOperationProgress { identity, status }`

Rules:
- file-operation workers must populate `identity` / `file_op: Some(...)`
- unrelated worker failures must continue using `file_op: None`
- `status.current_path` remains display-oriented and is not upgraded into a settlement key

### 3. State-side batch settlement
Replace queue-order settlement in `src/state/mod.rs` with identity-based settlement.

`PendingBatchOperation` should track:
- the pane owning the marks
- submitted source identities
- failed source identities
- settled count
- total count

Settlement behavior:
- completed result with `identity` -> mark `identity.source` successful
- collision result with `identity` -> mark `identity.source` failed
- failed result with `file_op: Some(identity)` -> mark `identity.source` failed
- failed result with `file_op: None` -> do not mutate file-operation batch state

Marks cleanup stays the same at the user level:
- full success clears marks
- partial failure keeps only failed items marked
- full failure keeps all submitted items marked

### 4. Progress semantics
`FileOperationStatus.current_path` stays as-is conceptually: it exists for status display, not settlement truth.

That means:
- copy may continue showing source-side progress paths
- move may show source paths during cross-device copy and destination on final completion
- extract may show extracted output paths

The key design choice is that none of those display choices can affect settlement correctness anymore.

## File Map
**Modify**
- `src/jobs.rs` — add `FileOperationIdentity`, derive identity from `FileOperation`, thread identity through local file-op results
- `src/state/mod.rs` — remove queue-order dependence from batch settlement, consume result identity directly
- `src/jobs.rs` tests — verify identity derivation per operation kind
- `src/state/mod.rs` tests — verify batch settlement uses identity rather than progress-path meaning

**Maybe modify**
- `src/action.rs` — only if the operation kind enum belongs there for code-organization reasons; avoid unless clearly cleaner

**Do not modify in this phase unless required by a failing test**
- `src/pane.rs`
- `src/session.rs`
- prompt/UI modules
- SFTP execution logic

## Testing Strategy

### Unit tests in `src/jobs.rs`
Add tests for `FileOperationIdentity::from_file_operation()` (or equivalent helper):
- copy identity uses source + destination
- move identity uses source + destination
- rename identity uses source + destination
- trash/delete identity use target path and no destination
- create file/directory identity use created path
- extract identity preserves `archive.join(inner_path)` as source and destination as destination

### Unit tests in `src/state/mod.rs`
Add tests proving:
- batch move success settles correctly even when completion/progress paths are destination-oriented
- batch extract success settles correctly even when completion/progress paths are output-oriented
- mixed batch outcomes still retain only failed marks
- non-file-op `JobFailed { file_op: None, .. }` does not corrupt pending batch settlement

### Regression target
After this phase, a future maintainer should be able to change progress display behavior without risking mark cleanup correctness.

## Acceptance Criteria
- local file-operation completion, collision, and failure results carry canonical identity
- batch settlement in state uses canonical identity rather than queue order
- batch settlement in state does not interpret `current_path` to determine success/failure ownership
- archive-member extract results settle against the member identity, not just the archive path
- existing operation-safety behavior remains intact
- `cargo fmt --all -- --check` passes
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- `cargo test --workspace` passes

## Recommended Follow-on After This Phase
Once this lands, the next most natural follow-on is one of:
1. adopt the same result identity contract in SFTP file-operation results
2. add explicit operation IDs only if future concurrency or observability needs justify them

The important constraint is order: establish a truthful result contract first, then add heavier coordination machinery only if reality demands it.