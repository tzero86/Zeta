use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::editor::EditorBuffer;
use crate::fs::EntryInfo;
use crate::pane::{PaneId, PaneState};
use crate::state::AppState;

pub fn render(frame: &mut Frame<'_>, state: &AppState) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_menu_bar(frame, areas[0], state);

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(areas[1]);

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
    frame.render_widget(status, areas[2]);
}

fn render_menu_bar(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let menu_text = if state.is_editor_focused() {
        " Zeta | File  Open:F4  Save:Ctrl+S  Discard:Ctrl+D  Close:Esc  Quit:Ctrl+Q | Editor "
    } else {
        " Zeta | Navigate  Open:Enter  Parent:Backspace  Editor:F4  Quit:Ctrl+Q | Browser "
    };

    let menu = Paragraph::new(Line::raw(menu_text)).style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Rgb(212, 196, 168))
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(menu, area);
}

fn render_pane(frame: &mut Frame<'_>, area: Rect, pane: &PaneState, is_focused: bool) {
    let border_style = if is_focused {
        Style::default()
            .fg(Color::Rgb(118, 196, 182))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!(
        "{} [{}]  {}",
        pane.title,
        pane.entries.len(),
        pane.cwd.display()
    );
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let pane_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let visible_height = pane_chunks[0].height as usize;
    let visible_entries = pane.visible_entries(visible_height);
    let items: Vec<ListItem<'_>> = if pane.entries.is_empty() {
        vec![ListItem::new("(empty)")]
    } else {
        visible_entries
            .iter()
            .enumerate()
            .map(|(index, entry)| render_item(entry, index + 1 == visible_entries.len()))
            .collect()
    };

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(47, 58, 66))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut list_state = ListState::default();
    if !pane.entries.is_empty() {
        list_state.select(pane.visible_selection(visible_height));
    }

    frame.render_stateful_widget(list, pane_chunks[0], &mut list_state);

    let legend = Paragraph::new(Line::raw(
        "|-- node  `-- last  [D] dir/  [F] file  [L] link",
    ))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(legend, pane_chunks[1]);
}

fn render_item(entry: &EntryInfo, is_last: bool) -> ListItem<'static> {
    let branch = if is_last { "`--" } else { "|--" };
    let label_style = match entry.kind {
        crate::fs::EntryKind::Directory => Style::default()
            .fg(Color::Rgb(118, 196, 182))
            .add_modifier(Modifier::BOLD),
        crate::fs::EntryKind::Symlink => Style::default().fg(Color::Rgb(214, 179, 92)),
        crate::fs::EntryKind::File => Style::default().fg(Color::Gray),
        crate::fs::EntryKind::Other => Style::default().fg(Color::DarkGray),
    };
    let name = match entry.kind {
        crate::fs::EntryKind::Directory => format!("{}/", entry.name),
        _ => entry.name.clone(),
    };

    ListItem::new(Line::from(vec![
        Span::styled(format!("{} ", branch), Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{} ", entry.kind.ascii_label()), label_style),
        Span::styled(name, label_style),
    ]))
}

fn render_editor(
    frame: &mut Frame<'_>,
    area: Rect,
    editor: &EditorBuffer,
    is_focused: bool,
    is_active: bool,
) {
    let border_style = if is_focused {
        Style::default()
            .fg(Color::Rgb(230, 188, 98))
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

    let editor_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(6), Constraint::Min(1)])
        .split(inner);

    let line_gutter = Block::default().style(Style::default().fg(Color::DarkGray).bg(Color::Black));
    frame.render_widget(line_gutter, editor_chunks[0]);

    let line_number_width = 4usize;
    let (visible_start, visible_lines) =
        editor.visible_line_window(editor_chunks[1].height as usize);
    let numbers = visible_lines
        .iter()
        .enumerate()
        .map(|(index, _)| {
            format!(
                "{:>width$}",
                visible_start + index + 1,
                width = line_number_width
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let gutter = Paragraph::new(numbers)
        .style(Style::default().fg(Color::DarkGray).bg(Color::Black))
        .wrap(Wrap { trim: false });
    frame.render_widget(gutter, editor_chunks[0]);

    let preview = visible_lines
        .into_iter()
        .map(|line| line.strip_suffix('\n').unwrap_or(&line).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let paragraph = Paragraph::new(preview).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, editor_chunks[1]);

    if is_active {
        let (line, column) = editor.cursor_line_col();
        let visible_line = line.saturating_sub(visible_start);
        let cursor_y = editor_chunks[1].y
            + (visible_line as u16).min(editor_chunks[1].height.saturating_sub(1));
        let cursor_x =
            editor_chunks[1].x + (column as u16).min(editor_chunks[1].width.saturating_sub(1));
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}
