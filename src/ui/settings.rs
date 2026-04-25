use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::state::{AppState, SettingsField, SettingsState, SettingsTab};
use crate::ui::overlay::render_modal_backdrop;
use crate::ui::styles::{elevated_surface_style, overlay_footer_style, overlay_title_style};

pub fn render_settings_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    settings: &SettingsState,
    state: &AppState,
    palette: ThemePalette,
) {
    let entries = state.settings_entries_for_tab(settings.active_tab);
    let width = ((area.width as f32 * 0.84) as u16)
        .clamp(64, 104)
        .min(area.width.saturating_sub(2).max(1));
    let height = (entries.len() as u16 + 10).min(area.height.saturating_sub(2).max(10));
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
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(inner);

    // Tab bar
    let tab_bar_spans: Vec<ratatui::text::Span> = [
        SettingsTab::Appearance,
        SettingsTab::Panels,
        SettingsTab::Editor,
        SettingsTab::Keymaps,
    ]
    .iter()
    .flat_map(|&tab| {
        let is_active = tab == settings.active_tab;
        let style = if is_active {
            ratatui::style::Style::default()
                .fg(palette.selection_fg)
                .bg(palette.border_focus)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            ratatui::style::Style::default()
                .fg(palette.text_muted)
                .bg(palette.tools_bg)
        };
        vec![
            ratatui::text::Span::styled(format!(" {} ", tab.label()), style),
            ratatui::text::Span::styled(
                "  ",
                ratatui::style::Style::default().bg(palette.tools_bg),
            ),
        ]
    })
    .collect();
    frame.render_widget(
        ratatui::widgets::Paragraph::new(ratatui::text::Line::from(tab_bar_spans)),
        chunks[0],
    );

    let rows: Vec<ListItem<'_>> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let is_selected = index == settings.selection;
            let is_rebinding = settings.rebind_mode == Some(index);

            let base_style = if is_selected {
                Style::default()
                    .fg(palette.selection_fg)
                    .bg(palette.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_primary)
            };

            // Rebindable entries get a small indicator.
            let rebind_marker = matches!(entry.field, SettingsField::KeymapBinding { .. });

            let value_display = if is_rebinding {
                String::from("[ press new key... ]")
            } else {
                entry.value.clone()
            };

            let hint_text = if rebind_marker { "Enter" } else { entry.hint };

            let line = Line::from(vec![
                Span::styled(format!(" {:<26}", entry.label), base_style),
                Span::styled(
                    value_display,
                    Style::default().fg(if is_rebinding {
                        palette.logo_accent
                    } else {
                        palette.key_hint_fg
                    }),
                ),
                Span::raw(format!("  {hint_text}")),
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

    let footer = Paragraph::new(
        " Ctrl+O opens settings  \u{2022}  Changes are saved immediately to config.toml",
    )
    .style(overlay_footer_style(palette));
    frame.render_widget(footer, chunks[2]);
}
