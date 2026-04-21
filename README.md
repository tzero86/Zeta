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

### Integrated Terminal
- Run shell commands directly in Zeta without switching windows
- Press `F2` to toggle the integrated terminal panel in the current workspace (or `Ctrl+\` as alternate binding)
- Runs your native shell: bash, zsh, pwsh, cmd.exe, fish, and others all work
- Full shell passthrough: Tab completion, all standard keyboard shortcuts work as expected
- Cross-platform support: Windows (ConPTY), macOS, and Linux all supported
- Auto-detects best available shell (PowerShell 7+ on Windows, bash/zsh on Unix)
- Each workspace has its own independent terminal session — switch workspaces to have multiple shell contexts open simultaneously

See [Terminal Behavior Guide](docs/TERMINAL_BEHAVIOR.md) for keyboard mappings, Tab completion troubleshooting, shell detection, and platform-specific tips.

### SSH/SFTP Remote Filesystems
- Connect to remote servers via SSH/SFTP directly from the file manager
- Authenticate using passwords, key files, or SSH Agent integration
- Strict host key verification against `~/.ssh/known_hosts` for secure connections
- Full support for remote file operations (copy, move, delete, rename, etc.)
- Transparent cross-filesystem support (copy from local to remote or vice versa)
- Use the SSH Connect dialog (opened via command palette or menu) to initiate a session

#### Connection Requirements

Before connecting, ensure you have:
- **OpenSSH client** installed (typically included on Linux/macOS; download from [openssh.com](https://www.openssh.com) for Windows)
- **`known_hosts` file** at `~/.ssh/known_hosts` (created automatically after first SSH connections)
- **Authentication credentials**: one of password, key file path, or SSH Agent configured

#### Authentication Priority

Zeta respects your explicit authentication choice in the dialog:

1. **SSH Agent (Recommended)** — Select Agent to use `SSH_AUTH_SOCK` if available. If Agent is unavailable or has no matching keys, connection fails with a clear error (no silent fallback).
2. **Password** — Select Password to authenticate with a password (Agent is skipped).
3. **Key File** — Select Key File to authenticate with a private key (Agent is skipped).

**Best Practices:**
- **SSH Agent is the recommended method** because:
  - Secure: private keys never leave your SSH Agent process
  - Convenient: no need to re-enter passwords for every connection
  - Supports passphrases: Agent caches unlocked keys during your session
- Set up SSH Agent on your machine:
  ```bash
  # Start SSH Agent (usually done in ~/.profile or ~/.bashrc)
  eval "$(ssh-agent)"
  # Add your keys to the agent
  ssh-add ~/.ssh/id_rsa
  # Verify SSH_AUTH_SOCK is set
  echo $SSH_AUTH_SOCK
  ```
- The SSH Connect dialog now shows "[Agent: Available]" or "[Agent: Not Available]" to help you understand why Agent selection might fail.
- If Agent selection fails with "No matching identity in SSH Agent", ensure your key is added via `ssh-add ~/.ssh/id_rsa`.
- **Fallback:** Use Password or Key File if Agent is unavailable, then restart Agent when possible.

#### Troubleshooting SSH/SFTP Connections

| Issue | Diagnosis | Solution |
|---|---|---|
| "SSH Agent not available" | SSH Agent is not running or `SSH_AUTH_SOCK` is not set | Run `eval "$(ssh-agent)"` to start agent, add keys with `ssh-add`, or select Password/Key File instead |
| "No matching identity in SSH Agent" | Agent is running but doesn't have the key needed for this host | Run `ssh-add ~/.ssh/id_rsa` to add your key, or use Password/Key File instead |
| "Authentication failed" | Wrong password or key file, or key permissions invalid | Verify credentials with `ssh user@host` from terminal first. For key files, ensure `chmod 600 ~/.ssh/id_rsa` |
| "Host key changed" (red error) | Known host key mismatch — possible security issue | Run `ssh-keygen -R host.example.com` to remove old entry, then retry |
| "Connection timeout" | Host unreachable or network issue | Check host is online with `ping` or `ssh` from terminal |
| "Host key not recognized" | New host, need manual verification | Verify the fingerprints (SHA256 preferred, MD5 for legacy compatibility) with server admin, then press Enter in the trust prompt |
| "Permission denied" (local copy issues) | SSH key file has wrong permissions | Run `chmod 600 ~/.ssh/id_rsa` and ensure ownership with `chown $USER ~/.ssh` |

**Key File Permissions:** SSH keys must be readable only by you. If you see "bad permissions" errors, run:
```bash
chmod 700 ~/.ssh
chmod 600 ~/.ssh/id_rsa
chown -R $USER ~/.ssh
```

#### Host Key Verification

When connecting to a host for the first time, Zeta presents a trust prompt showing the server's host key fingerprints for manual verification. This prevents man-in-the-middle (MITM) attacks by allowing you to confirm the server's identity before trusting its connection.

**Fingerprints displayed:**
- **SHA256** (preferred, modern format) — `SHA256:` prefix followed by base64-encoded hash
- **MD5** (legacy format, included for compatibility) — hex colon-separated hash

**Verification workflow:**
1. When connecting to an unknown host, Zeta shows a dialog with both fingerprints
2. Contact the server administrator out-of-band (email, phone, etc.) and compare the fingerprints
3. If they match, press Enter/Y to trust the host and continue
4. The host key is stored in `~/.ssh/known_hosts` for future connections
5. If the key changes on a future connection, Zeta will show a red security warning

**Verifying fingerprints manually from the command line:**
```bash
# Get SSH server's SHA256 fingerprint
ssh-keyscan -H example.com 2>/dev/null | ssh-keygen -lf - -E SHA256

# Get MD5 fingerprint (legacy)
ssh-keyscan -H example.com 2>/dev/null | ssh-keygen -lf - -E MD5
```

**Security Test Plan:**
- **Host Key Verification Failure:** Attempt to connect to a host with an invalid or changed host key in `~/.ssh/known_hosts`. The UI will reject the connection with a red error: `Host key changed! Possible MITM attack. Investigate manually.`
- **SSH Agent Authentication:** Run `ssh-agent`, add a key via `ssh-add`, and attempt an SSH connection without providing a password or key file. The connection will succeed automatically using the agent's identities.
- **Agent Unavailable:** Close SSH Agent and attempt connection; Zeta will show an informational message and fall back to password/key file if provided.
- **Unknown Host Trust Prompt:** Connect to a new SSH server not in `~/.ssh/known_hosts`. Verify both SHA256 and MD5 fingerprints match the server admin's values, then press Enter to trust and connect.

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
