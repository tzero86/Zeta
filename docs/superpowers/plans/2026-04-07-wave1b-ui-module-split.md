# Wave 1B — ui.rs Split into src/ui/ Modules + LayoutCache

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Break the 1,763-line `src/ui.rs` monolith into focused sub-modules under `src/ui/`, introduce a `LayoutCache` struct that records every panel's `Rect` during render, and change `render()` to return it so Wave 2B can do mouse hit-testing.

**Architecture:** `src/ui.rs` becomes `src/ui/mod.rs`. Ten focused files each own one rendering concern. `layout_cache.rs` is new code only — it captures the Rects computed during layout so the event loop can use them for mouse routing without re-computing layout. The `render()` signature changes from `fn render(…)` to `fn render(…) -> LayoutCache`. `app.rs` stores the returned cache on `App`.

**Tech Stack:** ratatui 0.29, crossterm 0.28, existing `src/` codebase. No new Cargo dependencies.

**Jira:** ZTA-79 (ZTA-95 through ZTA-101)

**Wave dependency:** Starts from `main`. Runs in parallel with Wave 1A and Wave 1C — owns only `src/ui.rs` and `src/app.rs`. Does NOT touch `src/state/`, `src/jobs.rs`, or `src/editor.rs`.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Delete | `src/ui.rs` | Replaced by the module directory |
| Create | `src/ui/mod.rs` | `render()` orchestration; re-exports `LayoutCache` |
| Create | `src/ui/layout_cache.rs` | `LayoutCache` struct + `rect_contains()` helper |
| Create | `src/ui/styles.rs` | All `Style`-returning helper functions |
| Create | `src/ui/code_view.rs` | `render_code_view` + `CodeViewRenderArgs` |
| Create | `src/ui/menu_bar.rs` | `render_menu_bar`, `top_bar_logo_spans`, `menu_spans` |
| Create | `src/ui/pane.rs` | `render_pane`, `render_item`, pane helpers |
| Create | `src/ui/preview.rs` | `render_preview_panel`, wrap helpers |
| Create | `src/ui/editor.rs` | `render_editor`, `editor_render_state`, helpers |
| Create | `src/ui/overlay.rs` | All four modal overlay renderers |
| Create | `src/ui/palette.rs` | `render_command_palette` |
| Create | `src/ui/settings.rs` | `render_settings_panel` |
| Modify | `src/app.rs` | Capture `LayoutCache` returned by `render()` |

---

## Task 1: Establish baseline and create LayoutCache

**Files:**
- Create: `src/ui/layout_cache.rs`

- [ ] **Step 1.1: Confirm tests pass on main**

```bash
cargo test
```

Expected: all tests pass (green).

- [ ] **Step 1.2: Create `src/ui/layout_cache.rs`**

```rust
use ratatui::layout::Rect;

/// Rects computed during each render frame.
/// Stored on `App` so the event loop can route mouse events
/// without re-running the layout algorithm.
#[derive(Clone, Copy, Debug, Default)]
pub struct LayoutCache {
    pub menu_bar: Rect,
    pub left_pane: Rect,
    pub right_pane: Rect,
    /// Present when the editor or preview panel is visible.
    pub tools_panel: Option<Rect>,
    pub status_bar: Rect,
}

/// Returns `true` if the terminal cell at (`col`, `row`) falls inside `rect`.
pub fn rect_contains(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x
        && col < rect.x + rect.width
        && row >= rect.y
        && row < rect.y + rect.height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_contains_returns_true_for_inner_cell() {
        let r = Rect { x: 5, y: 3, width: 10, height: 4 };
        assert!(rect_contains(r, 5, 3));
        assert!(rect_contains(r, 14, 6));
        assert!(rect_contains(r, 10, 5));
    }

    #[test]
    fn rect_contains_returns_false_for_border_outside() {
        let r = Rect { x: 5, y: 3, width: 10, height: 4 };
        assert!(!rect_contains(r, 15, 5)); // x == x + width (exclusive)
        assert!(!rect_contains(r, 10, 7)); // y == y + height (exclusive)
        assert!(!rect_contains(r, 4, 5)); // x < x
        assert!(!rect_contains(r, 10, 2)); // y < y
    }

    #[test]
    fn rect_contains_zero_size_rect_never_matches() {
        let r = Rect { x: 0, y: 0, width: 0, height: 0 };
        assert!(!rect_contains(r, 0, 0));
    }
}
```

- [ ] **Step 1.3: Run new tests**

```bash
cargo test layout_cache
```

Expected: 3 tests pass.

- [ ] **Step 1.4: Commit**

```bash
git add src/ui/layout_cache.rs
git commit -m "feat(ui): add LayoutCache and rect_contains helper"
```

---

## Task 2: Create styles.rs

**Files:**
- Create: `src/ui/styles.rs`

- [ ] **Step 2.1: Create `src/ui/styles.rs`**

Exact content extracted from `src/ui.rs` (functions `elevated_surface_style` through `command_palette_entry_hint_style`):

```rust
use ratatui::style::{Modifier, Style};

use crate::config::ThemePalette;

pub fn elevated_surface_style(palette: ThemePalette) -> Style {
    Style::default().bg(palette.tools_bg)
}

pub fn overlay_title_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.menu_mnemonic_fg)
        .add_modifier(Modifier::BOLD)
}

pub fn overlay_key_hint_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.key_hint_fg)
        .add_modifier(Modifier::BOLD)
}

pub fn overlay_footer_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.text_muted)
}

pub fn command_palette_header_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.text_muted)
        .add_modifier(Modifier::BOLD)
}

pub fn command_palette_entry_label_style(is_selected: bool, palette: ThemePalette) -> Style {
    if is_selected {
        Style::default()
            .fg(palette.selection_fg)
            .bg(palette.selection_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_primary)
    }
}

pub fn command_palette_entry_hint_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.key_hint_fg)
}
```

- [ ] **Step 2.2: Run cargo check**

```bash
cargo check
```

Expected: compiles (no errors — styles.rs is not yet used by anything).

- [ ] **Step 2.3: Commit**

```bash
git add src/ui/styles.rs
git commit -m "feat(ui): extract style helper functions to ui/styles.rs"
```

---

## Task 3: Create code_view.rs

**Files:**
- Create: `src/ui/code_view.rs`

- [ ] **Step 3.1: Create `src/ui/code_view.rs`**

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthChar;

use crate::config::ThemePalette;
use crate::highlight::HighlightedLine;

pub struct CodeViewRenderArgs<'a> {
    pub lines: &'a [HighlightedLine],
    pub first_line_number: usize,
    pub gutter_width: u16,
    pub scroll_col: usize,
    pub cursor_row: Option<usize>,
    pub palette: ThemePalette,
}

