# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **WSL NerdFont detection**: `detect_icon_mode()` previously used `fc-list` (Linux font system) which would erroneously detect NerdFont on WSL even though the Windows Terminal renders Windows-side fonts. On WSL, icon mode now defaults to Unicode. Users who have NerdFonts installed on Windows Terminal can opt in via `ZETA_ICON_MODE=nerd`.
- **Rich Columns not shown by default**: `details_view` now defaults to `true` so all panes open in Rich Columns mode (icon · git · name · size · date) as specified in the UI/UX design. Previously panes opened in tree/compact mode requiring manual Ctrl+L to enable columns.
- **Clippy warnings**: Merged identical `if/else` arms in menu bar item builder; replaced `&[entry.path.clone()]` with `std::slice::from_ref(&entry.path)` in delete handlers; removed unused `DestructiveAction` import.
- **Icon mode cycle missing NerdFont**: Cycling through icon modes (via Settings panel) skipped NerdFont entirely (`Unicode → Ascii → Unicode`). Cycle is now `Unicode → NerdFont → Ascii → Unicode`.
- **NerdFont icons invisible**: NerdFont PUA codepoints (U+E000–U+F8FF) are reported as width-1 by `unicode-width` but render as double-wide (2 columns) in NerdFont-configured terminals. Ratatui would write a space at column `x+1`, overwriting the right half of the glyph. Fixed by reserving 2 terminal columns for the glyph plus 1 trailing space (total 3-column icon slot), bypassing `unicode-width` for NerdFont mode layout.
- **Image preview freeze**: `render_image_preview()` called `imageops::resize()` with `Lanczos3` on the UI thread every render frame (~500ms on WSL), blocking all keyboard input. Fixed by: (1) caching the scaled result keyed by viewport dimensions so the resize only runs when the terminal is resized; (2) switching the render-time filter to `Triangle` (bilinear, ~10× faster, visually identical at halfblock resolution).
- **Tree-view connectors invisible in NerdFont mode**: Tree connectors (├ └ │) silently disappeared when icon mode was NerdFont because terminals like Warp + FiraCode NF activate font fallback for the entire row when PUA glyphs (U+E000–U+F8FF) are present, and the fallback face often lacks the light box-drawing glyphs. Switched to the heavy variants (┣ ┗ ┃) in NerdFont mode so the NerdFont itself supplies them, restoring the indented-tree affordance. Other modes are unchanged.
- **Diagnostic binary `zeta-font-test`**: Added under `src/bin/` to print NerdFont PUA glyphs alongside reference Unicode chars so users can verify their terminal's font face actually supports NerdFont before chasing a phantom Zeta bug.
- **Toggle Details View missing from command palette**: The `Ctrl+L` action existed in the View menu and the keymap but was absent from the command palette (`Ctrl+B`), making it invisible to discovery. Added under "View / Layout" with the `Ctrl+L` hint.
- **Toggle Details View only affecting one pane**: `Action::ToggleDetailsView` mutated only the active pane, leaving the other pane out of sync. The action now toggles both panes together so the user gets a consistent dual-pane view with one shortcut.
- **Toggle Details View status message ambiguous**: The status message after `Ctrl+L` read "details view on/off (both panes)" which was ambiguous — it was unclear whether "off" meant the view was just disabled or was currently in a disabled state. Reworded to "rich columns: enabled/hidden (both panes)" to clearly describe the resulting state.

## [0.5.0] - 2025-05-01

### Added
- **ThemePalette v2**: 13 new accent tokens (mauve, teal, green, yellow, peach, red, editor/preview/terminal focus borders) + CatppuccinMocha preset with exact RGB values across all 10 themes.
- **NerdFont icons v3**: `icon_mode = "nerd_font"` (alias `"custom"`) with per-extension codepoints for Rust, Python, JS/TS, Go, C/C++, Markdown, shell scripts, TOML/YAML/JSON, images, archives, symlinks. Falls back to Unicode/ASCII modes.
- **Modal halo ring**: Semi-transparent backdrop halo around all modals; all modal titles centered.
- **Menu bar context dimming**: Irrelevant menu tabs dim based on active panel (pane vs editor vs terminal); workspace switcher shows current CWD with home-dir replacement and truncation.
- **Pane column headers**: Name/Size/Date header row above file list. Active filter shown in teal accent bar with match count and "Esc clear" hint. Non-matching entries dimmed.
- **Zoned status bar**: Five zones — Git branch · active entry (icon, name, size, permissions) · job message · marks info · workspace name; animated progress bar during file operations.
- **Panel chrome titles**: Editor shows file icon, filename, parent dir, live Ln/Col, dirty indicator (●); Preview shows eye icon, filename, `.EXT` badge; Terminal shows terminal icon + Shell badge. Each panel uses its own accent color when focused.
- **Settings segmented tabs**: Tab bar with Appearance / Panels / Editor / Keymaps tabs; Tab/Shift+Tab/1–4 navigation; entries filtered per tab.
- **Two-column Help modal**: Key shortcuts rendered as pill spans with section headers; left column (Navigation + Files), right column (Editor + System). Independent column layout.
- **Mauve About logo**: ASCII art banner prefixed with `##LOGO` marker, rendered in accent mauve + bold.
- **Command palette match highlighting**: Per-character match highlighting in accent yellow; `⌕` input prefix; simplified footer.
- **Teal file finder**: `⌕` teal input, root hint, dir/filename split display, teal match highlighting. Unicode-safe `split_at_match()` utility for all highlighting.
- **`src/ui/highlight.rs`**: Reusable `split_at_match()` helper — character-aware, Unicode-safe case-insensitive substring splitting used by palette and finder.

### Fixed
- **Deletion modal bug**: Fixed issue where pressing F8 to delete marked items would show an incorrect prompt. When marking files/folders and pressing F8, the application now correctly displays a batch "Trash Marked Items" or "Delete Permanently Marked Items" confirmation prompt instead of showing "<missing target>" error. This fix aligns deletion behavior with copy and move operations, providing consistent batch operation UX across the application.

### Changed
- **Deletion workflow**: `OpenDeletePrompt` (F8) now uses batch prompts for multiple marked items (consistent with Copy/Move), while single selected items still use the DestructiveConfirm modal. Similarly, `OpenPermanentDeletePrompt` (Shift+F8) now opens batch Delete prompts for marked items.

### Added
- **CHANGELOG.md**: Created initial changelog to track fixes and enhancements going forward.
