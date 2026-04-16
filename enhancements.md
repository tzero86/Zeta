# Enhancements Roadmap: Zeta TUI File Manager

This document outlines strategic areas for enhancing the Zeta file manager. The goal of these enhancements is to improve performance, expand functionality, and refine the user experience while strictly adhering to core principles: **low overhead**, **deterministic state management**, and **keyboard-first workflow**.

---

## 🚀 I. Feature Depth (Expanding Core Functionality)

These features require adding new business logic across multiple modules (`fs`, `action`, `pane`). They increase capability but must be designed carefully to avoid performance hits.

### 📂 Advanced Filtering and Globbing
*   **Status:** ⚙️ **Partially implemented.** A substring filter already exists in `pane.rs` via `filter_query: String` and `filter_active: bool`, applied inside `rebuild_cache()`. The remaining work is upgrading the matcher to support shell-style glob patterns.
*   **Description:** Upgrade the existing filter to support shell-style patterns (e.g., `*.rs`, `!*~`, or complex globs), going beyond the current simple substring match.
*   **Impacted Modules:** `pane`.
*   **Implementation Note:** The glob matching logic should replace the lowercased substring check inside `rebuild_cache()` in `pane.rs`. A small utility (e.g., `src/utils/glob_match.rs`) can wrap a glob crate or implement basic pattern matching. The `fs` layer does not need to change — filtering operates on the already-scanned `Vec<EntryInfo>` held by the pane.

### 💾 Persistent History Management
*   **Status:** ⚙️ **Partially implemented.** In-session back/forward history is fully working in `pane.rs` via `history_back: Vec<PathBuf>`, `history_forward`, and `push_history()`, capped at 50 entries. The remaining work is **cross-session persistence** only.
*   **Description:** Persist the navigation history to disk so that recently visited directories survive application restarts, allowing users to quickly jump back to prior locations.
*   **Impacted Modules:** `config`, `state`.
*   **Implementation Note:** On exit, serialize `history_back` from each pane to the config file (via `serde` + `toml`). On startup, restore it. No new `Action` variant is needed for basic persistence — the existing `NavigateBack` / `NavigateForward` actions already cover runtime use. A new `Action` for jumping to an arbitrary history entry (e.g., `NavigateToHistory(usize)`) would be needed only if a history picker UI is added.

### 🔗 Symbolic Link Visualization & Handling
*   **Status:** ⚙️ **Partially implemented.** `EntryKind::Symlink` already exists in `fs.rs` and is detected during directory scans via `symlink_metadata`. Icons/labels for symlinks are present (`'l'`, `"[L]"`). The remaining work is richer visual treatment and dedicated actions.
*   **Description:** Improve visual differentiation of symlinks in the pane view and add dedicated actions (e.g., "Follow Link," "Show Target Path").
*   **Impacted Modules:** `fs`, `pane`, `ui`, `action`.
*   **Implementation Note:** Add a `link_target: Option<PathBuf>` field to `EntryInfo` in `fs.rs`, populated using `std::fs::read_link()` when `EntryKind::Symlink` is detected. This allows `pane` to display the target path and `action` to offer symlink-specific commands.

### 🏷️ Advanced Metadata Readout
*   **Description:** Expand the preview panel to display non-standard metadata (e.g., EXIF data for images, custom file tags).
*   **Impacted Modules:** `preview`, `jobs`.
*   **Implementation Note:** Since reading complex metadata is I/O-bound, dispatch it as a background job via the `jobs` module to avoid blocking the UI thread. Results feed into a dedicated metadata panel within the preview view. The `fs` layer should not be involved — metadata parsing is a preview concern, not a filesystem concern.

---

## 🧠 II. Architectural & Performance Improvements (System Core)

These are important improvements that increase stability, reduce resource usage, and improve perceived speed without requiring new visible features.

### ⚡ Metadata Caching Layer
*   **Problem:** When revisiting a directory, the filesystem is re-queried for all attributes even when nothing has changed.
*   **Solution:** Cache the `Vec<EntryInfo>` result of a directory scan alongside the directory's `mtime` at the time of scan. Before re-scanning, check whether the directory's current `mtime` still matches the cached value. Only rescan if it does not.
*   **Impacted Modules:** `pane`, `fs`.
*   **Implementation Note:** The cache fits naturally in `pane.rs` alongside the existing `cache_filter_*` fields (which already use `Cell`/`RefCell` for dirty tracking). A lightweight struct holding `(PathBuf, SystemTime, Vec<EntryInfo>)` is sufficient. `pane_set.rs` should not be used — it manages workspace/pane switching logic, not scan results.

