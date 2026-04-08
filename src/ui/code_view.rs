use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthChar;

use crate::config::ThemePalette;
use crate::highlight::HighlightedLine;

pub struct CodeViewRenderArgs<'a> {
    pub lines: &'a [HighlightedLine],
    pub first_line_number: usize,
    pub gutter_width: u16,
    pub scroll_col: usize,
    pub cursor_row: Option<usize>,
    pub palette: ThemePalette,
}

pub fn render_code_view(frame: &mut Frame<'_>, area: Rect, args: CodeViewRenderArgs<'_>) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(args.gutter_width), Constraint::Min(1)])
        .split(area);
    let gutter_area = chunks[0];
    let content_area = chunks[1];
    let viewport_cols = content_area.width as usize;

    let blank_style = Style::default().bg(args.palette.surface_bg);

    for row_idx in 0..area.height as usize {
        let y = area.y + row_idx as u16;
        if y >= area.y + area.height {
            break;
        }
        let gutter_rect = Rect {
            x: gutter_area.x,
            y,
            width: gutter_area.width,
            height: 1,
        };
        let content_rect = Rect {
            x: content_area.x,
            y,
            width: content_area.width,
            height: 1,
        };
        frame.render_widget(Paragraph::new(" ").style(blank_style), gutter_rect);
        frame.render_widget(Paragraph::new(" ").style(blank_style), content_rect);
    }

    for (row_idx, line_tokens) in args.lines.iter().enumerate() {
        let y = area.y + row_idx as u16;
        if y >= area.y + area.height {
            break;
        }

        let line_num = args.first_line_number + row_idx;
        let gutter_text = format!(
            "{:>width$} ",
            line_num,
            width = (args.gutter_width as usize).saturating_sub(2)
        );
        let gutter_rect = Rect {
            x: gutter_area.x,
            y,
            width: gutter_area.width,
            height: 1,
        };
        let gutter_style = Style::default()
            .fg(args.palette.text_muted)
            .bg(args.palette.surface_bg);
        frame.render_widget(Paragraph::new(gutter_text).style(gutter_style), gutter_rect);

        let content_rect = Rect {
            x: content_area.x,
            y,
            width: content_area.width,
            height: 1,
        };
        let row_bg = if args.cursor_row == Some(row_idx) {
            Style::default().bg(args.palette.selection_bg)
        } else {
            Style::default().bg(args.palette.surface_bg)
        };

        let mut spans: Vec<Span> = Vec::new();
        let mut raw_cols = 0usize;
        let mut visible_cols = 0usize;
        for (color, modifier, text) in line_tokens {
            let token_chars: Vec<char> = text.chars().collect();
            let token_start = raw_cols;
            let token_width = token_chars
                .iter()
                .map(|ch| UnicodeWidthChar::width(*ch).unwrap_or(0))
                .sum::<usize>();
            let token_end = raw_cols + token_width;
            raw_cols = token_end;

            if token_end <= args.scroll_col {
                continue;
            }

            let skip = args.scroll_col.saturating_sub(token_start);
            let mut visible_chars = String::new();
            let mut skipped = 0usize;
            let mut used_width = 0usize;
            for ch in token_chars.iter().skip_while(|_| {
                if skipped < skip {
                    skipped += 1;
                    true
                } else {
                    false
                }
            }) {
                let ch_width = UnicodeWidthChar::width(*ch).unwrap_or(0);
                if used_width + ch_width > viewport_cols.saturating_sub(visible_cols) {
                    break;
                }
                used_width += ch_width;
                visible_chars.push(*ch);
            }
            if !visible_chars.is_empty() {
                visible_cols += used_width;
                spans.push(Span::styled(
                    visible_chars,
                    Style::default().fg(*color).add_modifier(*modifier),
                ));
            }
        }

        if visible_cols < viewport_cols {
            spans.push(Span::raw(" ".repeat(viewport_cols - visible_cols)));
        }

        let line = Line::from(spans);
        frame.render_widget(Paragraph::new(line).style(row_bg), content_rect);
    }
}
