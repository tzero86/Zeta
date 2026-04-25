use std::borrow::Cow;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::config::{IconMode, ThemePalette};
use crate::fs::{EntryInfo, EntryKind};
use crate::git::{FileStatus, RepoStatus};
use crate::icon::icon_for_entry;
use crate::pane::PaneState;
use crate::state::AppState;

pub struct PaneChrome {
    pub border: Style,
    pub title: Style,
    pub surface: Style,
}

pub struct RenderPaneArgs<'a> {
    pub pane: &'a PaneState,
    pub label: &'a str,
    pub is_focused: bool,
    pub is_left: bool,
    pub borders: Borders,
    pub state: &'a AppState,
    pub git: Option<&'a RepoStatus>,
}

struct RenderItemArgs<'a> {
    entry: &'a EntryInfo,
    is_focused: bool,
    is_marked: bool,
    is_last: bool,
    available_width: usize,
    palette: ThemePalette,
    icon_mode: IconMode,
    git_status: Option<FileStatus>,
    diff_colour: Option<ratatui::style::Color>,
    details_view: bool,
    /// Optional display-name override (used by inline rename on the selected row).
    display_name: Option<String>,
    is_filtered_out: bool,
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

/// Produce a compact display of `path` for use in the pane title.
///
/// * Replaces the user's home directory prefix with `~`.
/// * If the result is still longer than `max_chars`, abbreviates to `…/parent/leaf`.
fn path_breadcrumb(path: &std::path::Path, max_chars: usize) -> String {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from);

    let display = if let Some(ref home) = home {
        if let Ok(rel) = path.strip_prefix(home) {
            let rel_str = rel.display().to_string();
            if rel_str.is_empty() {
                String::from("~")
            } else {
                format!("~/{rel_str}")
            }
        } else {
            path.display().to_string()
        }
    } else {
        path.display().to_string()
    };

    if display.chars().count() <= max_chars {
        return display;
    }

    let leaf = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    if parent.is_empty() {
        format!("…/{leaf}")
    } else {
        format!("…/{parent}/{leaf}")
    }
}

