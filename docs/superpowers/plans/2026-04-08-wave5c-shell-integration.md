# Wave 5C — Shell Integration

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Press F2 (or a configurable key) to open a shell in the active pane's current directory. On Windows this is PowerShell; on Unix it is `$SHELL` or `/bin/bash`. Zeta suspends its TUI, hands the terminal to the shell, and restores the TUI when the shell exits.

This is the simplest and most reliable approach — no terminal emulator embedding, no PTY management. The shell gets the full raw terminal. When done, Zeta resumes exactly where it left off.

**No new dependencies.** Uses `std::process::Command`.

**Jira:** ZTA-132 (ZTA-151 through ZTA-155)

**Wave dependency:** Starts AFTER Wave 5B.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/action.rs` | `OpenShell` action |
| Create | `src/shell.rs` | `open_shell(cwd: &Path) -> Result<()>` |
| Modify | `src/lib.rs` | `pub mod shell;` |
| Modify | `src/app.rs` | Handle `Command::OpenShell` — suspend TUI, run shell, restore |
| Modify | `src/action.rs` | Wire `F2` → `OpenShell` in `from_pane_key_event` |
| Modify | `src/state/mod.rs` | `OpenShell` → `Command::OpenShell { path }` |

---

## Architecture

The key insight: ratatui's TUI is just raw mode + alternate screen. Suspending it means:

```
1. Disable raw mode
2. Leave alternate screen
3. Show cursor
4. Spawn shell and wait for it to exit
5. Re-enable raw mode
6. Enter alternate screen
7. Clear and redraw
```

This maps directly to what `TerminalSession::drop` already does (steps 1–3) and `TerminalSession::enter` (steps 5–7).

Since we can't easily "pause" the `App::run` loop mid-way, the cleanest approach is:
- `Command::OpenShell { path: PathBuf }` is a new `Command` variant.
- `execute_command` handles it by calling `open_shell(&path)` which does all 7 steps above.
- The `App` struct doesn't need to change — `execute_command` already has mutable access to everything it needs.

---

## Implementation

### `src/shell.rs`

```rust
use std::path::Path;
use anyhow::{Context, Result};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    cursor::Show,
};

/// Suspend the TUI, open a shell in `cwd`, then restore.
/// Blocks until the shell exits.
pub fn open_shell(cwd: &Path) -> Result<()> {
    // 1–3: Suspend TUI.
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(std::io::stdout(), LeaveAlternateScreen, Show)
        .context("failed to leave alternate screen")?;

    // 4: Spawn shell and wait.
    let shell = detect_shell();
    let status = std::process::Command::new(&shell)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("failed to spawn shell: {shell}"))?;

    if !status.success() {
        eprintln!("[zeta] shell exited with status: {status}");
    }

    // 5–6: Restore TUI.
    enable_raw_mode().context("failed to re-enable raw mode")?;
    execute!(std::io::stdout(), EnterAlternateScreen)
        .context("failed to re-enter alternate screen")?;

    Ok(())
}

fn detect_shell() -> String {
    #[cfg(windows)]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| String::from("powershell.exe"))
    }
    #[cfg(not(windows))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| String::from("/bin/sh"))
    }
}
```

### `Command::OpenShell` variant

```rust
pub enum Command {
    // ... existing ...
    OpenShell { path: PathBuf },
}
```

### Action → Command in `AppState::apply_view`

```rust
Action::OpenShell => {
    let path = self.panes.active_pane().cwd.clone();
    commands.push(Command::OpenShell { path });
}
```

### `execute_command` in `app.rs`

```rust
Command::OpenShell { path } => {
    crate::shell::open_shell(&path)
        .unwrap_or_else(|e| {
            self.state.set_error_status(format!("shell error: {e}"));
        });
    // Force a full redraw after returning from shell.
    self.state.mark_dirty();
}
```

### Keybinding

In `from_pane_key_event`:
```rust
KeyCode::F(2) => Some(Self::OpenShell),
```

> **Note:** F2 was unused in the default keymap as of Wave 2A. If it gets assigned elsewhere before this wave, use `Ctrl+Shift+S` as the fallback.

---

## Tests

Shell integration is hard to unit test (requires a real PTY). Add integration-style tests that verify the action → command dispatch path without actually spawning a shell:

```rust
#[test]
fn f2_in_pane_dispatches_open_shell_command() {
    let mut state = test_state();
    let commands = state.apply(Action::OpenShell).unwrap();
    assert!(commands.iter().any(|c| matches!(c, Command::OpenShell { .. })));
}

#[test]
fn open_shell_uses_active_pane_cwd() {
    let mut state = test_state();
    state.panes.left.cwd = PathBuf::from("/tmp/mydir");
    let commands = state.apply(Action::OpenShell).unwrap();
    if let Some(Command::OpenShell { path }) = commands.first() {
        assert_eq!(path, &PathBuf::from("/tmp/mydir"));
    } else {
        panic!("expected OpenShell command");
    }
}
```

---

## Final verification

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
# Manual test: press F2, verify shell opens in current directory, type `exit`, verify Zeta restores
git commit -m "chore: Wave 5C complete — shell integration (F2 opens shell in cwd)"
```
