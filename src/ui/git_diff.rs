use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    config::ThemePalette,
    git::{DiffLine, DiffLineKind},
    state::AppState,
};

/// Top-level renderer: splits area 38/62, renders file list left, diff content right.
pub fn render_git_diff_view(f: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    render_diff_file_list(f, chunks[0], state);

    // Update viewport height before rendering content
    let inner_height = chunks[1].height.saturating_sub(2) as usize;
    state.git_diff_viewport_height = inner_height;

    render_diff_content(f, chunks[1], state);
}

/// Left pane: scrollable list of changed files with status indicator and add/remove counts.
pub fn render_diff_file_list(f: &mut Frame, area: Rect, state: &AppState) {
    let palette = state.theme().palette;
    let focused = !state.git_diff_focus_content;

    let border_style = if focused {
        Style::default()
            .fg(palette.border_focus)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_muted)
    };

    let items: Vec<ListItem> = state
        .git_diff_files
        .iter()
        .map(|f| {
            let label = format!(
                "{} {} +{} -{}",
                f.status.symbol(),
                f.path.display(),
                f.added,
                f.removed
            );
            ListItem::new(label)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.git_diff_selected));

    let block = Block::default()
        .title(" Changed Files ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(palette.selection_bg)
            .add_modifier(Modifier::BOLD),
    );

    f.render_stateful_widget(list, area, &mut list_state);
}

/// Right pane: scrollable unified diff with colour-coded lines and line-number gutter.
pub fn render_diff_content(f: &mut Frame, area: Rect, state: &AppState) {
    let palette = state.theme().palette;
    let focused = state.git_diff_focus_content;

    let border_style = if focused {
        Style::default()
            .fg(palette.border_focus)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_muted)
    };

    let inner_height = area.height.saturating_sub(2) as usize;
    let max_scroll = state.git_diff_lines.len().saturating_sub(inner_height);
    let effective_scroll = state.git_diff_scroll.min(max_scroll);

    // Pass 1: walk the FULL git_diff_lines list, tracking line counters
    let mut old_line: u32 = 0;
    let mut new_line: u32 = 0;

    let numbered: Vec<(Option<u32>, Option<u32>, &DiffLine)> = state
        .git_diff_lines
        .iter()
        .map(|dl| {
            use DiffLineKind::*;
            match dl.kind {
                HunkHeader => {
                    // Parse "@@ -OLD_START,... +NEW_START,... @@" to reset counters
                    if let Some((old_start, new_start)) = parse_hunk_header(&dl.content) {
                        old_line = old_start.saturating_sub(1);
                        new_line = new_start.saturating_sub(1);
                    }
                    (None, None, dl)
                }
                FileHeader => (None, None, dl),
                Added => {
                    new_line += 1;
                    (None, Some(new_line), dl)
                }
                Removed => {
                    old_line += 1;
                    (Some(old_line), None, dl)
                }
                Context => {
                    old_line += 1;
                    new_line += 1;
                    (Some(old_line), Some(new_line), dl)
                }
            }
        })
        .collect();

    // Compute the minimum digit width needed to display the largest line number.
    let max_line_num = numbered
        .iter()
        .flat_map(|(old, new, _)| old.iter().chain(new.iter()).copied())
        .max()
        .unwrap_or(1);
    let digit_width = format!("{max_line_num}").len().max(1);

    // Pass 2: skip to effective_scroll, take inner_height, render each triple
    let lines: Vec<Line> = numbered
        .iter()
        .skip(effective_scroll)
        .take(inner_height)
        .map(|(old, new, dl)| render_line_with_gutter(*old, *new, dl, palette, digit_width))
        .collect();

    let title = state
        .git_diff_files
        .get(state.git_diff_selected)
        .map(|f| format!(" {} ", f.path.display()))
        .unwrap_or_else(|| " Diff ".to_string());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

/// Parse "@@ -OLD_START[,OLD_COUNT] +NEW_START[,NEW_COUNT] @@" and return (old_start, new_start).
fn parse_hunk_header(content: &str) -> Option<(u32, u32)> {
    // Example: "@@ -10,6 +10,8 @@ fn foo()"
    let inner = content.strip_prefix("@@ ")?.split(" @@").next()?;
    let mut parts = inner.split_whitespace();
    let old_part = parts.next()?; // "-10,6" or "-10"
    let new_part = parts.next()?; // "+10,8" or "+10"
    let old_start: u32 = old_part
        .trim_start_matches('-')
        .split(',')
        .next()?
        .parse()
        .ok()?;
    let new_start: u32 = new_part
        .trim_start_matches('+')
        .split(',')
        .next()?
        .parse()
        .ok()?;
    Some((old_start, new_start))
}

fn render_line_with_gutter(
    old: Option<u32>,
    new: Option<u32>,
    dl: &DiffLine,
    palette: ThemePalette,
    digit_width: usize,
) -> Line<'static> {
    use DiffLineKind::*;

    let gutter_style = Style::default().fg(palette.text_muted).bg(palette.tools_bg);
    let sep_style = Style::default().fg(palette.text_muted).bg(palette.tools_bg);

    let old_str = match old {
        Some(n) => format!("{n:>digit_width$}"),
        None => " ".repeat(digit_width),
    };
    let new_str = match new {
        Some(n) => format!("{n:>digit_width$}"),
        None => " ".repeat(digit_width),
    };

    let line_style = match dl.kind {
        Added => Style::default().fg(Color::Green),
        Removed => Style::default().fg(Color::Red),
        HunkHeader => Style::default().fg(Color::Cyan),
        FileHeader => Style::default()
            .fg(palette.text_primary)
            .add_modifier(Modifier::BOLD),
        Context => Style::default().fg(palette.text_muted),
    };

    Line::from(vec![
        Span::styled(old_str, gutter_style),
        Span::styled(" ", sep_style),
        Span::styled(new_str, gutter_style),
        Span::styled(" │ ", sep_style),
        Span::styled(dl.content.clone(), line_style),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hunk_header_standard() {
        assert_eq!(parse_hunk_header("@@ -10,6 +10,8 @@"), Some((10, 10)));
    }

    #[test]
    fn parse_hunk_header_with_context() {
        assert_eq!(
            parse_hunk_header("@@ -10,6 +10,8 @@ fn foo()"),
            Some((10, 10))
        );
    }

    #[test]
    fn parse_hunk_header_single_line() {
        assert_eq!(parse_hunk_header("@@ -1 +1 @@"), Some((1, 1)));
    }

    #[test]
    fn parse_hunk_header_invalid_returns_none() {
        assert_eq!(parse_hunk_header("not a hunk header"), None);
    }
}
