use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{block::Title, Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use unicode_width::UnicodeWidthChar;

use crate::config::ThemePalette;
use crate::highlight::HighlightedLine;
use crate::preview::ViewBuffer;
use crate::ui::styles::{
    pane_column_header_style, panel_title_focused_style, panel_title_unfocused_style,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedPreviewRow {
    pub line_number: usize,
    pub line_tokens: HighlightedLine,
}

const PREVIEW_GUTTER_WIDTH: u16 = 4;

pub fn wrap_preview_line(
    line_number: usize,
    line_tokens: &HighlightedLine,
    viewport_cols: usize,
) -> Vec<WrappedPreviewRow> {
    if viewport_cols == 0 {
        return vec![];
    }

    let mut rows: Vec<WrappedPreviewRow> = Vec::new();
    let mut current_row: HighlightedLine = Vec::new();
    let mut current_width = 0usize;

    for (color, modifier, text) in line_tokens {
        let mut chunk = String::new();

        for ch in text.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1).max(1);

            if current_width > 0 && current_width + ch_width > viewport_cols {
                if !chunk.is_empty() {
                    current_row.push((
                        *color,
                        *modifier,
                        std::mem::take(&mut chunk).into_boxed_str(),
                    ));
                }
                rows.push(WrappedPreviewRow {
                    line_number,
                    line_tokens: std::mem::take(&mut current_row),
                });
                current_width = 0;
            }

            chunk.push(ch);
            current_width += ch_width;

            if current_width >= viewport_cols {
                current_row.push((
                    *color,
                    *modifier,
                    std::mem::take(&mut chunk).into_boxed_str(),
                ));
                rows.push(WrappedPreviewRow {
                    line_number,
                    line_tokens: std::mem::take(&mut current_row),
                });
                current_width = 0;
            }
        }

        if !chunk.is_empty() {
            current_row.push((*color, *modifier, chunk.into_boxed_str()));
        }
    }

    if !current_row.is_empty() {
        rows.push(WrappedPreviewRow {
            line_number,
            line_tokens: current_row,
        });
    }

    if rows.is_empty() {
        rows.push(WrappedPreviewRow {
            line_number,
            line_tokens: vec![],
        });
    }

    rows
}

pub fn preview_gutter_label(line_number: usize, is_continuation: bool) -> String {
    if is_continuation {
        " ".repeat(PREVIEW_GUTTER_WIDTH as usize)
    } else {
        format!("{:>3} ", line_number)
    }
}

pub fn render_wrapped_preview_view(
    frame: &mut Frame<'_>,
    area: Rect,
    lines: &[HighlightedLine],
    first_line_number: usize,
    palette: ThemePalette,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(PREVIEW_GUTTER_WIDTH), Constraint::Min(1)])
        .split(area);
    let gutter_area = chunks[0];
    let content_area = chunks[1];

    let blank_style = Style::default().bg(palette.tools_bg);
    for row_idx in 0..area.height as usize {
        let y = area.y + row_idx as u16;
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

    let mut visual_row = 0usize;
    for (source_idx, line_tokens) in lines.iter().enumerate() {
        let wrapped_rows = wrap_preview_line(
            first_line_number + source_idx,
            line_tokens,
            content_area.width as usize,
        );
        for (wrap_idx, row) in wrapped_rows.into_iter().enumerate() {
            let y = area.y + visual_row as u16;
            if y >= area.y + area.height {
                return;
            }

            let gutter_text = preview_gutter_label(row.line_number, wrap_idx > 0);
            let gutter_rect = Rect {
                x: gutter_area.x,
                y,
                width: gutter_area.width,
                height: 1,
            };
            frame.render_widget(
                Paragraph::new(gutter_text).style(
                    Style::default()
                        .fg(palette.text_muted)
                        .bg(palette.surface_bg),
                ),
                gutter_rect,
            );

            let content_rect = Rect {
                x: content_area.x,
                y,
                width: content_area.width,
                height: 1,
            };
            let spans: Vec<Span> = row
                .line_tokens
                .iter()
                .map(|(color, modifier, text)| {
                    Span::styled(
                        text.as_ref(),
                        Style::default().fg(*color).add_modifier(*modifier),
                    )
                })
                .collect();
            frame.render_widget(
                Paragraph::new(Line::from(spans)).style(Style::default().bg(palette.surface_bg)),
                content_rect,
            );
            visual_row += 1;
        }
    }
}

