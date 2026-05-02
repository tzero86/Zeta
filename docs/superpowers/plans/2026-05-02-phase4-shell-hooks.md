# Shell Hook System — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a fire-and-forget shell hook system so users can run arbitrary commands when Zeta changes directory, opens a file, starts, or exits.

**Architecture:** New pure-logic `src/hooks.rs` module converts `HookConfig` + `HookEnv` into `Command::RunHook` values; `execute_command_try` in `app.rs` spawns them off the main thread via `sh -c`; trigger points in `state/mod.rs` and `app.rs` emit the commands at the right moments. Error reporting (non-zero exit, spawn failure) surfaces through the existing status-bar mechanism.

**Tech Stack:** Rust stable, `std::process::Command` for spawning, `serde` + `basic_toml` for config, existing `Command` enum / `execute_command_try` dispatch pattern.

**Branch:** `feat/phase4-shell-hooks`

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src/config.rs` | Modify | Add `HookEvent`, `HookConfig` types; `hooks` field on `AppConfig`; hooks section in `generate_annotated_config` |
| `src/hooks.rs` | Create | `HookEnv` struct; `commands_for_event` pure function; unit tests |
| `src/action.rs` | Modify | Add `Command::RunHook { command: String, env: Vec<(String, String)> }` variant |
| `src/lib.rs` | Modify | Declare `pub mod hooks` |
| `src/state/mod.rs` | Modify | Fire `on_cd` in `apply_job_result`; fire `on_open` in `apply`; fire `on_start` in `initial_commands` |
| `src/app.rs` | Modify | Handle `Command::RunHook` in `execute_command_try`; fire `on_exit` after main loop |
| `tests/hooks_integration.rs` | Create | Integration tests for trigger points and config round-trip |

---

## Task 1: Config types — `HookEvent`, `HookConfig`, `AppConfig.hooks`

**Files:**
- Modify: `src/config.rs`

### Context

`AppConfig` is defined at line 45 of `src/config.rs`. The struct uses `#[serde(default)]` for all optional fields. `generate_annotated_config` at line 247 builds the annotated TOML string — it already handles `openers` as a `Vec` with a `for` loop. Follow that pattern for `hooks`.

- [ ] **Write failing unit test** (add inside the existing `#[cfg(test)]` block at the bottom of `src/config.rs`):

```rust
#[test]
fn hook_config_round_trips() {
    use crate::config::{AppConfig, HookConfig, HookEvent};
    let mut cfg = AppConfig::default();
    cfg.hooks = vec![
        HookConfig { event: HookEvent::OnCd, command: String::from("echo cd") },
        HookConfig { event: HookEvent::OnOpen, command: String::from("echo open") },
    ];
    let text = generate_annotated_config(&cfg);
    let parsed: AppConfig = basic_toml::from_str(&text).expect("valid TOML");
    assert_eq!(parsed.hooks.len(), 2);
    assert_eq!(parsed.hooks[0].event, HookEvent::OnCd);
    assert_eq!(parsed.hooks[0].command, "echo cd");
    assert_eq!(parsed.hooks[1].event, HookEvent::OnOpen);
}
```

- [ ] **Run test to verify it fails**

```bash
cargo test --lib config::tests::hook_config_round_trips -- --exact --nocapture
```

Expected: compile error — `HookEvent`, `HookConfig` do not exist yet.

- [ ] **Add types and field** — in `src/config.rs`, before `pub struct AppConfig`:

```rust
/// The lifecycle event that triggers a user-defined hook command.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    OnCd,
    OnOpen,
    OnStart,
    OnExit,
}

/// A single user-defined shell hook entry from `[[hooks]]` in `config.toml`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HookConfig {
    pub event: HookEvent,
    pub command: String,
}
```

Add `hooks` field to `AppConfig` (scalar fields must come before nested table fields — add it just before `pub theme: ThemeConfig`):

```rust
#[serde(default)]
pub hooks: Vec<HookConfig>,
```

Add `hooks: Vec::new()` to `AppConfig::default()`.

- [ ] **Add hooks section to `generate_annotated_config`** — after the `openers` block, before the final `format!` call, build the hooks string and insert it:

