use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::state::BookmarksState;
use crate::ui::overlay::render_modal_backdrop;
use crate::ui::styles::{elevated_surface_style, overlay_footer_style, overlay_title_style};

pub fn render_bookmarks_modal(
    frame: &mut Frame<'_>,
    area: Rect,
    bookmarks: &BookmarksState,
    paths: &[std::path::PathBuf],
    palette: ThemePalette,
) {
    let width = ((area.width as f32 * 0.82) as u16)
        .clamp(56, 96)
        .min(area.width.saturating_sub(2).max(1));
    let height = (paths.len() as u16 + 6)
        .clamp(8, 18)
        .min(area.height.saturating_sub(2).max(1));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect { x, y, width, height };

    let block = Block::default()
        .title(Span::styled(" Bookmarks ", overlay_title_style(palette)))
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
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let intro = Paragraph::new("Saved locations for fast navigation")
        .style(overlay_footer_style(palette));
    frame.render_widget(intro, chunks[0]);

    let rows: Vec<ListItem<'_>> = if paths.is_empty() {
        vec![ListItem::new(Span::styled(
            " no bookmarks yet ",
            Style::default().fg(palette.text_muted),
        ))]
    } else {
        paths.iter()
            .enumerate()
            .map(|(index, path)| {
                let selected = index == bookmarks.selection;
                let base_style = if selected {
                    Style::default()
                        .fg(palette.selection_fg)
                        .bg(palette.selection_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(palette.text_primary)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {:>2}. ", index + 1), base_style),
                    Span::styled(path.display().to_string(), base_style),
                ]))
            })
            .collect()
    };

    let mut list_state = ListState::default();
    if !paths.is_empty() {
        list_state.select(Some(bookmarks.selection.min(paths.len().saturating_sub(1))));
    }
    frame.render_stateful_widget(
        List::new(rows).style(elevated_surface_style(palette)),
        chunks[1],
        &mut list_state,
    );

    let footer = Paragraph::new("Enter=navigate  Del=remove  Esc=close")
        .style(overlay_footer_style(palette));
    frame.render_widget(footer, chunks[2]);
}
