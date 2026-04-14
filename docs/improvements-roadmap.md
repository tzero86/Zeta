# Improvements Roadmap

- Status: Active
- Date: 2026-04-14
- Branch at time of writing: `main`

Each item is scoped, grounded in a specific gap in the current codebase, and ordered by impact-to-effort ratio.
Items within a tier are independent and can be worked in any order or in parallel.

---

## Shipped — all roadmap items

Every item from all previous roadmap versions is complete.

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
| Editor text selection + clipboard | `sel_anchor: Option<usize>`; `EditorSelectAll` (Ctrl+A), `EditorCopy` (Ctrl+C), `EditorCut` (Ctrl+X); `SelectionHighlight` rendering; `text_sel_bg` palette field |
| SSH known-hosts trust prompt | `JobResult::SshHostUnknown/SshConnected`; `ModalState::SshTrustPrompt`; `Action::SshTrustAccept/Reject`; `Command::ConnectSSH` fully wired through SFTP worker; `render_ssh_trust_prompt` UI; trust bypasses check for this session (key persistence noted as future work) |

Also shipped beyond the original scope: multi-workspace support with session persistence per workspace, SSH/SFTP remote filesystem browsing, configurable hotkeys in the settings panel, markdown preview in the editor, file finder, bookmarks.

---

## Remaining — follow-on polish items

### P1: Persist trusted SSH host keys to known_hosts

**Gap**: When the user accepts an unknown host in the trust prompt (`trust_unknown_host=true`), the connection proceeds but the host key is NOT written to `~/.ssh/known_hosts`. Every subsequent connection to the same host will show the trust prompt again.

**Scope**:
- After authentication succeeds in `connect_sftp` (when `trust_unknown_host=true`), obtain the raw host key via `session.host_key()`.
- Encode the key bytes as base64 (standard alphabet) and format an OpenSSH known_hosts entry: `host:port key_type base64_key`.
- Append to `~/.ssh/known_hosts` (create file if absent; create `~/.ssh/` directory if needed).
- Alternatively, use `known_hosts.add()` + `known_hosts.write_file()` from the `ssh2` crate if the API permits encoding from raw bytes.

### P2: Editor text selection with Shift+arrow extension

**Gap**: `EditorSelectAll` (Ctrl+A) is the only way to create a selection. Shift+arrow does not extend the selection — `sel_anchor` is cleared on every movement. Partial selection (e.g., selecting a word or line range) is not possible.

**Scope**:
- Add `EditorSelectLeft`, `EditorSelectRight`, `EditorSelectUp`, `EditorSelectDown` actions.
- Wire Shift+Left/Right/Up/Down in `from_editor_key_event`.
- In `EditorBuffer`: if `sel_anchor` is None when a Shift+arrow fires, set it to the current cursor position before moving. If sel_anchor is already set, just move the cursor (extending the selection). Do NOT clear sel_anchor in the Shift+arrow movement path.
- `visible_selection_display_ranges` already handles partial selection correctly for non-wrap mode.

### P3: Search highlight correctness in word-wrap mode (low priority)

**Gap**: `visible_search_matches` is called with `render_state.visible_lines` (from `visible_line_window_h`). Both paths use the same `logical_start` and wrap at the same `viewport_cols`, so the visual rows are consistent. However, if the two paths ever diverge (e.g., due to a rounding difference in the visible-window calculation), search highlights in word-wrap mode would map to the wrong rows.

**Scope** (defensive fix, not urgent):
- Extract the wrapped visual row strings from the highlighted path as `Vec<String>` and pass them directly to `visible_search_matches` when `word_wrap=true`.
- Eliminates the theoretical divergence without changing correctness in the common case.

---

## Implementation notes

- Work each item on its own branch off `main`.
- Each branch must pass `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` before PR.
- Add unit tests with each change; add integration tests for any filesystem-touching feature.