pub fn render_code_view(frame: &mut Frame<'_>, area: Rect, args: CodeViewRenderArgs<'_>) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(args.gutter_width), Constraint::Min(1)])
        .split(area);
    let gutter_area = chunks[0];
    let content_area = chunks[1];
    let viewport_cols = content_area.width as usize;

    let blank_style = Style::default().bg(args.palette.surface_bg);

    for row_idx in 0..area.height as usize {
        let y = area.y + row_idx as u16;
        if y >= area.y + area.height {
            break;
        }
        let gutter_rect = Rect { x: gutter_area.x, y, width: gutter_area.width, height: 1 };
        let content_rect = Rect { x: content_area.x, y, width: content_area.width, height: 1 };
        frame.render_widget(Paragraph::new(" ").style(blank_style), gutter_rect);
        frame.render_widget(Paragraph::new(" ").style(blank_style), content_rect);
    }

    for (row_idx, line_tokens) in args.lines.iter().enumerate() {
        let y = area.y + row_idx as u16;
        if y >= area.y + area.height {
            break;
        }

        let line_num = args.first_line_number + row_idx;
        let gutter_text = format!(
            "{:>width$} ",
            line_num,
            width = (args.gutter_width as usize).saturating_sub(2)
        );
        let gutter_rect = Rect { x: gutter_area.x, y, width: gutter_area.width, height: 1 };
        let gutter_style = Style::default()
            .fg(args.palette.text_muted)
            .bg(args.palette.surface_bg);
        frame.render_widget(Paragraph::new(gutter_text).style(gutter_style), gutter_rect);

        let content_rect = Rect { x: content_area.x, y, width: content_area.width, height: 1 };
        let row_bg = if args.cursor_row == Some(row_idx) {
            Style::default().bg(args.palette.selection_bg)
        } else {
            Style::default().bg(args.palette.surface_bg)
        };

        let mut spans: Vec<Span> = Vec::new();
        let mut raw_cols = 0usize;
        let mut visible_cols = 0usize;
        for (color, modifier, text) in line_tokens {
            let token_chars: Vec<char> = text.chars().collect();
            let token_start = raw_cols;
            let token_width = token_chars
                .iter()
                .map(|ch| UnicodeWidthChar::width(*ch).unwrap_or(0))
                .sum::<usize>();
            let token_end = raw_cols + token_width;
            raw_cols = token_end;

            if token_end <= args.scroll_col {
                continue;
            }

            let skip = args.scroll_col.saturating_sub(token_start);
            let mut visible_chars = String::new();
            let mut skipped = 0usize;
            let mut used_width = 0usize;
            for ch in token_chars.iter().skip_while(|_| {
                if skipped < skip {
                    skipped += 1;
                    true
                } else {
                    false
                }
            }) {
                let ch_width = UnicodeWidthChar::width(*ch).unwrap_or(0);
                if used_width + ch_width > viewport_cols.saturating_sub(visible_cols) {
                    break;
                }
                used_width += ch_width;
                visible_chars.push(*ch);
            }
            if !visible_chars.is_empty() {
                visible_cols += used_width;
                spans.push(Span::styled(
                    visible_chars,
                    Style::default().fg(*color).add_modifier(*modifier),
                ));
            }
        }

        if visible_cols < viewport_cols {
            spans.push(Span::raw(" ".repeat(viewport_cols - visible_cols)));
        }

        let line = Line::from(spans);
        frame.render_widget(Paragraph::new(line).style(row_bg), content_rect);
    }
}
```

- [ ] **Step 3.2: Run cargo check**

```bash
cargo check
```

Expected: compiles.

- [ ] **Step 3.3: Commit**

```bash
git add src/ui/code_view.rs
git commit -m "feat(ui): extract render_code_view to ui/code_view.rs"
```

---

## Task 4: Create menu_bar.rs

**Files:**
- Create: `src/ui/menu_bar.rs`

- [ ] **Step 4.1: Create `src/ui/menu_bar.rs`**

```rust
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{layout::Rect, Frame};

use crate::action::MenuId;
use crate::config::ThemePalette;
use crate::state::AppState;

pub fn render_menu_bar(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    palette: ThemePalette,
) {
    let active = state.active_menu().is_none();
    let top_bar_bg = if active {
        palette.menu_active_bg
    } else {
        palette.menu_bg
    };
    let mut line = Line::default();
    line.spans.extend(top_bar_logo_spans(active, palette));
    line.spans.extend(menu_spans(
        " File ",
        Some('F'),
        state.active_menu() == Some(MenuId::File),
        palette,
    ));
    line.spans.extend(menu_spans(
        " Navigate ",
        Some('N'),
        state.active_menu() == Some(MenuId::Navigate),
        palette,
    ));
    line.spans.extend(menu_spans(
        " View ",
        Some('V'),
        state.active_menu() == Some(MenuId::View),
        palette,
    ));
    line.spans.extend(menu_spans(
        " Help ",
        Some('H'),
        state.active_menu() == Some(MenuId::Help),
        palette,
    ));

    let menu = Paragraph::new(line).style(
        Style::default()
            .fg(palette.menu_fg)
            .bg(top_bar_bg)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(menu, area);
}

pub fn top_bar_logo_spans(active: bool, palette: ThemePalette) -> Vec<Span<'static>> {
    let bg = if active {
        palette.menu_active_bg
    } else {
        palette.menu_bg
    };
    let bracket_style = Style::default()
        .fg(palette.logo_accent)
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    let letter_style = Style::default()
        .fg(palette.logo_accent)
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    let name_style = Style::default()
        .fg(palette.menu_fg)
        .bg(bg)
        .add_modifier(Modifier::BOLD);

    vec![
        Span::styled(" ", Style::default().bg(bg)),
        Span::styled("[", bracket_style),
        Span::styled("Z", letter_style),
        Span::styled("]", bracket_style),
        Span::styled("eta ", name_style),
    ]
}

fn menu_spans(
    label: &'static str,
    mnemonic: Option<char>,
    active: bool,
    palette: ThemePalette,
) -> Vec<Span<'static>> {
    let style = if active {
        Style::default()
            .fg(palette.menu_fg)
            .bg(palette.menu_active_bg)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(palette.menu_fg).bg(palette.menu_bg)
    };

    let highlighted = mnemonic.map(|value| value.to_ascii_uppercase());
    let mut spans = Vec::with_capacity(label.len());
    let mut used_highlight = false;

    for ch in label.chars() {
        let mut char_style = style;
        if !used_highlight && Some(ch.to_ascii_uppercase()) == highlighted {
            char_style = char_style.fg(palette.menu_mnemonic_fg);
            used_highlight = true;
        }
        spans.push(Span::styled(ch.to_string(), char_style));
    }

    spans
}
```

- [ ] **Step 4.2: Commit**

```bash
git add src/ui/menu_bar.rs
git commit -m "feat(ui): extract menu bar rendering to ui/menu_bar.rs"
```

---

## Task 5: Create pane.rs

**Files:**
- Create: `src/ui/pane.rs`

- [ ] **Step 5.1: Create `src/ui/pane.rs`**

```rust
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

use crate::config::{IconMode, ThemePalette};
use crate::fs::{EntryInfo, EntryKind};
use crate::icon::icon_for_kind;
use crate::pane::PaneState;
use crate::state::AppState;

pub struct PaneChrome {
    pub border: Style,
    pub title: Style,
    pub surface: Style,
}

