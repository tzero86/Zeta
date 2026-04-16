# 🚀 Enhancements Roadmap: Zeta TUI File Manager (Technical Deep Dive)

This document provides implementation-level detail for the enhancements described in `enhancements.md`. It maps each enhancement to the specific modules, types, and functions in the current codebase that are involved.

> **NOTE TO DEVELOPERS:** Module paths and symbol names are verified against the actual source tree. Do **not** rely on approximate line numbers — use `cargo`'s symbol search or your editor's "Go to definition" instead.

---

## 🧠 II. Architectural & Performance Improvements — **P1/P2 Priority**

### ⚡ Metadata Caching Layer (P1)

*   **Goal:** Avoid re-scanning a directory whose contents have not changed since the last scan.
*   **Mechanism:** Store the scan result alongside the directory's `mtime` at scan time. Before re-scanning, compare the directory's current `mtime` to the cached value. Re-scan only on mismatch.
*   **Key Symbols & Locations:**
    *   **Cache struct:** Add to `src/pane.rs`, adjacent to the existing `cache_filter_active: Cell<bool>` and `cache_filter_query: RefCell<String>` fields in `Pane`. A lightweight tuple `(PathBuf, SystemTime, Vec<EntryInfo>)` or a named `ScanCache` struct is sufficient.
    *   **Scan entry point:** `src/fs/local.rs` → `LocalBackend::scan_directory()` and the free function `src/fs.rs` → `scan_directory()`. Wrap the call site in `pane.rs` — do not add cache logic inside the `fs` layer itself.
    *   **`mtime` source:** `EntryInfo` in `src/fs.rs` already has `modified: Option<SystemTime>`. Use `std::fs::metadata(path).modified()` at the call site to get the directory's own `mtime`.
*   **Does not touch:** `state/pane_set.rs` (manages workspace/pane switching, unrelated to scan results).

---

### 🛡️ Comprehensive Error Context Propagation (P2)

*   **Goal:** Replace bare `FileSystemError` propagation with caller-supplied context at every subsystem boundary.
*   **Existing foundation:** `FileSystemError` is already defined with `thiserror` in `src/fs.rs`. All I/O operations return `Result<_, FileSystemError>`.
*   **Key Symbols & Locations:**
    *   **New top-level error type:** Define `ZetaError` in `src/state/types.rs` (which already holds shared types like `PaneFocus`, `PaneLayout`, `ModalKind`). Use `thiserror` for consistency.
    *   **Propagation points:** `src/jobs.rs` (background operations), `src/fs/local.rs` and `src/fs/sftp.rs` (I/O calls), `src/app.rs` (top-level boundary).
    *   **Pattern:** Use `.map_err(|e| ZetaError::from_context(e, format!("...{path}...")))` at each boundary. Avoid `.unwrap()` and `.expect()` in all production paths.

---

## 🔄 Scan Diffing — **P5 Priority** *(do not implement before caching baseline)*

*   **Goal:** When a refresh is triggered on an already-loaded directory, apply only the delta (`Added`, `Removed`, `Modified`) rather than replacing the full entry list.
*   **Prerequisite:** The metadata caching layer (P1) must be in place first, as it is the prerequisite for having a "previous state" to diff against.
*   **Key Symbols & Locations:**
    *   **New module:** `src/fs/scan_diff.rs`. Define a `ScanDiff` struct with `added: Vec<EntryInfo>`, `removed: Vec<EntryInfo>`, `modified: Vec<EntryInfo>`. Expose a `compute_scan_diff(old: &[EntryInfo], new: &[EntryInfo]) -> ScanDiff` function.
    *   **Do NOT extend `src/diff.rs`:** That module implements side-by-side pane content comparison (`DiffStatus`, `compute_diff()`) — a distinct, unrelated feature. Mixing scan-diff logic there would conflate two separate concerns.
    *   **Application point:** `src/pane.rs` → the scan result ingestion path. Instead of `self.entries = new_entries`, apply the `ScanDiff` incrementally.
    *   **State reducer:** `src/state/mod.rs` would pass the diff rather than a full list only once the pane-level API is updated.
