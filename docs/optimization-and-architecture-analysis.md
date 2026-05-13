# Zeta Optimization & Architecture Analysis

> Date: 2026-05-12  
> Version analyzed: 0.5.12  
> Scope: Performance, binary footprint, TUI evolution, extensibility  

---

## Executive Summary

Zeta is a well-architected modular monolith with clean separation (Action → Command → JobResult) and a worker-per-subsystem threading model that keeps the UI responsive. However, **three structural constraints are beginning to bite**:

1. **Rendering and state hot paths allocate heavily** — fresh `Vec`s, `String`s, and `PathBuf` clones on every frame and every cache rebuild.
2. **Heavy dependencies are unconditionally linked** — `syntect` + `two-face`, `image` + `ratatui-image`, `ssh2`, and `vt100` bloat the binary and startup even when unused.
3. **The Norton Commander dual-pane paradigm is becoming a ceiling** — complex workflows (multi-step operations, rich previews, plugin integrations) don't fit naturally into two static panes + modals.

The codebase has **no plugin system** (deliberately, per `AGENTS.md`), and the existing **shell hooks** (`on_cd`, `on_open`, `on_start`, `on_exit`) are too coarse for real extensibility.

This document breaks down specific findings and proposes a phased roadmap.

---

## 1. Performance & Memory Footprint

### 1.1 Rendering Hot Path (High Impact)

Every frame rebuilds collections from scratch:

| Location | Allocation | Frequency |
|----------|-----------|-----------|
| `src/ui/pane.rs:114` | `visible_entries()` → `Vec<&EntryInfo>` | Every draw |
| `src/ui/pane.rs:295` | `truncate_text()` → `String` per visible row | Every draw |
| `src/ui/mod.rs:443` | Status bar `spans`, `right_spans`, `"─".repeat()` | Every draw |
| `src/ui/mod.rs:640` | Key hints `vec![...]` + `format!()` strings | Every draw |
| `src/ui/preview.rs:262` | `collect::<Vec<_>>().join("\n")` | Every draw (cheap mode) |
| `src/ui/preview.rs:23` | `wrap_preview_line()` → `Vec<WrappedPreviewRow>` + per-char `String` boxing | Every draw |

**Recommendations:**
- **Pre-allocate with capacity** in `render_pane()`, `render_status_bar()`, and `render_key_hints()`.
- **Cache breadcrumb strings** in `PaneState` instead of computing `path.display().to_string()` + home-dir stripping every frame.
- **Reuse `Vec<ListItem>` buffers** across frames with `Vec::clear()` rather than `collect()`.
- **Replace `"─".repeat(filled)`** with a small static lookup table or `ratatui::symbols::bar::FULL` repeated via `Span::raw` with a count.
- **Defer markdown parsing** (`src/ui/markdown.rs`) to the preview worker thread. It currently runs on the UI thread when the cache is stale, blocking input for large files.

### 1.2 Pane Cache Rebuild (High Impact)

`PaneState::rebuild_cache()` (`src/pane.rs:511`) is called on every sort change, filter toggle, or directory scan:

```rust
let indices: Vec<usize> = (0..self.entries.len()).collect();
let lower_names: Vec<String> = self.entries.iter()
    .map(|e| e.name.to_lowercase()).collect();
```

- `lower_names` is rebuilt from scratch every time.
- Extension sort re-computes `to_lowercase()` inside the comparator closure.
- `filtered_indices: RefCell<Vec<usize>>` is replaced, not reused.

**Recommendations:**
- **Store a cached `lower_name: String`** inside `EntryInfo` at scan time. Sorting and filtering then use zero-allocation references.
- **Reuse the `filtered_indices` Vec** — call `.clear()` and `.extend(...)` instead of replacing it.
- **Use `Vec::with_capacity(self.entries.len())`** for `indices`.

### 1.3 String / PathBuf Cloning (Medium Impact)

- `src/state/mod.rs` has **168 `.clone()` calls** — many clone `PathBuf` and `String` during action dispatch.
- `PaneState::selected_path()` clones `PathBuf` on every call.
- `path.display().to_string()` is used pervasively for status messages, breadcrumbs, and error text.

