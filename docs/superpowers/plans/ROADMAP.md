# Zeta — Development Roadmap

This file is the single source of truth for all planned and completed work.
Each wave has a dedicated plan file in this directory. Update status here when
a wave ships.

---

## Status key

| Symbol | Meaning |
|---|---|
| ✅ | Shipped and merged to `main` |
| 🚧 | In progress |
| 📋 | Documented, not started |
| 💡 | Idea only, not yet documented |

---

## Completed waves

| Wave | Plan | Summary |
|---|---|---|
| 1A | `2026-04-07-wave1a-appstate-decomposition.md` | AppState decomposed into sub-states |
| 1B | `2026-04-07-wave1b-ui-module-split.md` | ui.rs split into focused modules + LayoutCache |
| 1C | `2026-04-07-wave1c-multiworker-editor-markdown.md` | 3-worker jobs, ropey editor, markdown ViewBuffer |
| 2A | `2026-04-07-wave2a-input-routing.md` | FocusLayer enum, RouteContext deleted |
| 2B | `2026-04-07-wave2b-mouse-support.md` | Full mouse support (click, scroll, menu bar) |
| 3A | `2026-04-08-wave3a-editor-rope-backend.md` | Rope backend, O(log n) ops, delta undo, highlight cache |
| 4A | `2026-04-08-wave4a-git-integration.md` | Git status indicators, branch name in status bar |
| 4B | `2026-04-08-wave4b-markdown-live-preview.md` | Native markdown renderer, split editor/preview panel |
| 4C | `2026-04-08-wave4c-editor-fullscreen-sync.md` | Full-window editor (F11), preview scroll sync (cursor-driven), preview focus/toggle |
| 4D | `2026-04-08-wave4d-quickfilter-fuzzy-find.md` | In-pane quick filter (`/`), fuzzy file finder (`Ctrl+P`) |
| 5A | `2026-04-08-wave5a-find-replace-watcher.md` | Find & Replace (`Ctrl+H`), directory watcher auto-refresh |
| 5B | `2026-04-08-wave5b-bookmarks-trash.md` | Bookmarks (`BookmarksState`), trash/recycle bin (`trash` crate v3) |
| 5C | `2026-04-08-wave5c-shell-integration.md` | F2 toggles embedded terminal at current pane directory |
| 6A | `2026-04-08-wave6a-archive-browsing.md` | Navigate into .zip / .tar.gz / .tar.bz2 / .tar.xz like directories |
| 6B | `2026-04-08-wave6b-directory-diff.md` | Left/right pane diff mode — colour-code unique/matching/different entries |
| 7A | `2026-04-08-wave7a-ssh-sftp.md` | SSH/SFTP Remote pane via ssh2 + FsBackend trait refactor |
| 7B | `2026-04-12-wave7b-ssh-agent.md` | SSH Agent and Host Key Verification |
| 8A | `2026-04-12-wave8a-embedded-terminal.md` | Fully embedded terminal emulator (PTY + rendering) |

---

## Post-wave shipped features

These features were shipped after the wave-based architecture work completed.
They are tracked in CHANGELOG.md and the enhancements roadmap.

### Update System
- **Auto-update checks** on startup (configurable; GitHub API, 5-second timeout, non-blocking)
- **Manual check** via Help → Check for Updates
- **On-demand install** via Help → Apply Update — runs `cargo install --git` and re-execs the new binary
- **On-exit install** — pressing Quit when an update is available shows a prompt; confirming installs on exit
- Status bar notification with color-coded symbols (✓ latest, ◆ update available, ⋯ checking, ✗ error)
- Pulsing "● Update" indicator persists after notification expires

### First-Run Wizard
- Multi-step wizard on first launch: theme selection, icon mode, details view, terminal preference
- Annotated config generation — writes `~/.config/zeta/config.toml` with inline comments

### Shell Hook System
- Configurable hooks that execute shell commands on file events (open, copy, move, delete, enter, exit, rename)
- Per-event hook lists with `{file}`, `{dir}`, `{src}`, `{dst}` placeholders
- Non-blocking execution; hooks run via worker thread without stalling the UI

### Git Diff Viewer
- Full-screen git diff view (`Ctrl+D`) — file list (38%) + unified diff content (62%)
- Arrow / vi-key navigation, page-through diff, `Tab` to switch focus between file list and diff panes

### Preview Panel Enhancements
- **Image preview**: halfblock rendering via `ratatui-image`; scaled result cached by viewport dimensions
- **Hex dump**: binary files rendered as `offset | hex bytes | ASCII printable`
- **Archive listing**: `.zip`, `.tar`, `.tar.gz` file listings in preview; `.tar.bz2`/`.tar.xz` via `archives-extra` feature flag