*   **Requires a benchmark:** Measure scan + rebuild time for large directories (1000+ entries) before investing here.

---

## 🚀 I. Feature Depth — **P3/P4 Priority**

### 📂 Advanced Filtering and Globbing (P4)

*   **Goal:** Upgrade the existing substring filter to support shell-style glob patterns.
*   **Existing foundation:** `pane.rs` already has `filter_query: String`, `filter_active: bool`, and filtering logic inside `rebuild_cache()`.
*   **Key Symbols & Locations:**
    *   **Glob utility:** Add `src/utils/glob_match.rs` with a `fn matches_glob(pattern: &str, name: &str) -> bool` function. Consider the `glob` crate (lightweight, well-maintained) or implement basic `*`/`?` matching manually to avoid a dependency.
    *   **Integration point:** Replace the substring check in `Pane::rebuild_cache()` in `src/pane.rs` with a call to `matches_glob()`. No changes to `src/fs/local.rs` or the `fs` layer are needed — filtering operates on the already-loaded `Vec<EntryInfo>`.

---

### 💾 Persistent History Management (P4)

*   **Goal:** Restore per-pane navigation history across application restarts.
*   **Existing foundation:** In-session history is fully implemented in `src/pane.rs` via `history_back: Vec<PathBuf>`, `history_forward: Vec<PathBuf>`, `push_history()`, capped at 50 entries.
*   **Key Symbols & Locations:**
    *   **Serialization:** `src/config.rs` (the config struct) — add a `history: Vec<PathBuf>` field per pane (or a `[pane_left_history]` / `[pane_right_history]` TOML section). `serde` + `toml` are already in use.
    *   **Save point:** `src/app.rs` on application exit — serialize `pane.history_back` for each pane.
    *   **Load point:** Startup initialization in `src/app.rs` or `src/state/pane_set.rs` — populate `history_back` from config before first render.
    *   **No new `Action` needed** for basic persistence. A `NavigateToHistory(usize)` action would only be required if a history picker overlay is added later.

---

### 🔗 Symbolic Link Visualization & Handling (P4)

*   **Goal:** Display symlink target paths and expose symlink-specific actions.
*   **Existing foundation:** `EntryKind::Symlink` is detected in `src/fs.rs` → `scan_directory()` via `file_type.is_symlink()`. Icons and labels (`'l'`, `"[L]"`) are already present on `EntryKind`.
*   **Key Symbols & Locations:**
    *   **Data model:** Add `link_target: Option<PathBuf>` to `EntryInfo` in `src/fs.rs`. Populate it using `std::fs::read_link(path)` in `scan_directory()` when `EntryKind::Symlink` is matched.
    *   **UI:** `src/ui/pane.rs` — render `→ target` alongside the entry name when `link_target` is `Some`.
    *   **Actions:** Add `FollowSymlink` and `ShowSymlinkTarget` variants to `Action` in `src/action.rs`. Handle in `src/state/mod.rs`.

---

### 🏷️ Advanced Metadata Readout (P4)

*   **Goal:** Display rich file metadata (e.g., EXIF, tags) in the preview panel without blocking the UI.
*   **Key Symbols & Locations:**
    *   **Job dispatch:** `src/jobs.rs` — add a new job variant for metadata fetch. The `jobs` module already manages background workers; follow the existing job pattern.
    *   **Preview state:** `src/state/preview_state.rs` — add a `metadata: Option<MetadataMap>` field to hold parsed results.
    *   **Rendering:** `src/ui/preview.rs` — render the metadata map when present.
    *   **Do not involve `src/fs/local.rs`** — metadata parsing (EXIF, tags) is a preview-layer concern, not a filesystem abstraction concern.

---

## ✨ III. UI/UX Enhancements — **P3 Priority**

### ✅ Confirmation Modals for Destructive Actions (P3)