pub fn render_pane(frame: &mut Frame<'_>, area: Rect, args: RenderPaneArgs<'_>) {
    let RenderPaneArgs {
        pane,
        label,
        is_focused,
        is_left,
        borders,
        state,
        git,
    } = args;
    let palette = state.theme().palette;
    let icon_mode = state.icon_mode();
    let chrome = pane_chrome_style(is_focused, palette);

    let branch = git.map(|g| format!("  ⎇ {}", g.branch)).unwrap_or_default();
    let diff_legend = if state.diff_mode {
        format!("  | {}", crate::diff::diff_summary(&state.diff_map))
    } else {
        String::new()
    };
    let cwd_breadcrumb: String;
    let cwd_display: &str = if pane.in_remote() {
        pane.remote_address().unwrap_or("unknown")
    } else {
        let max_chars = (area.width as usize).saturating_sub(40).max(10);
        cwd_breadcrumb = path_breadcrumb(&pane.cwd, max_chars);
        &cwd_breadcrumb
    };
    let title = format!(
        "{} [{}]  {}{}{}  ({})",
        label,
        pane.entries.len(),
        cwd_display,
        branch,
        diff_legend,
        pane.sort_mode.label()
    );
    let block = Block::default()
        .title(Span::styled(title, chrome.title))
        .borders(borders)
        .border_style(chrome.border)
        .style(chrome.surface);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (list_area, filter_area) = if pane.filter_active {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        (chunks[0], Some(chunks[1]))
    } else {
        (inner, None)
    };

    let (header_area, list_area) = if pane.details_view && list_area.height > 2 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(list_area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, list_area)
    };

    let visible_height = list_area.height as usize;
    let visible_entries = pane.visible_entries(visible_height);
    let selected_visible = pane.visible_selection(visible_height);
    let items: Vec<ListItem<'_>> = if pane.entries.is_empty() {
        vec![ListItem::new("(empty)")]
    } else {
        visible_entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let diff_colour = if state.diff_mode {
                    state.diff_map.get(&entry.name).map(|s| s.colour(is_left))
                } else {
                    None
                };
                let display_name = if selected_visible == Some(index) {
                    pane.rename_state
                        .as_ref()
                        .map(|rs| format!("{}│", rs.buffer))
                } else {
                    None
                };
                let is_filtered_out = pane.filter_active
                    && !pane.filter_query.is_empty()
                    && !crate::utils::glob_match::matches(&pane.filter_query, &entry.name);
                render_item(RenderItemArgs {
                    entry,
                    is_focused,
                    is_marked: pane.is_marked(&entry.path),
                    is_last: index + 1 == visible_entries.len(),
                    available_width: list_area.width as usize,
                    palette,
                    icon_mode,
                    git_status: git.and_then(|g| g.status_for(&entry.path)),
                    diff_colour,
                    details_view: pane.details_view,
                    display_name,
                    is_filtered_out,
                })
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
        .highlight_symbol("›");

    let mut list_state = ListState::default();
    if !pane.entries.is_empty() {
        list_state.select(pane.visible_selection(visible_height));
    }

    frame.render_stateful_widget(list.style(chrome.surface), list_area, &mut list_state);

    if let Some(header_area) = header_area {
        render_column_headers(frame, header_area, palette);
    }

    if let Some(filter_area) = filter_area {
        use crate::ui::styles::pane_filter_strip_style;
        let match_count = pane.filtered_count();
        let query_display = format!(" ⌕  {}│", pane.filter_query);
        let count_display = format!(" {} matches  Esc clear", match_count);
        let query_width = query_display.chars().count();
        let count_width = count_display.chars().count();
        let pad = (filter_area.width as usize).saturating_sub(query_width + count_width);
        let line = Line::from(vec![
            Span::styled(
                query_display,
                pane_filter_strip_style(palette).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ".repeat(pad), pane_filter_strip_style(palette)),
            Span::styled(
                count_display,
                Style::default()
                    .fg(palette.accent_green)
                    .bg(palette.pane_filter_bg),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), filter_area);
    }
}

/// Convert days since Unix epoch to (year, month, day) using the Proleptic Gregorian calendar.
fn days_to_ymd(days: u64) -> (u32, u32, u32) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m as u32, d as u32)
}

/// Format a `SystemTime` as `"YYYY-MM-DD HH:MM"` (exactly 16 chars).
fn format_timestamp(time: std::time::SystemTime) -> String {
    let Ok(dur) = time.duration_since(std::time::UNIX_EPOCH) else {
        return String::from("                "); // 16 spaces
    };
    let secs = dur.as_secs();
    let mins = (secs / 60) % 60;
    let hours = (secs / 3_600) % 24;
    let days = secs / 86_400;
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02} {hours:02}:{mins:02}")
}

