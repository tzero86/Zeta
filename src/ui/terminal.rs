use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;
use crate::config::ThemePalette;
use crate::state::terminal::TerminalState;

pub fn render_terminal(
    frame: &mut Frame<'_>,
    area: Rect,
    terminal: &TerminalState,
    palette: ThemePalette,
    focused: bool,
) {
    let block = Block::default()
        .borders(Borders::TOP)
        .title(" Terminal ")
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

        for row in 0..rows {
            if row as u16 >= inner.height {
                break;
            }
            for col in 0..cols {
                if col as u16 >= inner.width {
                    break;
                }
                let cell = screen.cell(row, col).unwrap();
                let x = inner.x + col as u16;
                let y = inner.y + row as u16;

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

                frame.buffer_mut().cell_mut((x, y)).map(|c| {
                    c.set_char(cell.contents().chars().next().unwrap_or(' ')).set_style(style);
                });
            }
        }
        
        // Render cursor
        if focused {
            let (cursor_row, cursor_col) = screen.cursor_position();
            if (cursor_row as u16) < inner.height && (cursor_col as u16) < inner.width {
                let x = inner.x + cursor_col as u16;
                let y = inner.y + cursor_row as u16;
                frame.buffer_mut().cell_mut((x, y)).map(|c| {
                    c.set_style(Style::default().add_modifier(Modifier::REVERSED));
                });
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
