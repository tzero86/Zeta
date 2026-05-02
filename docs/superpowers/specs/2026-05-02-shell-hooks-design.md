# Shell Hook System — Design Spec

**Date:** 2026-05-02  
**Phase:** 4  
**Status:** Approved

---

## Overview

When Zeta detects a trigger event (directory change, file open, app start, app exit), it runs
zero or more user-defined shell commands fire-and-forget, off the main thread. Output is silently
discarded; a non-zero exit code surfaces as a status-bar error. Hooks are defined in `config.toml`
alongside existing `openers`.

---

## Configuration

Hooks are declared as a `[[hooks]]` array in `config.toml`. Each entry maps one event name to one
shell command string. Multiple entries with the same event name are all executed (in declaration
order) when that event fires.

```toml
[[hooks]]
event = "on_cd"
command = "~/.config/zeta/hooks/on_cd.sh"

[[hooks]]
event = "on_open"
command = "notify-send \"Zeta opened $ZETA_PATH\""

[[hooks]]
event = "on_start"
command = "echo zeta started >> /tmp/zeta.log"

[[hooks]]
event = "on_exit"
command = "echo zeta exiting >> /tmp/zeta.log"
```

The `command` string is passed to `sh -c "<command>"` so standard shell quoting, redirection, and
`~` expansion work as expected.

### New config types (`src/config.rs`)

```rust
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    OnCd,
    OnOpen,
    OnStart,
    OnExit,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HookConfig {
    pub event: HookEvent,
    pub command: String,
}
```

`AppConfig` gains:

```rust
#[serde(default)]
pub hooks: Vec<HookConfig>,
```

---

## Environment Variables

Each hook event receives a set of `ZETA_*` environment variables. No template substitution in the
command string — all context is delivered through the environment.

| Event | Variables |
|-------|-----------|
| `on_cd` | `ZETA_PATH` (new directory), `ZETA_OLD_PATH` (previous directory), `ZETA_PANE` (`left` or `right`) |
| `on_open` | `ZETA_PATH` (absolute path of file being opened), `ZETA_PANE` (`left` or `right`) |
| `on_start` | `ZETA_PATH` (initial working directory of the active pane), `ZETA_VERSION` (binary version string) |
| `on_exit` | `ZETA_PATH` (current working directory of the active pane at quit time) |

---

## Architecture

### New module: `src/hooks.rs`

Single pure-logic module with no I/O side effects in its primary types. Exposes:

```rust
/// Spawn all hooks for `event` fire-and-forget. Returns one `Command::RunHook` per matching entry.
pub fn commands_for_event(
    hooks: &[HookConfig],
    event: HookEvent,
    env: HookEnv,
) -> Vec<Command>
```

`HookEnv` is a plain struct holding the optional env-var values for a given event:

```rust
pub struct HookEnv {
    pub path: Option<PathBuf>,
    pub old_path: Option<PathBuf>,
    pub pane: Option<String>,   // "left" or "right"
    pub version: Option<String>,
}
```

### New `Command` variant (`src/action.rs`)

```rust
Command::RunHook {
    command: String,        // the raw sh -c argument
    env: Vec<(String, String)>, // ZETA_* key-value pairs
}
```

### Execution (`src/app.rs` — `execute_command_try`)

`Command::RunHook` is handled in the existing `execute_command_try` match:

```rust
Command::RunHook { command, env } => {
    let mut child = std::process::Command::new("sh");
    child.arg("-c").arg(&command);
    for (k, v) in &env { child.env(k, v); }
    child.stdout(std::process::Stdio::null());
    child.stderr(std::process::Stdio::piped());
    match child.spawn() {
        Ok(mut child) => {
            // Collect stderr on a detached thread; report non-zero exit to status bar via a
            // dispatched Action::HookFailed message.
            std::thread::spawn(move || {
                let output = child.wait_with_output();
                // result sent back via existing job result channel if non-zero
            });
        }
        Err(e) => self.state.set_status_error(format!("hook spawn failed: {e}")),
    }
}
```

Hook errors (non-zero exit, spawn failure) surface as `set_status_error` messages. They are
non-fatal and do not affect Zeta's state.

### Trigger points

| Event | Location | Condition |
|-------|----------|-----------|
| `on_cd` | `AppState::apply_job_result` — `JobResult::DirectoryScanned` branch | Only when the scanned pane's `cwd` differs from the pre-scan value (actual navigation, not just a refresh) |
| `on_open` | `AppState::apply` — `Action::OpenSelection` branch | Always when a file (not directory) is opened |
| `on_start` | `AppState::initial_commands()` | Once, unconditionally, after all other startup commands |
| `on_exit` | `App::run()` — after the main loop exits, before session save | Once, synchronously (hooks fire before Zeta exits; child processes are detached so they may outlive Zeta) |

---

## Error Handling

- **Spawn failure** (command not found, permission denied): logged immediately to status bar via
  `set_status_error`.
- **Non-zero exit code**: collected on a background thread; status bar error shows the hook command
  and exit code. Stderr output (first 200 bytes) is appended to the message.
- **Hook timeout**: none enforced. Fire-and-forget; hooks that hang are the user's responsibility.
  Zeta never blocks waiting for a hook.
- **`on_exit` hook**: spawned before session save. Because hooks are `spawn()` (not
  `wait()`), they run as detached processes and may outlive the Zeta process. This is acceptable
  and documented.

---

## Testing

### Unit tests (`src/hooks.rs`)

- `commands_for_event` returns empty `Vec` when no hooks match the event.
- `commands_for_event` returns one `Command::RunHook` per matching hook entry.
- Multiple hooks for the same event produce multiple commands in declaration order.
- `HookEnv` fields map to the correct `ZETA_*` env-var names.
- Hooks for other events are not included.

### Integration tests (`tests/hooks_integration.rs`)

- `on_cd` hook fires when `DirectoryScanned` result arrives with a changed path; does not fire
  when the directory is unchanged (refresh).
- `on_open` hook fires on `Action::OpenSelection` for a file, not for a directory.
- `on_start` hook command appears in `initial_commands()` output when a hook is configured.
- Config round-trips: `[[hooks]]` TOML serialises and deserialises correctly via `basic_toml`.

---

## Files Changed

| File | Change |
|------|--------|
| `src/config.rs` | Add `HookEvent`, `HookConfig`; add `hooks` field to `AppConfig`; add to `generate_annotated_config` |
| `src/hooks.rs` | New module: `HookEnv`, `commands_for_event` |
| `src/action.rs` | Add `Command::RunHook { command, env }` |
| `src/state/mod.rs` | Fire `on_cd` in `apply_job_result`; fire `on_open` in `apply`; fire `on_start` in `initial_commands` |
| `src/app.rs` | Handle `Command::RunHook` in `execute_command_try`; fire `on_exit` after main loop |
| `src/lib.rs` | Declare `pub mod hooks` |
| `tests/hooks_integration.rs` | New integration test file |

---

## Out of Scope (this phase)

- Blocking / cancellable hooks (pre-cd, pre-open)
- Script-directory-based hooks (`~/.config/zeta/hooks/`)
- Hook output visible in a Zeta panel
- Per-workspace or per-pane hook filtering
- Plugin system