pub fn pane_chrome_style(is_focused: bool, palette: ThemePalette) -> PaneChrome {
    if is_focused {
        PaneChrome {
            border: Style::default()
                .fg(palette.border_focus)
                .add_modifier(Modifier::BOLD),
            title: Style::default()
                .fg(palette.border_focus)
                .add_modifier(Modifier::BOLD),
            surface: Style::default().bg(palette.surface_bg),
        }
    } else {
        PaneChrome {
            border: Style::default().fg(palette.text_muted),
            title: Style::default().fg(palette.text_muted),
            surface: Style::default().bg(palette.tools_bg),
        }
    }
}

pub fn render_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    pane: &PaneState,
    label: &str,
    is_focused: bool,
    borders: Borders,
    state: &AppState,
) {
    let palette = state.theme().palette;
    let icon_mode = state.icon_mode();
    let chrome = pane_chrome_style(is_focused, palette);

    let title = format!(
        "{} [{}]  {}  ({})",
        label,
        pane.entries.len(),
        pane.cwd.display(),
        pane.sort_mode.label()
    );
    let block = Block::default()
        .title(Span::styled(title, chrome.title))
        .borders(borders)
        .border_style(chrome.border)
        .style(chrome.surface);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let list_area = inner;
    let visible_height = list_area.height as usize;
    let visible_entries = pane.visible_entries(visible_height);
    let items: Vec<ListItem<'_>> = if pane.entries.is_empty() {
        vec![ListItem::new("(empty)")]
    } else {
        visible_entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                render_item(
                    entry,
                    is_focused,
                    pane.is_marked(&entry.path),
                    index + 1 == visible_entries.len(),
                    list_area.width as usize,
                    palette,
                    icon_mode,
                )
            })
            .collect()
    };

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(palette.selection_bg)
                .fg(palette.selection_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut list_state = ListState::default();
    if !pane.entries.is_empty() {
        list_state.select(pane.visible_selection(visible_height));
    }

    frame.render_stateful_widget(list.style(chrome.surface), list_area, &mut list_state);
}

pub fn render_item(
    entry: &EntryInfo,
    is_focused: bool,
    is_marked: bool,
    is_last: bool,
    available_width: usize,
    palette: ThemePalette,
    icon_mode: IconMode,
) -> ListItem<'static> {
    let row_styles = pane_row_styles(is_focused, is_marked, entry.kind, palette);
    let guide = if is_last { "  " } else { "│ " };
    let branch = if is_last { "└" } else { "├" };
    let icon = icon_for_kind(entry.kind, icon_mode);
    let mark_prefix = if is_marked { "* " } else { "  " };
    let name = match entry.kind {
        EntryKind::Directory => format!("{}/", entry.name),
        _ => entry.name.clone(),
    };
    let meta = format_entry_meta(entry);
    let icon_slot = format_icon_slot(icon, icon_mode);
    let prefix = format!("{}{}{} {} ", guide, branch, mark_prefix, icon_slot);
    let prefix_width = display_width(&prefix);
    let meta_width = display_width(&meta);
    let content_width = available_width.saturating_sub(2);
    let name_width = content_width
        .saturating_sub(prefix_width)
        .saturating_sub(meta_width)
        .saturating_sub(1)
        .max(1);
    let name = truncate_text(&name, name_width);
    let spacer_width = content_width
        .saturating_sub(prefix_width)
        .saturating_sub(display_width(&name))
        .saturating_sub(meta_width)
        .max(1);

    ListItem::new(Line::from(vec![
        Span::styled(guide, row_styles.guide),
        Span::styled(format!("{} ", branch), row_styles.branch),
        Span::styled(mark_prefix.to_string(), row_styles.mark),
        Span::styled(format!("{} ", icon_slot), row_styles.icon),
        Span::styled(name, row_styles.name),
        Span::styled(" ".repeat(spacer_width), Style::default()),
        Span::styled(meta, row_styles.meta),
    ]))
}

pub struct PaneRowStyles {
    pub guide: Style,
    pub branch: Style,
    pub mark: Style,
    pub icon: Style,
    pub name: Style,
    pub meta: Style,
}

pub fn pane_row_styles(
    is_focused: bool,
    is_marked: bool,
    kind: EntryKind,
    palette: ThemePalette,
) -> PaneRowStyles {
    let text_tone = if is_focused {
        palette.text_primary
    } else {
        palette.text_muted
    };
    let label_style = match kind {
        EntryKind::Directory => Style::default()
            .fg(palette.directory_fg)
            .add_modifier(Modifier::BOLD),
        EntryKind::Symlink => Style::default().fg(palette.symlink_fg),
        EntryKind::File => Style::default().fg(palette.file_fg),
        EntryKind::Other => Style::default().fg(text_tone),
    };

    PaneRowStyles {
        guide: Style::default().fg(text_tone),
        branch: Style::default().fg(text_tone),
        mark: if is_marked {
            Style::default().fg(palette.key_hint_fg)
        } else {
            Style::default().fg(text_tone)
        },
        icon: label_style,
        name: if is_focused {
            label_style
        } else {
            label_style.fg(match kind {
                EntryKind::Directory => palette.directory_fg,
                EntryKind::Symlink => palette.symlink_fg,
                EntryKind::File => palette.file_fg,
                EntryKind::Other => text_tone,
            })
        },
        meta: Style::default().fg(if is_focused {
            palette.text_primary
        } else {
            palette.text_muted
        }),
    }
}

pub fn format_icon_slot(icon: &str, icon_mode: IconMode) -> String {
    match icon_mode {
        IconMode::Unicode | IconMode::Custom => format!("{icon}  "),
        IconMode::Ascii => icon.to_string(),
    }
}

pub fn display_width(value: &str) -> usize {
    value.chars().count()
}

pub fn truncate_text(value: &str, max_width: usize) -> String {
    let width = display_width(value);
    if width <= max_width {
        return value.to_string();
    }
    if max_width <= 2 {
        return value.chars().take(max_width).collect();
    }
    let truncated: String = value.chars().take(max_width - 2).collect();
    format!("{}..", truncated)
}

pub fn format_entry_meta(entry: &EntryInfo) -> String {
    match entry.kind {
        EntryKind::Directory => String::from("dir"),
        EntryKind::Symlink => String::from("link"),
        EntryKind::Other => String::from("other"),
        EntryKind::File => {
            let ext = entry
                .path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase());
            let kind = match ext.as_deref() {
                Some("rs") => "rust",
                Some("md") => "markdown",
                Some("toml") => "config",
                Some("json") | Some("jsonc") => "json",
                Some("yml") | Some("yaml") => "yaml",
                Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("webp") => "image",
                Some("txt") => "text",
                Some(_) | None => "file",
            };
            match entry.size_bytes {
                Some(size) => format!("{} {}", kind, human_size(size)),
                None => String::from(kind),
            }
        }
    }
}

