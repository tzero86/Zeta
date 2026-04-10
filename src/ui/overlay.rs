use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::action::MenuId;
use crate::config::ThemePalette;
use crate::state::{menu_tabs, CollisionState, DialogState, MenuItem, PromptState};
use crate::ui::styles::{
    elevated_surface_style, overlay_footer_style, overlay_key_hint_style, overlay_title_style,
};

pub fn render_menu_popup(
    frame: &mut Frame<'_>,
    area: Rect,
    menu: MenuId,
    items: &[MenuItem],
    selection: usize,
    palette: ThemePalette,
    editor_mode: bool,
) {
    let mut x = area.x + 1;
    let mut cursor = area.x + 8;
    for tab in menu_tabs(editor_mode) {
        if tab.id == menu {
            x = cursor;
            break;
        }
        cursor += tab.label.len() as u16;
    }
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

pub fn render_prompt(
    frame: &mut Frame<'_>,
    area: Rect,
    prompt: &PromptState,
    palette: ThemePalette,
) {
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

pub fn render_dialog(
    frame: &mut Frame<'_>,
    area: Rect,
    dialog: &DialogState,
    palette: ThemePalette,
) {
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
            } else if raw == " ____  ________  ____             __               " {
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled(
                        "____  ________  ____             __               ",
                        Style::default().fg(palette.text_primary),
                    ),
                ])
            } else {
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

#[allow(dead_code)]
pub fn render_footer_hint(frame: &mut Frame<'_>, area: Rect, text: &str, palette: ThemePalette) {
    frame.render_widget(
        Paragraph::new(text).style(overlay_footer_style(palette)),
        area,
    );
}
