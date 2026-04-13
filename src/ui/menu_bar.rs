use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{layout::Rect, Frame};

use crate::config::ThemePalette;
use crate::state::{menu_tabs, AppState};

pub fn render_menu_bar(frame: &mut Frame<'_>, area: Rect, state: &AppState, palette: ThemePalette) {
    let active = state.active_menu().is_none();
    let top_bar_bg = if active {
        palette.menu_active_bg
    } else {
        palette.menu_bg
    };
    let mut line = Line::default();
    line.spans.extend(top_bar_logo_spans(active, palette));
    for tab in menu_tabs(state.is_editor_fullscreen() && state.editor().is_some()) {
        line.spans.extend(menu_spans(
            tab.label,
            Some(tab.mnemonic),
            state.active_menu() == Some(tab.id),
            palette,
        ));
    }
    line.spans.extend(workspace_switcher_spans(
        state.active_workspace_index(),
        state.workspace_count(),
        active,
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

pub fn top_bar_logo_spans(active: bool, palette: ThemePalette) -> Vec<Span<'static>> {
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

pub fn workspace_switcher_spans(
    active_workspace: usize,
    workspace_count: usize,
    bar_active: bool,
    palette: ThemePalette,
) -> Vec<Span<'static>> {
    let bg = if bar_active {
        palette.menu_active_bg
    } else {
        palette.menu_bg
    };
    let inactive_style = Style::default()
        .fg(palette.menu_fg)
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    let active_style = Style::default()
        .fg(palette.selection_fg)
        .bg(palette.selection_bg)
        .add_modifier(Modifier::BOLD);

    let mut spans = Vec::with_capacity(workspace_count.saturating_mul(2) + 1);
    spans.push(Span::styled(" ", Style::default().bg(bg)));
    for idx in 0..workspace_count {
        let style = if idx == active_workspace {
            active_style
        } else {
            inactive_style
        };
        spans.push(Span::styled(format!("[{}]", idx + 1), style));
        spans.push(Span::styled(" ", Style::default().bg(bg)));
    }
    spans
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
