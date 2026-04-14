# Improvements Roadmap

- Status: Active
- Date: 2026-04-14
- Branch at time of writing: `main` (post-PR #5 workspaces, post settings-panel expansion)

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
| Syntax highlighting in editor | `highlight_cache` + `editor_highlighted_render_state`; falls back to plain in word-wrap mode |
| System clipboard | `arboard`; `CopyPathToClipboard` (Ctrl+Shift+C); `EditorPaste` (Ctrl+V) |
| Batch file operations | Mark set drained into per-entry jobs; `pending_batch` tracks settlement |
| Directory size calculation | `DirSizeCalculated` job; `dir_sizes` cache in `PaneState` |
| Column / details view | `details_view` toggle (Ctrl+D); size + modified columns |
| Config hot-reload | Watcher emits `ConfigChanged`; app recompiles keymap and reloads palette live |
| Editor tab stops + word wrap | `tab_width` / `word_wrap` in `AppConfig.editor`; settings panel wires both |
| Inline rename | `InlineRenameState`; `FocusLayer::PaneInlineRename`; F2 to enter |

Also shipped beyond the original scope: multi-workspace support with session persistence per workspace, SSH/SFTP remote filesystem browsing, configurable hotkeys in the settings panel, markdown preview in the editor, file finder, bookmarks.

---

## Tier 1 — High impact, low effort

### T1-1: Wire editor undo / redo

**Gap**: `EditorBuffer` has a complete delta-based `UndoStack` with `undo()` and `redo()` methods (see `src/editor.rs` line ~222). No `EditorUndo` or `EditorRedo` action exists. Ctrl+Z does nothing in the editor.

**Scope**:
- Add `Action::EditorUndo` and `Action::EditorRedo` to `src/action.rs`.
- Wire `Ctrl+Z` → `EditorUndo` and `Ctrl+Y` / `Ctrl+Shift+Z` → `EditorRedo` in `from_editor_key_event`.
- Handle both in `EditorState::apply` in `src/state/editor_state.rs`: call `editor.undo()` / `editor.redo()`.
- No new crate dependency; no data structure changes.

---

### T1-2: Editor text selection and clipboard copy

**Gap**: `EditorPaste` (Ctrl+V) is wired. `CopyPathToClipboard` copies the focused pane entry's path. But there is no selection model in `EditorBuffer`, so `Ctrl+C` in the editor has no text to copy. The roadmap note "Ctrl+C with selection, once selection is added" was never resolved.

**Scope**:
- Add `sel_anchor: Option<usize>` (byte offset) to `EditorBuffer` in `src/editor.rs`.
- Add `Action::EditorSelectAll`, `Action::EditorCopy`, `Action::EditorCut` to `src/action.rs`.
- Wire `Ctrl+A` → `EditorSelectAll`, `Ctrl+C` → `EditorCopy`, `Ctrl+X` → `EditorCut` in `from_editor_key_event`. Selection extension via Shift+arrow is a follow-on; start with select-all.
- `EditorCopy`: write selected text to `arboard::Clipboard`. `EditorCut`: same then delete selection. `EditorSelectAll`: set `sel_anchor = Some(0)`, cursor to end.
- Render the selection highlight in `src/ui/editor.rs` — a background span over the selected byte range on each visible line.
- The existing `insert_str_at_cursor` and `delete` primitives cover the cut case without new buffer methods.

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

### T2-2: Word wrap + syntax highlighting

**Gap**: `use_plain_wrapped = cheap_mode || render_state.word_wrap` in `src/ui/editor.rs`. When word wrap is on, the highlighted render path is bypassed; the editor renders unstyled text.

**Scope**:
- `EditorBuffer::visible_highlighted_window` already computes the highlight cache over the full file. Extend it (or add a sibling) to accept `word_wrap: bool` and `viewport_cols: usize`; after collecting the highlighted `HighlightedLine` slice, split any line whose rendered char width exceeds `viewport_cols` into continuation rows, preserving span boundaries.
- Update `render_editor` in `src/ui/editor.rs` to call the highlighted path even when `word_wrap` is true, passing `viewport_cols` from the content area width.
- The cursor row mapping in `EditorRenderState` must account for wrapped rows — `cursor_wrap_row` already tracks this for the plain path; apply the same offset logic to the highlighted path.
- Remove the `use_plain_wrapped` early-exit; `cheap_mode` can keep bypassing highlights (it's for low-resource terminals).

---

## Tier 3 — Lower priority / larger scope

### T3-1: Editor search highlight

**Gap**: Search (`Ctrl+F`) finds the next match and moves the cursor, but does not highlight all matches in the visible viewport. The match at the cursor is not visually distinct from surrounding text.

**Scope**:
- Add `search_matches: Vec<(usize, usize)>` (byte ranges) to `EditorRenderState` — computed from the current search query over the visible slice.
- In `src/ui/editor.rs`, overlay match ranges onto rendered spans: split `HighlightedLine` spans at match boundaries and apply a `search_match_bg` color from `ThemePalette`.
- The active match (cursor position) gets a distinct `search_match_active_bg`.
- No new crate dependency; uses the existing `search_query` field already in `EditorBuffer`.

---

### T3-2: Copy marked paths to clipboard

**Gap**: `CopyPathToClipboard` copies only the single focused entry. When multiple files are marked, there is no way to get their paths into the clipboard (e.g., for use in a terminal).

**Scope**:
- Extend `Action::CopyPathToClipboard` handling in `src/state/mod.rs`: if `active_pane().marked` is non-empty, join all marked paths newline-separated and write that string to the clipboard; otherwise copy the focused entry (current behavior).
- No new action, no new crate dependency.

---

## Implementation notes

- Work each item on its own branch off `main`.
- Each branch must pass `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` before PR.
- Add unit tests with each change; add integration tests for any filesystem-touching feature.
- Do not bundle multiple tier items in one PR unless they share a non-trivial prerequisite.
- T1-1 (undo/redo) and T1-2 (selection) are independent but T1-2 is easier to review after T1-1 since both touch `from_editor_key_event`.
