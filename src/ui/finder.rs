use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::finder::FileFinderState;
use crate::ui::overlay::render_modal_backdrop;
use crate::ui::styles::{
    elevated_surface_style, finder_match_highlight_style, overlay_footer_style, overlay_title_style,
};

pub fn render_file_finder(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &FileFinderState,
    palette: ThemePalette,
) {
    let width = ((area.width as f32 * 0.90) as u16)
        .clamp(50, 100)
        .min(area.width);
    let height = (18u16).min(area.height.saturating_sub(2)).max(6);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect {
        x,
        y,
        width,
        height,
    };

    let block = Block::default()
        .title(Span::styled(" File Finder ", overlay_title_style(palette)))
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(palette.border_focus)
                .add_modifier(Modifier::BOLD),
        )
        .style(elevated_surface_style(palette));
    let inner = block.inner(popup);
    render_modal_backdrop(frame, area, popup, palette);
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let input = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " ⌕  ",
                Style::default()
                    .fg(palette.accent_teal)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                state.query.clone(),
                Style::default()
                    .fg(palette.text_primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("│", Style::default().fg(palette.text_muted)),
        ]),
        Line::from(Span::styled(
            format!(" root: {} ", state.root.display()),
            Style::default().fg(palette.text_muted),
        )),
    ])
    .style(
        Style::default()
            .fg(palette.text_primary)
            .bg(palette.tools_bg),
    );
    frame.render_widget(input, chunks[0]);

    let visible_height = chunks[1].height as usize;
    let scroll_start = state
        .selection
        .saturating_sub(visible_height.saturating_sub(1));
    let items: Vec<ListItem> = if state.filtered.is_empty() {
        vec![ListItem::new(Span::styled(
            " no matches ",
            Style::default().fg(palette.text_muted),
        ))]
    } else {
        state
            .filtered
            .iter()
            .enumerate()
            .skip(scroll_start)
            .take(visible_height)
            .map(|(index, path)| {
                let rel = state.relative_display_path(path);
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                let is_selected = index == state.selection;
                let base = if is_selected {
                    Style::default()
                        .fg(palette.selection_fg)
                        .bg(palette.selection_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(palette.text_primary)
                };
                let dir_style = Style::default().fg(palette.text_muted);
                let query_lower = state.query.to_lowercase();

                let dir_part = if let Some(parent) = std::path::Path::new(&rel).parent() {
                    let s = parent.display().to_string();
                    if s.is_empty() || s == "." {
                        String::new()
                    } else {
                        format!("{}/", s)
                    }
                } else {
                    String::new()
                };
                let mut spans: Vec<Span> =
                    vec![Span::styled(" ", base), Span::styled(dir_part, dir_style)];
                if query_lower.is_empty() {
                    spans.push(Span::styled(
                        filename.to_string(),
                        base.add_modifier(Modifier::BOLD),
                    ));
                } else {
                    let mut rem = filename;
                    while !rem.is_empty() {
                        if let Some(pos) = rem.to_lowercase().find(&query_lower) {
                            if pos > 0 {
                                spans.push(Span::styled(rem[..pos].to_string(), base));
                            }
                            spans.push(Span::styled(
                                rem[pos..pos + query_lower.len()].to_string(),
                                finder_match_highlight_style(palette),
                            ));
                            rem = &rem[pos + query_lower.len()..];
                        } else {
                            spans.push(Span::styled(rem.to_string(), base));
                            break;
                        }
                    }
                }
                ListItem::new(Line::from(spans))
            })
            .collect()
    };
    frame.render_widget(
        List::new(items).style(elevated_surface_style(palette)),
        chunks[1],
    );

    let footer = Paragraph::new("  Enter open  ·  Ctrl+Enter open in editor  ·  Esc close")
        .style(overlay_footer_style(palette));
    frame.render_widget(footer, chunks[2]);
}
