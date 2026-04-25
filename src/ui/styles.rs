use ratatui::style::{Color, Modifier, Style};

use crate::config::ThemePalette;

pub fn elevated_surface_style(palette: ThemePalette) -> Style {
    Style::default().bg(palette.tools_bg)
}

pub fn modal_backdrop_style(_palette: ThemePalette) -> Style {
    Style::default().bg(Color::Rgb(36, 38, 42))
}

pub fn modal_halo_style(palette: ThemePalette) -> Style {
    Style::default().bg(palette.modal_halo)
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

pub fn dim_text_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.text_muted)
}

pub fn subtext_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.text_subtext)
}

pub fn accent_mauve_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.accent_mauve)
}

pub fn accent_teal_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.accent_teal)
}

pub fn accent_green_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.accent_green)
}

pub fn accent_yellow_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.accent_yellow)
}

pub fn accent_peach_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.accent_peach)
}

pub fn key_pill_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.accent_yellow)
        .bg(palette.modal_halo)
        .add_modifier(Modifier::BOLD)
}

pub fn section_divider_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.border_focus)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

pub fn panel_title_focused_style(accent: Color) -> Style {
    Style::default().fg(accent).add_modifier(Modifier::BOLD)
}

pub fn panel_title_unfocused_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.text_muted)
}

pub fn dirty_indicator_style(palette: ThemePalette) -> Style {
    Style::default().fg(palette.accent_peach)
}

pub fn pane_filter_strip_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.text_primary)
        .bg(palette.pane_filter_bg)
}

pub fn pane_column_header_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.text_muted)
        .add_modifier(Modifier::BOLD)
}

pub fn category_badge_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.surface_bg)
        .bg(palette.accent_teal)
        .add_modifier(Modifier::BOLD)
}

pub fn match_highlight_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.accent_yellow)
        .add_modifier(Modifier::BOLD)
}

pub fn finder_match_highlight_style(palette: ThemePalette) -> Style {
    Style::default()
        .fg(palette.accent_teal)
        .add_modifier(Modifier::BOLD)
}
