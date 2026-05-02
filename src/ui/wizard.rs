use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Widget,
};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::state::wizard::{WizardState, WizardStep, WIZARD_THEMES};

const CHEATSHEET: &[(&str, &str)] = &[
    ("↑ / ↓", "Navigate files"),
    ("Enter", "Open file / enter directory"),
    ("Backspace", "Go up one directory"),
    ("Tab", "Switch pane focus"),
    ("Space", "Toggle mark on file"),
    ("F1", "Help dialog"),
    ("F2", "Toggle embedded terminal"),
    ("F3", "Toggle preview panel"),
    ("F4", "Open file in editor"),
    ("F5", "Copy selected files"),
    ("F6", "Rename"),
    ("Shift+F6", "Move"),
    ("F7", "New directory"),
    ("F8", "Move to trash"),
    ("Shift+F8", "Permanently delete"),
    ("F9", "Toggle diff mode"),
    ("F10 / q", "Quit"),
    ("Ctrl+P", "Command palette"),
    ("Ctrl+F", "Find files"),
    ("F11", "Toggle editor fullscreen"),
    ("F12", "Debug panel"),
    ("Shift+M", "Clear marks"),
    ("m", "Add bookmark (pane context)"),
    ("F12 / Ctrl+,", "Settings panel"),
];

struct Dim;

impl Widget for Dim {
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

pub fn render_first_run_wizard(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &WizardState,
    palette: &ThemePalette,
) {
    frame.render_widget(Dim, area);

    let width = (area.width * 6 / 10)
        .max(60)
        .min(area.width.saturating_sub(4));
    let height = (area.height * 8 / 10)
        .max(20)
        .min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal = Rect {
        x,
        y,
        width,
        height,
    };

    frame.render_widget(Clear, modal);

    match state.step {
        WizardStep::ThemePicker => render_theme_picker(frame, modal, state, palette),
        WizardStep::Cheatsheet => render_cheatsheet(frame, modal, state, palette),
    }
}

fn render_theme_picker(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &WizardState,
    palette: &ThemePalette,
) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            " 🎨 Welcome to Zeta — Choose a Theme ",
            Style::default()
                .fg(palette.text_primary)
                .add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette.border_focus))
        .style(Style::default().bg(palette.surface_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(2)])
        .split(inner);

    let items: Vec<ListItem> = WIZARD_THEMES
        .iter()
        .map(|(label, _)| {
            ListItem::new(format!("  {label}  ")).style(Style::default().fg(palette.text_primary))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.theme_selection));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(palette.selection_bg)
                .fg(palette.selection_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, chunks[0], &mut list_state);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("  ↑/↓ ", Style::default().fg(palette.key_hint_fg)),
        Span::styled("select   ", Style::default().fg(palette.text_muted)),
        Span::styled("Enter ", Style::default().fg(palette.key_hint_fg)),
        Span::styled("confirm   ", Style::default().fg(palette.text_muted)),
        Span::styled("Esc ", Style::default().fg(palette.key_hint_fg)),
        Span::styled("skip", Style::default().fg(palette.text_muted)),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[1]);
}

fn render_cheatsheet(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &WizardState,
    palette: &ThemePalette,
) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            " ⌨  Keyboard Reference ",
            Style::default()
                .fg(palette.text_primary)
                .add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette.border_focus))
        .style(Style::default().bg(palette.surface_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(2)])
        .split(inner);

    let visible_height = chunks[0].height as usize;
    let start = state
        .cheatsheet_scroll
        .min(CHEATSHEET.len().saturating_sub(visible_height));
    let rows: Vec<ListItem> = CHEATSHEET
        .iter()
        .skip(start)
        .take(visible_height)
        .map(|(key, desc)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("  {key:<14}"),
                    Style::default().fg(palette.key_hint_fg),
                ),
                Span::styled(desc.to_string(), Style::default().fg(palette.text_primary)),
            ]))
        })
        .collect();

    let list = List::new(rows);
    frame.render_widget(list, chunks[0]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("  ↑/↓ ", Style::default().fg(palette.key_hint_fg)),
        Span::styled("scroll   ", Style::default().fg(palette.text_muted)),
        Span::styled("Enter / Esc ", Style::default().fg(palette.key_hint_fg)),
        Span::styled("start using Zeta", Style::default().fg(palette.text_muted)),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[1]);
}
