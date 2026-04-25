# Zeta UI/UX Revamp — Design Spec

**Date:** 2026-04-25  
**Status:** Approved for implementation  
**Scope:** Full visual overhaul of all major UI surfaces

---

## 1. Problem Statement

Zeta's current UI is functional but visually inconsistent. Modals lack depth, the icon system uses incorrect codepoints, the settings panel is a flat unorganized list, the status bar is a dense unstructured string, and panel chrome (editor/preview/terminal) is minimal and unstyled. This spec defines a cohesive visual language for every major surface.

---

## 2. Color Palette

Replace ad-hoc colors with **Catppuccin Mocha** as the canonical dark theme foundation. All new style helpers reference these named tokens.

| Token | Hex | Usage |
|-------|-----|-------|
| `bg` | `#1e1e2e` | App background |
| `surface` | `#181825` | Panel/pane backgrounds |
| `overlay` | `#313244` | Modal surfaces, headers |
| `crust` | `#11111b` | Deepest background |
| `text_primary` | `#cdd6f4` | Body text |
| `text_subtext` | `#a6adc8` | Secondary text |
| `text_muted` | `#6c7086` | Dimmed/inactive text |
| `blue` | `#89b4fa` | Focus accents, editor border |
| `mauve` | `#cba6f7` | About modal, workspace, settings |
| `teal` | `#94e2d5` | File finder, preview border |
| `green` | `#a6e3a1` | Terminal border, success states |
| `yellow` | `#f9e2af` | Key pill hints, marks |
| `peach` | `#fab387` | Dirty/modified indicators |
| `red` | `#f38ba8` | Errors, destructive actions |
| `lavender` | `#b4befe` | Secondary accents |

---

## 3. Menu Bar

**Style:** Modern Dark with rounded corner suggestion via `╭─` fills and `╮` at edges (approximated in ratatui with styled spans).

**Context badge:** `◈ <mode>` indicator left of menu items — changes label based on `MenuContext` (Pane, Editor, Preview, Terminal).

**Menu items:** File · Edit · View · Go · Tools — always visible. Items irrelevant to the current context render in `text_muted` color (not hidden, just dimmed). This preserves muscle memory while communicating availability.

**Workspace selector** (right-aligned in menu bar):  
- Active workspace: `1:~/projects` — accent color, truncated CWD up to 12 chars  
- Inactive workspaces: `2 3 4` — `text_muted`  
- Separator between active and inactive is a single space, no brackets

---

## 4. Pane Entries

**Default view — Rich Columns:**  
```
 Icon  Name                Size    Modified     Git
 󰉋    src/                 —       Apr 24       M
 󰈙    main.rs              4.2 KB  Apr 23       ✓
 󰈙    config.rs            1.8 KB  Apr 20       —
```
Column headers rendered as a single row above the list in `text_muted`. Columns: icon (2 cols) · name (flex) · size (7 cols) · modified (12 cols) · git (4 cols).

**Density toggle — Two-Line view** (Ctrl+T):  
Row 1: icon + name + git status  
Row 2 (indented): permissions + size + modified date in `text_muted`

**Selection highlight:** `selection_bg` + `selection_fg` + bold, with `›` prefix symbol.

**Mark indicator:** `✦` prefix in `yellow` for marked entries.

---

## 5. Icon System

**Rename** `IconMode::Custom` → `IconMode::NerdFont`. Update config key `icon_mode = "nerdfont"`.

**NerdFont codepoints** (NF v3):

| Kind | Codepoint | Color |
|------|-----------|-------|
| Directory | `\u{f07b}` 󰉋 | `blue` |
| File (generic) | `\u{f15b}` 󰖧 | `text_subtext` |
| Rust `.rs` | `\u{e7a8}` | `peach` |
| Markdown `.md` | `\u{f48a}` | `blue` |
| TOML/config | `\u{e615}` | `yellow` |
| Shell `.sh` | `\u{f489}` | `green` |
| Image | `\u{f1c5}` | `mauve` |
| Archive | `\u{f410}` | `yellow` |
| Symlink | `\u{f481}` | `teal` |
| Hidden (dot files) | `\u{f023}` | `text_muted` |

