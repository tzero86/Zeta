use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{layout::Rect, Frame};

use crate::action::MenuId;
use crate::config::ThemePalette;
use crate::state::AppState;

pub fn render_menu_bar(frame: &mut Frame<'_>, area: Rect, state: &AppState, palette: ThemePalette) {
    let active = state.active_menu().is_none();
    let top_bar_bg = if active {
        palette.menu_active_bg
    } else {
        palette.menu_bg
    };
    let mut line = Line::default();
    line.spans.extend(top_bar_logo_spans(active, palette));
    line.spans.extend(menu_spans(
        " File ",
        Some('F'),
        state.active_menu() == Some(MenuId::File),
        palette,
    ));
    line.spans.extend(menu_spans(
        " Navigate ",
        Some('N'),
        state.active_menu() == Some(MenuId::Navigate),
        palette,
    ));
    line.spans.extend(menu_spans(
        " View ",
        Some('V'),
        state.active_menu() == Some(MenuId::View),
        palette,
    ));
    line.spans.extend(menu_spans(
        " Help ",
        Some('H'),
        state.active_menu() == Some(MenuId::Help),
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
