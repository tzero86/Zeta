# Improvements Roadmap

- Status: Active
- Date: 2026-04-13
- Branch at time of writing: `optimize/binary-size-and-perf` (PR #1)

Each item is scoped, grounded in a specific gap in the current codebase, and ordered by impact-to-effort ratio.
Items within a tier are independent and can be worked in any order or in parallel.

---

## Tier 1 — High impact, low effort (ship first)

### T1-1: Open with default application

**Gap**: No `OpenInDefaultApp` action exists. Binary files, PDFs, and images have no useful affordance — preview shows noise, the editor opens raw bytes.

**Scope**:
- Add `Action::OpenInDefaultApp` to `src/action.rs`.
- Wire it to `F3` (or `Enter` when the entry is not a directory and not a text file) in `from_pane_key_event` in `src/action.rs`.
- Handle it in `AppState::apply` in `src/state/mod.rs`: spawn `std::process::Command` (or the `open` crate) with the path. Keep it non-blocking — spawn detached.
- Add a menu item under File menu in `src/state/menu.rs`.

**Dependency**: add `open = "5"` to `Cargo.toml`. Thin OS-API wrapper, no runtime, ~10 KB binary impact.

---

### T1-2: Range selection (Shift+arrow / Shift+click)

**Gap**: Marking is toggle-only. No contiguous range selection. Every file manager since 1990 has this.

**Scope**:
- Add `anchor: Option<usize>` to `PaneState` in `src/pane.rs`.
- Add `Action::ExtendSelectionUp` / `Action::ExtendSelectionDown` to `src/action.rs`.
- Wire `Shift+Up` / `Shift+Down` in `from_pane_key_event`.
- On activation: set `anchor` to current selection if not set; mark all entries between `anchor` and new selection; clear anchor on any non-shift move.
- `Shift+Click` (`PaneClick` with shift modifier): same anchor logic.
- No new data structures needed — `BTreeSet<PathBuf>` already handles the mark set.

---

### T1-3: Session persistence

**Gap**: Restart loses both panes' cwd, sort mode, scroll offset, pane layout, and hidden-files toggle. Only bookmarks survive.

**Scope**:
- Add `src/session.rs` with a `SessionState` struct: `left_cwd`, `right_cwd`, `left_sort`, `right_sort`, `left_hidden`, `right_hidden`, `layout: PaneLayout`.
- Derive `serde::Serialize` / `Deserialize`. Write to `session.toml` alongside `config.toml` on clean exit (in `main.rs` after the event loop).
- Read and apply on startup before the first `ScanPane` commands are enqueued, in `AppState::bootstrap` in `src/state/mod.rs`.
- Treat a missing or malformed session file as a no-op (first run).

---

### T1-4: Syntax highlighting in the editor

**Gap**: `highlight_text()` is used for file preview but the editor render path only holds `Vec<String>` (plain text). The highlight infrastructure is already in `src/highlight.rs`.

**Scope**:
- Change `EditorRenderState::visible_lines` in `src/editor.rs` from `Vec<String>` to `Vec<HighlightedLine>` (already defined in `src/highlight.rs`).
- In `EditorBuffer::render_state()`, call `highlight_text()` on the visible slice rather than returning raw strings.
- Update `src/ui/editor.rs` to render `HighlightedLine` spans instead of plain text — same pattern as `src/ui/preview.rs`.
- No new crate dependency; highlight worker and token types are already in the binary.

---

### T1-5: System clipboard integration

**Gap**: No `CopyPathToClipboard` in the pane; no OS clipboard paste in the editor. The editor has internal cut/copy but no system clipboard connection.

**Scope**:
- Add `arboard` to `Cargo.toml`. Thin OS-API wrapper.
- Add `Action::CopyPathToClipboard` to `src/action.rs`. Wire to `Ctrl+Shift+C` in pane context.
- Handle in `AppState::apply`: `arboard::Clipboard::new()?.set_text(path.to_string_lossy())`.
- In the editor, wire `Ctrl+V` to read from `arboard::Clipboard` and insert at cursor. Wire `Ctrl+C` (with selection, once selection is added) to write to clipboard.
- Clipboard errors are non-fatal — show a status bar message, do not crash.

---

## Tier 2 — High impact, medium effort

### T2-1: Batch operations on marked files

**Gap**: `FileOperation` operates on a single `PathBuf`. When files are marked, copy/move/delete acts on the current selection only, not the full mark set. The mark set (`BTreeSet<PathBuf>`) is tracked but never drained into multiple jobs.

**Scope**:
- In `AppState::apply` in `src/state/mod.rs`, for `OpenCopyPrompt` / `OpenMovePrompt` / `OpenDeletePrompt`: if `active_pane().marked` is non-empty, use the mark set as the source list; otherwise use the single selected entry (current behavior).
- For copy/move: enqueue one `RunFileOperation` per marked entry into the command queue. Progress reporting already aggregates by job ID — verify it sums across the batch correctly.
- For delete: same — one job per marked entry, or extend `FileOperation::Delete` to accept `Vec<PathBuf>` if the overhead of N round-trips is measurable.
- After any batch job completes, clear marks on the source pane.
- Add integration tests in `tests/` for batch copy and batch delete with a temp directory tree.

---

### T2-2: Directory size background calculation

**Gap**: `SortMode::Size` is useless for directory trees — all dirs show as 0 bytes. `EntryInfo.size` is `Option<u64>` and is `None` for directories.

**Scope**:
- Add `DirSizeRequest { pane: PaneId, path: PathBuf }` and `DirSizeResult { pane: PaneId, path: PathBuf, bytes: u64 }` to `src/jobs.rs`.
- Add a `dir_size_worker` that receives `DirSizeRequest`, walks the tree with `walkdir` (already a transitive dep, verify) or `std::fs::read_dir` recursively, sums sizes, and sends back `DirSizeResult`.
- In `PaneState`, add `dir_sizes: HashMap<PathBuf, u64>` in `src/pane.rs`.
- When a scan result arrives and the sort mode is `Size` or `SizeDesc`, enqueue `DirSizeRequest` for each directory entry in the result.
- When a `DirSizeResult` arrives, update `dir_sizes` and trigger a re-render.
- The size worker must be cancelable — send a new generation token with each request and drop stale results (same pattern as the preview worker).

---

### T2-3: Column / details view toggle

**Gap**: The pane renders name + icon only. `EntryInfo` carries `size`, `modified`, and `kind` but they are not displayed.

**Scope**:
- Add `details_view: bool` to `PaneState` in `src/pane.rs`.
- Add `Action::ToggleDetailsView` to `src/action.rs`. Wire to a key (e.g., `Ctrl+D` or `F1`).
- In `src/ui/` pane rendering: when `details_view` is true, render a fixed-width column layout — `[icon] name … size  date`. Measure available width from `Rect` and truncate the name column to fill the remainder. Size column: right-align, human-readable (B/KB/MB/GB). Date column: `YYYY-MM-DD HH:MM`.
- When `details_view` is false, render as today (name + icon only).

---

## Tier 3 — Lower priority / larger scope

### T3-1: Config hot-reload

**Gap**: The `notify` watcher worker already exists for directory change detection but does not watch the config file.

**Scope**:
- In `src/jobs.rs`, after the watcher is set up, also watch the config file path.
- On a `DebouncedEvent` for the config path, emit an `AppEvent::ConfigChanged` from the watcher channel.
- In `src/app.rs`, handle `AppEvent::ConfigChanged`: re-read config with `Config::load()`, diff the relevant fields (theme, keymap), and dispatch `SetTheme` / keymap update actions.

---

### T3-2: Editor tab stops and word wrap

**Gap**: `\t` is passed through raw; display width is terminal-dependent. No word wrap for long lines.

**Scope**:
- Add `tab_width: u8` (default 4) to `EditorConfig` in `src/config.rs`.
- In the editor render path, expand `\t` to `tab_width` spaces before computing column offsets.
- Add `word_wrap: bool` to `EditorConfig`. When enabled, split rendered lines at the viewport width and adjust cursor-to-rendered-row mapping accordingly. Wrap is display-only — the underlying rope is unchanged.

---

### T3-3: Inline rename

**Gap**: Rename opens a full prompt dialog. Norton Commander-style inline rename edits the filename in the pane row.

**Scope**:
- Add a new `FocusLayer` variant or a `PaneState` sub-mode for inline rename: `rename_state: Option<InlineRenameState>` where `InlineRenameState` holds the current edit buffer and the path being renamed.
- Render the selected pane row with the edit buffer in place of the filename, with a cursor.
- On `Enter`: submit the rename. On `Esc`: cancel. Route character input to the inline buffer when the mode is active.
- This is the most rendering-invasive of the tier-3 items.

---

## Implementation notes

- Work each item on its own branch off `main`.
- Each branch must pass `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` before PR.
- Add unit tests with each change; add integration tests for any filesystem-touching feature.
- Do not bundle multiple tier items in one PR unless they share a non-trivial prerequisite.
- T1 items are fully independent. T2-1 (batch ops) should land before T2-2 and T2-3 since it changes how marks are consumed.
