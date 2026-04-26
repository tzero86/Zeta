use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Widget, Wrap,
};
use ratatui::Frame;

use crate::action::{Action, MenuId};
use crate::config::ThemePalette;
use crate::state::{menu_tabs, CollisionState, DialogState, MenuContext, MenuItem, PromptState};
use crate::ui::styles::{
    elevated_surface_style, key_pill_style, modal_halo_style, overlay_footer_style,
    overlay_key_hint_style, overlay_title_style, section_divider_style,
};

struct DimOverlay;

impl Widget for DimOverlay {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_style(Style::default().add_modifier(Modifier::DIM));
                }
            }
        }
    }
}

pub fn render_modal_backdrop(
    frame: &mut Frame<'_>,
    area: Rect,
    popup: Rect,
    palette: ThemePalette,
) {
    // Dim the pane content visible behind the modal (no Clear — preserve cell chars)
    frame.render_widget(DimOverlay, area);
    // Halo ring: one-cell border around the modal
    let halo = Rect {
        x: popup.x.saturating_sub(1).max(area.x),
        y: popup.y.saturating_sub(1).max(area.y),
        width: (popup.width + 2).min(area.width.saturating_sub(popup.x.saturating_sub(area.x))),
        height: (popup.height + 2).min(area.height.saturating_sub(popup.y.saturating_sub(area.y))),
    };
    frame.render_widget(Paragraph::new("").style(modal_halo_style(palette)), halo);
}

pub fn menu_popup_width(items: &[MenuItem]) -> u16 {
    let content_width = items
        .iter()
        .map(|item| item.label.chars().count() + item.shortcut.chars().count() + 4)
        .max()
        .unwrap_or(24)
        .max(24);
    (content_width as u16).saturating_add(2)
}

fn build_menu_row<'a>(
    item: &'a MenuItem,
    index: usize,
    selection: usize,
    inner_width: usize,
    palette: ThemePalette,
    shortcut_str: &'a str,
) -> ListItem<'a> {
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
    let shortcut_width = shortcut_str.chars().count();
    let label_width = inner_width.saturating_sub(shortcut_width + 2).max(1);
    let row = format!(" {:<label_width$} {}", item.label, shortcut_str);
    let pad = inner_width.saturating_sub(row.chars().count());
    ListItem::new(Line::from(vec![Span::styled(
        format!("{}{}", row, " ".repeat(pad)),
        base_style,
    )]))
}

pub fn render_menu_popup(
    frame: &mut Frame<'_>,
    area: Rect,
    menu: MenuId,
    items: &[MenuItem],
    selection: usize,
    palette: ThemePalette,
    menu_ctx: MenuContext,
) {
    let mut x = area.x + 1;
    let mut cursor = area.x + 8;
    for tab in menu_tabs(menu_ctx) {
        if tab.id == menu {
            x = cursor;
            break;
        }
        cursor += tab.label.len() as u16;
    }
    let width = menu_popup_width(items);
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
            let shortcut_str = if matches!(item.action, Action::OpenMenu(_)) {
                "►"
            } else {
                item.shortcut
            };
            build_menu_row(
                item,
                index,
                selection,
                inner.width as usize,
                palette,
                shortcut_str,
            )
        })
        .collect::<Vec<_>>();

    let list = List::new(rows);
    let mut state = ListState::default();
    state.select(Some(selection.min(items.len().saturating_sub(1))));
    frame.render_stateful_widget(list, inner, &mut state);
}

