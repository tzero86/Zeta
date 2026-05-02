# Configuration

Zeta is configured via a TOML file. The First-Run Wizard generates a fully annotated version on first launch, but you can also create or edit it manually at any time. Zeta hot-reloads the config while it's running — no restart needed.

---

## Config File Location

Zeta looks for your config file in this order:

1. Path in the `ZETA_CONFIG` environment variable
2. `%APPDATA%\zeta\config.toml` (Windows)
3. `~/.config/zeta/config.toml` (Linux/macOS)

---

## Full Config Reference

```toml
# ─── Appearance ───────────────────────────────────────────────────────────────

# Theme
# Options: "zeta" | "fjord" | "sandbar" | "oxide" | "matrix" | "norton" | "dracula" | "neon" | "monochrome"
theme = "zeta"

# Icon mode
# Options: "unicode" (default) | "ascii" | "custom"
# For "custom", install assets/fonts/zeta-icons.ttf in your terminal first.
icon_mode = "unicode"

# ─── Pane behavior ────────────────────────────────────────────────────────────

# Open preview panel on startup
preview_panel_open = false

# Automatically preview the file under the cursor when moving selection
preview_on_selection = true

# ─── Editor ───────────────────────────────────────────────────────────────────

[editor]
# Number of spaces inserted when Tab is pressed
tab_width = 4

# Wrap long lines at viewport width
word_wrap = false

# ─── Terminal ─────────────────────────────────────────────────────────────────

[terminal]
# Override the shell used in the integrated terminal.
# Defaults: pwsh/powershell.exe/cmd.exe on Windows, $SHELL/bash on Unix.
# shell = "/bin/zsh"

# ─── Updates ──────────────────────────────────────────────────────────────────

[update]
# Check for new Zeta releases on startup
check_on_startup = true

# ─── Shell Hooks (v0.5) ───────────────────────────────────────────────────────
# Define commands that fire on Zeta lifecycle events.
# Events: on_start | on_exit | on_cd | on_open
# Hooks run fire-and-forget in a background thread; errors appear in the status bar.

# [[hooks]]
# event = "on_start"
# command = "echo Zeta started"

# [[hooks]]
# event = "on_cd"
# command = "~/.config/zeta/hooks/on_cd.sh"

# [[hooks]]
# event = "on_open"
# command = "~/.config/zeta/hooks/on_open.sh"

# [[hooks]]
# event = "on_exit"
# command = "~/.config/zeta/hooks/on_exit.sh"
```

---

## Runtime Settings Panel

Press `Ctrl+O` (or View menu → Settings) to open the settings panel inside Zeta. Changes made here update `config.toml` immediately and take effect live.

Settings available in the panel:
- Theme
- Icon mode
- Layout (side-by-side vs stacked)
- Preview on selection
- Editor tab width and word wrap

---

## Config Hot-Reload

Zeta watches your config file for changes using a filesystem watcher. If you edit `config.toml` with an external editor while Zeta is open, the new config is applied within a second — including theme changes, keymap updates, and palette changes.

---

## Keymap Customization

Custom keybindings can be defined in your config under `[keymap]`. The default bindings are documented on the [Key Bindings](Key-Bindings) page.

---

*See also: [First-Run Wizard](First-Run-Wizard) · [Themes](Themes) · [Shell Hooks](Shell-Hooks) · [Key Bindings](Key-Bindings)*