fn render_item(args: RenderItemArgs<'_>) -> ListItem<'static> {
    let RenderItemArgs {
        entry,
        is_focused,
        is_marked,
        is_last,
        available_width,
        palette,
        icon_mode,
        git_status,
        diff_colour,
        details_view,
        display_name,
        is_filtered_out,
    } = args;
    let icon = icon_for_entry(
        entry.kind,
        entry.path.extension().and_then(|e| e.to_str()),
        icon_mode,
    );
    // Return dimmed text if this entry is filtered out
    if is_filtered_out {
        let name = entry.name.clone();
        return ListItem::new(Line::from(Span::styled(
            format!("  {} {}", icon, name),
            Style::default().fg(palette.text_muted),
        )));
    }
    // --- Details view: flat columns (mark | icon | git | name | size | date) ---
    if details_view {
        let row_styles = pane_row_styles(is_focused, is_marked, entry.kind, palette);
        let icon_slot = format_icon_slot(icon, icon_mode);
        let icon_slot_width = icon_slot_width(icon, icon_mode);
        let mark_prefix = if is_marked { "* " } else { "  " };
        let (git_char, git_colour) = match git_status {
            Some(s) => (s.symbol(), s.colour()),
            None => (" ", palette.text_muted),
        };
        // Fixed right: 9 (size) + 1 (gap) + 16 (date) = 26
        let right_fixed = 26usize;
        // Fixed left: 2 (mark) + icon_slot_width + 1 (space) + 1 (git) + 1 (space)
        let left_fixed = 2 + icon_slot_width + 3;
        let name_width = available_width
            .saturating_sub(left_fixed + right_fixed + 1)
            .max(1);
        let name = display_name.unwrap_or_else(|| match entry.kind {
            EntryKind::Directory => format!("{}/", entry.name),
            _ => entry.name.clone(),
        });
        let name = truncate_text(&name, name_width);
        let spacer_width = available_width
            .saturating_sub(left_fixed + display_width(&name) + right_fixed)
            .max(0);
        let size_str: Cow<'static, str> = match (entry.kind, entry.size_bytes) {
            (EntryKind::Directory, Some(b)) => Cow::Owned(format!("{:>9}", human_size(b))),
            (EntryKind::Directory, None) => Cow::Borrowed("      dir"),
            (EntryKind::Symlink, _) => Cow::Borrowed("     link"),
            (EntryKind::Archive, _) => Cow::Borrowed("  archive"),
            (EntryKind::Other, _) => Cow::Borrowed("    other"),
            (EntryKind::File, Some(b)) => Cow::Owned(format!("{:>9}", human_size(b))),
            (EntryKind::File, None) => Cow::Borrowed("     file"),
        };
        let date_str = entry
            .modified
            .map(format_timestamp)
            .unwrap_or_else(|| String::from("                "));
        let name_style = if let Some(dc) = diff_colour {
            row_styles.name.fg(dc)
        } else {
            row_styles.name
        };
        return ListItem::new(Line::from(vec![
            Span::styled(mark_prefix, row_styles.mark),
            Span::styled(format!("{icon_slot} "), row_styles.icon),
            Span::styled(git_char, Style::default().fg(git_colour)),
            Span::raw(" "),
            Span::styled(name, name_style),
            Span::raw(" ".repeat(spacer_width)),
            Span::styled(size_str, row_styles.meta),
            Span::raw(" "),
            Span::styled(date_str, row_styles.meta),
        ]));
    }
    let row_styles = pane_row_styles(is_focused, is_marked, entry.kind, palette);
    // NerdFont's PUA glyphs cause some terminals (Warp + FiraCode NF, etc.) to
    // pick a font fallback for the whole row, which can blank out the standard
    // box-drawing tree connectors (U+2514, U+251C, U+2502). Use the heavy
    // variants in NerdFont mode — they're rendered consistently because the
    // NerdFont itself supplies them. Other modes keep the lighter line.
    let (guide, branch_span): (&'static str, &'static str) = match icon_mode {
        IconMode::NerdFont => {
            let g = if is_last { "  " } else { "\u{2503} " };
            let b = if is_last { "\u{2517} " } else { "\u{2523} " };
            (g, b)
        }
        _ => {
            let g = if is_last { "  " } else { "\u{2502} " };
            let b = if is_last { "\u{2514} " } else { "\u{251c} " };
            (g, b)
        }
    };
    let _branch = branch_span;
    let mark_prefix = if is_marked { "* " } else { "  " };
    let name = display_name.unwrap_or_else(|| match entry.kind {
        EntryKind::Directory => format!("{}/", entry.name),
        EntryKind::Symlink => {
            if let Some(ref target) = entry.link_target {
                let target_str = target.to_string_lossy();
                format!("{} → {}", entry.name, target_str)
            } else {
                entry.name.clone()
            }
        }
        _ => entry.name.clone(),
    });
    let meta = format_entry_meta(entry);
    let icon_slot = format_icon_slot(icon, icon_mode);
    let icon_slot_w = icon_slot_width(icon, icon_mode);
    let prefix_display_width = display_width(guide)
        + display_width(branch_span)
        + display_width(mark_prefix)
        + icon_slot_w
        + 1; // space after icon
    let prefix_width = prefix_display_width + 2; // +2 for git indicator + space
                                                 // Git status indicator — always 1 char wide so column alignment is stable.
    let (git_char, git_colour) = match git_status {
        Some(s) => (s.symbol(), s.colour()),
        None => (" ", palette.text_muted),
    };
    let meta_width = display_width(&meta);
    let content_width = available_width;
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
        Span::styled(branch_span, row_styles.branch),
        Span::styled(mark_prefix, row_styles.mark),
        Span::styled(format!("{} ", icon_slot), row_styles.icon),
        Span::styled(git_char, Style::default().fg(git_colour)),
        Span::raw(" "),
        Span::styled(
            name,
            if let Some(dc) = diff_colour {
                row_styles.name.fg(dc)
            } else {
                row_styles.name
            },
        ),
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
        EntryKind::Archive => Style::default().fg(palette.file_fg),
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
                EntryKind::Archive => palette.file_fg,
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
        // Unicode glyphs are single-width per unicode-width; add 2 spaces for a
        // consistent 3-column slot.
        IconMode::Unicode => format!("{icon}  "),
        // Ascii icons like "[D]" / "[F]" are already multi-char and consume
        // correct layout width on their own — no extra padding needed.
        IconMode::Ascii => icon.to_string(),
        // NerdFont PUA glyphs (U+E000–U+F8FF) render as double-wide (2 terminal
        // columns) in fonts configured with NerdFont, but unicode-width reports
        // them as ambiguous (width 1). Reserve the extra column explicitly by
        // only adding 1 trailing space so the total terminal width = 2 + 1 = 3.
        IconMode::NerdFont => format!("{icon} "),
    }
}