**Recommendations:**
- `selected_path()` should return `Option<&Path>` instead of `Option<PathBuf>` where callers only need a reference.
- Cache `display_name: String` in `EntryInfo` (already has `name: String`; add a `display_path` for the full path).
- Use `Cow<'_, str>` for status messages that are usually static literals but occasionally formatted.

### 1.4 Event Loop (Low-Medium Impact)

The main loop is generally efficient (~60 Hz poll, non-blocking channels), but:

- `dispatch()` formats `action_name = format!("{:?}", action)` on **every action** for debug logging, even when the debug panel is hidden.
- `mark_drawn()` increments `redraw_count` but previously failed to clear `needs_redraw` (fixed in v0.5.12).

**Recommendations:**
- Gate the debug string behind `if self.state.debug_visible` or a compile-time `cfg!(debug_assertions)` check.

---

## 2. Binary Size & Startup Time

### 2.1 Dependency Bloat (Very High Impact)

| Dependency | Size Impact | Currently Optional? |
|-----------|-------------|---------------------|
| `syntect` + `two-face` | ~5-8 MB (regex-fancy + all syntax defs) | ❌ No |
| `image` + `ratatui-image` | ~3-5 MB (decoders, color management) | ❌ No |
| `ssh2` + `libssh2-sys` | ~1-2 MB + C build | ❌ No |
| `vt100` | ~500 KB-1 MB (terminal emulator) | ❌ No |
| `ureq` | ~500 KB | ❌ No |
| `arboard` | Moderate | ❌ No |
| `notify` | Moderate | ❌ No |

**Recommendation — add Cargo features:**

```toml
[features]
default = ["syntax-highlight", "image-preview", "sftp", "terminal-panel", "auto-update"]
syntax-highlight = ["syntect", "two-face"]
image-preview = ["ratatui-image", "image"]
sftp = ["ssh2"]
terminal-panel = ["vt100", "portable-pty"]
auto-update = ["ureq"]
```

This would let users build a **minimal Zeta** (~5-10 MB smaller) with:
```bash
cargo install zeta --no-default-features
```

A CI matrix should build and test both `default` and `minimal` feature sets.

### 2.2 Eager Initialization (Medium Impact)

- **11+ worker threads** are spawned unconditionally at boot (`spawn_workers()` in `src/jobs.rs:516`). Workers for disabled features (e.g., SFTP when no remotes are configured) still sit idle.
- **WSL timezone detection** (`UTC_OFFSET_MINUTES` at `src/state/mod.rs:58`) spawns PowerShell with a 2-second blocking timeout at startup.
- **`syntax_set()` / `theme_set()`** (`src/highlight.rs:18-27`) are `OnceLock`-lazy but may stutter on first preview if a file is pre-selected.

**Recommendations:**
- **Lazy-spawn workers** — only start the SFTP worker on first remote connection, the terminal worker on first terminal panel open, etc.
- **Cache timezone offset** to a temp file and read it synchronously; spawn PowerShell only on cache miss.
- **Pre-warm syntax sets** in a background thread during idle ticks if `syntax-highlight` is enabled.

### 2.3 Directory Scanning Memory (Medium Impact)

`scan_directory()` (`src/fs.rs:117`) collects all entries into a `Vec<EntryInfo>` before returning. For directories with 100k+ entries, this causes a large memory spike and blocks the scan worker until completion.

**Recommendation:**
- Consider a **streaming scan API** that yields `EntryInfo` batches (e.g., `Vec<EntryInfo>` chunks of 1,000) across the channel. `PaneState` appends incrementally, and the UI can show partial results. This is a significant refactor but dramatically improves perceived performance on huge directories.

---

## 3. TUI Paradigm Limitations

### 3.1 The Norton Commander Ceiling

The dual-pane layout (left/right + preview/editor + modals) is fast and familiar, but it creates friction for:

| Workflow | Current Fit | Friction |
|----------|-------------|----------|
| Multi-step batch operations (e.g., "find all `.log` files > 7 days, compress, upload to SFTP, delete local") | Poor | Each step is a separate modal or file-op; no pipeline composition |
| Rich media preview (video, audio, PDF) | Poor | Text/image only; no embedded player |
| Custom views (tree view, flattened view, git log graph, du visualization) | Poor | Fixed pane layout; no alternative view modes |
| Plugin-driven panels (e.g., a fuzzy-finder panel, a git branch graph, a process list) | Impossible | No dynamic panel API |
| Floating tool windows (persistent search, command palette, quick nav) | Limited | Modals are transient and modal-only |

