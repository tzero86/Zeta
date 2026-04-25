use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    config::ThemePalette,
    git::{DiffLine, DiffLineKind, FileStatus},
    state::AppState,
};

/// Top-level renderer: splits area 38/62, renders file list left, diff content right.
pub fn render_git_diff_view(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    render_diff_file_list(f, chunks[0], state);
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
            let status_char = match f.status {
                FileStatus::Untracked => "?",
                FileStatus::Added => "A",
                FileStatus::Deleted => "D",
                FileStatus::Modified => "M",
                FileStatus::Renamed => "R",
                FileStatus::Conflicted => "C",
            };
            let label = format!(
                "{} {} +{} -{}",
                status_char,
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

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(palette.selection_bg)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut list_state);
}

/// Right pane: scrollable unified diff with colour-coded lines.
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
    let lines: Vec<Line> = state
        .git_diff_lines
        .iter()
        .skip(state.git_diff_scroll)
        .take(inner_height)
        .map(|dl| diff_line_to_ratatui(dl, palette))
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

fn diff_line_to_ratatui(dl: &DiffLine, palette: ThemePalette) -> Line<'static> {
    let style = match dl.kind {
        DiffLineKind::Added => Style::default().fg(Color::Green),
        DiffLineKind::Removed => Style::default().fg(Color::Red),
        DiffLineKind::HunkHeader => Style::default().fg(Color::Cyan),
        DiffLineKind::FileHeader => Style::default()
            .fg(palette.text_primary)
            .add_modifier(Modifier::BOLD),
        DiffLineKind::Context => Style::default().fg(palette.text_muted),
    };
    Line::from(Span::styled(dl.content.clone(), style))
}
