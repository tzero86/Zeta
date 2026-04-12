# Zeta — Development Roadmap

This file is the single source of truth for all planned and completed work.
Each wave has a dedicated plan file in this directory. Update status here when
a wave ships.

---

## Status key

| Symbol | Meaning |
|---|---|
| ✅ | Shipped and merged to `main` |
| 🚧 | In progress |
| 📋 | Documented, not started |
| 💡 | Idea only, not yet documented |

---

## Completed waves

| Wave | Plan | Summary |
|---|---|---|
| 1A | `2026-04-07-wave1a-appstate-decomposition.md` | AppState decomposed into sub-states |
| 1B | `2026-04-07-wave1b-ui-module-split.md` | ui.rs split into focused modules + LayoutCache |
| 1C | `2026-04-07-wave1c-multiworker-editor-markdown.md` | 3-worker jobs, ropey editor, markdown ViewBuffer |
| 2A | `2026-04-07-wave2a-input-routing.md` | FocusLayer enum, RouteContext deleted |
| 2B | `2026-04-07-wave2b-mouse-support.md` | Full mouse support (click, scroll, menu bar) |
| 3A | `2026-04-08-wave3a-editor-rope-backend.md` | Rope backend, O(log n) ops, delta undo, highlight cache |
| 4A | `2026-04-08-wave4a-git-integration.md` | Git status indicators, branch name in status bar |
| 4B | `2026-04-08-wave4b-markdown-live-preview.md` | Native markdown renderer, split editor/preview panel |

---

## Active roadmap

### Phase 4 — Editor + Navigation maturity

| Wave | Plan | Summary | Status |
|---|---|---|---|
| 4C | `2026-04-08-wave4c-editor-fullscreen-sync.md` | Full-window editor, scroll sync, preview focus/toggle | 📋 |
| 4D | `2026-04-08-wave4d-quickfilter-fuzzy-find.md` | In-pane quick filter, Ctrl+P fuzzy file find | 📋 |

### Phase 5 — Power features

| Wave | Plan | Summary | Status |
|---|---|---|---|
| 5A | `2026-04-08-wave5a-find-replace-watcher.md` | Find & Replace in editor, directory watcher auto-refresh | 📋 |
| 5B | `2026-04-08-wave5b-bookmarks-trash.md` | Bookmarks (persist in config), trash/recycle bin instead of permanent delete | 📋 |
| 5C | `2026-04-08-wave5c-shell-integration.md` | F2 opens shell in current pane directory | 📋 |

### Phase 6 — Advanced file operations

| Wave | Plan | Summary | Status |
|---|---|---|---|
| 6A | `2026-04-08-wave6a-archive-browsing.md` | Navigate into .zip / .tar.gz / .tar.bz2 / .tar.xz like directories | ✅ |
| 6B | `2026-04-08-wave6b-directory-diff.md` | Left/right pane diff mode — colour-code unique/matching/different entries | ✅ |

### Phase 7 — Remote filesystems

| Wave | Plan | Summary | Status |
|---|---|---|---|
| 7A | `2026-04-08-wave7a-ssh-sftp.md` | SSH/SFTP Remote pane via ssh2 + FsBackend trait refactor | 📋 |
| 7B | `2026-04-12-wave7b-ssh-agent.md` | SSH Agent and Host Key Verification | ✅ |

### Phase 8 — Embedded Terminal

| Wave | Plan | Summary | Status |
|---|---|---|---|
| 8A | `2026-04-12-wave8a-embedded-terminal.md` | Fully embedded terminal emulator (PTY + rendering) | 📋 |

---

## Jira epic mapping

| Epic | Wave |
|---|---|
| ZTA-122 — Wave 1C: Multi-Worker Jobs + Editor + Markdown Preview | 1C |
| ZTA-123 — Wave 2A: FocusLayer + Input Routing Redesign | 2A |
| ZTA-124 — Wave 2B: Full Mouse Support | 2B |
| ZTA-125 — Wave 3A: Editor Rope Backend + Lightweight Undo Stack | 3A |
| ZTA-126 — Wave 4A: Git Integration | 4A |
| ZTA-127 — Wave 4B: Markdown Live Preview | 4B |
| ZTA-128 — Wave 4C: Editor Fullscreen + Scroll Sync + Preview Focus/Toggle | 4C |
| ZTA-129 — Wave 4D: In-Pane Quick Filter + Fuzzy File Find | 4D |
| ZTA-130 — Wave 5A: Find & Replace in Editor + Directory Watcher | 5A |
| ZTA-131 — Wave 5B: Bookmarks + Trash | 5B |
| ZTA-132 — Wave 5C: Shell Integration | 5C |
| ZTA-156 — Wave 6A: Archive Browsing | 6A |
| ZTA-157 — Wave 6B: Directory Diff Mode | 6B |
| ZTA-168 — Wave 7A: SSH/SFTP Remote Filesystems | 7A |
| ZTA-169 — Wave 7B: SSH Agent and Host Key Verification | 7B |
| ZTA-170 — Wave 8A: Embedded Terminal | 8A |
