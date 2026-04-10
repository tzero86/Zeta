use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::finder::FileFinderState;
use crate::ui::overlay::render_modal_backdrop;
use crate::ui::styles::{elevated_surface_style, overlay_footer_style, overlay_title_style};

pub fn render_file_finder(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &FileFinderState,
    palette: ThemePalette,
) {
    let width = ((area.width as f32 * 0.90) as u16).clamp(50, 100).min(area.width);
    let height = (18u16).min(area.height.saturating_sub(2)).max(6);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect { x, y, width, height };

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
        Line::from(format!("> {}_", state.query)),
        Line::from(Span::styled(
            format!("root: {}", state.root.display()),
            Style::default().fg(palette.text_muted),
        )),
    ])
    .style(Style::default().fg(palette.text_primary).bg(palette.tools_bg));
    frame.render_widget(input, chunks[0]);

    let visible_height = chunks[1].height as usize;
    let scroll_start = state.selection.saturating_sub(visible_height.saturating_sub(1));
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
                let mut spans = Vec::new();
                spans.push(Span::styled(" ", base));
                spans.push(Span::styled(rel.clone(), base));
                if rel != filename {
                    spans.push(Span::styled(
                        format!("  ({filename})"),
                        base.add_modifier(Modifier::BOLD),
                    ));
                }
                ListItem::new(Line::from(spans))
            })
            .collect()
    };
    frame.render_widget(List::new(items).style(elevated_surface_style(palette)), chunks[1]);

    let footer = Paragraph::new("Type to search • Enter to jump • Esc to close")
        .style(overlay_footer_style(palette));
    frame.render_widget(footer, chunks[2]);
}
