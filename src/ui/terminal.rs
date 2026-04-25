use crate::config::ThemePalette;
use crate::state::terminal::TerminalState;
use crate::ui::styles::{panel_title_focused_style, panel_title_unfocused_style};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{block::Title, Block, Borders};
use ratatui::Frame;

pub fn render_terminal(
    frame: &mut Frame<'_>,
    area: Rect,
    terminal: &TerminalState,
    palette: ThemePalette,
    focused: bool,
) {
    let accent = palette.accent_green;
    let title_style = if focused {
        panel_title_focused_style(accent)
    } else {
        panel_title_unfocused_style(palette)
    };
    let badge_style = Style::default()
        .fg(palette.surface_bg)
        .bg(accent)
        .add_modifier(Modifier::BOLD);
    let title_line = Line::from(vec![
        Span::styled(" \u{f489} Terminal ", title_style),
        Span::styled(" Shell ", badge_style),
    ]);
    let block = Block::default()
        .borders(Borders::TOP)
        .title(Title::from(title_line))
        .border_style(if focused {
            Style::default().fg(palette.border_focus)
        } else {
            Style::default().fg(palette.text_muted)
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Ok(parser) = terminal.parser.lock() {
        let screen = parser.screen();
        let (rows, cols) = screen.size();
        let (cursor_row, cursor_col) = screen.cursor_position();

        // Calculate vertical scroll offset to keep cursor visible
        let scroll_top = if cursor_row < inner.height {
            0
        } else {
            cursor_row.saturating_sub(inner.height.saturating_sub(1))
        };

        for row in 0..rows {
            let visible_row = (row as i32) - (scroll_top as i32);
            if visible_row < 0 {
                continue;
            }
            if visible_row >= inner.height as i32 {
                break;
            }

            let y = inner.y + visible_row as u16;

            for col in 0..cols {
                if col >= inner.width {
                    break;
                }
                let cell = screen.cell(row, col).unwrap();
                let x = inner.x + col;

                let mut style = Style::default();

                // Foreground color
                if let Some(c) = map_vt100_color(cell.fgcolor()) {
                    style = style.fg(c);
                }
                // Background color
                if let Some(c) = map_vt100_color(cell.bgcolor()) {
                    style = style.bg(c);
                }
                // Modifiers
                if cell.bold() {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if cell.italic() {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if cell.underline() {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }

                if let Some(c) = frame.buffer_mut().cell_mut((x, y)) {
                    let contents = cell.contents();
                    c.set_symbol(if contents.is_empty() { " " } else { &contents })
                        .set_style(style);
                }
            }
        }

        // Render cursor
        if focused {
            let visible_cursor_row = (cursor_row as i32) - (scroll_top as i32);
            if visible_cursor_row >= 0
                && (visible_cursor_row as u16) < inner.height
                && cursor_col < inner.width
            {
                let x = inner.x + cursor_col;
                let y = inner.y + visible_cursor_row as u16;
                if let Some(c) = frame.buffer_mut().cell_mut((x, y)) {
                    c.set_style(Style::default().add_modifier(Modifier::REVERSED));
                }
            }
        }

        // Diagnostic overlay if no bytes received
        if terminal.bytes_received == 0 {
            let msg = " [ Waiting for shell output... ] ";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            if x < inner.x + inner.width && y < inner.y + inner.height {
                frame
                    .buffer_mut()
                    .set_string(x, y, msg, Style::default().fg(palette.text_muted));
            }
        }
    }
}

fn map_vt100_color(color: vt100::Color) -> Option<Color> {
    match color {
        vt100::Color::Default => None,
        vt100::Color::Idx(i) => Some(Color::Indexed(i)),
        vt100::Color::Rgb(r, g, b) => Some(Color::Rgb(r, g, b)),
    }
}