/// Render a flyout submenu popup to the right of `parent_area`.
/// If the flyout would overflow the right edge of `area`, it flips to the left of the parent.
pub fn render_flyout_popup(
    frame: &mut Frame<'_>,
    area: Rect,
    parent_area: Rect,
    items: &[MenuItem],
    selection: usize,
    palette: ThemePalette,
) {
    if items.is_empty() {
        return;
    }

    let width = menu_popup_width(items);
    let height = (items.len() as u16 + 2).min(area.height.saturating_sub(parent_area.y));

    let flyout_x = if parent_area.x + parent_area.width + width <= area.x + area.width {
        parent_area.x + parent_area.width
    } else {
        parent_area.x.saturating_sub(width)
    };

    let flyout_area = Rect {
        x: flyout_x,
        y: parent_area.y,
        width,
        height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.prompt_border))
        .style(Style::default().bg(palette.surface_bg));
    let inner = block.inner(flyout_area);
    frame.render_widget(Clear, flyout_area);
    frame.render_widget(block, flyout_area);

    let rows = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            build_menu_row(
                item,
                index,
                selection,
                inner.width as usize,
                palette,
                item.shortcut,
            )
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
            (area.width.min(76), area.height.min(10))
        }
        crate::state::PromptKind::Trash | crate::state::PromptKind::Delete => {
            let n = prompt.source_paths.len().max(1);
            let shown = n.min(5);
            // header line + item lines + ("+N more" if truncated) + blank + hint
            let inner = 1 + shown + usize::from(n > shown) + 1 + 1;
            let needed = (inner as u16 + 2).max(7);
            (area.width.min(64), area.height.min(needed))
        }
        _ => (area.width.min(60), area.height.min(8)),
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
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.prompt_border))
        .style(elevated_surface_style(palette));
    let inner = block.inner(popup_area);
    render_modal_backdrop(frame, area, popup_area, palette);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let is_confirmation = prompt.kind.is_confirmation_only();

    // Build description text (source path info + hint).
    let desc_text = match prompt.kind {
        crate::state::PromptKind::Trash => {
            let paths = &prompt.source_paths;
            if paths.len() > 1 {
                let shown = paths.len().min(5);
                let mut s = format!("Move to trash: {} items\n", paths.len());
                for p in paths.iter().take(shown) {
                    let name = p
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| p.display().to_string());
                    s.push_str(&format!("  {}\n", name));
                }
                if paths.len() > shown {
                    s.push_str(&format!("  ... and {} more\n", paths.len() - shown));
                }
                s.push_str("\nEnter confirm  Esc cancel");
                s
            } else {
                format!(
                    "Move to trash:\n{}\n\nEnter confirm  Esc cancel",
                    prompt
                        .source_path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| String::from("<missing target>")),
                )
            }
        }
        crate::state::PromptKind::Delete => {
            let paths = &prompt.source_paths;
            if paths.len() > 1 {
                let shown = paths.len().min(5);
                let mut s = format!("Delete permanently: {} items\n", paths.len());
                for p in paths.iter().take(shown) {
                    let name = p
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| p.display().to_string());
                    s.push_str(&format!("  {}\n", name));
                }
                if paths.len() > shown {
                    s.push_str(&format!("  ... and {} more\n", paths.len() - shown));
                }
                s.push_str("\nEnter confirm  Esc cancel");
                s
            } else {
                format!(
                    "Delete permanently:\n{}\n\nEnter confirm  Esc cancel",
                    prompt
                        .source_path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| String::from("<missing target>")),
                )
            }
        }
        crate::state::PromptKind::Copy | crate::state::PromptKind::Move => format!(
            "Source:\n{}\n\nDestination:  (Enter submit  Esc cancel)",
            prompt
                .source_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| String::from("<missing source>")),
        ),
        _ => format!(
            "Path: {}\n(Enter submit  Esc cancel)",
            prompt.base_path.display()
        ),
    };

    // Split inner area: description rows on top, input field on bottom (if not confirmation).
    let (desc_area, input_area) = if is_confirmation {
        (inner, Rect::default())
    } else {
        let input_height = 3_u16; // border top + text + border bottom
        if inner.height <= input_height {
            (inner, Rect::default())
        } else {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(input_height)])
                .split(inner);
            (chunks[0], chunks[1])
        }
    };

    // Render description paragraph.
    let desc_para = Paragraph::new(desc_text)
        .style(Style::default().fg(palette.text_primary))
        .wrap(Wrap { trim: false });
    frame.render_widget(desc_para, desc_area);

    // Render styled input field with cursor.
    if !is_confirmation && input_area.width > 0 && input_area.height > 0 {
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.prompt_border))
            .style(Style::default().bg(palette.tools_bg));
        let input_inner = input_block.inner(input_area);
        frame.render_widget(input_block, input_area);

        let value = prompt.value();
        // Show only the last N characters that fit in the visible width.
        let visible_width = input_inner.width as usize;
        let cursor_pos = prompt.input.visual_cursor();
        let scroll_offset = if cursor_pos >= visible_width {
            cursor_pos - visible_width + 1
        } else {
            0
        };
        let visible_text: String = value
            .chars()
            .skip(scroll_offset)
            .take(visible_width)
            .collect();

        let input_para = Paragraph::new(visible_text).style(
            Style::default()
                .fg(palette.text_primary)
                .bg(palette.tools_bg),
        );
        frame.render_widget(input_para, input_inner);

        // Position the blinking cursor.
        let cursor_x =
            input_inner.x + (cursor_pos.saturating_sub(scroll_offset)).min(visible_width) as u16;
        let cursor_y = input_inner.y;
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_two_column_help(
    frame: &mut Frame<'_>,
    inner: Rect,
    lines: &[String],
    scroll: u16,
    palette: ThemePalette,
) {
    let mut left: Vec<&str> = Vec::new();
    let mut right: Vec<&str> = Vec::new();
    let mut in_left = false;
    let mut in_right = false;
    for line in lines {
        if line == "##COLSTART" {
            in_left = true;
            continue;
        }
        if line == "##COLBREAK" {
            in_left = false;
            in_right = true;
            continue;
        }
        if line == "##COLEND" {
            break;
        }
        if in_left {
            left.push(line.as_str());
        }
        if in_right {
            right.push(line.as_str());
        }
    }

    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let render_col = |col_lines: &[&str]| -> Vec<Line<'static>> {
        col_lines
            .iter()
            .map(|raw| {
                if raw.is_empty() {
                    Line::raw("")
                } else if let Some(header) = raw.strip_prefix("## ") {
                    Line::from(Span::styled(
                        header.to_string(),
                        section_divider_style(palette),
                    ))
                } else if let Some((key, desc)) = raw.split_once('\t') {
                    let key_part = key.trim();
                    Line::from(vec![
                        Span::styled(format!(" {} ", key_part), key_pill_style(palette)),
                        Span::raw("  "),
                        Span::styled(desc.to_string(), Style::default().fg(palette.text_primary)),
                    ])
                } else {
                    Line::from(Span::styled(
                        raw.to_string(),
                        Style::default().fg(palette.text_primary),
                    ))
                }
            })
            .collect()
    };

    let left_lines = render_col(&left);
    let right_lines = render_col(&right);
    frame.render_widget(Paragraph::new(left_lines).scroll((scroll, 0)), halves[0]);
    frame.render_widget(Paragraph::new(right_lines).scroll((scroll, 0)), halves[1]);
}

