use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::ui::overlay::render_modal_backdrop;
use crate::ui::styles::{
    command_palette_entry_hint_style, command_palette_entry_label_style,
    command_palette_header_style, elevated_surface_style, match_highlight_style,
    overlay_footer_style, overlay_title_style,
};

pub fn render_command_palette(
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
    render_modal_backdrop(frame, area, popup_area, palette);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let input_line = Line::from(vec![
        Span::styled(
            " ⌕  ",
            Style::default()
                .fg(palette.border_focus)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            state.query.clone(),
            Style::default()
                .fg(palette.text_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(palette.text_muted)),
    ]);
    let input = Paragraph::new(input_line).style(
        Style::default()
            .fg(palette.text_primary)
            .bg(palette.tools_bg),
    );
    frame.render_widget(input, chunks[0]);

    let footer = Paragraph::new("  Enter run  ·  Esc close").style(overlay_footer_style(palette));
    frame.render_widget(footer, chunks[2]);

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
                let label_max = (inner.width as usize).saturating_sub(hint.len() + 4);
                let label_text: String = entry.label.chars().take(label_max).collect();
                let pad = label_max.saturating_sub(label_text.chars().count());
                let padding = " ".repeat(pad);

                let query_lower = state.query.to_lowercase();
                let mut label_spans: Vec<Span> = Vec::new();
                if query_lower.is_empty() {
                    label_spans.push(Span::styled(label_text.clone() + &padding, label_style));
                } else {
                    let mut remaining = label_text.as_str();
                    let mut out = String::new();
                    while !remaining.is_empty() {
                        if let Some(pos) = remaining.to_lowercase().find(&query_lower) {
                            if pos > 0 {
                                out.push_str(&remaining[..pos]);
                                label_spans
                                    .push(Span::styled(std::mem::take(&mut out), label_style));
                            }
                            let matched = &remaining[pos..pos + query_lower.len()];
                            label_spans.push(Span::styled(
                                matched.to_string(),
                                match_highlight_style(palette),
                            ));
                            remaining = &remaining[pos + query_lower.len()..];
                        } else {
                            out.push_str(remaining);
                            break;
                        }
                    }
                    if !out.is_empty() {
                        label_spans.push(Span::styled(out, label_style));
                    }
                    label_spans.push(Span::styled(padding, label_style));
                }
                let mut line_spans: Vec<Span> = vec![Span::raw(" ")];
                line_spans.extend(label_spans);
                line_spans.push(Span::raw("  "));
                line_spans.push(Span::styled(hint.to_string(), hint_style));
                line_spans.push(Span::raw(" "));
                let line = Line::from(line_spans);
                ListItem::new(line)
            }
        })
        .collect();

    let list = List::new(items).style(elevated_surface_style(palette));
    frame.render_widget(list, chunks[1]);
}
