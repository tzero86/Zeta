# Improvements Roadmap

- Status: Active
- Date: 2026-04-14
- Branch at time of writing: `main`

---

## Shipped — all items complete

| Item | Notes |
|---|---|
| Open with default application | `OpenInDefaultApp`, `open` crate, F3 in pane |
| Range selection (Shift+arrow) in pane | `mark_anchor` in `PaneState`; `ExtendSelectionUp/Down` |
| Session persistence | `src/session.rs`; save/restore cwd, sort, hidden, layout |
| Syntax highlighting in editor | `highlight_cache`; word-wrap path also highlighted |
| System clipboard | `arboard`; `CopyPathToClipboard` (multi-path when marks set); `EditorPaste` |
| Batch file operations | Mark set drained into per-entry jobs; `pending_batch` tracks settlement |
| Directory size calculation | `DirSizeCalculated` job; lazy per-entry size in details view |
| Column / details view | `details_view` toggle (Ctrl+D); size + modified columns |
| Config hot-reload | Watcher emits `ConfigChanged`; keymap + palette reloaded live |
| Editor tab stops + word wrap | `tab_width` / `word_wrap` in `AppConfig.editor`; settings panel |
| Inline rename | `InlineRenameState`; `FocusLayer::PaneInlineRename`; F2 / r |
| Editor undo / redo | `Action::EditorUndo/Redo`; Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z |
| Word wrap + syntax highlighting | `wrap_highlighted_line` splits spans at viewport cols; separate wrap cache |
| Editor search highlight | `visible_search_matches`; `SearchHighlight` per-char bg in `build_row_spans` |
| Copy marked paths to clipboard | `CopyPathToClipboard` joins marked paths with newlines |
| Status bar deduplication | Removed "workspace N active" toast; `ws:N/M` indicator sufficient |
| Editor text selection + clipboard | `sel_anchor`; `EditorSelectAll/Copy/Cut`; `SelectionHighlight` rendering; `text_sel_bg` |
| Editor Shift+arrow selection | `extend_left/right/up/down`; Shift+arrow wired; typing replaces selection |
| SSH known-hosts trust prompt | `JobResult::SshHostUnknown/SshConnected`; `ModalState::SshTrustPrompt`; trust prompt UI; `Command::ConnectSSH` wired through SFTP worker |
| SSH host key persistence | `persist_host_key` writes OpenSSH entry to `~/.ssh/known_hosts` after user accepts; inline base64 encoder; creates `~/.ssh/` with mode 0700 |

Also shipped beyond original scope: multi-workspace, SSH/SFTP browsing, configurable hotkeys via settings panel, markdown preview, file finder, bookmarks.

---

## Open — one low-priority defensive item

### P3: Search highlight row alignment in word-wrap mode

**Status**: Likely not a real bug. Both `visible_line_window_h` and `visible_highlighted_window` use the same `logical_start` and wrap at the same `viewport_cols`, so `render_state.visible_lines` and the highlighted visual rows should be identical in content and order.

**If it ever manifests**: extract the text of each `HighlightedLine` from the wrap cache into a `Vec<String>` inside `editor_highlighted_render_state` and pass those strings to `visible_search_matches` when `word_wrap=true`, bypassing `render_state.visible_lines`. This eliminates any theoretical divergence.

---

## Implementation notes

- Branch off `main` per feature; pass `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` before PR.
- Add unit tests for any pure logic; integration tests for filesystem-touching features.
