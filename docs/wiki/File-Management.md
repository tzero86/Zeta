# File Management

Zeta's dual-pane file browser is the heart of the application. It gives you two independent directory views side by side — just like Norton Commander — so you can copy, move, and compare files between locations without juggling windows.

---

## Dual-Pane Browser

- **Side-by-side layout** (default) or **stacked layout** — toggle in settings (`Ctrl+O`)
- Press `Tab` to switch which pane is active
- Each pane has its own directory, sort order, hidden-file toggle, and selection state
- Both panes update in the background — large directories never block scrolling

---

## Four Workspaces

Zeta has four completely isolated workspaces, each with its own pair of panes, editor, terminal, and preview state.

- Switch with `Alt+1` through `Alt+4` (or `Shift+1`–`Shift+4` as a terminal fallback)
- The active workspace is highlighted in the top-bar workspace pills: `[1] [2] [3] [4]`
- The status bar shows `ws:N/4` at all times
- Switching workspaces is instant — no reloads, no flicker

**Use case:** Keep a local filesystem in workspace 1, an SSH remote in workspace 2, a git diff review in workspace 3, and a long-running build log in workspace 4.

---

## File Operations

| Key | Action |
|---|---|
| `F5` | Copy selected file(s) to the opposite pane |
| `Shift+F6` | Move selected file(s) to the opposite pane |
| `F6` | Rename file |
| `F7` | Create new directory |
| `F8` | Delete file(s) |
| `Ins` | Create new file |

Operations run in a **background worker** — Zeta never freezes while copying large trees.

### Collision Resolution

When a copy or move would overwrite an existing file, Zeta shows a collision resolution dialog:

- **Skip** — leave the destination file untouched
- **Overwrite** — replace the destination file
- **Rename** — write the source file with an auto-incremented name

The dialog appears per-file; you can apply the same choice to all remaining collisions.

---

## Marking Files for Batch Operations

Mark multiple files and then run a single copy, move, or delete on all of them at once.

- Press `Space` (or `Ins`) on a file to toggle its mark; marked files appear highlighted
- `F5` / `Shift+F6` / `F8` operate on the entire mark set when marks are present
- `Ctrl+C` (copy path) joins all marked paths with newlines onto the clipboard

---

## Sorting and Filtering

- Sort by **name**, **size**, **extension**, or **modified date** — cycle through modes in settings
- Toggle **hidden files** (dot-files on Unix, system-hidden on Windows) with `Ctrl+H` or via the View menu
- Sort direction (ascending / descending) toggles alongside sort mode

---

## Directory History

Navigate back and forward through your directory history within each pane:

- `Alt+←` — go back
- `Alt+→` — go forward

---

## Session Restore

Zeta remembers your last session across restarts:

- Both panes in all four workspaces restore their last directory
- Sort mode, hidden-file state, and layout preference are all restored
- The active workspace index is restored

Session state is written to `~/.local/share/zeta/session.json` (Linux/macOS) or `%APPDATA%\zeta\session.json` (Windows).

---

## Git Status Indicators

Every file in the pane shows its Git status at a glance:

| Symbol | Meaning |
|---|---|
| `M` | Modified |
| `A` | Added (staged) |
| `?` | Untracked |
| `D` | Deleted |
| `R` | Renamed |
| `U` | Conflicted (merge) |

The current branch name appears in the status bar. Git status refreshes automatically alongside every directory scan. Zeta shells out to `git` on your PATH — no git crate is bundled. Paths outside a git repo are silently skipped.

---

*See also: [Key Bindings](Key-Bindings) · [Workspaces & Terminal](Terminal) · [SSH/SFTP](SSH-SFTP)*
