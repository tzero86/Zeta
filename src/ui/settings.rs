use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::state::{AppState, SettingsState};
use crate::ui::overlay::render_modal_backdrop;
use crate::ui::styles::{elevated_surface_style, overlay_footer_style, overlay_title_style};

pub fn render_settings_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    settings: &SettingsState,
    state: &AppState,
    palette: ThemePalette,
) {
    let entries = state.settings_entries();
    let width = ((area.width as f32 * 0.84) as u16)
        .clamp(64, 104)
        .min(area.width.saturating_sub(2).max(1));
    let height = (entries.len() as u16 + 8).min(area.height.saturating_sub(2).max(8));
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
    render_modal_backdrop(frame, area, popup_area, palette);
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

    let intro = Paragraph::new(
        "Settings affect the entire app. Enter/Space toggles • Esc closes • future keymap controls reserved",
    )
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