### 3.2 Modal Fatigue

Current UX relies heavily on modals for:
- Confirmation dialogs (delete, overwrite)
- Settings panel
- Bookmarks panel
- Help/cheatsheet
- Find/replace in editor
- Git diff view

As features accumulate, modals stack and the user loses spatial context. The NC paradigm has no concept of **docked panels** or **split views** beyond the fixed left/right panes.

### 3.3 Proposed Evolution: "Composable Layout Engine"

Rather than abandoning the NC roots, **generalize the pane concept**:

```
┌─────────────────────────────────────┐
│ [Pane A: files] │ [Pane B: files]  │  ← Classic NC
├─────────────────┴───────────────────┤
│ [Panel: preview / terminal / git]   │  ← Flexible bottom panel
├─────────────────────────────────────┤
│ [Panel: command palette / search]   │  ← Overlay or docked
└─────────────────────────────────────┘
```

**Concrete steps:**
1. **Extract `Pane` into a generic `Panel` trait** that can host:
   - `FilePanel` (current pane)
   - `PreviewPanel` (current preview)
   - `TerminalPanel` (current terminal)
   - `EditorPanel` (current editor, currently fullscreen)
   - `CustomPanel` (future plugin surface)
2. **Add a layout grid** (2×2, 3×1, etc.) configurable per workspace.
3. **Support side/bottom docks** for persistent tool panels (search, git status, bookmarks) that don't block the main view.
4. **Keep modals for true interruptions** (confirmations) but move tools to docked panels.

This is a large refactor but preserves the keyboard-first NC workflow while removing the layout ceiling.

---

## 4. Extensibility: Hooks → Plugins

### 4.1 Current Hooks (Too Coarse)

Zeta has 4 config-driven shell hooks (`src/hooks.rs`):
- `on_cd` — fires on directory change
- `on_open` — fires on file open
- `on_start` — fires at boot
- `on_exit` — fires at shutdown

Execution model: **fire-and-forget `sh -c` processes** with env vars (`ZETA_PATH`, `ZETA_PANE`, etc.).

**Limitations:**
- No return value — hooks cannot influence Zeta's behavior (e.g., a hook cannot say "don't open this file, preview it instead").
- No async lifecycle — hooks run detached; Zeta doesn't know when they finish.
- No state access — hooks cannot read the current selection, marks, or workspace state.
- Cross-platform shell dependency — Windows users need `sh` available.

### 4.2 A Pragmatic Plugin Roadmap

Rather than a heavy WASM or Lua runtime (which violates the "low overhead" mandate), Zeta can adopt a **"micro-plugin" model** inspired by `helix` and `kakoune`:

#### Phase 1: Richer Hooks (Immediate, Low Risk)

Add more lifecycle points and a **JSON-RPC/stdin protocol** for two-way communication:

```toml
[[hooks]]
event = "on_preview"
command = "zeta-lsp-hook"
mode = "jsonrpc"  # new: two-way protocol
```

The hook process receives a JSON payload on stdin and can emit commands on stdout:

```json
// Zeta sends:
{"event":"on_preview","path":"/src/main.rs","workspace":0,"pane":"left"}

// Hook responds:
{"commands":[{"type":"show_message","text":"LSP: 3 errors found"}]}
```

New hook events:
- `on_preview` — before/after preview load
- `on_mark` — when entries are marked/unmarked
- `on_pre_command` — before executing a command (allows cancellation/modification)
- `on_post_command` — after command completion
- `on_key` — custom key handling (fallback before default binding)

#### Phase 2: Native Dynamic Libraries (Medium Effort)

For performance-critical extensions (custom preview renderers, new filesystem backends):

```rust
// src/plugin.rs
pub trait ZetaPlugin: Send + Sync {
    fn on_event(&mut self, event: PluginEvent, ctx: &PluginContext) -> Vec<PluginCommand>;
}

// Loaded via dlopen / libloading at startup from ~/.config/zeta/plugins/
```

**Pros:** Zero serialization overhead, full Rust type safety.  
**Cons:** ABI stability, unsafe code for loading. Mitigate by versioning the trait and requiring plugins to be recompiled per minor version.