/// Returns the logical column width that `format_icon_slot` occupies in the terminal.
/// For NerdFont mode, PUA glyphs are treated as double-wide (2 cols) + 1 space = 3.
/// For other modes, delegates to `display_width`.
pub fn icon_slot_width(icon: &str, icon_mode: IconMode) -> usize {
    match icon_mode {
        IconMode::NerdFont => 3, // 2 (double-wide glyph) + 1 (space gap)
        _ => display_width(&format_icon_slot(icon, icon_mode)),
    }
}

pub fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

pub fn truncate_text(value: &str, max_width: usize) -> String {
    let width = display_width(value);
    if width <= max_width {
        return value.to_string();
    }
    if max_width <= 2 {
        return value.chars().take(max_width).collect();
    }

    let mut truncated = String::new();
    let mut current_width = 0usize;
    let target_width = max_width - 2;
    for ch in value.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1).max(1);
        if current_width + ch_width > target_width {
            break;
        }
        truncated.push(ch);
        current_width += ch_width;
    }

    format!("{}..", truncated)
}

pub fn format_entry_meta(entry: &EntryInfo) -> Cow<'static, str> {
    match entry.kind {
        EntryKind::Directory => match entry.size_bytes {
            Some(size) => Cow::Owned(format!("dir {}", human_size(size))),
            None => Cow::Borrowed("dir"),
        },
        EntryKind::Symlink => Cow::Borrowed("link"),
        EntryKind::Archive => Cow::Borrowed("archive"),
        EntryKind::Other => Cow::Borrowed("other"),
        EntryKind::File => {
            let ext = entry
                .path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase());
            let kind: &'static str = match ext.as_deref() {
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
                Some(size) => Cow::Owned(format!("{} {}", kind, human_size(size))),
                None => Cow::Borrowed(kind),
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

fn render_column_headers(frame: &mut Frame<'_>, area: Rect, palette: ThemePalette) {
    use crate::ui::styles::pane_column_header_style;
    let w = area.width as usize;
    let right_fixed = 30usize; // 9 (size) + 1 + 16 (date) + 1 + 3 (git)
    let left_fixed = 5usize; // 2 (mark) + 2 (icon+space) + 1
    let name_width = w.saturating_sub(left_fixed + right_fixed).max(4);
    let header = format!(
        "  {icon:<2}{name:<name_width$}{size:>9} {date:<16} {git:<3}",
        icon = "",
        name = "Name",
        name_width = name_width,
        size = "Size",
        date = "Modified",
        git = "Git",
    );
    let para = Paragraph::new(header).style(pane_column_header_style(palette));
    frame.render_widget(para, area);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use ratatui::style::Color;

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
            search_match_bg: Color::Rgb(80, 64, 0),
            search_match_active_bg: Color::Rgb(185, 140, 10),
            text_sel_bg: Color::Rgb(35, 85, 145),
            text_subtext: Color::Rgb(220, 221, 222),
            accent_mauve: Color::Rgb(230, 231, 232),
            accent_teal: Color::Rgb(240, 241, 242),
            accent_green: Color::Rgb(250, 251, 252),
            accent_yellow: Color::Rgb(10, 20, 30),
            accent_peach: Color::Rgb(40, 50, 60),
            accent_red: Color::Rgb(70, 80, 90),
            modal_halo: Color::Rgb(100, 110, 120),
            pane_filter_bg: Color::Rgb(130, 140, 150),
            pane_filter_border: Color::Rgb(160, 170, 180),
            status_git_bg: Color::Rgb(190, 200, 210),
            status_entry_bg: Color::Rgb(220, 230, 240),
            status_workspace_bg: Color::Rgb(250, 10, 20),
        }
    }

    #[test]
    fn nerdfont_tree_row_includes_branch_connectors() {
        for (icon_mode, label) in [
            (IconMode::Ascii, "ascii"),
            (IconMode::Unicode, "unicode"),
            (IconMode::NerdFont, "nerdfont"),
        ] {
            let item = render_item(RenderItemArgs {
                entry: &EntryInfo {
                    name: String::from("note.txt"),
                    path: PathBuf::from("./note.txt"),
                    kind: EntryKind::File,
                    size_bytes: Some(1024),
                    modified: None,
                    link_target: None,
                },
                is_focused: true,
                is_marked: false,
                is_last: false,
                available_width: 60,
                palette: test_palette(),
                icon_mode,
                git_status: None,
                diff_colour: None,
                details_view: false,
                display_name: None,
                is_filtered_out: false,
            });
            // Render to a buffer and assert connector chars survive.
            let area = Rect::new(0, 0, 60, 1);
            let mut buf = ratatui::buffer::Buffer::empty(area);
            ratatui::widgets::Widget::render(
                ratatui::widgets::List::new(vec![item]),
                area,
                &mut buf,
            );
            let row: String = (0..area.width)
                .map(|x| {
                    buf.cell((x, 0))
                        .map(|c| c.symbol().to_string())
                        .unwrap_or_default()
                })
                .collect();
            let expected_branch = if matches!(icon_mode, IconMode::NerdFont) {
                '\u{2523}' // ┣
            } else {
                '\u{251c}' // ├
            };
            assert!(
                row.contains(expected_branch),
                "{label} mode tree row missing branch connector. row was: {row:?}"
            );
        }
    }

    #[test]
    fn normal_pane_row_uses_full_available_width() {
        let item = render_item(RenderItemArgs {
            entry: &EntryInfo {
                name: String::from("note.txt"),
                path: PathBuf::from("./note.txt"),
                kind: EntryKind::File,
                size_bytes: Some(1024),
                modified: None,
                link_target: None,
            },
            is_focused: true,
            is_marked: false,
            is_last: false,
            available_width: 40,
            palette: test_palette(),
            icon_mode: IconMode::Unicode,
            git_status: None,
            diff_colour: None,
            details_view: false,
            display_name: None,
            is_filtered_out: false,
        });

        assert_eq!(item.width(), 40);
    }
}