pub fn render_dialog(
    frame: &mut Frame<'_>,
    area: Rect,
    dialog: &DialogState,
    palette: ThemePalette,
) {
    let content_len = dialog.lines.len() as u16;
    let width = area.width.min(68);
    // Cap height so the dialog always fits: reserve space for menu bar, status bar,
    // key hint bar (3 rows), plus 1 row of breathing room.
    let max_height = area.height.saturating_sub(4).max(6);
    let height = (content_len + 2).min(max_height);
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
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.prompt_border))
        .style(elevated_surface_style(palette));
    let inner = block.inner(popup_area);
    render_modal_backdrop(frame, area, popup_area, palette);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    // Clamp scroll so the last visible line is always reachable but we never
    // scroll past the end.
    let visible_lines = inner.height as usize;
    let max_scroll = dialog.lines.len().saturating_sub(visible_lines);
    let scroll = dialog.scroll.min(max_scroll) as u16;

    let has_two_col = dialog.lines.iter().any(|l| l == "##COLSTART");
    if has_two_col {
        render_two_column_help(frame, inner, &dialog.lines, scroll, palette);
    } else {
        let styled_lines: Vec<Line> = dialog
            .lines
            .iter()
            .map(|raw| {
                if raw.is_empty() {
                    Line::raw("")
                } else if let Some(art) = raw.strip_prefix("##LOGO ") {
                    Line::from(Span::styled(
                        art.to_string(),
                        Style::default()
                            .fg(palette.accent_mauve)
                            .add_modifier(Modifier::BOLD),
                    ))
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
                } else {
                    Line::from(Span::styled(
                        raw.clone(),
                        Style::default().fg(palette.text_primary),
                    ))
                }
            })
            .collect();

        let paragraph = Paragraph::new(styled_lines)
            .style(elevated_surface_style(palette))
            .scroll((scroll, 0));
        frame.render_widget(paragraph, inner);
    }

    // Render a scrollbar when content overflows the visible area.
    if dialog.lines.len() > visible_lines {
        let mut scrollbar_state = ScrollbarState::new(max_scroll + 1).position(scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼")),
            popup_area,
            &mut scrollbar_state,
        );
    }
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
    let popup_area = Rect {
        x,
        y,
        width,
        height,
    };

    let block = Block::default()
        .title("Resolve Collision")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(palette.prompt_border)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(palette.prompt_bg));
    let inner = block.inner(popup_area);
    render_modal_backdrop(frame, area, popup_area, palette);
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