```rust
let mut hooks_toml = String::new();
for hook in &config.hooks {
    let event_str = match hook.event {
        HookEvent::OnCd => "on_cd",
        HookEvent::OnOpen => "on_open",
        HookEvent::OnStart => "on_start",
        HookEvent::OnExit => "on_exit",
    };
    hooks_toml.push_str(&format!(
        "\n[[hooks]]\nevent = \"{event_str}\"\ncommand = \"{}\"\n",
        esc(&hook.command)
    ));
}
```

Then add to the format string (after the openers section at the end):

```
# Shell hooks — run commands when Zeta changes directory, opens a file, starts, or exits.\n\
# Available events: \"on_cd\", \"on_open\", \"on_start\", \"on_exit\"\n\
# Environment variables: ZETA_PATH, ZETA_OLD_PATH (on_cd), ZETA_PANE, ZETA_VERSION (on_start)\n\
# Example:\n\
#   [[hooks]]\n\
#   event = \"on_cd\"\n\
#   command = \"~/.config/zeta/hooks/on_cd.sh\"\n\
{hooks_toml}",
```

- [ ] **Run test to verify it passes**

```bash
cargo test --lib config::tests::hook_config_round_trips -- --exact --nocapture
```

Expected: PASS

- [ ] **Run full test suite to check for regressions**

```bash
cargo test --workspace --quiet
```

Expected: same pass count as before (439/441, 2 pre-existing failures unrelated to hooks).

- [ ] **Commit**

```bash
git add src/config.rs
git commit -m "feat(hooks): add HookEvent, HookConfig types and AppConfig.hooks field"
```

---

## Task 2: `Command::RunHook` variant

**Files:**
- Modify: `src/action.rs`

### Context

`pub enum Command` is defined at line 294 of `src/action.rs`. Add a new variant at the end of the enum (before the closing `}`). The `env` field carries pre-computed `ZETA_*` key-value pairs so the executor doesn't need to know about `HookEvent`.

- [ ] **Write failing test** (add in `src/action.rs` tests or a standalone compile check — the simplest is a doc-test style in the enum):

Add inside the existing `#[cfg(test)]` block in `src/action.rs`:

```rust
#[test]
fn run_hook_command_builds() {
    let cmd = Command::RunHook {
        command: String::from("echo hi"),
        env: vec![(String::from("ZETA_PATH"), String::from("/tmp"))],
    };
    if let Command::RunHook { command, env } = cmd {
        assert_eq!(command, "echo hi");
        assert_eq!(env[0].0, "ZETA_PATH");
    } else {
        panic!("expected RunHook");
    }
}
```

- [ ] **Run test to verify it fails**

```bash
cargo test --lib -- run_hook_command_builds --exact --nocapture
```

Expected: compile error — `Command::RunHook` does not exist.

- [ ] **Add the variant** — append to `pub enum Command` in `src/action.rs`:

```rust
RunHook {
    /// The raw argument passed to `sh -c`.
    command: String,
    /// Pre-computed `ZETA_*` environment variable pairs for this event.
    env: Vec<(String, String)>,
},
```

- [ ] **Run test to verify it passes**

```bash
cargo test --lib -- run_hook_command_builds --exact --nocapture
```

Expected: PASS