### 🔄 Optimized Scanning via Diffing
*   **Problem:** After a background rescan (e.g., triggered by a file watcher or manual refresh), the entire pane entry list is replaced, even when only one file changed.
*   **Solution:** When a rescan is triggered on an already-loaded directory, compute a diff against the cached list (`Added`, `Removed`, `Modified`) and apply only the delta to the pane's entry list.
*   **Impacted Modules:** `pane`, `fs`.
*   **Implementation Note:** This is **lower priority than the caching layer** — the cache eliminates the majority of redundant I/O, making diffing a refinement rather than a fix. When implementing, add scan-diff logic to a new `src/fs/scan_diff.rs` module. Do **not** extend the existing `src/diff.rs`, which is used for side-by-side pane content comparison (a separate, unrelated feature). Diffing requires a benchmark baseline before it can be justified.

### 🛡️ Comprehensive Error Context Propagation
*   **Goal:** Turn generic failures into diagnostic, user-actionable messages.
*   **Approach:** `FileSystemError` using `thiserror` already exists in `fs.rs`. Standardize a top-level `ZetaError` type that wraps module errors and always requires the caller to supply context (e.g., `"Failed to move '{path}': {source}"`).
*   **Impacted Modules:** All modules using file I/O (`fs`, `jobs`), plus `action` and `state` at subsystem boundaries.

---

## ✨ III. UI/UX Enhancements (Polish & Interactivity)

These changes focus on refining the interaction model, making the application feel more polished and predictable.

### ✅ Confirmation Modals for Destructive Actions
*   **Problem:** Deleting or permanently overwriting files is irreversible.
*   **Solution:** For destructive actions (Delete, Permanent Delete, Overwrite), transition the app to an `AwaitingConfirmation` state. The `ui` renders a non-blocking modal overlay (`[Y/N]?`) via `state/dialog.rs`. The destructive job in `jobs` is only dispatched upon explicit confirmation.
*   **Impacted Modules:** `state/dialog.rs`, `app`, `action`.
*   **Implementation Note:** `state/dialog.rs` already has `DialogState` and `CollisionState` for similar overlay patterns. `AwaitingConfirmation` can follow the same structure and be rendered via the existing overlay infrastructure in `ui/overlay.rs`.

### 🎯 Global Command Palette (Ctrl+P)
*   **Status:** ⚙️ **Structurally scaffolded.** `src/palette.rs` (491L) holds `PaletteState`, `PaletteEntry`, and `all_entries()`. `src/ui/palette.rs` (148L) handles rendering. The remaining work is wiring the palette into the main state machine.
*   **Description:** Make the command palette universally accessible via a keybinding, suspending normal pane input and routing all keystrokes to palette search/execution logic.
*   **Implementation Note:** Introduce a `FocusLayer::Palette` variant (or equivalent mode) in `state/types.rs` so that `app.rs` routes input to `Action::from_palette_key_event()` (which already exists in `action.rs`). State integration in `state/mod.rs` is the primary remaining task.

### 🎨 Focus Visualization Improvement
*   **Description:** Sharpen the visual distinction between *selection* (highlighted row) and *focus* (the pane/element receiving keyboard input). The active-focus indicator should be unambiguous at a glance.
*   **Impacted Modules:** `ui/pane.rs`, `ui/styles.rs`.
*   **Implementation Note:** `FocusLayer` and `PaneFocus` types already exist in `state/types.rs`. This is a rendering-only change: update border/highlight styles in `ui/pane.rs` and `ui/styles.rs` to visually distinguish focused vs. non-focused panes more strongly.

---

### 📝 Architectural Summary & Next Steps

| Area | Focus Goal | Key Module(s) Affected | Priority |
| :--- | :--- | :--- | :--- |
| **Performance** | Metadata Caching Layer | `pane`, `fs` | **P1 (Highest)** |
| **Robustness** | Error Context Propagation | All Modules | **P2** |
| **Interaction** | Confirmation Modals, Command Palette | `state/dialog.rs`, `action`, `state/mod.rs` | **P3** |
| **Functionality** | Glob Filter Upgrade, History Persistence, Symlink Actions | `pane`, `config`, `fs` | **P4** |
| **Performance** | Scan Diffing (after caching baseline) | `pane`, `fs/scan_diff.rs` | **P5** |