pub fn human_size(size: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = size as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{}{}", size, UNITS[unit])
    } else {
        format!("{value:.1}{}", UNITS[unit])
    }
}
```

- [ ] **Step 5.2: Commit**

```bash
git add src/ui/pane.rs
git commit -m "feat(ui): extract pane rendering to ui/pane.rs"
```

---

## Task 6: Create preview.rs

**Files:**
- Create: `src/ui/preview.rs`

- [ ] **Step 6.1: Create `src/ui/preview.rs`**

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthChar;

use crate::config::ThemePalette;
use crate::highlight::HighlightedLine;
use crate::preview::ViewBuffer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedPreviewRow {
    pub line_number: usize,
    pub line_tokens: HighlightedLine,
}

pub fn wrap_preview_line(
    line_number: usize,
    line_tokens: &HighlightedLine,
    viewport_cols: usize,
) -> Vec<WrappedPreviewRow> {
    if viewport_cols == 0 {
        return vec![];
    }

    let mut rows: Vec<WrappedPreviewRow> = Vec::new();
    let mut current_row: HighlightedLine = Vec::new();
    let mut current_width = 0usize;

    for (color, modifier, text) in line_tokens {
        let mut chunk = String::new();

        for ch in text.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1).max(1);

            if current_width > 0 && current_width + ch_width > viewport_cols {
                if !chunk.is_empty() {
                    current_row.push((*color, *modifier, std::mem::take(&mut chunk)));
                }
                rows.push(WrappedPreviewRow {
                    line_number,
                    line_tokens: std::mem::take(&mut current_row),
                });
                current_width = 0;
            }

            chunk.push(ch);
            current_width += ch_width;

            if current_width >= viewport_cols {
                current_row.push((*color, *modifier, std::mem::take(&mut chunk)));
                rows.push(WrappedPreviewRow {
                    line_number,
                    line_tokens: std::mem::take(&mut current_row),
                });
                current_width = 0;
            }
        }

        if !chunk.is_empty() {
            current_row.push((*color, *modifier, chunk));
        }
    }

    if !current_row.is_empty() {
        rows.push(WrappedPreviewRow { line_number, line_tokens: current_row });
    }

    if rows.is_empty() {
        rows.push(WrappedPreviewRow { line_number, line_tokens: vec![] });
    }

    rows
}

pub fn preview_gutter_label(line_number: usize, is_continuation: bool) -> String {
    if is_continuation {
        "     ".to_string()
    } else {
        format!("{:>4} ", line_number)
    }
}

pub fn render_wrapped_preview_view(
    frame: &mut Frame<'_>,
    area: Rect,
    lines: &[HighlightedLine],
    first_line_number: usize,
    palette: ThemePalette,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(6), Constraint::Min(1)])
        .split(area);
    let gutter_area = chunks[0];
    let content_area = chunks[1];

    let blank_style = Style::default().bg(palette.tools_bg);
    for row_idx in 0..area.height as usize {
        let y = area.y + row_idx as u16;
        let gutter_rect = Rect { x: gutter_area.x, y, width: gutter_area.width, height: 1 };
        let content_rect = Rect { x: content_area.x, y, width: content_area.width, height: 1 };
        frame.render_widget(Paragraph::new(" ").style(blank_style), gutter_rect);
        frame.render_widget(Paragraph::new(" ").style(blank_style), content_rect);
    }

    let mut visual_row = 0usize;
    for (source_idx, line_tokens) in lines.iter().enumerate() {
        let wrapped_rows = wrap_preview_line(
            first_line_number + source_idx,
            line_tokens,
            content_area.width as usize,
        );
        for (wrap_idx, row) in wrapped_rows.into_iter().enumerate() {
            let y = area.y + visual_row as u16;
            if y >= area.y + area.height {
                return;
            }

            let gutter_text = preview_gutter_label(row.line_number, wrap_idx > 0);
            let gutter_rect = Rect { x: gutter_area.x, y, width: gutter_area.width, height: 1 };
            frame.render_widget(
                Paragraph::new(gutter_text).style(
                    Style::default()
                        .fg(palette.text_muted)
                        .bg(palette.surface_bg),
                ),
                gutter_rect,
            );

            let content_rect = Rect { x: content_area.x, y, width: content_area.width, height: 1 };
            let spans: Vec<Span> = row
                .line_tokens
                .iter()
                .map(|(color, modifier, text)| {
                    Span::styled(
                        text.clone(),
                        Style::default().fg(*color).add_modifier(*modifier),
                    )
                })
                .collect();
            frame.render_widget(
                Paragraph::new(Line::from(spans)).style(Style::default().bg(palette.surface_bg)),
                content_rect,
            );
            visual_row += 1;
        }
    }
}

pub fn render_preview_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    view: Option<&ViewBuffer>,
    filename: &str,
    is_focused: bool,
    palette: ThemePalette,
) {
    let border_style = if is_focused {
        Style::default()
            .fg(palette.border_focus)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_muted)
    };
    let title = format!(" Preview  {} ", filename);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(palette.tools_bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    match view {
        None => frame.render_widget(
            Paragraph::new("select a file to preview")
                .style(Style::default().fg(palette.text_muted).bg(palette.tools_bg)),
            inner,
        ),
        Some(v) => {
            let height = inner.height as usize;
            let (first_line_num, window) = v.visible_window(height);
            if window.is_empty() {
                return;
            }
            render_wrapped_preview_view(frame, inner, window, first_line_num + 1, palette);
        }
    }
}
```

- [ ] **Step 6.2: Commit**

```bash
git add src/ui/preview.rs
git commit -m "feat(ui): extract preview rendering to ui/preview.rs"
```

---

## Task 7: Create ui/editor.rs

**Files:**
- Create: `src/ui/editor.rs`

- [ ] **Step 7.1: Create `src/ui/editor.rs`**

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::editor::{EditorBuffer, EditorRenderState};
use crate::ui::code_view::{render_code_view, CodeViewRenderArgs};

pub fn editor_render_state(
    editor: &mut EditorBuffer,
    area: Rect,
    is_active: bool,
) -> EditorRenderState {
    let viewport_cols = area.width.saturating_sub(6) as usize;
    let viewport_rows = area.height.saturating_sub(2) as usize;
    editor.render_state(viewport_rows, viewport_cols, is_active)
}

pub fn editor_highlighted_render_state(
    editor: &EditorBuffer,
    area: Rect,
    syntect_theme: &str,
    palette: ThemePalette,
) -> (usize, Vec<crate::highlight::HighlightedLine>) {
    let height = area.height.saturating_sub(2) as usize;
    editor.visible_highlighted_window(height, syntect_theme, palette.text_primary)
}