- [ ] **Check clippy (exhaustive match)**

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | grep -i "RunHook\|non-exhaustive\|match"
```

If any existing `match command { ... }` in `app.rs` is non-exhaustive, add a `Command::RunHook { .. } => {}` placeholder arm for now (it will be replaced in Task 5).

- [ ] **Commit**

```bash
git add src/action.rs
git commit -m "feat(hooks): add Command::RunHook variant"
```

---

## Task 3: `src/hooks.rs` — pure hook logic + unit tests

**Files:**
- Create: `src/hooks.rs`
- Modify: `src/lib.rs`

### Context

This module is intentionally pure: no I/O, no spawning. It converts `&[HookConfig]` + `HookEvent` + `HookEnv` into `Vec<Command>`. The `HookEnv` struct holds optional values; `build_env_vars` converts them to `Vec<(String, String)>` using the correct `ZETA_*` keys per event.

`PaneFocus` is `pub enum PaneFocus { Left, Right, Preview }` from `src/state/types.rs`.

- [ ] **Declare module** — add to `src/lib.rs` (keep alphabetical order with other `pub mod` lines):

```rust
pub mod hooks;
```

- [ ] **Write failing unit tests** — create `src/hooks.rs` with only the tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Command;
    use crate::config::{HookConfig, HookEvent};
    use std::path::PathBuf;

    fn cd_hooks() -> Vec<HookConfig> {
        vec![
            HookConfig { event: HookEvent::OnCd, command: String::from("echo cd1") },
            HookConfig { event: HookEvent::OnCd, command: String::from("echo cd2") },
        ]
    }

    #[test]
    fn no_matching_hooks_returns_empty() {
        let hooks = cd_hooks();
        let cmds = commands_for_event(&hooks, HookEvent::OnOpen, HookEnv::default());
        assert!(cmds.is_empty());
    }

    #[test]
    fn matching_hooks_return_one_command_each() {
        let hooks = cd_hooks();
        let cmds = commands_for_event(&hooks, HookEvent::OnCd, HookEnv::default());
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn commands_are_in_declaration_order() {
        let hooks = cd_hooks();
        let cmds = commands_for_event(&hooks, HookEvent::OnCd, HookEnv::default());
        if let Command::RunHook { command, .. } = &cmds[0] {
            assert_eq!(command, "echo cd1");
        } else { panic!("expected RunHook"); }
        if let Command::RunHook { command, .. } = &cmds[1] {
            assert_eq!(command, "echo cd2");
        } else { panic!("expected RunHook"); }
    }

    #[test]
    fn on_cd_env_vars_are_correct() {
        let hooks = vec![HookConfig { event: HookEvent::OnCd, command: String::from("x") }];
        let env_in = HookEnv {
            path: Some(PathBuf::from("/new")),
            old_path: Some(PathBuf::from("/old")),
            pane: Some(String::from("left")),
            version: None,
        };
        let cmds = commands_for_event(&hooks, HookEvent::OnCd, env_in);
        if let Command::RunHook { env, .. } = &cmds[0] {
            let map: std::collections::HashMap<_, _> = env.iter().cloned().collect();
            assert_eq!(map.get("ZETA_PATH").map(|s| s.as_str()), Some("/new"));
            assert_eq!(map.get("ZETA_OLD_PATH").map(|s| s.as_str()), Some("/old"));
            assert_eq!(map.get("ZETA_PANE").map(|s| s.as_str()), Some("left"));
            assert!(!map.contains_key("ZETA_VERSION"));
        } else { panic!("expected RunHook"); }
    }

    #[test]
    fn on_start_env_includes_version() {
        let hooks = vec![HookConfig { event: HookEvent::OnStart, command: String::from("x") }];
        let env_in = HookEnv {
            path: Some(PathBuf::from("/home/user")),
            old_path: None,
            pane: None,
            version: Some(String::from("0.4.6")),
        };
        let cmds = commands_for_event(&hooks, HookEvent::OnStart, env_in);
        if let Command::RunHook { env, .. } = &cmds[0] {
            let map: std::collections::HashMap<_, _> = env.iter().cloned().collect();
            assert_eq!(map.get("ZETA_VERSION").map(|s| s.as_str()), Some("0.4.6"));
            assert_eq!(map.get("ZETA_PATH").map(|s| s.as_str()), Some("/home/user"));
        } else { panic!("expected RunHook"); }
    }

    #[test]
    fn none_env_fields_are_omitted() {
        let hooks = vec![HookConfig { event: HookEvent::OnExit, command: String::from("x") }];
        let env_in = HookEnv { path: None, old_path: None, pane: None, version: None };
        let cmds = commands_for_event(&hooks, HookEvent::OnExit, env_in);
        if let Command::RunHook { env, .. } = &cmds[0] {
            assert!(env.is_empty(), "None fields must not appear in env");
        } else { panic!("expected RunHook"); }
    }
}
```

- [ ] **Run tests to verify they fail**

```bash
cargo test --lib hooks:: -- --nocapture
```

Expected: compile errors — `commands_for_event`, `HookEnv` not defined.

- [ ] **Write the implementation** — add above the `#[cfg(test)]` block in `src/hooks.rs`:

