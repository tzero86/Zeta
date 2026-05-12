# Zeta Implementation Plan

> Created: 2026-05-12  
> Branch: `feat/feature-gating`  
> Based on: `docs/optimization-and-architecture-analysis.md`

---

## Phase 1: Feature Gating (v0.6.0) — IN PROGRESS

**Goal:** Make heavy dependencies optional via Cargo features to reduce binary size and startup overhead. A minimal build (`--no-default-features`) should compile and run core file manager functionality.

**Branch:** `feat/feature-gating`

### 1.1 Cargo.toml Features

| Feature | Dependencies | Default? | Files Impacted |
|---------|-------------|----------|----------------|
| `syntax-highlight` | `syntect`, `two-face` | ✅ Yes | `src/highlight.rs`, `src/jobs.rs`, `src/preview.rs`, `src/ui/mod.rs`, `src/ui/pane.rs`, `src/config.rs` |
| `image-preview` | `image`, `ratatui-image` | ✅ Yes | `src/app.rs`, `src/jobs.rs`, `src/preview.rs`, `src/state/mod.rs`, `src/ui/preview.rs` |
| `sftp` | `ssh2` | ✅ Yes | `src/jobs.rs`, `src/fs/sftp.rs`, `src/state/ssh.rs`, `src/ui/ssh.rs` |
| `terminal-panel` | `vt100`, `portable-pty`, `conpty` | ✅ Yes | `src/pty.rs`, `src/state/terminal.rs`, `src/ui/terminal.rs`, `src/jobs.rs` |
| `auto-update` | `ureq` | ✅ Yes | `src/update.rs`, `src/app.rs` |
| `archives-extra` | `bzip2`, `xz2` | ❌ No | `src/jobs.rs` (already gated) |

### 1.2 Checklist

- [x] Create branch `feat/feature-gating`
- [ ] Gate `syntax-highlight`
  - [ ] Make `syntect`, `two-face` optional in Cargo.toml
  - [ ] `#[cfg(feature = "syntax-highlight")]` on `src/highlight.rs` module
  - [ ] Stub `highlight_text()` returning `None` when feature disabled
  - [ ] Remove `syntect_theme` from `PreviewRequest` / `AppConfig` when disabled
  - [ ] Fix `ui/mod.rs` markdown preview path
  - [ ] Fix `ui/pane.rs` test code
- [ ] Gate `image-preview`
  - [ ] Make `image`, `ratatui-image` optional in Cargo.toml
  - [ ] `#[cfg(feature = "image-preview")]` on image preview code in `src/preview.rs`, `src/ui/preview.rs`
  - [ ] Stub image preview returning "image preview disabled" when feature off
  - [ ] Handle `Picker` type in `AppState` — use `()` or feature-gate field
- [ ] Gate `sftp`
  - [ ] Make `ssh2` optional in Cargo.toml
  - [ ] `#[cfg(feature = "sftp")]` on `src/fs/sftp.rs`
  - [ ] Feature-gate SFTP commands/worker in `src/jobs.rs`
  - [ ] Feature-gate SSH state/UI modules
- [ ] Gate `terminal-panel`
  - [ ] Make `vt100` optional; `portable-pty` and `conpty` already target-scoped
  - [ ] `#[cfg(feature = "terminal-panel")]` on `src/pty.rs`, `src/state/terminal.rs`, `src/ui/terminal.rs`
  - [ ] Feature-gate terminal worker/commands in `src/jobs.rs`
  - [ ] Handle `TerminalState` in `WorkspaceState` — use `Option` or feature-gate field
- [ ] Gate `auto-update`
  - [ ] Make `ureq` optional in Cargo.toml
  - [ ] `#[cfg(feature = "auto-update")]` on `src/update.rs`
  - [ ] Stub `UpdateChecker` / `UpdateState` when disabled
  - [ ] Feature-gate update check at startup in `src/app.rs`
- [ ] Testing
  - [ ] `cargo test --workspace --features default` passes
  - [ ] `cargo test --workspace --no-default-features` passes (minimal build)
  - [ ] Add CI workflow matrix for both feature sets
- [ ] Validation
  - [ ] `cargo fmt --all -- --check`
  - [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - [ ] `cargo test --workspace`
  - [ ] `cargo build --release --no-default-features` (minimal binary)
  - [ ] `cargo build --release` (full binary)

---

## Phase 2: Rendering & Cache Optimizations (v0.6.x)

**Branch:** TBD (`feat/render-optimizations`)

### 2.1 Pane Cache Rebuild
- Cache `lower_name: String` in `EntryInfo` at scan time
- Reuse `filtered_indices` Vec with `clear()` + `extend()`
- Pre-allocate `Vec` capacity in `rebuild_cache()`

### 2.2 Rendering Hot Path
- Pre-allocate `Vec<Span>` capacity in `render_status_bar()`, `render_key_hints()`, `render_pane()`
- Cache breadcrumb strings in `PaneState`
- Reuse `Vec<ListItem>` buffer across frames
- Replace `"─".repeat()` with static lookup
- Move markdown parsing to preview worker thread

### 2.3 String/Path Optimizations
- `selected_path()` → return `Option<&Path>`
- Cache `display_name: String` in `EntryInfo`
- Gate debug string formatting behind `debug_visible`

---

## Phase 3: TUI Evolution — Composable Layout (v0.6.x)

**Branch:** TBD (`feat/composable-layout`)

### 3.1 Panel Trait Abstraction
- Extract `Panel` trait from pane concepts
- Implementors: `FilePanel`, `PreviewPanel`, `TerminalPanel`, `EditorPanel`

### 3.2 Docked Panels
- Add bottom/side dock areas to workspace layout
- Migrate search, bookmarks, git status from modals to docks

### 3.3 Configurable Layouts
- Per-workspace layout config (2×2, 3×1, classic dual-pane)
- Session persistence for layout state

---

## Phase 4: Extensibility — Rich Hooks (v0.6.x / v0.7.x)

**Branch:** TBD (`feat/rich-hooks`)

### 4.1 JSON-RPC Hook Protocol
- New `mode = "jsonrpc"` for hooks
- stdin/stdout protocol for two-way communication
- Hook can emit `PluginCommand` responses

### 4.2 New Hook Events
- `on_preview`, `on_mark`, `on_pre_command`, `on_post_command`, `on_key`

### 4.3 Plugin Context
- Serialize `PluginContext` (selection, marks, workspace, pane) to JSON
- Allow hooks to return `Vec<PluginCommand>` to influence Zeta state

---

## Phase 5: Native Dynamic Plugins (v0.7.x)

**Branch:** TBD (`feat/native-plugins`)

### 5.1 `ZetaPlugin` Trait
- Define stable(ish) plugin trait
- Load `.so`/`.dll` from `~/.config/zeta/plugins/` via `libloading`

### 5.2 Plugin Points
- Custom preview generators
- Custom filesystem backends
- Custom panels
- Custom actions

---

## Progress Log

| Date | Phase | Action | Status |
|------|-------|--------|--------|
| 2026-05-12 | 1 | Created branch, analyzed dependency usage | ✅ Done |
