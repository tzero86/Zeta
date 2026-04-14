# Zeta

```
 ____  ________  ____             __
|    \|        \|    \           |  \
| $$$$ \$$$$$$$$ \$$$$  ______  _| $$_     ______
| $$      /  $$   | $$ /      \|   $$ \   |      \
| $$     /  $$    | $$ |  $$$$$$\\$$$$$$    \$$$$$$\
| $$    /  $$     | $$ | $$    $$ | $$ __  /      $$
| $$_  /  $$___  _| $$ | $$$$$$$$ | $$|  \|  $$$$$$$
| $$ \|  $$    \|   $$ \$$     \  \$$  $$ \$$    $$
 \$$$$ \$$$$$$$$ \$$$$  \$$$$$$$   \$$$$   \$$$$$$$
```

A keyboard-first terminal file manager and embedded editor for developers —
Norton Commander workflow, modern TUI, written in Rust.

---

## Download

Precompiled binaries for Linux and Windows are attached to every [GitHub Release](https://github.com/tzero86/Zeta/releases/latest).

| Platform | File |
|---|---|
| Linux x86\_64 | `zeta-linux-x86_64.tar.gz` |
| Windows x86\_64 | `zeta-windows-x86_64.zip` |

SHA-256 checksums are included alongside each binary.

### Build from source

Requires [rustup](https://rustup.rs) (stable toolchain) and git.

```bash
cargo install --git https://github.com/tzero86/Zeta
```

#### Native build dependencies

Some crates link against system libraries. Install these before running `cargo install` if you hit linker errors:

| Platform | Install |
|---|---|
| Debian / Ubuntu | `sudo apt install build-essential pkg-config libssl-dev libbz2-dev liblzma-dev` |
| Fedora / RHEL | `sudo dnf install gcc pkg-config openssl-devel bzip2-devel xz-devel` |
| Windows | [Visual Studio Build Tools](https://aka.ms/vs/17/release/vs_BuildTools.exe) — "Desktop development with C++" workload |

`ssh2` requires OpenSSL; `bzip2` and `xz2` require their respective compression libraries. The `conpty` crate (Windows terminal) requires Windows 10 version 1809 or later.
---

## Features

### File management
- Dual-pane browser with side-by-side and stacked layouts
- Four isolated in-app workspaces with independent pane, preview, editor, terminal, and transient runtime state
- Top-bar workspace pills (`[1] [2] [3] [4]`) highlight the active workspace at a glance
- Session restore for all four workspaces plus the active workspace index
- Create, copy, move, rename, and delete files and directories
- Collision resolution dialog (skip, overwrite, rename)
- Background workers — file operations, directory scans, and previews never block each other
- Mark multiple files for batch operations
- Hidden file toggle, sort by name / size / extension / modified date
- Navigate history (Alt+Left / Alt+Right)

### SSH/SFTP Remote Filesystems
- Connect to remote servers via SSH/SFTP directly from the file manager
- Authenticate using passwords, key files, or SSH Agent integration
- Strict host key verification against `~/.ssh/known_hosts` for secure connections
- Full support for remote file operations (copy, move, delete, rename, etc.)
- Transparent cross-filesystem support (copy from local to remote or vice versa)
- Use the SSH Connect dialog (opened via command palette or menu) to initiate a session

**Security Test Plan:**
- **Host Key Verification Failure:** Attempt to connect to a host with an invalid or changed host key in `~/.ssh/known_hosts`. The UI will reject the connection with: `WARNING: Host key changed! Please investigate manually.`
- **SSH Agent Authentication:** Run `ssh-agent`, add a key via `ssh-add`, and attempt an SSH connection without providing a password or key file. The connection will succeed automatically using the agent's identities.

### Editor
- Embedded text editor with syntax highlighting (via syntect)
- Undo / redo with delta-based history (O(log n), handles large files)
- In-editor search with match count and wrap-around
- Save / discard flow with dirty-state guard

### Markdown live preview
- When editing a `.md` file the tools panel splits vertically — editor on the left, rendered preview on the right
- Native ratatui 0.29 renderer: headings, bold, italic, inline code, fenced blocks, bullets, ordered lists, blockquotes, horizontal rules
- Preview updates on every keystroke — no background job, zero latency

### Preview panel
- Syntax-highlighted file preview for source files (F3)
- Markdown files rendered as formatted text
- Binary file detection with size display

### Git integration
- Per-file status indicators in both panes (`M` modified, `A` added, `?` untracked, `D` deleted, `R` renamed, `U` conflicted)
- Current branch name in the status bar
- Refreshes automatically alongside every directory scan
- No git crate dependency — shells out to `git` on PATH; absent or non-repo paths are silently skipped

### Navigation & UX
- Menu bar with File, Navigate, View, Help menus (keyboard mnemonics + mouse click)
- Command palette (`Shift+P`) with a dedicated Workspaces section
- Navigate menu starts with explicit `Switch to Workspace 1..4` commands
- `q` / `F10` to quit; `F2` toggle terminal; `F9` toggle diff mode
- Direct workspace switching uses `Alt+1..Alt+4` as the primary shortcut; `Shift+1..Shift+4` is also supported as a terminal fallback. The active workspace also remains visible as `ws:N/4` in the status bar
- In-app settings panel for theme, icon mode, layout, and preview preferences (`Ctrl+O`)
- Full mouse support: click to focus panes, scroll to navigate, click menu items, hover highlights
- Zeta is the default theme; Fjord, Sandbar, Oxide, Matrix, Norton, Dracula, Neon, and Monochrome remain available
- Unicode icons by default, ASCII fallback, custom icon font mode

---

## Key bindings (defaults)

| Key | Action |
|---|---|
| `Alt+1..4` | Switch to workspace 1..4 |
| `Shift+1..4` | Switch workspace (terminal fallback) |
| `F2` | Toggle embedded terminal |
| `F3` | Toggle file preview panel |
| `F4` | Open selected file in editor |
| `F5` | Copy |
| `F6` | Rename |
| `Shift+F6` | Move |
| `F7` | New directory |
| `F8` | Delete |
| `F9` | Toggle diff mode |
| `F10` / `q` | Quit |
| `Ins` | New file |
| `Tab` | Switch active pane |
| `Shift+P` | Command palette |
| `Ctrl+O` | Settings |
| `Alt+F/N/V/H` | Open menu |
| `Alt+←/→` | Navigate directory history |
| `Alt+F3` | Focus preview panel |

---

## Tech stack

| Crate | Purpose |
|---|---|
| `crossterm 0.28` | Terminal I/O, raw mode, mouse capture |
| `ratatui 0.29` | Rendering and layout |
| `ropey 1.6` | O(log n) editor buffer |
| `syntect 5.3` | Syntax highlighting |
| `crossbeam-channel 0.5` | Background worker messaging |
| `serde` + `toml` | Config serialisation |
| `thiserror` | Typed error handling |

---

## Architecture

```
App (event loop)
├── AppState (global shell state)
│   ├── workspaces: [WorkspaceState; 4]
│   ├── active_workspace_idx
│   ├── OverlayState    — menus, prompts, dialogs, palette
│   └── shared config/theme/runtime shell state
├── WorkspaceState
│   ├── PaneSetState    — dual-pane navigation and selection
│   ├── EditorState     — editor buffer + search + preview state
│   ├── PreviewState    — preview panel content and scroll
│   ├── TerminalState   — per-workspace terminal session state
│   └── local transient state — git cache, progress, pending batch/collision, scan timing
├── WorkerChannels (bounded background workers)
│   ├── Scan / FileOp / Preview / Git / Finder / Archive / DirSize / Watch / Terminal
│   └── workspace-scoped requests/results carry workspace identity where needed
└── FocusLayer          — compiler-enforced input routing
    (Pane | Editor | Preview | MarkdownPreview | Modal(…))
```

Key design decisions:
- One UI thread plus bounded worker threads. No async runtime.
- `WorkspaceState` keeps full task context isolated inside one process; switching workspaces is instant.
- `FocusLayer` enum makes illegal input-routing states unrepresentable.
- Editor uses `ropey::Rope` (B-tree) for O(log n) insert/delete on large files.
- Highlight cache keyed by `edit_version` — syntect runs once per edit, not per frame.
- Git status is a fire-and-forget background job triggered alongside every scan and routed back to the launching workspace.
Full ADR: [`docs/adr-0001-architecture.md`](docs/adr-0001-architecture.md)

---

## Project layout

```
src/
  app.rs              — event loop, command dispatch, mouse routing
  state/              — AppState and all sub-states
  ui/                 — rendering modules (pane, editor, preview, markdown, …)
  editor.rs           — EditorBuffer (ropey backend, delta undo)
  jobs.rs             — WorkerChannels, four background workers
  git.rs              — RepoStatus, parse_porcelain, detect_repo
  fs.rs               — filesystem operations
  highlight.rs        — syntect wrapper
  action.rs           — Action enum, FocusLayer-aware key routing
  config.rs           — AppConfig, ThemePalette, keymaps
docs/
  superpowers/plans/  — wave-by-wave development plans and ROADMAP.md
  adr-0001-architecture.md
assets/
  fonts/              — bundled zeta-icons.ttf (custom icon mode)
```

---

## Running locally

```bash
cargo run --
```

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --release
```

---

## Configuration

Config is loaded from (in order of preference):

1. `ZETA_CONFIG` environment variable path
2. `%APPDATA%\zeta\config.toml` (Windows)
3. `~/.config/zeta/config.toml` (Linux/macOS)

```toml
# Theme: "zeta" (default) | "fjord" | "sandbar" | "oxide" | "matrix" | "norton" | "dracula" | "neon" | "monochrome"
theme = "zeta"

# Icon mode: "unicode" (default) | "ascii" | "custom"
# For "custom", install assets/fonts/zeta-icons.ttf in your terminal first.
icon_mode = "unicode"

# Open preview panel on startup
preview_panel_open = false

# Auto-preview on cursor move
preview_on_selection = true
```

Access settings at runtime with `Ctrl+O` or via the View menu.

---

## Roadmap

See [`docs/superpowers/plans/ROADMAP.md`](docs/superpowers/plans/ROADMAP.md) for the full development plan.

Core features — dual-pane navigation, embedded editor, integrated terminal, SSH/SFTP, diff mode, four workspaces, command palette, and markdown preview — are all shipped as of v0.3.x.
---

## License

MIT — see [LICENSE](LICENSE) for the full text.
