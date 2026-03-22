use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::editor::EditorBuffer;
use crate::fs::EntryInfo;
use crate::pane::{PaneId, PaneState};
use crate::state::AppState;

pub fn render(frame: &mut Frame<'_>, state: &AppState) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(areas[0]);

    render_pane(
        frame,
        panes[0],
        state.left_pane(),
        state.focus() == PaneId::Left,
    );

    if let Some(editor) = state.editor() {
        render_editor(
            frame,
            panes[1],
            editor,
            state.focus() == PaneId::Right,
            state.is_editor_focused(),
        );
    } else {
        render_pane(
            frame,
            panes[1],
            state.right_pane(),
            state.focus() == PaneId::Right,
        );
    }

    let status = Paragraph::new(Line::raw(state.status_line())).style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(status, areas[1]);
}

fn render_pane(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    pane: &PaneState,
    is_focused: bool,
) {
    let border_style = if is_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!("{}  {}", pane.title, pane.cwd.display());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem<'_>> = if pane.entries.is_empty() {
        vec![ListItem::new("(empty)")]
    } else {
        pane.entries.iter().map(render_item).collect()
    };

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .highlight_symbol("> ");

    let mut list_state = ListState::default();
    if !pane.entries.is_empty() {
        list_state.select(Some(pane.selection));
    }

    frame.render_stateful_widget(list, inner, &mut list_state);
}

fn render_item(entry: &EntryInfo) -> ListItem<'static> {
    let line = format!("{} {}", entry.kind.symbol(), entry.name);
    ListItem::new(line)
}

fn render_editor(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    editor: &EditorBuffer,
    is_focused: bool,
    is_active: bool,
) {
    let border_style = if is_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
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

    let line_number_width = 4usize;
    let preview = editor
        .visible_lines()
        .into_iter()
        .enumerate()
        .take(inner.height.saturating_sub(1) as usize)
        .map(|(index, line)| {
            let trimmed = line.strip_suffix('\n').unwrap_or(&line);
            format!(
                "{:>width$} {}",
                index + 1,
                trimmed,
                width = line_number_width
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let paragraph = Paragraph::new(preview).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);

    if is_active {
        let (line, column) = editor.cursor_line_col();
        let cursor_y = inner.y + (line as u16).min(inner.height.saturating_sub(1));
        let cursor_x =
            inner.x + ((column + line_number_width + 1) as u16).min(inner.width.saturating_sub(1));
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}