pub fn render_editor(
    frame: &mut Frame<'_>,
    area: Rect,
    editor: &mut EditorBuffer,
    render_state: &EditorRenderState,
    is_focused: bool,
    palette: ThemePalette,
    syntect_theme: &str,
) {
    let border_style = if is_focused {
        Style::default()
            .fg(palette.border_editor_focus)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_muted)
    };

    let path = editor
        .path
        .as_ref()
        .map(|value| value.display().to_string())
        .unwrap_or_else(|| String::from("<unnamed>"));
    let dirty_marker = if editor.is_dirty { "*" } else { "" };
    let title = format!("Editor{}  {}", dirty_marker, path);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (content_area, search_bar_area) = if editor.search_active {
        let splits = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        (splits[0], Some(splits[1]))
    } else {
        (inner, None)
    };

    let gutter_width = 6u16;
    let (first_line_num, highlighted) =
        editor_highlighted_render_state(editor, content_area, syntect_theme, palette);

    render_code_view(
        frame,
        content_area,
        CodeViewRenderArgs {
            lines: &highlighted,
            first_line_number: first_line_num + 1,
            gutter_width,
            scroll_col: render_state.scroll_col,
            cursor_row: render_state.cursor_visible_row,
            palette,
        },
    );

    if let Some(bar_area) = search_bar_area {
        let matches = editor.find_matches(&editor.search_query.clone());
        let count_str = if editor.search_query.is_empty() {
            String::new()
        } else if matches.is_empty() {
            String::from("  0/0")
        } else {
            let current = editor.search_match_idx.min(matches.len() - 1) + 1;
            format!("  {current}/{count}", count = matches.len())
        };
        let bar_text = format!(
            " Find: {}{}  [Enter/F3 next  Shift+F3 prev  Esc close]",
            editor.search_query, count_str
        );
        let bar = Paragraph::new(bar_text).style(
            Style::default()
                .fg(palette.text_primary)
                .bg(palette.selection_bg),
        );
        frame.render_widget(bar, bar_area);
    }

    if render_state.cursor_visible_row.is_some() {
        let (line, column) = editor.cursor_line_col();
        let visible_line = line.saturating_sub(render_state.visible_start);
        let content_x = content_area.x + gutter_width;
        let cursor_y =
            content_area.y + (visible_line as u16).min(content_area.height.saturating_sub(1));
        let visible_col = column.saturating_sub(render_state.scroll_col);
        let cursor_x = content_x
            + (visible_col as u16).min(content_area.width.saturating_sub(gutter_width + 1));
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}
```

- [ ] **Step 7.2: Commit**

```bash
git add src/ui/editor.rs
git commit -m "feat(ui): extract editor rendering to ui/editor.rs"
```

---

## Task 8: Create overlay.rs

**Files:**
- Create: `src/ui/overlay.rs`

- [ ] **Step 8.1: Create `src/ui/overlay.rs`**

```rust
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::action::MenuId;
use crate::config::ThemePalette;
use crate::state::{CollisionState, DialogState, MenuItem, PromptState};
use crate::ui::styles::{elevated_surface_style, overlay_footer_style, overlay_key_hint_style, overlay_title_style};

pub fn render_menu_popup(
    frame: &mut Frame<'_>,
    area: Rect,
    menu: MenuId,
    items: &[MenuItem],
    selection: usize,
    palette: ThemePalette,
) {
    let x = match menu {
        MenuId::File => area.x + 1,
        MenuId::Navigate => area.x + 8,
        MenuId::View => area.x + 19,
        MenuId::Help => area.x + 26,
    };
    let width = 28;
    let height = items.len() as u16 + 2;
    let popup_area = Rect { x, y: area.y, width, height };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.prompt_border))
        .style(Style::default().bg(palette.surface_bg));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let rows = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let selected = index == selection;
            let base_style = if selected {
                Style::default()
                    .fg(palette.menu_fg)
                    .bg(palette.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(palette.text_primary)
                    .bg(palette.surface_bg)
            };

            let content_width = inner.width.saturating_sub(2) as usize;
            let label_width = content_width.saturating_sub(item.shortcut.len() + 1);
            let label = format!(" {:<width$}", item.label, width = label_width.max(1));
            let shortcut = item.shortcut.to_string();
            ListItem::new(Line::from(vec![
                Span::styled(label, base_style),
                Span::styled(shortcut, base_style),
            ]))
        })
        .collect::<Vec<_>>();

    let list = List::new(rows);
    let mut state = ListState::default();
    state.select(Some(selection.min(items.len().saturating_sub(1))));
    frame.render_stateful_widget(list, inner, &mut state);
}

pub fn render_prompt(
    frame: &mut Frame<'_>,
    area: Rect,
    prompt: &PromptState,
    palette: ThemePalette,
) {
    let (width, height) = match prompt.kind {
        crate::state::PromptKind::Copy | crate::state::PromptKind::Move => {
            (area.width.min(76), area.height.min(8))
        }
        crate::state::PromptKind::Delete => (area.width.min(64), area.height.min(6)),
        _ => (area.width.min(56), area.height.min(6)),
    };
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect { x, y, width, height };

    let block = Block::default()
        .title(Span::styled(prompt.title, overlay_title_style(palette)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.prompt_border))
        .style(elevated_surface_style(palette));
    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let body = match prompt.kind {
        crate::state::PromptKind::Delete => format!(
            "Delete target:\n{}\n\nEnter confirm | Esc cancel",
            prompt
                .source_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| String::from("<missing target>")),
        ),
        crate::state::PromptKind::Copy | crate::state::PromptKind::Move => format!(
            "Source:\n{}\n\nDestination:\n{}\n\nEnter submit | Esc cancel",
            prompt
                .source_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| String::from("<missing source>")),
            prompt.value,
        ),
        _ => format!(
            "Path: {}\nValue: {}\nEnter submit | Esc cancel",
            prompt.base_path.display(),
            prompt.value
        ),
    };
    let paragraph = Paragraph::new(body)
        .style(Style::default().bg(palette.tools_bg).fg(palette.text_primary))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

