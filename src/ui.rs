use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::action::MenuId;
use crate::config::ThemePalette;
use crate::editor::EditorBuffer;
use crate::fs::EntryInfo;
use crate::pane::{PaneId, PaneState};
use crate::state::{AppState, MenuItem, PromptState};

pub fn render(frame: &mut Frame<'_>, state: &AppState) {
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

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(areas[1]);

    render_pane(
        frame,
        panes[0],
        state.left_pane(),
        state.focus() == PaneId::Left,
        palette,
    );

    if let Some(editor) = state.editor() {
        render_editor(
            frame,
            panes[1],
            editor,
            state.focus() == PaneId::Right,
            state.is_editor_focused(),
            palette,
        );
    } else {
        render_pane(
            frame,
            panes[1],
            state.right_pane(),
            state.focus() == PaneId::Right,
            palette,
        );
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

    let status = Paragraph::new(Line::raw(state.status_line())).style(
        Style::default()
            .fg(palette.status_fg)
            .bg(palette.status_bg)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(status, areas[2]);
}

fn render_menu_bar(frame: &mut Frame<'_>, area: Rect, state: &AppState, palette: ThemePalette) {
    let menu = Paragraph::new(Line::from(vec![
        menu_span(" Zeta ", None, false, palette),
        menu_span(
            " File ",
            Some('F'),
            state.active_menu() == Some(MenuId::File),
            palette,
        ),
        menu_span(
            " Navigate ",
            Some('N'),
            state.active_menu() == Some(MenuId::Navigate),
            palette,
        ),
        menu_span(
            " View ",
            Some('V'),
            state.active_menu() == Some(MenuId::View),
            palette,
        ),
    ]))
    .style(
        Style::default()
            .fg(palette.menu_fg)
            .bg(palette.menu_bg)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(menu, area);
}

fn menu_span(
    label: &'static str,
    mnemonic: Option<char>,
    active: bool,
    palette: ThemePalette,
) -> Span<'static> {
    let style = if active {
        Style::default()
            .fg(palette.menu_fg)
            .bg(palette.menu_active_bg)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(palette.menu_fg).bg(palette.menu_bg)
    };

    let highlighted = mnemonic.map(|value| value.to_ascii_uppercase());
    let mut line = Line::default();
    let mut used_highlight = false;

    for ch in label.chars() {
        let mut char_style = style;
        if !used_highlight && Some(ch.to_ascii_uppercase()) == highlighted {
            char_style = char_style.fg(palette.menu_mnemonic_fg);
            used_highlight = true;
        }
        line.spans.push(Span::styled(ch.to_string(), char_style));
    }

    Span::from(line.to_string())
}

fn render_menu_popup(
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
    };
    let width = 28;
    let height = items.len() as u16 + 2;
    let popup_area = Rect {
        x,
        y: area.y,
        width,
        height,
    };

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

fn render_prompt(frame: &mut Frame<'_>, area: Rect, prompt: &PromptState, palette: ThemePalette) {
    let width = area.width.min(48);
    let height = 6;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect {
        x,
        y,
        width,
        height,
    };

    let block = Block::default()
        .title(prompt.title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.prompt_border))
        .style(Style::default().bg(palette.prompt_bg));
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
        _ => format!(
            "Path: {}\nValue: {}\nEnter submit | Esc cancel",
            prompt.base_path.display(),
            prompt.value
        ),
    };
    let paragraph = Paragraph::new(body)
        .style(
            Style::default()
                .bg(palette.prompt_bg)
                .fg(palette.text_primary),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

fn render_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    pane: &PaneState,
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
            .map(|(index, entry)| render_item(entry, index + 1 == visible_entries.len(), palette))
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

    frame.render_stateful_widget(list, pane_chunks[0], &mut list_state);

    let legend = Paragraph::new(Line::raw(
        "|-- node  `-- last  [D] dir/  [F] file  [L] link",
    ))
    .style(Style::default().fg(palette.text_muted));
    frame.render_widget(legend, pane_chunks[1]);
}

fn render_item(entry: &EntryInfo, is_last: bool, palette: ThemePalette) -> ListItem<'static> {
    let branch = if is_last { "`--" } else { "|--" };
    let label_style = match entry.kind {
        crate::fs::EntryKind::Directory => Style::default()
            .fg(palette.directory_fg)
            .add_modifier(Modifier::BOLD),
        crate::fs::EntryKind::Symlink => Style::default().fg(palette.symlink_fg),
        crate::fs::EntryKind::File => Style::default().fg(palette.file_fg),
        crate::fs::EntryKind::Other => Style::default().fg(palette.text_muted),
    };
    let name = match entry.kind {
        crate::fs::EntryKind::Directory => format!("{}/", entry.name),
        _ => entry.name.clone(),
    };

    ListItem::new(Line::from(vec![
        Span::styled(
            format!("{} ", branch),
            Style::default().fg(palette.text_muted),
        ),
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
    palette: ThemePalette,
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

    let editor_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(6), Constraint::Min(1)])
        .split(inner);

    let line_gutter = Block::default().style(
        Style::default()
            .fg(palette.text_muted)
            .bg(palette.surface_bg),
    );
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
        .style(
            Style::default()
                .fg(palette.text_muted)
                .bg(palette.surface_bg),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(gutter, editor_chunks[0]);

    let preview = visible_lines
        .into_iter()
        .map(|line| line.strip_suffix('\n').unwrap_or(&line).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let paragraph = Paragraph::new(preview)
        .style(
            Style::default()
                .fg(palette.text_primary)
                .bg(palette.surface_bg),
        )
        .wrap(Wrap { trim: false });
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