```rust
use std::path::PathBuf;

use crate::action::Command;
use crate::config::{HookConfig, HookEvent};

/// Context values passed to a hook event; `None` fields are omitted from the environment.
#[derive(Clone, Debug, Default)]
pub struct HookEnv {
    /// New directory (on_cd, on_open, on_start, on_exit).
    pub path: Option<PathBuf>,
    /// Previous directory (on_cd only).
    pub old_path: Option<PathBuf>,
    /// Active pane label: "left" or "right" (on_cd, on_open).
    pub pane: Option<String>,
    /// Zeta version string (on_start only).
    pub version: Option<String>,
}

impl HookEnv {
    fn into_vars(self) -> Vec<(String, String)> {
        let mut vars = Vec::new();
        if let Some(p) = self.path {
            vars.push((String::from("ZETA_PATH"), p.display().to_string()));
        }
        if let Some(p) = self.old_path {
            vars.push((String::from("ZETA_OLD_PATH"), p.display().to_string()));
        }
        if let Some(pane) = self.pane {
            vars.push((String::from("ZETA_PANE"), pane));
        }
        if let Some(ver) = self.version {
            vars.push((String::from("ZETA_VERSION"), ver));
        }
        vars
    }
}

/// Returns one `Command::RunHook` per hook entry that matches `event`, in declaration order.
/// Pure function — no I/O, no spawning.
pub fn commands_for_event(
    hooks: &[HookConfig],
    event: HookEvent,
    env: HookEnv,
) -> Vec<Command> {
    let vars = env.into_vars();
    hooks
        .iter()
        .filter(|h| h.event == event)
        .map(|h| Command::RunHook {
            command: h.command.clone(),
            env: vars.clone(),
        })
        .collect()
}
```

- [ ] **Run tests to verify they pass**

```bash
cargo test --lib hooks:: -- --nocapture
```

Expected: 6 tests pass.

- [ ] **Commit**

```bash
git add src/hooks.rs src/lib.rs
git commit -m "feat(hooks): add hooks module with HookEnv and commands_for_event"
```

---

## Task 4: Trigger `on_start` in `initial_commands` and `on_cd` in `apply_job_result`

**Files:**
- Modify: `src/state/mod.rs`

### Context

**`initial_commands`** is at line 455 of `src/state/mod.rs`. It returns `Vec<Command>`. Append `on_start` hook commands after all existing entries.

