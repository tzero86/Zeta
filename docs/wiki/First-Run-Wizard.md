# First-Run Wizard

When you launch Zeta for the first time — or when no `config.toml` exists — the **First-Run Wizard** greets you with an interactive setup experience right in your terminal.

---

## What the Wizard Does

The wizard has one goal: get you up and running with a config that fits your taste, without requiring you to edit TOML files manually before you can use the app.

1. **Welcome screen** — brief intro to Zeta's key features
2. **Theme picker** — scroll through all 9 built-in themes with a live preview rendered in the terminal
3. **Confirm** — write the config and launch Zeta, or dismiss without writing anything

---

## Live Theme Preview

The theme picker shows a miniature rendering of Zeta's UI using actual ratatui widgets — pane borders, file list rows, status bar, and editor area — all drawn in the selected theme's colors. Scroll up and down through the theme list to see the preview update instantly.

Available themes you can preview:
- Zeta (default)
- Fjord
- Sandbar
- Oxide
- Matrix
- Norton
- Dracula
- Neon
- Monochrome

See the [Themes](Themes) page for descriptions of each.

---

## Annotated Config Generation

When you confirm your theme choice, Zeta writes a fully annotated `config.toml` to:

| Platform | Path |
|---|---|
| Linux / macOS | `~/.config/zeta/config.toml` |
| Windows | `%APPDATA%\zeta\config.toml` |

The generated file documents **every available option** inline with comments:

```toml
# Theme: "zeta" (default) | "fjord" | "sandbar" | "oxide" | "matrix" | "norton" | "dracula" | "neon" | "monochrome"
theme = "fjord"

# Icon mode: "unicode" (default) | "ascii" | "custom"
# For "custom", install assets/fonts/zeta-icons.ttf in your terminal first.
icon_mode = "unicode"

# Open preview panel on startup
preview_panel_open = false

# Auto-preview on cursor move
preview_on_selection = true

[editor]
# Number of spaces per tab stop
tab_width = 4

# Wrap long lines at viewport width
word_wrap = false
```

You can edit this file freely at any time — Zeta hot-reloads config changes while it's running.

---

## Dismissing the Wizard

Press `Esc` at any point during the wizard to dismiss it **without writing any config file**. Zeta will launch with its built-in defaults (Zeta theme, unicode icons, all other defaults).

The wizard will appear again on the next launch if no config file is present. To suppress it permanently without customizing, just let it generate the default config by pressing Enter to confirm.

---

## Re-Running the Wizard

The wizard only appears automatically when no config file exists. If you want to run it again:

1. Delete or rename your `config.toml`
2. Launch Zeta — the wizard will appear again

Or use the settings panel (`Ctrl+O`) to change the theme and other settings interactively at any time without re-running the wizard.

---

*See also: [Themes](Themes) · [Configuration](Configuration) · [Key Bindings](Key-Bindings)*