Fallback chain: `NerdFont` → `Unicode` → `Ascii`. User sets `icon_mode` in `config.toml`. Documentation recommends installing a Nerd Font (e.g. JetBrainsMono Nerd Font).

---

## 6. Modal Depth System

All modals use a **Halo + Dim Zone** approach:

1. **Backdrop dim:** app content behind modal rendered at ~30% opacity (Clear widget + dim overlay pass)
2. **Halo ring:** one cell of `overlay` color surrounds the modal box, creating a lifted appearance
3. **Modal surface:** `#24273a` — slightly lighter than `overlay`
4. **Border:** 1px solid focus accent color for the modal type
5. **Title:** always centered in the top border

Applied to: prompts, dialogs, palette, finder, settings, bookmarks, SSH, collision, destructive confirm, open-with.

---

## 7. Settings Panel

**Layout:** Segmented control header (tab bar) + content area below.

**Tabs:**
| Key | Tab | Contents |
|-----|-----|----------|
| `1` | Appearance | Theme, icon mode, color overrides |
| `2` | Panels | Layout, column widths, sort defaults, hidden files |
| `3` | Editor | Tab width, word wrap, syntax theme, line numbers |
| `4` | Keymaps | Key binding overrides (read-only display in v1) |

`Tab` / `Shift+Tab` cycles tabs. Number keys `1–4` jump directly. Active tab indicator: top border accent line in `blue`.

---

## 8. Help Modal

**Layout:** Two-column key table with colored section dividers.

**Key pills:** each keybinding rendered as a styled badge — `overlay` background, `yellow` text, thin border.

**Sections:** Navigation · Files · Editor · System — each with a thin `blue` underline divider and uppercase `text_muted` label.

**Two columns** side by side, fitting ~2× the shortcuts in the same height. Scrollable. Footer hint bar: `▲▼ scroll · Esc close`.

---

## 9. About Modal

**Logo:** Keep existing ASCII art banner (lines starting with `$`-style characters), rendered in `mauve` accent color.

**Below logo:**
- Tagline in `text_subtext`
- Version badge (green pill) + Beta badge (peach pill)

**Info grid** (two-column, monospace):
```
Version   v0.4.x  [badge]
Author    tzero86
Theme     zeta dark        ← accent blue
Icons     NerdFont
Config    ~/.config/zeta/config.toml
```

**Features section:** chip array — `Dual panes · 4 Workspaces · SSH/SFTP · Archives · Diff mode · Editor · Sessions · Mouse`

**Other themes** section: comma-separated list in `text_subtext`.

---

## 10. Command Palette

**Input row:** `⌕` icon + query text + blinking cursor.

**Results grouped by category** with section labels (`FILES · GIT · NAVIGATION · SSH · VIEW`) in `text_muted` uppercase.

**Each result row:** icon (16px) · label with matched chars highlighted in `yellow` · category badge (colored pill) · keybinding hint right-aligned.

**Active row:** `rgba(blue, 0.12)` background.

**Footer:** `Enter run · Esc close`

---

## 11. File Finder

**Input row:** `⌕` icon in `teal` + query + blinking `teal` cursor.

**Root hint row:** `root: ~/projects/zeta` in `text_muted`.

**Result rows:** dir path in `text_muted` + filename bold + matched chars highlighted in `teal` + file type hint right-aligned.

**Active row:** `rgba(teal, 0.10)` background.

**Footer:** `Enter open · Ctrl+Enter open in editor · Esc close`

---

## 12. Status Bar

**Four zones** (left to right), each with a distinct tint:

| Zone | Background tint | Content |
|------|----------------|---------|
| Git | `rgba(blue, 0.15)` | `⎇ branch-name` |
| Entry | `rgba(mauve, 0.10)` | icon + filename + permissions + size |
| Message / Progress | `surface` | status message (italic, `text_subtext`) or progress bar |
| Workspace | `rgba(mauve, 0.18)` | `ws N/4` bold |

**Marks zone** appears between Message and Workspace when marks exist: `✦ N · X MB` in `yellow` tint.

**Progress mode:** during file operations, the Entry and Message zones merge into a single progress bar zone showing: `copy 2/5 ──────░░░░── large-file.zip`.

Zones separated by 1px `overlay` dividers.

> **Note:** ratatui does not support true RGBA. Zone tints are approximated as the nearest solid `ThemePalette` color at design time and mapped to exact palette values during implementation.

**Git zone** only renders when the active pane is inside a git repository. When no git repo is detected, the Entry zone expands to fill the left side.

---

## 13. Pane Inline Filter

Activated by `/` (current keybinding). Renders as a 1-row strip at the bottom of the active pane:

```
⌕  src_│   2 matches   Esc clear
```

- Strip background: `rgba(blue, 0.10)`, top border `rgba(blue, 0.30)`
- `⌕` icon in `blue`
- Query text + blinking cursor
- Match count in `green` (right side)
- `Esc clear` hint in `text_muted`
- Non-matching entries in the list above are dimmed to `text_muted` (not hidden)

---

## 14. Panel Chrome (Editor / Preview / Terminal)

All three panels use the same rich title bar pattern. Each has its own focus accent color.

**Title bar layout:**
```
 [icon]  [filename] [●]  [path/context]        [metadata]  [type badge]
```

| Panel | Focus color | Icon | Metadata | Badge |
|-------|-------------|------|----------|-------|
| Editor | `blue` | `󰈙` file icon | `Ln 42 · Col 8` | `Editor` (blue pill) |
| Preview | `teal` | file-type icon | file type label | `Preview` (teal pill) |
| Terminal | `green` | `󰆍` terminal icon | `zsh · ~/cwd` | `Shell` (green pill) |

**Dirty indicator:** `●` in `peach` after filename, when editor buffer has unsaved changes.

**Unfocused state:** border color drops to `text_muted`, title text to `text_subtext`.

**Editor title path:** shows parent directory only (not full path) to keep it short. Full path visible in status bar entry zone when editor is focused.

---

## 15. Implementation Phases (suggested)

| Phase | Surfaces | Key files |
|-------|----------|-----------|
| 1 | Palette tokens + `styles.rs` foundation | `src/ui/styles.rs`, `src/config.rs` |
| 2 | Icon system (NerdFont rename + codepoints) | `src/icon.rs`, `src/config.rs` |
| 3 | Modal depth system (halo + dim) | `src/ui/overlay.rs` |
| 4 | Menu bar + workspace selector | `src/ui/menu_bar.rs` |
| 5 | Pane entries (rich columns + filter bar) | `src/ui/pane.rs` |
| 6 | Status bar (zoned) | `src/ui/mod.rs`, `src/state/mod.rs` |
| 7 | Panel chrome (editor + preview + terminal) | `src/ui/editor.rs`, `src/ui/preview.rs`, `src/ui/terminal.rs` |
| 8 | Settings tabbed panel | `src/ui/settings.rs`, `src/state/settings.rs` |
| 9 | Help + About modals | `src/state/dialog.rs`, `src/ui/overlay.rs` |
| 10 | Command palette + file finder polish | `src/ui/palette.rs`, `src/ui/finder.rs` |

---

## 16. Non-Goals (v1 revamp)

- No runtime font detection — user sets `icon_mode` in config
- No animation or transition effects
- No plugin system for themes
- No SSH or remote filesystem UI changes
- Keymaps tab in settings is read-only display only in this phase