pub fn render_destructive_confirm(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &crate::state::dialog::DestructiveConfirmState,
    palette: ThemePalette,
) {
    let lines = state.lines();
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

    // Background
    render_modal_backdrop(frame, area, popup_area, palette);
    frame.render_widget(Clear, popup_area);

    // Modal border
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(palette.prompt_border)
                .add_modifier(Modifier::BOLD),
        )
        .title(" Destructive Action ")
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(palette.prompt_bg));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Content
    let styled_lines: Vec<Line> = lines
        .iter()
        .map(|line| {
            if line.starts_with("Y/") || line.starts_with("N/") {
                let key_hint_style = Style::default()
                    .fg(palette.key_hint_fg)
                    .add_modifier(Modifier::BOLD);
                if let Some((key, _)) = line.split_once(char::is_whitespace) {
                    let rest = &line[key.len()..].trim_start();
                    Line::from(vec![
                        Span::styled(key.to_string(), key_hint_style),
                        Span::raw("  "),
                        Span::styled(rest.to_string(), Style::default().fg(palette.text_primary)),
                    ])
                } else {
                    Line::styled(line.clone(), key_hint_style)
                }
            } else if line.starts_with("⚠") {
                Line::styled(
                    line.clone(),
                    Style::default()
                        .fg(palette.prompt_border)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Line::styled(line.clone(), Style::default().fg(palette.text_primary))
            }
        })
        .collect();

    let paragraph = Paragraph::new(styled_lines).style(Style::default().bg(palette.prompt_bg));
    frame.render_widget(paragraph, inner);
}

#[allow(dead_code)]
pub fn render_footer_hint(frame: &mut Frame<'_>, area: Rect, text: &str, palette: ThemePalette) {
    frame.render_widget(
        Paragraph::new(text).style(overlay_footer_style(palette)),
        area,
    );
}

/// Render the "Open With" context menu popup centred in `area`.
pub fn render_open_with_popup(
    frame: &mut Frame<'_>,
    area: Rect,
    items: &[(String, String)],
    selection: usize,
    palette: ThemePalette,
) {
    let max_name_len = items
        .iter()
        .map(|(n, _)| n.chars().count())
        .max()
        .unwrap_or(16)
        .max(16);
    let width = (max_name_len as u16 + 4).min(area.width);
    let height = (items.len() as u16 + 2).min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let popup_area = Rect {
        x,
        y,
        width,
        height,
    };

    render_modal_backdrop(frame, area, popup_area, palette);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(" Open With ", overlay_title_style(palette)))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(elevated_surface_style(palette));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let list_items: Vec<ListItem<'_>> = items
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            let style = if i == selection {
                Style::default()
                    .bg(palette.selection_bg)
                    .fg(palette.selection_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_primary)
            };
            ListItem::new(Span::styled(format!(" {name} "), style))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selection));
    frame.render_stateful_widget(List::new(list_items), inner, &mut list_state);
}
