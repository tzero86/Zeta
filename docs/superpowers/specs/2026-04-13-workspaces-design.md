# Workspaces Design

**Date:** 2026-04-13
**Branch:** `main` (planning only)

## Goal
Add Linux-desktop-style workspaces to Zeta so the user can keep multiple independent dual-pane working contexts inside one app and switch between them instantly without mutating the others.

## Non-goals
- No plugin or remote-filesystem redesign
- No arbitrary/unbounded workspace counts in v1
- No deep restart restoration of editor buffer contents or live terminal sessions in v1
- No hidden multi-process architecture
- No UI redesign beyond the minimum needed to expose and indicate workspaces

## Scope
This phase covers:
- fixed 4 workspaces
- full runtime isolation for each workspace’s pane, preview, editor, terminal, and local transient state
- workspace-aware async job routing
- lightweight per-workspace session persistence
- keybindings and visual indication for switching workspaces

This phase does not cover:
- user-defined workspace counts
- named workspaces in v1 unless they fall out nearly free
- persisting dirty editor buffers across full app restart
- restoring live terminal/PTY sessions across full app restart

## Problem Statement
Zeta currently has one working context: one `PaneSetState`, one `PreviewState`, one `EditorState`, one `TerminalState`, and one persisted left/right session pair. That model works for a single task, but it breaks down when the user is actively working across multiple repositories or folder sets.

Dual-pane navigation reduces friction within one task, but it does not solve context switching between several concurrent tasks. The user needs multiple independent dual-pane desktops inside the same app, so switching workspaces feels like switching desktops, not repurposing the same pane pair.

## Design Invariants
1. Switching workspaces must be instant.
2. Each workspace must preserve its own pane, preview, editor, terminal, and local transient state.
3. Async job results must only update the workspace that launched them.
4. One workspace’s edits, preview focus, terminal focus, or job progress must not leak into another workspace.
5. Session persistence must restore lightweight per-workspace state safely.
6. The implementation must fit the current modular monolith and not introduce a hidden multi-app architecture.

## Proposed Architecture

### 1. Global shell state vs workspace state
Split current state into:

#### Global app shell state
Owned at `AppState` level:
- config path/config values
- theme/icon mode
- keymap-driven shared behavior where appropriate
- global overlays that are truly app-wide
- active workspace index
- fixed list of workspaces
- worker handles/channels remain global in the app layer

#### Per-workspace state
Introduce a `WorkspaceState` that owns what the user experiences as one desktop:
- `PaneSetState`
- `PreviewState`
- `EditorState`
- `TerminalState`
- local transient/runtime state currently stored flat in `AppState`, including at minimum:
  - pending reveal
  - pending batch
  - file operation progress
  - diff mode / diff map
  - local status text and scan timing if we want status to reflect the active workspace truthfully

This keeps the app shell global while making the working context truly per-workspace.

### 2. Fixed v1 workspace count
Use a fixed array/list of 4 workspaces in v1.

Rationale:
- matches the user’s immediate goal
- simplifies persistence and keybindings
- avoids speculative workspace management UX before real usage data

## Switching Semantics

### 1. Runtime behavior
Switching workspaces should:
- swap the visible workspace immediately
- preserve the previous workspace exactly as it was left
- not close editor, preview, or terminal just because the workspace is no longer active
- not reset selection, marks, filters, sort mode, or pane directories

### 2. Editor/preview/terminal isolation
Each workspace keeps independent runtime state for:
- preview target and scroll
- editor buffer, cursor, dirty state, search/replace state, markdown preview state
- terminal open/focused state and parser/session state

This means the user can, for example:
- edit a file in workspace 1
- browse another repo in workspace 2
- keep a terminal open in workspace 3
- switch back without any of those contexts overwriting each other

### 3. Keybindings
For v1, use direct workspace switching via fixed bindings such as:
- `Alt+1`
- `Alt+2`
- `Alt+3`
- `Alt+4`

Cycling actions can come later if desired, but direct jumps are the clearest first version.

## Async Jobs And Result Routing
This is the most important architectural change.

Current model assumes one working context. With workspaces, every async command/result that affects workspace-local state must carry workspace identity.

### 1. Commands
Commands launched from a workspace should carry `workspace_id` when they can yield async results that later mutate state, such as:
- scan requests
- preview loads
- editor loads/saves where needed
- file operations
- terminal-related async output/state
- finder/search requests if they are workspace-local

### 2. Job results
Job results returned from workers should also carry `workspace_id` for workspace-scoped work.

Reducer/app handling rule:
- apply each result only to the matching `WorkspaceState`
- do this even if that workspace is not currently active

This ensures that:
- background copy in workspace 2 continues while user is viewing workspace 1
- completion/failure/progress updates mutate workspace 2 only
- switching back to workspace 2 reveals the current truthful state immediately

## Persistence

### 1. v1 persistence target
Persist lightweight session state per workspace:
- left/right cwd
- sort mode
- hidden-files flag
- layout
- active workspace index

### 2. Explicit v1 non-goal
Do not persist across full app restart:
- full dirty editor buffer contents
- live PTY/terminal session state
- rich preview buffers

Reason:
- runtime workspace isolation is the core value
- full restart restoration of live editors/terminals is a distinct complexity tier and should not be bundled into v1

## UI/UX

### 1. Visual indication
Add a lightweight workspace indicator showing:
- active workspace
- optionally busy/dirty indicators later if low-cost

Good candidates are the menu bar, status bar, or hint bar; choose the one with the least layout churn.

### 2. Interaction model
The user should think of a workspace as “one complete dual-pane desktop.”
That means switching workspaces changes the whole working context, not just folder paths.

## File Map
**Modify**
- `src/state/mod.rs` — split global vs workspace-local state, add active workspace handling
- `src/state/pane_set.rs` — likely reused inside workspace state with minimal change
- `src/state/editor_state.rs` — moved under workspace ownership, behavior mostly reused
- `src/state/preview_state.rs` — moved under workspace ownership, behavior mostly reused
- `src/state/terminal.rs` — moved under workspace ownership, behavior mostly reused
- `src/action.rs` — add workspace switch actions/bindings
- `src/app.rs` — route commands/results with workspace identity
- `src/jobs.rs` — add workspace identity to async request/result types where needed
- `src/session.rs` — persist multiple workspaces plus active workspace
- `src/ui/mod.rs` and possibly menu/status rendering files — add workspace indicator

**Maybe modify**
- `src/config.rs` — only if workspace count or bindings need explicit config surface in v1

## Testing Strategy

### Unit/state tests
Add tests proving:
- switching workspace preserves independent pane cwd/selection/marks
- preview/editor/terminal state remains isolated across switches
- file operation progress in workspace A does not update workspace B
- scan/preview/editor/job results tagged with workspace A only mutate workspace A
- active workspace index persists and restores

### Integration/regression tests
Add focused regression coverage for:
- switching while a file operation is running in another workspace
- switching with a dirty editor in one workspace and no editor in another
- switching with preview focus in one workspace
- switching while a terminal is open in one workspace and closed in another

## Acceptance Criteria
- User can switch among 4 workspaces instantly.
- Each workspace preserves independent pane, preview, editor, terminal, and local transient runtime state.
- Workspace-local async jobs continue and settle correctly even when that workspace is not active.
- Lightweight per-workspace session state restores on restart.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` pass.

## Recommended v1 Slice
Implement in this order:
1. introduce `WorkspaceState`
2. move current single-context state under workspace ownership
3. add active workspace switching actions/keybindings
4. make async requests/results workspace-aware
5. add lightweight persistence
6. add minimal workspace indicator

That sequence preserves correctness first, then UX exposure.