### UI/UX Revamp (v0.5.0)
- **ThemePalette v2**: 13 new accent tokens + Catppuccin Mocha preset with exact RGB values across 10 themes
- **NerdFont icons v3**: per-extension codepoints for Rust, Python, JS/TS, Go, C/C++, Markdown, shell, config, images, archives, symlinks
- **Modal halo ring**: semi-transparent backdrop around all modals; modal titles centered
- **Panel chrome titles**: Editor shows icon, filename, parent dir, live Ln/Col, dirty indicator; Preview shows eye icon + `.EXT` badge; Terminal shows terminal icon + Shell badge
- **Settings segmented tabs**: Appearance / Panels / Editor / Keymaps with Tab/1–4 navigation
- **Help modal two-column layout**: key shortcuts as pill spans; left (Navigation + Files) + right (Editor + System)
- **Command palette match highlighting**: per-character match in accent yellow; `⌕` input prefix
- **File finder**: teal input, root hint, dir/filename split display, teal match highlighting

### Status Bar & Pane Polish
- **Five-zone status bar**: Git branch · active entry (icon, name, size, permissions) · job message · marks info · workspace name
- **Animated progress bar** during file operations
- **Live clock** in rightmost zone (ticks every second)
- **Pane column headers**: Name/Size/Date header row; active filter shown in teal accent bar with match count
- **Status message severity colors**: info (teal), warning (yellow), error (red)
- **Contextual pane hints**: action hints update based on active entry type

### File Operations & Navigation
- **Destructive confirm modal**: confirmation for delete/trash with multi-item listing (up to 5 + "and N more")
- **Jump-to-path** (`Ctrl+G`): navigate to an arbitrary directory by typing a path
- **Bulk rename** (`Ctrl+R`): rename all marked files with pattern substitution
- **Pane resize** (`Alt+[` / `Alt+]`): adjust left/right split ratio
- **Open-with menu** (`Alt+O`): launch file with a custom command
- **F7 MakeDirectory**: create a new directory inline
- **Modal input cursor**: visible text cursor in all modal text fields

### Filesystem & Metadata
- **Symlink rendering**: `link_target` field on `EntryInfo`; link target shown inline in pane
- **FollowSymlink** / **ShowSymlinkTarget** actions
- **Glob filter**: pane filter upgraded from substring to shell-style glob patterns (`*.rs`, `!*~`, etc.)
- **Metadata cache + scan diffing**: directory scan results cached by `mtime`; incremental diffs on rescan
- **ZetaError context propagation**: typed errors with caller-supplied context at all subsystem boundaries
- **Session persistence**: per-pane navigation history (back/forward) survives application restarts

### Performance & Developer Tools
- **Dependency slimming** (v0.4.5): 387 → 297 transitive crates; Windows uses `conpty` instead of `portable-pty`
- **`archives-extra` feature flag**: `.tar.bz2` / `.tar.xz` support opt-in (avoids `bzip2-sys`/`lzma-sys` C builds by default)
- **`zeta-font-test` diagnostic binary**: prints NerdFont PUA glyphs for font validation
- **F12 debug panel**: live key event, action dispatch, and state display overlay
- **Context-aware menu bar**: irrelevant menu tabs dim based on active panel
- **State decomposition refactor**: `apply_view()` monolith split into focused handler methods

---

## Active roadmap

_All planned waves shipped. All major post-wave enhancements shipped. No pending tracked work._

---

## Jira epic mapping

| Epic | Wave |
|---|---|
| ZTA-122 — Wave 1C: Multi-Worker Jobs + Editor + Markdown Preview | 1C |
| ZTA-123 — Wave 2A: FocusLayer + Input Routing Redesign | 2A |
| ZTA-124 — Wave 2B: Full Mouse Support | 2B |
| ZTA-125 — Wave 3A: Editor Rope Backend + Lightweight Undo Stack | 3A |
| ZTA-126 — Wave 4A: Git Integration | 4A |
| ZTA-127 — Wave 4B: Markdown Live Preview | 4B |
| ZTA-128 — Wave 4C: Editor Fullscreen + Scroll Sync + Preview Focus/Toggle | 4C |
| ZTA-129 — Wave 4D: In-Pane Quick Filter + Fuzzy File Find | 4D |
| ZTA-130 — Wave 5A: Find & Replace in Editor + Directory Watcher | 5A |
| ZTA-131 — Wave 5B: Bookmarks + Trash | 5B |
| ZTA-132 — Wave 5C: Shell Integration | 5C |
| ZTA-156 — Wave 6A: Archive Browsing | 6A |
| ZTA-157 — Wave 6B: Directory Diff Mode | 6B |
| ZTA-168 — Wave 7A: SSH/SFTP Remote Filesystems | 7A |
| ZTA-169 — Wave 7B: SSH Agent and Host Key Verification | 7B |
| ZTA-170 — Wave 8A: Embedded Terminal | 8A |
