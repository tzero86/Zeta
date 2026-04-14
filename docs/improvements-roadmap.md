# Improvements Roadmap

- Status: Active
- Date: 2026-04-14
- Branch at time of writing: `main`

Each item is scoped, grounded in a specific gap in the current codebase, and ordered by impact-to-effort ratio.
Items within a tier are independent and can be worked in any order or in parallel.

---

## Shipped — original T1–T3 items

All items from the previous roadmap version are complete.

| Item | Notes |
|---|---|
| Open with default application | `OpenInDefaultApp`, `open` crate, F3 in pane |
| Range selection (Shift+arrow) | `mark_anchor` in `PaneState`; `ExtendSelectionUp/Down` |
| Session persistence | `src/session.rs`; save on exit; restore cwd, sort, hidden flag, layout |
| Syntax highlighting in editor | `highlight_cache` + `editor_highlighted_render_state`; word-wrap path now also highlighted |
| System clipboard | `arboard`; `CopyPathToClipboard` (Ctrl+Shift+C, multi-path when marks set); `EditorPaste` (Ctrl+V) |
| Batch file operations | Mark set drained into per-entry jobs; `pending_batch` tracks settlement |
| Directory size calculation | `DirSizeCalculated` job; `dir_sizes` cache in `PaneState` |
| Column / details view | `details_view` toggle (Ctrl+D); size + modified columns |
| Config hot-reload | Watcher emits `ConfigChanged`; app recompiles keymap and reloads palette live |
| Editor tab stops + word wrap | `tab_width` / `word_wrap` in `AppConfig.editor`; settings panel wires both |
| Inline rename | `InlineRenameState`; `FocusLayer::PaneInlineRename`; F2 to enter |
| Editor undo / redo | `Action::EditorUndo/Redo`; Ctrl+Z / Ctrl+Y; backed by `UndoStack` in `EditorBuffer` |
| Word wrap + syntax highlighting | `wrap_highlighted_line` splits highlighted spans at `viewport_cols`; wrap cache keyed by `(version, theme, tab_width, cols)` |
| Editor search highlight | `visible_search_matches` computes per-row match ranges; `SearchHighlight` overlay in `render_code_view`; active match uses distinct amber bg |
| Copy marked paths to clipboard | `CopyPathToClipboard` joins all marked paths with newlines when marks are set |
| Status bar deduplication | Removed "workspace N active" toast; `ws:N/M` field already communicates this |

Also shipped beyond the original scope: multi-workspace support with session persistence per workspace, SSH/SFTP remote filesystem browsing, configurable hotkeys in the settings panel, markdown preview in the editor, file finder, bookmarks.

---

## Tier 1 — High impact, low effort

### T1-1: Editor text selection and clipboard copy

**Gap**: `EditorPaste` (Ctrl+V) is wired. But there is no selection model in `EditorBuffer`, so `Ctrl+C` in the editor has no text to copy. The roadmap note "Ctrl+C with selection, once selection is added" was never resolved.

**Scope**:
- Add `sel_anchor: Option<usize>` (char index) to `EditorBuffer` in `src/editor.rs`.
- Add `selected_text() -> Option<String>` and `delete_selection()` methods.
- Add `Action::EditorSelectAll`, `Action::EditorCopy`, `Action::EditorCut` to `src/action.rs`.
- Wire `Ctrl+A` → `EditorSelectAll`, `Ctrl+C` → `EditorCopy`, `Ctrl+X` → `EditorCut` in `from_editor_key_event`. Selection extension via Shift+arrow is a follow-on; start with select-all.
- `EditorCopy`: write selected text to `arboard::Clipboard`. `EditorCut`: copy then delete selection. `EditorSelectAll`: set `sel_anchor = Some(0)`, cursor to end.
- Render the selection highlight in `src/ui/editor.rs` — a background span over the selected char range, using `palette.selection_bg`. Reuse the `build_row_spans` infrastructure in `code_view.rs`.

---

## Tier 2 — High impact, medium effort

### T2-1: SSH known-hosts trust prompt

**Gap**: `Command::ConnectSSH` hits `ssh2::KnownHosts::check()`. On `CheckResult::NotFound` the connection is rejected with an error string and the user has no way to accept the host. Connecting to any new server always fails.

**Scope**:
- Add a `ModalState::SshTrustPrompt { host: String, fingerprint: String, pane: PaneId, pending: ConnectSSHArgs }` variant to `src/state/overlay.rs`.
- When the jobs layer returns `JobResult::SshUnknownHost { host, fingerprint, pane, pending }`, open the trust prompt instead of failing.
- User can Accept (writes the host to `~/.ssh/known_hosts` via `ssh2::KnownHosts::writefile`) or Reject (closes modal, shows error).
- Add `Action::SshTrustAccept` and `Action::SshTrustReject`; wire Enter/Esc in the modal.
- Until this is done SSH is only usable for hosts already in `known_hosts`.

---

## Tier 3 — Lower priority / larger scope

### T3-1: Search highlight in word-wrap mode

**Gap**: `visible_search_matches` queries `render_state.visible_lines` which is populated by the plain `visible_line_window_h` path. In word-wrap mode the highlighted path uses `visible_highlighted_window` which returns different visual rows (wrap-expanded) not present in `visible_lines`. The search highlight data computed from `visible_lines` therefore maps to the wrong row indices when word wrap is on.

**Scope**:
- Expose the wrap-expanded row strings from the highlighted path (e.g., collect the text of each `HighlightedLine` from the wrap cache into a `Vec<String>` in `editor_highlighted_render_state`).
- Pass these to `visible_search_matches` instead of `render_state.visible_lines` when word wrap is active.
- Alternatively, teach `visible_search_matches` to accept the highlighted rows directly.

---

## Implementation notes

- Work each item on its own branch off `main`.
- Each branch must pass `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` before PR.
- Add unit tests with each change; add integration tests for any filesystem-touching feature.
- Do not bundle multiple tier items in one PR unless they share a non-trivial prerequisite.