*   **Goal:** Require explicit `[Y/N]` confirmation before Delete, Permanent Delete, or Overwrite jobs are dispatched.
*   **Key Symbols & Locations:**
    *   **State:** `src/state/dialog.rs` — already contains `DialogState` and `CollisionState` for overlay patterns. Add an `AwaitingConfirmation` variant to `ModalKind` in `src/state/types.rs` (where `ModalKind` is already defined).
    *   **Rendering:** `src/ui/overlay.rs` (368L) — already renders modal overlays. Add a confirmation modal branch here.
    *   **Input routing:** `src/app.rs` — check for `FocusLayer::Modal` (or equivalent) before dispatching destructive actions; route `Y`/`N` input to `Action::CollisionOverwrite` / `Action::CollisionCancel` (both already exist in `action.rs`).
    *   **No new action variants needed** for basic yes/no confirmation — reuse `CollisionOverwrite` and `CollisionCancel`.

---

### 🎯 Global Command Palette (P3)

*   **Goal:** Wire the existing palette infrastructure into the main state machine so it is universally accessible.
*   **Existing foundation:** `src/palette.rs` has `PaletteState`, `PaletteEntry`, `all_entries()`, and `filter_entries()`. `src/ui/palette.rs` handles rendering. `Action::from_palette_key_event()` exists in `src/action.rs`.
*   **Key Symbols & Locations:**
    *   **State integration:** `src/state/mod.rs` — add a `palette_open: bool` flag (or use an existing `FocusLayer` variant from `src/state/types.rs`) to the main app state.
    *   **Input routing:** `src/app.rs` — when palette is open, route all input through `Action::from_palette_key_event()` instead of the normal pane/workspace handlers.
    *   **Keybinding:** `src/action.rs` → `from_workspace_key_event()` — register `Ctrl+P` to emit `Action::OpenPalette` (add this variant).
    *   **Close:** Emit `Action::ClosePalette` on `Escape` within palette input handling.

---

### 🎨 Focus Visualization Improvement (P3)

*   **Goal:** Make the focused pane/element visually unambiguous at a glance.
*   **Key Symbols & Locations:**
    *   **Style definitions:** `src/ui/styles.rs` (48L) — add or update border/highlight style tokens for focused vs. unfocused states.
    *   **Pane rendering:** `src/ui/pane.rs` — consume `PaneFocus` from `src/state/types.rs` to conditionally apply focused border styles.
    *   **This is a rendering-only change** — no state machine or action changes required.

---

### 💡 Summary of Technical Focus Areas

| Enhancement | Primary File(s) | Key Symbol(s) | Priority |
| :--- | :--- | :--- | :--- |
| **Metadata Caching** | `src/pane.rs`, `src/fs.rs` | `Pane` (add `ScanCache`), `scan_directory()` | **P1** |
| **Error Propagation** | `src/state/types.rs`, `src/jobs.rs` | New `ZetaError`, propagation in `fs/*` | **P2** |
| **Confirmation Modals** | `src/state/types.rs`, `src/ui/overlay.rs`, `src/app.rs` | `ModalKind::AwaitingConfirmation`, existing `CollisionCancel`/`CollisionOverwrite` | **P3** |
| **Command Palette** | `src/state/mod.rs`, `src/action.rs`, `src/app.rs` | `PaletteState`, `Action::OpenPalette`, input routing | **P3** |
| **Focus Visualization** | `src/ui/pane.rs`, `src/ui/styles.rs` | `PaneFocus`, border styles | **P3** |
| **Glob Filter Upgrade** | `src/pane.rs`, new `src/utils/glob_match.rs` | `Pane::rebuild_cache()`, `matches_glob()` | **P4** |
| **History Persistence** | `src/config.rs`, `src/app.rs` | `history_back`, config serialization | **P4** |
| **Symlink Actions** | `src/fs.rs`, `src/ui/pane.rs`, `src/action.rs` | `EntryInfo.link_target`, `Action::FollowSymlink` | **P4** |
| **Metadata Readout** | `src/jobs.rs`, `src/state/preview_state.rs`, `src/ui/preview.rs` | New job variant, `PreviewState.metadata` | **P4** |
| **Scan Diffing** | New `src/fs/scan_diff.rs`, `src/pane.rs` | `ScanDiff`, `compute_scan_diff()` | **P5** |
