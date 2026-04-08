use ratatui::style::{Modifier, Style};

use crate::config::ThemePalette;

pub fn elevated_surface_style(palette: ThemePalette) -> Style {
    Style::default().bg(palette.tools_bg)
}

pub fn overlay_title_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.menu_mnemonic_fg)
        .add_modifier(Modifier::BOLD)
}

pub fn overlay_key_hint_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.key_hint_fg)
        .add_modifier(Modifier::BOLD)
}

pub fn overlay_footer_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.text_muted)
}

pub fn command_palette_header_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.text_muted)
        .add_modifier(Modifier::BOLD)
}

pub fn command_palette_entry_label_style(is_selected: bool, palette: ThemePalette) -> Style {
    if is_selected {
        Style::default()
            .fg(palette.selection_fg)
            .bg(palette.selection_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_primary)
    }
}

pub fn command_palette_entry_hint_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.key_hint_fg)
}
