use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::action::MenuId;
use crate::config::{IconMode, ThemePalette};
use crate::editor::EditorBuffer;
use crate::fs::EntryInfo;
use crate::fs::EntryKind;
use crate::icon::icon_for_kind;
use crate::pane::{PaneId, PaneState};
use crate::preview::ViewBuffer;
use crate::state::{AppState, CollisionState, DialogState, MenuItem, PaneLayout, PromptState};
use unicode_width::UnicodeWidthChar;

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
        state,
    );

    // Right pane is always rendered — editor now lives in the tools panel below.
    render_pane(
        frame,
        panes[1],
        state.right_pane(),
        second_label,
        right_focused,
        Borders::ALL,
        state,
    );

    // Tools panel — editor takes priority over preview when both could be shown.
    if let Some(tools_area) = tools_area_opt {
        if has_editor {
            if let Some(editor) = state.editor_mut() {
                render_editor(frame, tools_area, editor, true, true, palette);
            }
        } else if is_preview_open {
            let preview_view = state.preview_view().map(|(_, v)| v);
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
}

fn render_menu_bar(frame: &mut Frame<'_>, area: Rect, state: &AppState, palette: ThemePalette) {
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

pub(super) fn top_bar_logo_spans(active: bool, palette: ThemePalette) -> Vec<Span<'static>> {
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

struct PaneChrome {
    border: Style,
    title: Style,
    surface: Style,
}

fn pane_chrome_style(is_focused: bool, palette: ThemePalette) -> PaneChrome {
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
        .style(
            Style::default()
                .bg(palette.tools_bg)
                .fg(palette.text_primary),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

fn render_dialog(frame: &mut Frame<'_>, area: Rect, dialog: &DialogState, palette: ThemePalette) {
    let width = area.width.min(68);
    let height = ((dialog.lines.len() as u16) + 2).min(area.height.saturating_sub(2).max(6));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect {
        x,
        y,
        width,
        height,
    };

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
                // Explicit section header marker — styled accent, no "##" shown.
                Line::from(Span::styled(
                    header.to_string(),
                    Style::default()
                        .fg(palette.menu_mnemonic_fg)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if let Some((key, desc)) = raw.split_once('\t') {
                // Entry line: "  KEY\tdescription"
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
                // Plain line (About text, ASCII art, etc.) — no accent.
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

fn render_pane(
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

/// Shared renderer for both preview and editor content.
/// Renders each visible line into its own 1-row Rect to prevent soft-wrap
/// and eliminate ghost cells from previous frames.
///
/// - `area`: the full content area (no border, no title — inner area only)
/// - `lines`: slice of highlighted lines to render, pre-sliced to viewport height
/// - `first_line_number`: line number of lines[0] for gutter display (1-based)
/// - `gutter_width`: number of columns to reserve for line numbers (suggest 5)
/// - `scroll_col`: horizontal scroll offset in chars
/// - `cursor_row`: if Some(r), highlight that row with selection_bg (editor cursor)
/// - `palette`: theme colours
fn render_code_view(
    frame: &mut Frame<'_>,
    area: Rect,
    lines: &[crate::highlight::HighlightedLine],
    first_line_number: usize,
    gutter_width: u16,
    scroll_col: usize,
    cursor_row: Option<usize>,
    palette: ThemePalette,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    // Split area into gutter | content horizontally.
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(gutter_width), Constraint::Min(1)])
        .split(area);
    let gutter_area = chunks[0];
    let content_area = chunks[1];
    let viewport_cols = content_area.width as usize;

    let blank_style = Style::default().bg(palette.surface_bg);

    for row_idx in 0..area.height as usize {
        let y = area.y + row_idx as u16;
        if y >= area.y + area.height {
            break;
        }
        let gutter_rect = Rect {
            x: gutter_area.x,
            y,
            width: gutter_area.width,
            height: 1,
        };
        let content_rect = Rect {
            x: content_area.x,
            y,
            width: content_area.width,
            height: 1,
        };
        frame.render_widget(Paragraph::new(" ").style(blank_style), gutter_rect);
        frame.render_widget(Paragraph::new(" ").style(blank_style), content_rect);
    }

    for (row_idx, line_tokens) in lines.iter().enumerate() {
        let y = area.y + row_idx as u16;
        if y >= area.y + area.height {
            break;
        }

        // Gutter: right-aligned line number.
        let line_num = first_line_number + row_idx;
        let gutter_text = format!(
            "{:>width$} ",
            line_num,
            width = (gutter_width as usize).saturating_sub(2)
        );
        let gutter_rect = Rect {
            x: gutter_area.x,
            y,
            width: gutter_area.width,
            height: 1,
        };
        let gutter_style = Style::default()
            .fg(palette.text_muted)
            .bg(palette.surface_bg);
        frame.render_widget(Paragraph::new(gutter_text).style(gutter_style), gutter_rect);

        // Content row.
        let content_rect = Rect {
            x: content_area.x,
            y,
            width: content_area.width,
            height: 1,
        };
        let row_bg = if cursor_row == Some(row_idx) {
            Style::default().bg(palette.selection_bg)
        } else {
            Style::default().bg(palette.surface_bg)
        };

        // Build spans, applying horizontal scroll and column truncation.
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

            if token_end <= scroll_col {
                continue;
            }

            let skip = scroll_col.saturating_sub(token_start);
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

fn render_preview_panel(
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
            render_code_view(
                frame,
                inner,
                window,
                first_line_num + 1,
                5,
                0,
                None,
                palette,
            );
        }
    }
}

fn render_item(
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
        crate::fs::EntryKind::Directory => format!("{}/", entry.name),
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

struct PaneRowStyles {
    guide: Style,
    branch: Style,
    mark: Style,
    icon: Style,
    name: Style,
    meta: Style,
}

fn pane_row_styles(
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

fn format_icon_slot(icon: &str, icon_mode: IconMode) -> String {
    match icon_mode {
        IconMode::Unicode | IconMode::Custom => format!("{icon}  "),
        IconMode::Ascii => icon.to_string(),
    }
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

fn render_command_palette(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &crate::palette::PaletteState,
    palette: ThemePalette,
) {
    let width = ((area.width as f32 * 0.90) as u16)
        .clamp(40, 80)
        .min(area.width);
    let max_results = 15usize;
    let height = (max_results as u16 + 5).min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect {
        x,
        y,
        width,
        height,
    };

    let block = Block::default()
        .title(Span::styled(
            " Command Palette ",
            overlay_title_style(palette),
        ))
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

    // Split inner: 1 row for the query input, rest for results.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // Input line: "> query_"
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

    // Results list.
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

    // Scroll window around selection.
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
                // Use inner.width (excludes borders) so the row fits exactly.
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

fn render_settings_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    settings: &crate::state::SettingsState,
    state: &AppState,
    palette: ThemePalette,
) {
    let entries = state.settings_entries();
    let width = ((area.width as f32 * 0.72) as u16)
        .clamp(52, 84)
        .min(area.width);
    let height = (entries.len() as u16 + 6).min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect {
        x,
        y,
        width,
        height,
    };

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
                Span::styled(
                    entry.value.clone(),
                    Style::default().fg(palette.key_hint_fg),
                ),
                Span::raw(format!("  {}", entry.hint)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(
        settings.selection.min(entries.len().saturating_sub(1)),
    ));
    frame.render_stateful_widget(
        List::new(rows).style(elevated_surface_style(palette)),
        chunks[1],
        &mut list_state,
    );

    let footer = Paragraph::new("Ctrl+O opens settings • theme, icons, preview, layout")
        .style(overlay_footer_style(palette));
    frame.render_widget(footer, chunks[2]);
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

    // Reserve 1 row at the bottom for the search bar when search is active.
    let (content_area, search_bar_area) = if editor.search_active {
        let splits = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        (splits[0], Some(splits[1]))
    } else {
        (inner, None)
    };

    // Gutter width: enough for 5-digit line numbers + 1 space separator.
    let gutter_width = 6u16;

    // Derive viewport size from content_area (render_code_view will split it,
    // but we need the content column width to compute scroll clamping).
    let (visible_start, visible_lines) = editor.visible_line_window(content_area.height as usize);
    let viewport_cols = content_area.width.saturating_sub(gutter_width) as usize;
    editor.clamp_horizontal_scroll(viewport_cols);
    let scroll_col = editor.scroll_col;

    // Convert visible lines to HighlightedLine (single plain token per line).
    let highlighted: Vec<crate::highlight::HighlightedLine> = visible_lines
        .iter()
        .map(|line| {
            let text = line.strip_suffix('\n').unwrap_or(line).to_string();
            vec![(
                palette.text_primary,
                ratatui::style::Modifier::empty(),
                text,
            )]
        })
        .collect();

    let cursor_visible_row = if is_active {
        Some(editor.cursor_line_col().0.saturating_sub(visible_start))
    } else {
        None
    };

    render_code_view(
        frame,
        content_area,
        &highlighted,
        visible_start + 1, // 1-based
        gutter_width,
        scroll_col,
        cursor_visible_row,
        palette,
    );

    // Render the inline search bar when active.
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

    // Set terminal cursor position for blinking cursor.
    if is_active {
        let (line, column) = editor.cursor_line_col();
        let visible_line = line.saturating_sub(visible_start);
        // gutter_width columns are reserved; content starts at content_area.x + gutter_width.
        let content_x = content_area.x + gutter_width;
        let cursor_y =
            content_area.y + (visible_line as u16).min(content_area.height.saturating_sub(1));
        let visible_col = column.saturating_sub(scroll_col);
        let cursor_x = content_x
            + (visible_col as u16).min(content_area.width.saturating_sub(gutter_width + 1));
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn elevated_surface_style(palette: ThemePalette) -> Style {
    Style::default().bg(palette.tools_bg)
}

fn overlay_title_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.menu_mnemonic_fg)
        .add_modifier(Modifier::BOLD)
}

fn overlay_key_hint_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.key_hint_fg)
        .add_modifier(Modifier::BOLD)
}

fn overlay_footer_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.text_muted)
}

fn command_palette_header_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.text_muted)
        .add_modifier(Modifier::BOLD)
}

fn command_palette_entry_label_style(is_selected: bool, palette: ThemePalette) -> Style {
    if is_selected {
        Style::default()
            .fg(palette.selection_fg)
            .bg(palette.selection_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_primary)
    }
}

fn command_palette_entry_hint_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.key_hint_fg)
}

#[cfg(test)]
mod tests {
    use super::{
        command_palette_entry_hint_style, command_palette_entry_label_style,
        command_palette_header_style, elevated_surface_style, format_icon_slot,
        overlay_title_style, pane_chrome_style, top_bar_logo_spans,
    };
    use crate::config::{IconMode, ThemePalette};
    use crate::fs::EntryKind;
    use crate::icon::icon_for_kind;
    use crate::palette::all_entries;
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
    fn custom_icons_use_private_use_glyphs() {
        assert_eq!(
            icon_for_kind(EntryKind::Directory, IconMode::Custom),
            "\u{e001}"
        );
        assert_eq!(icon_for_kind(EntryKind::File, IconMode::Custom), "\u{e002}");
        assert_eq!(
            icon_for_kind(EntryKind::Symlink, IconMode::Custom),
            "\u{e003}"
        );
        assert_eq!(
            icon_for_kind(EntryKind::Other, IconMode::Custom),
            "\u{e004}"
        );
    }

    #[test]
    fn unicode_icon_slot_reserves_two_columns() {
        assert_eq!(format_icon_slot("▣", IconMode::Unicode), "▣  ");
    }

    #[test]
    fn ascii_icon_slot_uses_label_width() {
        assert_eq!(format_icon_slot("[D]", IconMode::Ascii), "[D]");
    }

    #[test]
    fn custom_icon_slot_reserves_two_columns() {
        assert_eq!(format_icon_slot("\u{e001}", IconMode::Custom), "\u{e001}  ");
    }

    #[test]
    fn top_bar_logo_uses_logo_accent() {
        let spans = top_bar_logo_spans(true, test_palette());

        assert_eq!(spans[1].style.fg, Some(Color::Rgb(200, 201, 202)));
        assert_eq!(spans[2].style.fg, Some(Color::Rgb(200, 201, 202)));
        assert_eq!(spans[3].style.fg, Some(Color::Rgb(200, 201, 202)));
    }

    #[test]
    fn focused_pane_uses_focus_border_and_surface() {
        let chrome = pane_chrome_style(true, test_palette());

        assert_eq!(chrome.border.fg, Some(Color::Rgb(50, 51, 52)));
        assert_eq!(chrome.title.fg, Some(Color::Rgb(50, 51, 52)));
        assert_eq!(chrome.surface.bg, Some(Color::Rgb(90, 91, 92)));
    }

    #[test]
    fn inactive_pane_uses_muted_border_and_quieter_surface() {
        let chrome = pane_chrome_style(false, test_palette());

        assert_eq!(chrome.border.fg, Some(Color::Rgb(140, 141, 142)));
        assert_eq!(chrome.title.fg, Some(Color::Rgb(140, 141, 142)));
        assert_eq!(chrome.surface.bg, Some(Color::Rgb(100, 101, 102)));
    }

    #[test]
    fn elevated_overlays_use_tools_surface() {
        let style = elevated_surface_style(test_palette());

        assert_eq!(style.bg, Some(Color::Rgb(100, 101, 102)));
    }

    #[test]
    fn overlay_titles_keep_accent_styling() {
        let style = overlay_title_style(test_palette());

        assert_eq!(style.fg, Some(Color::Rgb(40, 41, 42)));
        assert!(style.add_modifier.contains(ratatui::style::Modifier::BOLD));
    }

    #[test]
    fn command_palette_rows_keep_category_and_hint_emphasis() {
        let entries = all_entries();
        let entry = entries
            .iter()
            .find(|entry| entry.category == "Navigation" && entry.label == "Open / enter selection")
            .expect("expected navigation entry");

        let category_style = command_palette_header_style(test_palette());
        let label_style = command_palette_entry_label_style(true, test_palette());
        let hint_style = command_palette_entry_hint_style(test_palette());

        assert_eq!(category_style.fg, Some(Color::Rgb(140, 141, 142)));
        assert_eq!(label_style.fg, Some(Color::Rgb(80, 81, 82)));
        assert_eq!(hint_style.fg, Some(Color::Rgb(210, 211, 212)));
        assert_eq!(entry.hint, "Enter");
    }
}