**`apply_job_result` / `DirectoryScanned`** is around line 2355. The `is_refresh` variable (already computed there) tells us if the directory actually changed. The `on_cd` hook fires only when `!is_refresh`. The active pane label comes from `self.panes.focus`: `PaneFocus::Left => "left"`, `PaneFocus::Right => "right"`, `PaneFocus::Preview => "left"` (preview focuses left pane's entries).

Imports to add at the top of `src/state/mod.rs` (inside the `use` block):
```rust
use crate::hooks::{commands_for_event, HookEnv};
use crate::config::HookEvent;
```

- [ ] **Write integration test stubs in `tests/hooks_integration.rs`** (the file this task also sets up):

```rust
use std::path::PathBuf;
use std::time::Instant;

use zeta::action::{Action, Command};
use zeta::config::{AppConfig, ConfigSource, HookConfig, HookEvent, LoadedConfig};
use zeta::jobs::{EntryInfo, JobResult, PaneId};
use zeta::state::AppState;
use zeta::fs::EntryKind;

fn make_state_with_hooks(hooks: Vec<HookConfig>) -> AppState {
    let mut config = AppConfig::default();
    config.hooks = hooks;
    let loaded = LoadedConfig {
        config,
        path: PathBuf::from(""),
        source: ConfigSource::File,
    };
    AppState::bootstrap(loaded, Instant::now()).expect("bootstrap ok")
}

#[test]
fn on_start_hook_appears_in_initial_commands() {
    let mut state = make_state_with_hooks(vec![
        HookConfig { event: HookEvent::OnStart, command: String::from("echo start") },
    ]);
    let cmds = state.initial_commands();
    let hook_cmds: Vec<_> = cmds
        .iter()
        .filter(|c| matches!(c, Command::RunHook { command, .. } if command == "echo start"))
        .collect();
    assert_eq!(hook_cmds.len(), 1, "expected one on_start RunHook command");
}

#[test]
fn on_cd_hook_fires_on_directory_change_not_refresh() {
    let mut state = make_state_with_hooks(vec![
        HookConfig { event: HookEvent::OnCd, command: String::from("echo cd") },
    ]);
    let _init = state.initial_commands();

    // Simulate a navigation scan (new path).
    let new_path = PathBuf::from("/tmp");
    let cmds = state.apply_job_result_commands(zeta::jobs::JobResult::DirectoryScanned {
        workspace_id: 0,
        pane: PaneId::Left,
        path: new_path.clone(),
        entries: vec![],
        elapsed_ms: 0,
    });
    let hook_count = cmds
        .iter()
        .filter(|c| matches!(c, Command::RunHook { command, .. } if command == "echo cd"))
        .count();
    assert_eq!(hook_count, 1, "expected RunHook on navigation");

    // Simulate a refresh (same path, entries already populated).
    let cmds2 = state.apply_job_result_commands(zeta::jobs::JobResult::DirectoryScanned {
        workspace_id: 0,
        pane: PaneId::Left,
        path: new_path,
        entries: vec![],
        elapsed_ms: 0,
    });
    let hook_count2 = cmds2
        .iter()
        .filter(|c| matches!(c, Command::RunHook { .. }))
        .count();
    assert_eq!(hook_count2, 0, "must not fire on refresh");
}
```

> **Note:** `apply_job_result_commands` is a new public method you will add — see below.

- [ ] **Run integration tests to verify they fail**

```bash
cargo test --test hooks_integration -- --nocapture 2>&1 | head -30
```

Expected: compile errors.

- [ ] **Add imports** at the top of `src/state/mod.rs` use block:

```rust
use crate::config::HookEvent;
use crate::hooks::{commands_for_event, HookEnv};
```

- [ ] **Fire `on_start` in `initial_commands`** — append before the closing `commands` return in `initial_commands` (~line 476):

```rust
// Fire on_start hooks.
let start_env = HookEnv {
    path: Some(self.panes.active_pane().cwd.clone()),
    version: Some(String::from(env!("CARGO_PKG_VERSION"))),
    ..HookEnv::default()
};
commands.extend(commands_for_event(&self.config.hooks, HookEvent::OnStart, start_env));
```

- [ ] **Fire `on_cd` in `apply_job_result`** — inside the `JobResult::DirectoryScanned` match arm, after `self.panes.pane_mut(pane).cwd = path.clone();` and the `is_refresh` block, add (find the line just after the `is_refresh`/`is_local` checks where `cwd` is set):

```rust
// Fire on_cd hooks when the pane actually navigated to a new directory.
if !is_refresh {
    let pane_label = match self.panes.focus {
        crate::state::types::PaneFocus::Right => String::from("right"),
        _ => String::from("left"),
    };
    let old_path = self.panes.pane(pane).cwd.parent()
        .map(|p| p.to_path_buf());
    let cd_env = HookEnv {
        path: Some(path.clone()),
        old_path,
        pane: Some(pane_label),
        ..HookEnv::default()
    };
    // `apply_job_result` does not return Commands directly; store in a field
    // or extend the pending commands vec if one is available.
    // Use the existing `commands` Vec if in scope, otherwise use `self.pending_hook_cmds`.
}
```

> **Important:** `apply_job_result` currently has signature `pub fn apply_job_result(&mut self, result: JobResult)` (returns nothing). To surface hook commands, add a new public wrapper method:

```rust
/// Applies a job result and returns any hook commands that should be dispatched.
pub fn apply_job_result_commands(&mut self, result: JobResult) -> Vec<Command> {
    let mut hook_cmds = Vec::new();
    // Determine if this is a navigation (not refresh) before mutating state.
    if let JobResult::DirectoryScanned { ref pane, ref path, .. } = result {
        let is_refresh =
            self.panes.pane(*pane).cwd == *path && !self.panes.pane(*pane).entries.is_empty();
        if !is_refresh {
            let pane_label = match self.panes.focus {
                crate::state::types::PaneFocus::Right => String::from("right"),
                _ => String::from("left"),
            };
            let cd_env = HookEnv {
                path: Some(path.clone()),
                old_path: Some(self.panes.pane(*pane).cwd.clone()),
                pane: Some(pane_label),
                ..HookEnv::default()
            };
            hook_cmds.extend(commands_for_event(
                &self.config.hooks,
                HookEvent::OnCd,
                cd_env,
            ));
        }
    }
    self.apply_job_result(result);
    hook_cmds
}
```

Add this method to `impl AppState` in `src/state/mod.rs`.

Call `apply_job_result_commands` from `app.rs` instead of `apply_job_result` and dispatch the returned commands (Task 5 covers the dispatch; for now just add the method).

- [ ] **Run integration tests**

```bash
cargo test --test hooks_integration on_start_hook_appears_in_initial_commands on_cd_hook_fires_on_directory_change_not_refresh -- --nocapture
```

Expected: both pass.

- [ ] **Run full test suite**

```bash
cargo test --workspace --quiet
```

Expected: no new failures.

- [ ] **Commit**

```bash
git add src/state/mod.rs tests/hooks_integration.rs
git commit -m "feat(hooks): fire on_start in initial_commands, on_cd in apply_job_result"
```

---

## Task 5: `on_open` trigger + `Command::RunHook` executor in `app.rs`

**Files:**
- Modify: `src/state/mod.rs` (on_open trigger)
- Modify: `src/app.rs` (executor + on_exit + wire `apply_job_result_commands`)

### Context

**`on_open` trigger** — `Action::OpenSelectedInEditor` at ~line 804 of `src/state/mod.rs` is the canonical "user opened a file" action. It already gates on `entry.kind == EntryKind::File`. Append hook commands to the returned `commands` vec inside that branch.

**Executor** — `execute_command_try` in `src/app.rs` handles `Command` variants in a `match`. Add a `Command::RunHook` arm that spawns `sh -c <command>` with stdout null, stderr piped, then on a background thread collects the exit status and calls `set_status_error` via the existing job-result channel if the exit code is non-zero.

Because we can't call `self.state.set_status_error` from a spawned thread, use the existing `workers.result_tx` channel: send a synthetic `JobResult::JobFailed` with the error message. The `apply_job_result` handler already shows `JobFailed` as a status error.

**`on_exit`** — after `while !self.state.should_quit()` exits, before the session save block (~line 120 of `src/app.rs`), emit and immediately execute `on_exit` hooks.

**Wire `apply_job_result_commands`** — in `app.rs`, find all call sites of `self.state.apply_job_result(result)` and replace with:

```rust
for cmd in self.state.apply_job_result_commands(result) {
    self.execute_command_try(cmd)?;
}
```

- [ ] **Write integration test for on_open** (add to `tests/hooks_integration.rs`):

```rust
#[test]
fn on_open_hook_fires_on_file_open_not_directory() {
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, b"hello").unwrap();

    let mut state = make_state_with_hooks(vec![
        HookConfig { event: HookEvent::OnOpen, command: String::from("echo open") },
    ]);
    let _init = state.initial_commands();

    // Put the file as the selected entry by scanning.
    let entries = vec![EntryInfo {
        name: String::from("test.txt"),
        path: file_path.clone(),
        kind: EntryKind::File,
        size_bytes: None,
        modified: None,
        link_target: None,
    }];
    state.apply_job_result(JobResult::DirectoryScanned {
        workspace_id: 0,
        pane: PaneId::Left,
        path: dir.path().to_path_buf(),
        entries,
        elapsed_ms: 0,
    });

    let cmds = state.apply(Action::OpenSelectedInEditor).expect("apply ok");
    let hook_count = cmds
        .iter()
        .filter(|c| matches!(c, Command::RunHook { command, .. } if command == "echo open"))
        .count();
    assert_eq!(hook_count, 1, "expected one on_open RunHook");
}
```

- [ ] **Run test to verify it fails**

```bash
cargo test --test hooks_integration on_open_hook_fires_on_file_open_not_directory -- --nocapture
```

Expected: fails (no hook command returned).

- [ ] **Add `on_open` trigger in `src/state/mod.rs`** — inside `Action::OpenSelectedInEditor`, after `commands.push(Command::OpenEditor { path: entry.path.clone() })`:

```rust
// Fire on_open hooks when a file is opened.
if entry.kind == EntryKind::File {
    let pane_label = match self.panes.focus {
        crate::state::types::PaneFocus::Right => String::from("right"),
        _ => String::from("left"),
    };
    let open_env = HookEnv {
        path: Some(entry.path.clone()),
        pane: Some(pane_label),
        ..HookEnv::default()
    };
    commands.extend(commands_for_event(
        &self.config.hooks,
        HookEvent::OnOpen,
        open_env,
    ));
}
```

- [ ] **Run on_open test to verify it passes**

```bash
cargo test --test hooks_integration on_open_hook_fires_on_file_open_not_directory -- --nocapture
```

Expected: PASS

- [ ] **Add `Command::RunHook` executor in `src/app.rs`** — in `execute_command_try`, replace the placeholder `Command::RunHook { .. } => {}` arm (added in Task 2) with:

```rust
Command::RunHook { command, env } => {
    let result_tx = self.workers.result_tx.clone();
    let cmd_display = command.clone();
    let mut child_cmd = std::process::Command::new("sh");
    child_cmd.arg("-c").arg(&command);
    for (k, v) in &env {
        child_cmd.env(k, v);
    }
    child_cmd.stdout(std::process::Stdio::null());
    child_cmd.stderr(std::process::Stdio::piped());
    match child_cmd.spawn() {
        Ok(child) => {
            std::thread::spawn(move || {
                if let Ok(output) = child.wait_with_output() {
                    if !output.status.success() {
                        let code = output.status.code().unwrap_or(-1);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let stderr_snippet: String =
                            stderr.chars().take(200).collect();
                        let msg = if stderr_snippet.is_empty() {
                            format!("hook exited {code}: {cmd_display}")
                        } else {
                            format!("hook exited {code}: {cmd_display} — {stderr_snippet}")
                        };
                        let _ = result_tx.send(crate::jobs::JobResult::JobFailed {
                            workspace_id: 0,
                            pane: crate::jobs::PaneId::Left,
                            path: std::path::PathBuf::new(),
                            file_op: None,
                            message: msg,
                            elapsed_ms: 0,
                        });
                    }
                }
            });
        }
        Err(e) => {
            self.state
                .set_status_error(format!("hook spawn failed: {e}"));
        }
    }
}
```

- [ ] **Wire `apply_job_result_commands` in `src/app.rs`** — find every call to `self.state.apply_job_result(result)` (search for it) and replace with:

```rust
for cmd in self.state.apply_job_result_commands(result) {
    self.execute_command_try(cmd)?;
}
```

- [ ] **Add `on_exit` in `src/app.rs`** — after the `while !self.state.should_quit()` loop ends and before the session save block (~line 120), add:

```rust
// Fire on_exit hooks synchronously before session save.
// Hooks are spawned (not waited on) so they may outlive the process.
let exit_env = crate::hooks::HookEnv {
    path: Some(self.state.panes().active_pane().cwd.clone()),
    ..crate::hooks::HookEnv::default()
};
for cmd in crate::hooks::commands_for_event(
    &self.state.config().hooks,
    crate::config::HookEvent::OnExit,
    exit_env,
) {
    let _ = self.execute_command_try(cmd);
}
```

> Note: `self.state.panes()` — check whether there is already a public accessor for panes in `AppState`. If not, use `self.state.config().hooks` (already public) and get the cwd from a public path accessor such as `self.state.active_pane_cwd()` if it exists. Check `src/state/mod.rs` for existing public methods. An alternative is to capture the cwd into a local before the loop: `let exit_cwd = self.state.panes.active_pane().cwd.clone();` — if `panes` is a public field, access it directly.

- [ ] **Run all integration tests**

```bash
cargo test --test hooks_integration -- --nocapture
```

Expected: all pass.

- [ ] **Run full test suite**

```bash
cargo test --workspace --quiet
```

Expected: no new failures beyond the 2 pre-existing ones.

- [ ] **Commit**

```bash
git add src/state/mod.rs src/app.rs tests/hooks_integration.rs
git commit -m "feat(hooks): on_open trigger, RunHook executor, on_exit, wire apply_job_result_commands"
```

---

## Task 6: Pre-PR validation + graphify update

**Files:**
- No code changes

- [ ] **Format check**

```bash
cargo fmt --all -- --check
```

If it fails: `cargo fmt --all` then re-run check.

- [ ] **Clippy clean**

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Fix any warnings before proceeding.

- [ ] **Full test suite**

```bash
cargo test --workspace
```

Expected: 439/441 or better (the 2 pre-existing failures in `route_mouse_left_click_on_workspace_pill_{2,4}` are unrelated and expected).

- [ ] **Update graphify knowledge graph**

```bash
graphify . --update
```

This keeps the graph current with Phase 4 additions.

- [ ] **Commit any fmt fixes**

```bash
git add -A
git commit -m "chore(hooks): fmt and clippy fixes" 2>/dev/null || echo "nothing to commit"
```

- [ ] **Push branch**

```bash
git push --set-upstream origin feat/phase4-shell-hooks
```

- [ ] **Create PR**

```bash
gh pr create \
  --title "feat(hooks): Phase 4 — Shell Hook System" \
  --body "## Phase 4 — Shell Hook System

Adds fire-and-forget shell hooks triggered on directory change, file open, app start, and app exit.

### Config

\`\`\`toml
[[hooks]]
event = \"on_cd\"
command = \"~/.config/zeta/hooks/on_cd.sh\"

[[hooks]]
event = \"on_start\"
command = \"echo zeta started >> /tmp/zeta.log\"
\`\`\`

### Environment variables

| Event | Variables |
|-------|-----------|
| \`on_cd\` | \`ZETA_PATH\`, \`ZETA_OLD_PATH\`, \`ZETA_PANE\` |
| \`on_open\` | \`ZETA_PATH\`, \`ZETA_PANE\` |
| \`on_start\` | \`ZETA_PATH\`, \`ZETA_VERSION\` |
| \`on_exit\` | \`ZETA_PATH\` |

### Changes
- \`src/config.rs\` — \`HookEvent\`, \`HookConfig\`, \`AppConfig.hooks\`, annotated config section
- \`src/hooks.rs\` — pure \`commands_for_event\` + \`HookEnv\` (new)
- \`src/action.rs\` — \`Command::RunHook\`
- \`src/state/mod.rs\` — \`on_start\`, \`on_cd\`, \`on_open\` triggers; \`apply_job_result_commands\`
- \`src/app.rs\` — \`RunHook\` executor (detached thread, stderr → status bar); \`on_exit\` trigger
- \`tests/hooks_integration.rs\` — integration tests (new)

### Validation
- \`cargo fmt --all -- --check\` ✅
- \`cargo clippy --workspace --all-targets --all-features -- -D warnings\` ✅
- \`cargo test --workspace\` ✅" \
  --base main \
  --head feat/phase4-shell-hooks
```

---

## Self-Review

**Spec coverage:**
- ✅ `HookEvent` / `HookConfig` / `AppConfig.hooks` — Task 1
- ✅ `Command::RunHook` variant — Task 2
- ✅ `HookEnv` + `commands_for_event` pure logic — Task 3
- ✅ `on_start` trigger in `initial_commands` — Task 4
- ✅ `on_cd` trigger (navigation only, not refresh) — Task 4
- ✅ `on_open` trigger (file only, not directory) — Task 5
- ✅ `on_exit` trigger in `app.rs` shutdown path — Task 5
- ✅ Executor: detached thread, stderr → status bar — Task 5
- ✅ `generate_annotated_config` hooks section — Task 1
- ✅ `apply_job_result_commands` wrapper — Task 4/5
- ✅ Integration tests — Tasks 4 & 5
- ✅ Pre-PR validation + graphify update — Task 6

**Type consistency check:**
- `HookEnv` defined in Task 3, used in Tasks 3/4/5 ✅
- `commands_for_event(hooks: &[HookConfig], event: HookEvent, env: HookEnv) -> Vec<Command>` — consistent across all tasks ✅
- `Command::RunHook { command: String, env: Vec<(String, String)> }` — consistent ✅
- `apply_job_result_commands` returns `Vec<Command>` — consistent with `execute_command_try` call pattern ✅
- `HookEvent::OnCd/OnOpen/OnStart/OnExit` — used consistently ✅

**Placeholder scan:** None found.