pub struct RenderPreviewArgs<'a> {
    pub view: Option<&'a ViewBuffer>,
    pub filename: &'a str,
    pub is_focused: bool,
    pub palette: ThemePalette,
    pub cheap_mode: bool,
}

pub fn render_preview_panel(frame: &mut Frame<'_>, area: Rect, args: RenderPreviewArgs<'_>) {
    let RenderPreviewArgs {
        view,
        filename,
        is_focused,
        palette,
        cheap_mode,
    } = args;
    let border_style = if is_focused {
        Style::default()
            .fg(palette.border_focus)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_muted)
    };
    let accent = palette.accent_teal;
    let title_style = if is_focused {
        panel_title_focused_style(accent)
    } else {
        panel_title_unfocused_style(palette)
    };
    let badge_style = Style::default()
        .fg(palette.surface_bg)
        .bg(accent)
        .add_modifier(Modifier::BOLD);

    // Extract extension from filename
    let ext_hint = std::path::Path::new(filename)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();

    let title_line = Line::from(vec![
        Span::styled(format!(" \u{f02d5} {} ", filename), title_style),
        Span::styled(
            if ext_hint.is_empty() {
                String::new()
            } else {
                format!(" .{} ", ext_hint)
            },
            pane_column_header_style(palette),
        ),
        Span::styled(" Preview ", badge_style),
    ]);
    let block = Block::default()
        .title(Title::from(title_line))
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(palette.tools_bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    match view {
        None => frame.render_widget(
            Paragraph::new("select a file to preview")
                .style(Style::default().fg(palette.text_muted).bg(palette.tools_bg)),
            inner,
        ),
        Some(v) => {
            if cheap_mode {
                if v.is_markdown() {
                    if let Some(source) = v.markdown_source() {
                        let text: String = source
                            .lines()
                            .take(inner.height as usize)
                            .collect::<Vec<_>>()
                            .join("\n");
                        frame.render_widget(
                            Paragraph::new(text).style(Style::default().bg(palette.tools_bg)),
                            inner,
                        );
                    }
                } else {
                    let height = inner.height as usize;
                    let (_, window) = v.visible_window(height);
                    let text = window
                        .iter()
                        .map(|line| {
                            line.iter()
                                .map(|(_, _, text)| text.as_ref())
                                .collect::<String>()
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    frame.render_widget(
                        Paragraph::new(text).style(Style::default().bg(palette.tools_bg)),
                        inner,
                    );
                }
            } else if v.is_markdown() {
                if let Some(source) = v.markdown_source() {
                    let widget = Paragraph::new(source)
                        .style(Style::default().bg(palette.tools_bg))
                        .wrap(Wrap { trim: false });
                    frame.render_widget(widget, inner);
                }
            } else {
                let height = inner.height as usize;
                let (first_line_num, window) = v.visible_window(height);
                if window.is_empty() {
                    return;
                }
                render_wrapped_preview_view(frame, inner, window, first_line_num + 1, palette);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::preview_gutter_label;

    #[test]
    fn preview_gutter_label_uses_four_columns_for_line_numbers() {
        let label = preview_gutter_label(7, false);
        assert_eq!(label, "  7 ");
        assert_eq!(label.chars().count(), 4);
    }

    #[test]
    fn preview_gutter_label_uses_four_columns_for_wrapped_rows() {
        let label = preview_gutter_label(7, true);
        assert_eq!(label, "    ");
        assert_eq!(label.chars().count(), 4);
    }
}
