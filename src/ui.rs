use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::action::MenuId;
use crate::config::ThemePalette;
use crate::editor::EditorBuffer;
use crate::fs::EntryInfo;
use crate::fs::EntryKind;
use crate::jobs::PreviewContent;
use crate::pane::{PaneId, PaneState};
use crate::state::{AppState, CollisionState, DialogState, MenuItem, PaneLayout, PromptState};

fn get_entry_icon(kind: EntryKind) -> &'static str {
    // Single-width chars that render on any UTF-8 terminal.
    // Directories get a trailing "/" in the name already, so the icon
    // here is just a subtle type hint.
    match kind {
        EntryKind::Directory => ">",
        EntryKind::Symlink => "~",
        EntryKind::File => " ",
        EntryKind::Other => "?",
    }
}

pub fn render(frame: &mut Frame<'_>, state: &mut AppState) {
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
        // Split vertically: panes on top, tools panel on bottom.
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

    // Horizontal split of pane_area into left/right (or top/bottom when stacked).
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
        // Left pane: no right border so it shares one line with the right pane.
        Borders::TOP | Borders::LEFT | Borders::BOTTOM,
        palette,
    );

    // Right pane is always rendered — editor now lives in the tools panel below.
    render_pane(
        frame,
        panes[1],
        state.right_pane(),
        second_label,
        right_focused,
        Borders::ALL,
        palette,
    );

    // Tools panel — editor takes priority over preview when both could be shown.
    if let Some(tools_area) = tools_area_opt {
        if has_editor {
            if let Some(editor) = state.editor_mut() {
                render_editor(frame, tools_area, editor, true, true, palette);
            }
        } else if is_preview_open {
            let preview_content = state.preview().map(|(_, c)| c);
            let filename = state.active_pane_title().to_string();
            render_preview_panel(frame, tools_area, preview_content, &filename, palette);
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

    let status = Paragraph::new(Line::raw(state.status_line())).style(
        Style::default()
            .fg(palette.status_fg)
            .bg(palette.status_bg)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(status, areas[2]);
}

fn render_menu_bar(frame: &mut Frame<'_>, area: Rect, state: &AppState, palette: ThemePalette) {
    let mut line = Line::default();
    line.spans
        .extend(logo_spans(state.active_menu().is_none(), palette));
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
            .bg(palette.menu_bg)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(menu, area);
}

fn logo_spans(active: bool, palette: ThemePalette) -> Vec<Span<'static>> {
    let style = if active {
        Style::default()
            .fg(palette.menu_mnemonic_fg)
            .bg(palette.menu_active_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(palette.menu_mnemonic_fg)
            .bg(palette.menu_bg)
            .add_modifier(Modifier::BOLD)
    };

    vec![
        Span::styled(" [Z] ", style),
        Span::styled(
            "Zeta ",
            Style::default()
                .fg(palette.menu_fg)
                .bg(style.bg.unwrap_or(palette.menu_bg))
                .add_modifier(Modifier::BOLD),
        ),
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
        MenuId::Help => area.x + 26,
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
    let (width, height) = match prompt.kind {
        crate::state::PromptKind::Copy | crate::state::PromptKind::Move => {
            (area.width.min(76), area.height.min(8))
        }
        crate::state::PromptKind::Delete => (area.width.min(64), area.height.min(6)),
        _ => (area.width.min(56), area.height.min(6)),
    };
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
        .style(
            Style::default()
                .bg(palette.prompt_bg)
                .fg(palette.text_primary),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

fn render_dialog(frame: &mut Frame<'_>, area: Rect, dialog: &DialogState, palette: ThemePalette) {
    let width = area.width.min(58);
    let height = (dialog.lines.len() as u16 + 2).min(area.height.saturating_sub(2).max(3));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect {
        x,
        y,
        width,
        height,
    };

    let block = Block::default()
        .title(dialog.title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.prompt_border))
        .style(Style::default().bg(palette.prompt_bg));
    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let body = dialog.lines.join("\n");
    let paragraph = Paragraph::new(body)
        .style(
            Style::default()
                .bg(palette.prompt_bg)
                .fg(palette.text_primary),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

fn render_collision_dialog(
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
    let popup_area = Rect {
        x,
        y,
        width,
        height,
    };

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

    let paragraph = Paragraph::new(lines.join("\n"))
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
    label: &str,
    is_focused: bool,
    borders: Borders,
    palette: ThemePalette,
) {
    let border_style = if is_focused {
        Style::default()
            .fg(palette.border_focus)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_muted)
    };

    let title = format!("{} [{}]  {}", label, pane.entries.len(), pane.cwd.display());
    let block = Block::default()
        .title(title)
        .borders(borders)
        .border_style(border_style);
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
                    index + 1 == visible_entries.len(),
                    list_area.width as usize,
                    palette,
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

    frame.render_stateful_widget(list, list_area, &mut list_state);
}

fn render_preview_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    content: Option<&PreviewContent>,
    filename: &str,
    palette: ThemePalette,
) {
    let title = format!(" Preview  {} ", filename);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.text_muted));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let body = match content {
        Some(PreviewContent::Text(t)) => t.clone(),
        Some(PreviewContent::Binary { size_bytes }) => format!("[binary — {size_bytes} bytes]"),
        Some(PreviewContent::Empty) => String::from("[empty file]"),
        None => String::from("[directory — select a file to preview]"),
    };

    let paragraph = Paragraph::new(body)
        .style(Style::default().fg(palette.text_primary))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

fn render_item(
    entry: &EntryInfo,
    is_last: bool,
    available_width: usize,
    palette: ThemePalette,
) -> ListItem<'static> {
    let guide = if is_last { "  " } else { "│ " };
    let branch = if is_last { "└" } else { "├" };
    let label_style = match entry.kind {
        crate::fs::EntryKind::Directory => Style::default()
            .fg(palette.directory_fg)
            .add_modifier(Modifier::BOLD),
        crate::fs::EntryKind::Symlink => Style::default().fg(palette.symlink_fg),
        crate::fs::EntryKind::File => Style::default().fg(palette.file_fg),
        crate::fs::EntryKind::Other => Style::default().fg(palette.text_muted),
    };
    let icon = get_entry_icon(entry.kind);
    let name = match entry.kind {
        crate::fs::EntryKind::Directory => format!("{}/", entry.name),
        _ => entry.name.clone(),
    };
    let meta = format_entry_meta(entry);
    let prefix = format!("{}{} {} ", guide, branch, icon);
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
        Span::styled(guide, Style::default().fg(palette.text_muted)),
        Span::styled(
            format!("{} ", branch),
            Style::default().fg(palette.text_muted),
        ),
        Span::styled(format!("{} ", icon), label_style),
        Span::styled(name, label_style),
        Span::styled(" ".repeat(spacer_width), Style::default()),
        Span::styled(meta, Style::default().fg(palette.text_muted)),
    ]))
}