pub fn render_dialog(
    frame: &mut Frame<'_>,
    area: Rect,
    dialog: &DialogState,
    palette: ThemePalette,
) {
    let width = area.width.min(68);
    let height = ((dialog.lines.len() as u16) + 2).min(area.height.saturating_sub(2).max(6));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect { x, y, width, height };

    let block = Block::default()
        .title(Span::styled(dialog.title, overlay_title_style(palette)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.prompt_border))
        .style(elevated_surface_style(palette));
    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let styled_lines: Vec<Line> = dialog
        .lines
        .iter()
        .map(|raw| {
            if raw.is_empty() {
                Line::raw("")
            } else if let Some(header) = raw.strip_prefix("##") {
                Line::from(Span::styled(
                    header.to_string(),
                    Style::default()
                        .fg(palette.menu_mnemonic_fg)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if let Some((key, desc)) = raw.split_once('\t') {
                let key_part = key.trim_start();
                let indent_len = raw.len() - raw.trim_start().len();
                let indent = " ".repeat(indent_len);
                Line::from(vec![
                    Span::raw(indent),
                    Span::styled(key_part.to_string(), overlay_key_hint_style(palette)),
                    Span::raw("  "),
                    Span::styled(desc.to_string(), Style::default().fg(palette.text_primary)),
                ])
            } else if raw == " ____  ________  ____             __               " {
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled(
                        "____  ________  ____             __               ",
                        Style::default().fg(palette.text_primary),
                    ),
                ])
            } else {
                Line::from(Span::styled(
                    raw.clone(),
                    Style::default().fg(palette.text_primary),
                ))
            }
        })
        .collect();

    let paragraph = Paragraph::new(styled_lines).style(elevated_surface_style(palette));
    frame.render_widget(paragraph, inner);
}

pub fn render_collision_dialog(
    frame: &mut Frame<'_>,
    area: Rect,
    collision: &CollisionState,
    palette: ThemePalette,
) {
    let lines = collision.lines();
    let width = area.width.min(72);
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2).max(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect { x, y, width, height };

    let block = Block::default()
        .title("Resolve Collision")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(palette.prompt_border)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(palette.prompt_bg));
    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let styled_lines: Vec<Line> = lines
        .iter()
        .map(|raw| {
            if raw.is_empty() {
                Line::raw("")
            } else if let Some((key, desc)) = raw.split_once('\t') {
                let key_part = key.trim_start();
                let indent_len = raw.len() - raw.trim_start().len();
                let indent = " ".repeat(indent_len);
                Line::from(vec![
                    Span::raw(indent),
                    Span::styled(
                        key_part.to_string(),
                        Style::default()
                            .fg(palette.key_hint_fg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(desc.to_string(), Style::default().fg(palette.text_primary)),
                ])
            } else {
                Line::from(Span::styled(
                    raw.clone(),
                    Style::default().fg(palette.text_primary),
                ))
            }
        })
        .collect();

    let paragraph = Paragraph::new(styled_lines).style(Style::default().bg(palette.prompt_bg));
    frame.render_widget(paragraph, inner);
}

pub fn render_footer_hint(
    frame: &mut Frame<'_>,
    area: Rect,
    text: &str,
    palette: ThemePalette,
) {
    frame.render_widget(
        Paragraph::new(text).style(overlay_footer_style(palette)),
        area,
    );
}
```

- [ ] **Step 8.2: Commit**

```bash
git add src/ui/overlay.rs
git commit -m "feat(ui): extract modal overlay renderers to ui/overlay.rs"
```

---

## Task 9: Create palette.rs and settings.rs

**Files:**
- Create: `src/ui/palette.rs`
- Create: `src/ui/settings.rs`

- [ ] **Step 9.1: Create `src/ui/palette.rs`**

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::ui::styles::{
    command_palette_entry_hint_style, command_palette_entry_label_style,
    command_palette_header_style, elevated_surface_style, overlay_footer_style,
    overlay_title_style,
};

pub fn render_command_palette(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &crate::palette::PaletteState,
    palette: ThemePalette,
) {
    let width = ((area.width as f32 * 0.90) as u16).clamp(40, 80).min(area.width);
    let max_results = 15usize;
    let height = (max_results as u16 + 5).min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect { x, y, width, height };

    let block = Block::default()
        .title(Span::styled(" Command Palette ", overlay_title_style(palette)))
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(palette.border_focus)
                .add_modifier(Modifier::BOLD),
        )
        .style(elevated_surface_style(palette));
    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let input_line = format!("> {}_", state.query);
    let input = Paragraph::new(input_line).style(
        Style::default()
            .fg(palette.text_primary)
            .bg(palette.tools_bg)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(input, chunks[0]);

    let footer = Paragraph::new("Type to filter • Enter to run • Esc to close")
        .style(overlay_footer_style(palette));
    frame.render_widget(footer, chunks[2]);

    let entries = crate::palette::all_entries();
    let matches = crate::palette::filter_entries(&entries, &state.query);
    let visible_height = chunks[1].height as usize;

    #[derive(Clone, Copy)]
    enum Row<'a> {
        Header(&'a str),
        Entry(&'a crate::palette::PaletteEntry),
    }

    let mut rows: Vec<Row<'_>> = Vec::new();
    let mut last_category = "";
    for entry in &matches {
        if entry.category != last_category {
            rows.push(Row::Header(entry.category));
            last_category = entry.category;
        }
        rows.push(Row::Entry(entry));
    }

    let selected_match_index = state.selection.min(matches.len().saturating_sub(1));
    let mut selected_row_index = None;
    let mut match_index = 0usize;
    for (row_index, row) in rows.iter().enumerate() {
        if let Row::Entry(_) = row {
            if match_index == selected_match_index {
                selected_row_index = Some(row_index);
                break;
            }
            match_index += 1;
        }
    }

    let selected_row_index = selected_row_index.unwrap_or(0);
    let scroll_start = if selected_row_index >= visible_height {
        selected_row_index - visible_height + 1
    } else {
        0
    };

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .skip(scroll_start)
        .take(visible_height)
        .map(|(row_index, row)| match row {
            Row::Header(category) => ListItem::new(Line::from(Span::styled(
                format!(" {category}"),
                command_palette_header_style(palette),
            ))),
            Row::Entry(entry) => {
                let is_selected = row_index == selected_row_index;
                let label_style = command_palette_entry_label_style(is_selected, palette);
                let hint_style = command_palette_entry_hint_style(palette);
                let hint = entry.hint;
                let label_max = (inner.width as usize).saturating_sub(hint.len() + 4);
                let label_text: String = entry.label.chars().take(label_max).collect();
                let pad = label_max.saturating_sub(label_text.chars().count());
                let padding = " ".repeat(pad);

                let line = Line::from(vec![
                    Span::raw(" "),
                    Span::styled(label_text + &padding, label_style),
                    Span::raw("  "),
                    Span::styled(hint.to_string(), hint_style),
                    Span::raw(" "),
                ]);
                ListItem::new(line)
            }
        })
        .collect();

    let list = List::new(items).style(elevated_surface_style(palette));
    frame.render_widget(list, chunks[1]);
}
```

- [ ] **Step 9.2: Create `src/ui/settings.rs`**

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::state::{AppState, SettingsState};
use crate::ui::styles::{elevated_surface_style, overlay_footer_style, overlay_title_style};

pub fn render_settings_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    settings: &SettingsState,
    state: &AppState,
    palette: ThemePalette,
) {
    let entries = state.settings_entries();
    let width = ((area.width as f32 * 0.72) as u16).clamp(52, 84).min(area.width);
    let height = (entries.len() as u16 + 6).min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect { x, y, width, height };

    let block = Block::default()
        .title(Span::styled(" Settings ", overlay_title_style(palette)))
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(palette.border_focus)
                .add_modifier(Modifier::BOLD),
        )
        .style(elevated_surface_style(palette));
    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let intro =
        Paragraph::new("Enter/Space toggles • Esc closes • future keymap controls reserved")
            .style(overlay_footer_style(palette));
    frame.render_widget(intro, chunks[0]);

    let rows: Vec<ListItem<'_>> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let selected = index == settings.selection;
            let base_style = if selected {
                Style::default()
                    .fg(palette.selection_fg)
                    .bg(palette.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_primary)
            };
            let line = Line::from(vec![
                Span::styled(format!(" {:<24}", entry.label), base_style),
                Span::styled(entry.value.clone(), Style::default().fg(palette.key_hint_fg)),
                Span::raw(format!("  {}", entry.hint)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(settings.selection.min(entries.len().saturating_sub(1))));
    frame.render_stateful_widget(
        List::new(rows).style(elevated_surface_style(palette)),
        chunks[1],
        &mut list_state,
    );

    let footer = Paragraph::new("Ctrl+O opens settings • theme, icons, preview, layout")
        .style(overlay_footer_style(palette));
    frame.render_widget(footer, chunks[2]);
}
```

- [ ] **Step 9.3: Commit**

```bash
git add src/ui/palette.rs src/ui/settings.rs
git commit -m "feat(ui): extract palette and settings panel renderers"
```

---

## Task 10: Create mod.rs, wire modules, delete ui.rs

**Files:**
- Create: `src/ui/mod.rs`
- Delete: `src/ui.rs`
- Modify: `src/app.rs`

- [ ] **Step 10.1: Create `src/ui/mod.rs`**

```rust
mod code_view;
mod editor;
mod menu_bar;
mod overlay;
mod palette;
mod pane;
mod preview;
mod settings;
mod styles;

pub mod layout_cache;
pub use layout_cache::LayoutCache;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::pane::PaneId;
use crate::state::{AppState, PaneLayout};
use crate::ui::editor::{editor_render_state, render_editor};
use crate::ui::menu_bar::render_menu_bar;
use crate::ui::overlay::{
    render_collision_dialog, render_dialog, render_menu_popup, render_prompt,
};
use crate::ui::palette::render_command_palette;
use crate::ui::pane::render_pane;
use crate::ui::preview::render_preview_panel;
use crate::ui::settings::render_settings_panel;

use ratatui::widgets::Borders;

/// Render the full TUI. Returns a `LayoutCache` recording each panel's `Rect`
/// so the event loop can use it for mouse hit-testing without re-running layout.
pub fn render(frame: &mut Frame<'_>, state: &mut AppState) -> LayoutCache {
    let palette = state.theme().palette;
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_menu_bar(frame, areas[0], state, palette);

    let pane_direction = match state.pane_layout() {
        PaneLayout::SideBySide => Direction::Horizontal,
        PaneLayout::Stacked => Direction::Vertical,
    };

    let is_preview_open = state.is_preview_panel_open();
    let has_editor = state.editor().is_some();
    let show_tools = has_editor || is_preview_open;

    let tools_pct = if has_editor { 50u16 } else { 40u16 };
    let panes_pct = 100 - tools_pct;

    let (pane_area, tools_area_opt) = if show_tools {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(panes_pct),
                Constraint::Percentage(tools_pct),
            ])
            .split(areas[1]);
        (vertical[0], Some(vertical[1]))
    } else {
        (areas[1], None)
    };

    let panes = Layout::default()
        .direction(pane_direction)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(pane_area);

    let left_focused = state.focus() == PaneId::Left;
    let right_focused = state.focus() == PaneId::Right;

    let is_stacked = state.pane_layout() == PaneLayout::Stacked;
    let (first_label, second_label) = if is_stacked {
        ("Top", "Bottom")
    } else {
        ("Left", "Right")
    };

    render_pane(
        frame,
        panes[0],
        state.left_pane(),
        first_label,
        left_focused,
        Borders::TOP | Borders::LEFT | Borders::BOTTOM,
        state,
    );

    render_pane(
        frame,
        panes[1],
        state.right_pane(),
        second_label,
        right_focused,
        Borders::ALL,
        state,
    );

    if let Some(tools_area) = tools_area_opt {
        if has_editor {
            let syntect_theme = state.theme().palette.syntect_theme;
            if let Some(editor) = state.editor_mut() {
                let editor_view = editor_render_state(editor, tools_area, true);
                render_editor(
                    frame,
                    tools_area,
                    editor,
                    &editor_view,
                    true,
                    palette,
                    syntect_theme,
                );
            }
        } else if is_preview_open {
            let preview_view = state.preview_view().map(|(_, view)| view);
            let filename = state.active_pane_title().to_string();
            render_preview_panel(
                frame,
                tools_area,
                preview_view,
                &filename,
                state.is_preview_focused(),
                palette,
            );
        }
    }

    if let Some(menu) = state.active_menu() {
        render_menu_popup(
            frame,
            areas[1],
            menu,
            &state.menu_items(),
            state.menu_selection(),
            palette,
        );
    }

    if let Some(prompt) = state.prompt() {
        render_prompt(frame, areas[1], prompt, palette);
    }

    if let Some(dialog) = state.dialog() {
        render_dialog(frame, areas[1], dialog, palette);
    }

    if let Some(collision) = state.collision() {
        render_collision_dialog(frame, areas[1], collision, palette);
    }

    if let Some(palette_state) = state.palette() {
        render_command_palette(frame, areas[1], palette_state, palette);
    }

    if let Some(settings_state) = state.settings() {
        render_settings_panel(frame, areas[1], settings_state, state, palette);
    }

    let status = Paragraph::new(Line::raw(state.status_line())).style(
        Style::default()
            .fg(palette.status_fg)
            .bg(palette.status_bg)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(status, areas[2]);

    LayoutCache {
        menu_bar: areas[0],
        left_pane: panes[0],
        right_pane: panes[1],
        tools_panel: tools_area_opt,
        status_bar: areas[2],
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{IconMode, ThemePalette};
    use crate::editor::EditorBuffer;
    use crate::fs::EntryKind;
    use crate::icon::icon_for_kind;
    use crate::palette::all_entries;
    use crate::preview::ViewBuffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Modifier};

    use super::layout_cache::rect_contains;
    use super::menu_bar::top_bar_logo_spans;
    use super::pane::{format_icon_slot, pane_chrome_style};
    use super::styles::{
        command_palette_entry_hint_style, command_palette_entry_label_style,
        command_palette_header_style, elevated_surface_style, overlay_title_style,
    };
    use super::editor::editor_render_state;

    fn test_palette() -> ThemePalette {
        ThemePalette {
            menu_bg: Color::Rgb(10, 11, 12),
            menu_fg: Color::Rgb(20, 21, 22),
            menu_active_bg: Color::Rgb(30, 31, 32),
            menu_mnemonic_fg: Color::Rgb(40, 41, 42),
            border_focus: Color::Rgb(50, 51, 52),
            border_editor_focus: Color::Rgb(60, 61, 62),
            selection_bg: Color::Rgb(70, 71, 72),
            selection_fg: Color::Rgb(80, 81, 82),
            surface_bg: Color::Rgb(90, 91, 92),
            tools_bg: Color::Rgb(100, 101, 102),
            prompt_bg: Color::Rgb(110, 111, 112),
            prompt_border: Color::Rgb(120, 121, 122),
            text_primary: Color::Rgb(130, 131, 132),
            text_muted: Color::Rgb(140, 141, 142),
            directory_fg: Color::Rgb(150, 151, 152),
            symlink_fg: Color::Rgb(160, 161, 162),
            file_fg: Color::Rgb(170, 171, 172),
            status_bg: Color::Rgb(180, 181, 182),
            status_fg: Color::Rgb(190, 191, 192),
            logo_accent: Color::Rgb(200, 201, 202),
            key_hint_fg: Color::Rgb(210, 211, 212),
            syntect_theme: "test",
        }
    }

    #[test]
    fn unicode_icons_use_glyphs() {
        assert_eq!(icon_for_kind(EntryKind::Directory, IconMode::Unicode), "▣");
        assert_eq!(icon_for_kind(EntryKind::File, IconMode::Unicode), "•");
        assert_eq!(icon_for_kind(EntryKind::Symlink, IconMode::Unicode), "↗");
        assert_eq!(icon_for_kind(EntryKind::Other, IconMode::Unicode), "◦");
    }

    #[test]
    fn ascii_icons_use_labels() {
        assert_eq!(icon_for_kind(EntryKind::Directory, IconMode::Ascii), "[D]");
        assert_eq!(icon_for_kind(EntryKind::File, IconMode::Ascii), "[F]");
        assert_eq!(icon_for_kind(EntryKind::Symlink, IconMode::Ascii), "[L]");
        assert_eq!(icon_for_kind(EntryKind::Other, IconMode::Ascii), "[?]");
    }

    #[test]
    fn elevated_surface_uses_tools_bg() {
        let p = test_palette();
        let s = elevated_surface_style(p);
        assert_eq!(s.bg, Some(p.tools_bg));
    }

    #[test]
    fn overlay_title_is_bold_and_mnemonic_fg() {
        let p = test_palette();
        let s = overlay_title_style(p);
        assert_eq!(s.fg, Some(p.menu_mnemonic_fg));
        assert!(s.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn pane_chrome_focused_uses_border_focus_color() {
        let p = test_palette();
        let chrome = pane_chrome_style(true, p);
        assert_eq!(chrome.border.fg, Some(p.border_focus));
    }

    #[test]
    fn top_bar_logo_has_five_spans() {
        let p = test_palette();
        let spans = top_bar_logo_spans(true, p);
        assert_eq!(spans.len(), 5);
        assert_eq!(spans[2].content, "Z");
    }

    #[test]
    fn command_palette_selected_entry_uses_selection_bg() {
        let p = test_palette();
        let s = command_palette_entry_label_style(true, p);
        assert_eq!(s.bg, Some(p.selection_bg));
        assert!(s.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn command_palette_unselected_entry_uses_text_primary() {
        let p = test_palette();
        let s = command_palette_entry_label_style(false, p);
        assert_eq!(s.fg, Some(p.text_primary));
    }

    #[test]
    fn command_palette_header_is_muted_and_bold() {
        let p = test_palette();
        let s = command_palette_header_style(p);
        assert_eq!(s.fg, Some(p.text_muted));
        assert!(s.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn command_palette_hint_uses_key_hint_fg() {
        let p = test_palette();
        let s = command_palette_entry_hint_style(p);
        assert_eq!(s.fg, Some(p.key_hint_fg));
    }

    #[test]
    fn icon_slot_unicode_appends_two_spaces() {
        let slot = format_icon_slot("▣", IconMode::Unicode);
        assert_eq!(slot, "▣  ");
    }

    #[test]
    fn icon_slot_ascii_returns_icon_only() {
        let slot = format_icon_slot("[D]", IconMode::Ascii);
        assert_eq!(slot, "[D]");
    }

    #[test]
    fn all_palette_entries_have_non_empty_labels() {
        for entry in all_entries() {
            assert!(!entry.label.is_empty(), "entry label is empty: {:?}", entry);
        }
    }

    #[test]
    fn editor_render_state_tracks_viewport() {
        let mut editor = EditorBuffer::default();
        let area = Rect { x: 0, y: 0, width: 80, height: 24 };
        let rs = editor_render_state(&mut editor, area, true);
        assert_eq!(rs.visible_start, 0);
    }

    #[test]
    fn view_buffer_visible_window_returns_correct_slice() {
        let vb = ViewBuffer::from_plain("line1\nline2\nline3");
        let (first, window) = vb.visible_window(2);
        assert_eq!(first, 0);
        assert_eq!(window.len(), 2);
    }
}
```

- [ ] **Step 10.2: Delete `src/ui.rs`**

```bash
git rm src/ui.rs
```

- [ ] **Step 10.3: Update `src/app.rs` — store layout_cache on `App`**

In `src/app.rs`, add the import and field. Replace the existing `App` struct, `bootstrap`, and `run` with:

```rust
use crate::ui::LayoutCache;

pub struct App {
    job_requests: Sender<JobRequest>,
    job_results: Receiver<JobResult>,
    keymap: RuntimeKeymap,
    state: AppState,
    layout_cache: LayoutCache,
}

// In bootstrap(), initialize with:
//   layout_cache: LayoutCache::default(),
// (add this field to the struct literal in bootstrap())

// In run(), change the draw call to capture the returned cache:
pub fn run(&mut self) -> Result<()> {
    let mut terminal = TerminalSession::enter()?;

    while !self.state.should_quit() {
        if self.state.needs_redraw() {
            let mut cache = LayoutCache::default();
            terminal.draw(|frame| {
                cache = ui::render(frame, &mut self.state);
            })?;
            self.layout_cache = cache;
            self.state.mark_drawn();
        }

        self.process_next_event()?;
    }

    Ok(())
}
```

Full updated `App` struct (only App, bootstrap, and run change — rest of app.rs is unchanged):

```rust
use crate::ui::LayoutCache;

pub struct App {
    job_requests: Sender<JobRequest>,
    job_results: Receiver<JobResult>,
    keymap: RuntimeKeymap,
    state: AppState,
    pub layout_cache: LayoutCache,
}

impl App {
    pub fn bootstrap() -> Result<Self> {
        let started_at = Instant::now();
        let loaded_config =
            AppConfig::load_default_location().context("failed to resolve application config")?;
        let keymap = loaded_config
            .config
            .compile_keymap()
            .context("failed to compile configured key bindings")?;
        let (job_requests, job_results) = jobs::spawn_scan_worker();
        let state = AppState::bootstrap(loaded_config, started_at)
            .context("failed to bootstrap application state")?;
        let mut app = Self {
            job_requests,
            job_results,
            keymap,
            state,
            layout_cache: LayoutCache::default(),
        };

        for command in app.state.initial_commands() {
            app.execute_command(command)?;
        }

        Ok(app)
    }

    pub fn run(&mut self) -> Result<()> {
        let mut terminal = TerminalSession::enter()?;

        while !self.state.should_quit() {
            if self.state.needs_redraw() {
                let mut cache = LayoutCache::default();
                terminal.draw(|frame| {
                    cache = ui::render(frame, &mut self.state);
                })?;
                self.layout_cache = cache;
                self.state.mark_drawn();
            }

            self.process_next_event()?;
        }

        Ok(())
    }

    // ... rest of methods unchanged ...
}
```

- [ ] **Step 10.4: Run full test suite**

```bash
cargo test
```

Expected: all tests pass (same count as baseline established in Task 1, plus 3 new layout_cache tests).

- [ ] **Step 10.5: Commit**

```bash
git add src/ui/ src/app.rs
git commit -m "refactor(ui): split ui.rs into focused modules under src/ui/"
```

---

## Task 11: Final verification

**Files:** None modified

- [ ] **Step 11.1: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: zero warnings (fix any that appear before proceeding).

- [ ] **Step 11.2: Run tests with output**

```bash
cargo test -- --nocapture 2>&1 | tail -5
```

Expected: `test result: ok. N passed; 0 failed`.

- [ ] **Step 11.3: Verify module structure**

```bash
ls src/ui/
```

Expected output includes: `code_view.rs  editor.rs  layout_cache.rs  menu_bar.rs  mod.rs  overlay.rs  palette.rs  pane.rs  preview.rs  settings.rs  styles.rs`

- [ ] **Step 11.4: Final commit**

```bash
git add -p  # verify nothing unexpected staged
git commit -m "chore(ui): Wave 1B complete — ui.rs split into modules with LayoutCache"
```
