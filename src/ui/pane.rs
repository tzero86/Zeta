use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::config::{IconMode, ThemePalette};
use crate::fs::{EntryInfo, EntryKind};
use crate::git::{FileStatus, RepoStatus};
use crate::icon::icon_for_kind;
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
    let cwd_display = if pane.in_remote() {
        pane.remote_address().unwrap_or("unknown")
    } else {
        &pane.cwd.display().to_string()
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
        .highlight_symbol("> ");

    let mut list_state = ListState::default();
    if !pane.entries.is_empty() {
        list_state.select(pane.visible_selection(visible_height));
    }

    frame.render_stateful_widget(list.style(chrome.surface), list_area, &mut list_state);

    if let Some(filter_area) = filter_area {
        let filter = Paragraph::new(format!(" Filter: {}_", pane.filter_query)).style(
            Style::default()
                .fg(palette.text_primary)
                .bg(palette.selection_bg),
        );
        frame.render_widget(filter, filter_area);
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
    } = args;
    let icon = icon_for_kind(entry.kind, icon_mode);
    // --- Details view: flat columns (mark | icon | git | name | size | date) ---
    if details_view {
        let row_styles = pane_row_styles(is_focused, is_marked, entry.kind, palette);
        let icon_slot = format_icon_slot(icon, icon_mode);
        let icon_slot_width = display_width(&icon_slot);
        let mark_prefix = if is_marked { "* " } else { "  " };
        let (git_char, git_colour) = match git_status {
            Some(s) => (s.symbol(), s.colour()),
            None => (' ', palette.text_muted),
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
        let size_str = match (entry.kind, entry.size_bytes) {
            (EntryKind::Directory, Some(b)) => format!("{:>9}", human_size(b)),
            (EntryKind::Directory, None) => format!("{:>9}", "dir"),
            (EntryKind::Symlink, _) => format!("{:>9}", "link"),
            (EntryKind::Archive, _) => format!("{:>9}", "archive"),
            (EntryKind::Other, _) => format!("{:>9}", "other"),
            (EntryKind::File, Some(b)) => format!("{:>9}", human_size(b)),
            (EntryKind::File, None) => format!("{:>9}", "file"),
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
            Span::styled(mark_prefix.to_string(), row_styles.mark),
            Span::styled(format!("{icon_slot} "), row_styles.icon),
            Span::styled(git_char.to_string(), Style::default().fg(git_colour)),
            Span::raw(" "),
            Span::styled(name, name_style),
            Span::raw(" ".repeat(spacer_width)),
            Span::styled(size_str, row_styles.meta),
            Span::raw(" "),
            Span::styled(date_str, row_styles.meta),
        ]));
    }
    let row_styles = pane_row_styles(is_focused, is_marked, entry.kind, palette);
    let guide = if is_last { "  " } else { "│ " };
    let branch = if is_last { "└" } else { "├" };
    // icon already bound above
    let mark_prefix = if is_marked { "* " } else { "  " };
    let name = display_name.unwrap_or_else(|| match entry.kind {
        EntryKind::Directory => format!("{}/", entry.name),
        _ => entry.name.clone(),
    });
    let meta = format_entry_meta(entry);
    let icon_slot = format_icon_slot(icon, icon_mode);
    let prefix = format!("{}{}{} {} ", guide, branch, mark_prefix, icon_slot);
    let prefix_width = display_width(&prefix) + 2; // +2 for git indicator + space
                                                   // Git status indicator — always 1 char wide so column alignment is stable.
    let (git_char, git_colour) = match git_status {
        Some(s) => (s.symbol(), s.colour()),
        None => (' ', palette.text_muted),
    };
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
        Span::styled(git_char.to_string(), Style::default().fg(git_colour)),
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
        IconMode::Unicode | IconMode::Custom => format!("{icon}  "),
        IconMode::Ascii => icon.to_string(),
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

pub fn format_entry_meta(entry: &EntryInfo) -> String {
    match entry.kind {
        EntryKind::Directory => match entry.size_bytes {
            Some(size) => format!("dir {}", human_size(size)),
            None => String::from("dir"),
        },
        EntryKind::Symlink => String::from("link"),
        EntryKind::Archive => String::from("archive"),
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