fn display_width(value: &str) -> usize {
    value.chars().count()
}

fn truncate_text(value: &str, max_width: usize) -> String {
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

fn format_entry_meta(entry: &EntryInfo) -> String {
    match entry.kind {
        crate::fs::EntryKind::Directory => String::from("dir"),
        crate::fs::EntryKind::Symlink => String::from("link"),
        crate::fs::EntryKind::Other => String::from("other"),
        crate::fs::EntryKind::File => {
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

fn human_size(size: u64) -> String {
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

fn render_editor(
    frame: &mut Frame<'_>,
    area: Rect,
    editor: &mut EditorBuffer,
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

    // Gutter width: enough for 5-digit line numbers + 1 space separator.
    let gutter_width = 6u16;
    let editor_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(gutter_width), Constraint::Min(1)])
        .split(inner);

    let viewport_cols = editor_chunks[1].width as usize;
    editor.clamp_horizontal_scroll(viewport_cols);
    let scroll_col = editor.scroll_col;

    let line_number_width = (gutter_width as usize).saturating_sub(1);
    let (visible_start, visible_lines) =
        editor.visible_line_window(editor_chunks[1].height as usize);

    // Build gutter: right-align numbers, pad with a space on the right.
    let numbers = visible_lines
        .iter()
        .enumerate()
        .map(|(index, _)| {
            format!(
                "{:>width$} ",
                visible_start + index + 1,
                width = line_number_width.saturating_sub(1),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let gutter = Paragraph::new(numbers).style(
        Style::default()
            .fg(palette.text_muted)
            .bg(palette.surface_bg),
    );
    frame.render_widget(gutter, editor_chunks[0]);

    // Content: slice each line by scroll_col so the viewport pans horizontally.
    // No word-wrap so line numbers stay in sync with visible rows.
    let preview = visible_lines
        .into_iter()
        .map(|line| {
            let stripped = line.strip_suffix('\n').unwrap_or(&line);
            let chars: Vec<char> = stripped.chars().collect();
            chars.into_iter().skip(scroll_col).collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let paragraph = Paragraph::new(preview).style(
        Style::default()
            .fg(palette.text_primary)
            .bg(palette.surface_bg),
    );
    frame.render_widget(paragraph, editor_chunks[1]);

    if is_active {
        let (line, column) = editor.cursor_line_col();
        let visible_line = line.saturating_sub(visible_start);
        let cursor_y = editor_chunks[1].y
            + (visible_line as u16).min(editor_chunks[1].height.saturating_sub(1));
        // Offset cursor X by the horizontal scroll position.
        let visible_col = column.saturating_sub(scroll_col);
        let cursor_x =
            editor_chunks[1].x + (visible_col as u16).min(editor_chunks[1].width.saturating_sub(1));
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}