#### Phase 3: WASM Sandbox (Long-term, High Effort)

For cross-platform, user-safe plugins:

```rust
// wasmtime or wasmer runtime
wasm_plugin.on_event(event) -> Vec<PluginCommand>
```

**Pros:** Safe, cross-platform, language-agnostic (Rust, Go, JS compiled to WASM).  
**Cons:** Adds ~1-2 MB to binary, runtime overhead, complex host bindings.

**Recommendation:** Skip Phase 3 until v2.0. Implement Phase 1 (rich JSON-RPC hooks) in v0.6.x and Phase 2 (native `.so`/`.dll` plugins) in v0.7.x.

### 4.3 Plugin Surface Areas

The most valuable extension points, ordered by user demand:

1. **Custom preview generators** — render PDFs, video thumbnails, database schemas, API responses.
2. **Custom filesystem backends** — beyond local and SFTP (e.g., S3, WebDAV, Docker containers).
3. **Custom views/panels** — a tree-view panel, a git log graph, a `du`-style size treemap.
4. **Custom actions/commands** — new keyboard-driven operations without recompiling Zeta.
5. **Theme extensions** — load external themes from files (currently only 10 built-in presets).

---

## 5. Prioritized Action Plan

### Immediate (v0.6.0 — Performance Patch)

| # | Task | Impact | Effort |
|---|------|--------|--------|
| 1 | Feature-gate `syntect`/`two-face`, `image`/`ratatui-image`, `ssh2`, `vt100` | Binary size ↓ 30-50% | Medium |
| 2 | Cache `lower_name` in `EntryInfo`; reuse `filtered_indices` Vec | Cache rebuild ↓ allocations | Low |
| 3 | Pre-allocate render Vecs with capacity; cache breadcrumbs | Frame time ↓ | Low |
| 4 | Move markdown parsing to preview worker | UI thread unblock | Low |
| 5 | Lazy-spawn workers (SFTP, terminal) | Startup time ↓, RAM ↓ | Medium |
| 6 | Fix WSL timezone detection to non-blocking | Startup time ↓ | Low |

### Short-term (v0.6.x — Evolving the TUI)

| # | Task | Impact | Effort |
|---|------|--------|--------|
| 7 | Design `Panel` trait abstraction | Enables custom panels | High |
| 8 | Add bottom/side docked panels (search, git, bookmarks) | Reduced modal fatigue | High |
| 9 | Support 2×2 and 3×1 workspace layouts | Power-user productivity | Medium |
| 10 | Richer JSON-RPC hooks (Phase 1 extensibility) | Plugin ecosystem seed | Medium |

### Medium-term (v0.7.x — Extensibility)

| # | Task | Impact | Effort |
|---|------|--------|--------|
| 11 | Native dynamic library plugin API (Phase 2) | High-performance extensions | High |
| 12 | Streaming directory scan (batched `EntryInfo` chunks) | Huge directory performance | High |
| 13 | External theme loading from files | Customization | Low |
| 14 | Custom preview generator hooks | PDF, video, database previews | Medium |

---

## 6. Measurement Checklist

Before and after each optimization:

```bash
# Binary size
cargo bloat --release
cargo build --release && ls -lh target/release/zeta

# Startup time (hyperfine)
hyperfine --warmup 3 'target/release/zeta --version'

# Frame time (add a `--profile` flag that prints avg draw time on quit)
# Memory (valgrind massif, or heaptrack on Linux)
heaptrack target/release/zeta

# Directory scan throughput
# cd /usr/lib && time zeta --scan-only --quit
```

---

## Appendix: Existing Modularity Patterns (To Preserve)

These patterns are working well and should be the foundation for future evolution:

1. **`Action` / `Command` / `JobResult` triad** — clean deterministic state machine.
2. **`FsBackend` trait** — compile-time polymorphism for local vs remote filesystems.
3. **Worker-per-subsystem + `try_send()`** — non-blocking, backpressure-aware concurrency.
4. **Config-driven behavior** — hot-reloadable TOML for keymap, theme, hooks, openers.
5. **Session persistence** — seamless restart continuity.

The goal is to **extend** these patterns (e.g., make `FsBackend` dynamically loadable, generalize `Panel` from the pane concept) rather than replace them.
