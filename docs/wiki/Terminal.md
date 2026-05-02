# Integrated Terminal

Zeta includes an integrated terminal panel in every workspace so you can run shell commands without leaving the file manager.

---

## Opening and Closing

| Key | Action |
|---|---|
| `F2` | Toggle the terminal panel in the current workspace |
| `Ctrl+\` | Alternate binding to toggle terminal |

The terminal panel opens at the bottom of the current workspace. Press `F2` again (or `Ctrl+\`) to hide it.

---

## Shell Support

Zeta auto-detects the best available shell for your platform:

| Platform | Default |
|---|---|
| Windows | PowerShell 7+ (`pwsh`), falls back to `powershell.exe` then `cmd.exe` |
| Linux / macOS | `$SHELL` env var, falls back to `bash` then `sh` |

You can override the shell in your config:

```toml
[terminal]
shell = "/bin/zsh"
```

All shells work: `bash`, `zsh`, `fish`, `pwsh`, `cmd.exe`, and others.

---

## Full Shell Passthrough

The integrated terminal is a proper PTY (pseudo-terminal), not a command runner:

- **Tab completion** works exactly as in your standalone terminal
- All standard keyboard shortcuts work: `Ctrl+C`, `Ctrl+D`, `Ctrl+L`, history navigation, etc.
- Colors, cursor movement, and interactive programs (e.g., `vim`, `htop`) all render correctly
- Cross-platform: uses **ConPTY** on Windows 10 1809+, standard PTY on Unix

---

## Workspace Isolation

Each of Zeta's four workspaces has its own independent terminal session.

- Switching workspaces does not kill or pause the terminal in the previous workspace
- You can have four separate shell sessions running simultaneously
- Use `Alt+1`–`Alt+4` to switch workspaces while keeping shells alive

**Practical use:** Start a `cargo watch` build in workspace 2's terminal while browsing files in workspace 1, and keep a git log open in workspace 3.

---

## Platform Notes

### Windows

- Requires Windows 10 version 1809 or later (ConPTY support)
- PowerShell 7+ is recommended for the best experience
- The `conpty` crate handles terminal emulation

### Linux / macOS

- Standard POSIX PTY — no extra dependencies required
- Fish shell users: fish works as expected; Zeta does not interfere with fish's initialization

---

*For deeper troubleshooting and keyboard mapping details, see [`docs/TERMINAL_BEHAVIOR.md`](https://github.com/tzero86/Zeta/blob/main/docs/TERMINAL_BEHAVIOR.md) in the repository.*

---

*See also: [Key Bindings](Key-Bindings) · [Configuration](Configuration) · [File Management](File-Management)*
