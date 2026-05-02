# Shell Hooks

> **Status:** Coming in Zeta v0.5 (branch: `feat/phase4-shell-hooks`)

Shell hooks let you run arbitrary commands automatically when Zeta hits lifecycle events — directory changes, file opens, startup, and exit. Use hooks to update your prompt, run scripts, log activity, or integrate with other tools.

---

## How Hooks Work

Hooks are defined in your `config.toml` using TOML array-of-tables syntax (`[[hooks]]`). Each entry specifies an event and a command to run when that event fires.

- Hooks run **fire-and-forget** in a background thread — they never block the UI
- If a hook command fails or exits with a non-zero code, the error is shown briefly in the Zeta status bar
- Environment variables are injected so your hook scripts know what triggered them

---

## Supported Events

| Event | When it fires |
|---|---|
| `on_start` | Zeta starts up |
| `on_exit` | Zeta is about to quit |
| `on_cd` | The active pane navigates to a new directory |
| `on_open` | A file is opened in the embedded editor |

---

## Configuration Syntax

Add `[[hooks]]` blocks to your `config.toml`:

```toml
[[hooks]]
event = "on_start"
command = "echo Zeta started >> ~/.zeta.log"

[[hooks]]
event = "on_exit"
command = "echo Zeta exited >> ~/.zeta.log"

[[hooks]]
event = "on_cd"
command = "~/.config/zeta/hooks/on_cd.sh"

[[hooks]]
event = "on_open"
command = "~/.config/zeta/hooks/on_open.sh"
```

You can have multiple hooks for the same event — all of them run:

```toml
[[hooks]]
event = "on_cd"
command = "echo Changed to $ZETA_PATH"

[[hooks]]
event = "on_cd"
command = "direnv allow . 2>/dev/null || true"
```

---

## Environment Variables

Zeta injects these environment variables into every hook process:

| Variable | Available on | Description |
|---|---|---|
| `ZETA_PATH` | all events | Absolute path of the current directory |
| `ZETA_OLD_PATH` | `on_cd` only | Absolute path of the previous directory |
| `ZETA_PANE` | `on_cd`, `on_open` | Which pane triggered (`left` or `right`) |
| `ZETA_VERSION` | `on_start` | Zeta version string (e.g., `0.5.0`) |

---

## Use Case Examples

### Update your shell's directory environment on `on_cd`

If you use [direnv](https://direnv.net/):

```bash
#!/usr/bin/env bash
# ~/.config/zeta/hooks/on_cd.sh
direnv allow "$ZETA_PATH" 2>/dev/null || true
```

### Log opened files

```bash
#!/usr/bin/env bash
# ~/.config/zeta/hooks/on_open.sh
echo "$(date -Iseconds) OPEN $ZETA_PATH (pane: $ZETA_PANE)" >> ~/.zeta-open.log
```

### Run a custom prompt script on startup

```toml
[[hooks]]
event = "on_start"
command = "~/.config/zeta/hooks/startup.sh"
```

```bash
#!/usr/bin/env bash
# ~/.config/zeta/hooks/startup.sh
# Load project-specific environment, notify a dashboard, etc.
source ~/.nvm/nvm.sh
nvm use default --silent
```

### Sync bookmarks on exit

```toml
[[hooks]]
event = "on_exit"
command = "rsync -q ~/.config/zeta/ user@backup-server:~/.config/zeta/"
```

---

## Script Permissions

Make sure your hook scripts are executable:

```bash
chmod +x ~/.config/zeta/hooks/on_cd.sh
```

---

## Error Handling

If a hook command exits with a non-zero status or cannot be found, Zeta shows a brief error message in the status bar (styled in red). The error does not interrupt your workflow — Zeta continues normally.

---

*See also: [Configuration](Configuration) · [First-Run Wizard](First-Run-Wizard) · [Key Bindings](Key-Bindings)*
