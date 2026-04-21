use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthChar;

use crate::config::ThemePalette;
use crate::highlight::HighlightedLine;

/// Search match ranges for a single call to `render_code_view`.
pub struct SearchHighlight<'a> {
    /// Per visual-row char-column ranges to highlight as search matches.
    /// The outer slice is indexed by visual row; inner vecs are `(col_start, col_end)`.
    pub row_matches: &'a [Vec<(usize, usize)>],
    /// The active match expressed as `(row_idx, col_start, col_end)`, if visible.
    pub active_row_match: Option<(usize, usize, usize)>,
}

/// Selection highlight data for a single call to `render_code_view`.
pub struct SelectionHighlight<'a> {
    /// Per visual-row optional display-char range `(col_start, col_end)` that is selected.
    pub row_ranges: &'a [Option<(usize, usize)>],
    /// Background color for selected text.
    pub bg: Color,
}

pub struct CodeViewRenderArgs<'a> {
    pub lines: &'a [HighlightedLine],
    pub first_line_number: usize,
    pub gutter_width: u16,
    pub scroll_col: usize,
    pub cursor_row: Option<usize>,
    pub palette: ThemePalette,
    /// Optional search highlight data; `None` when search is inactive.
    pub search: Option<SearchHighlight<'a>>,
    /// Optional text selection highlight data; `None` when no selection.
    pub selection: Option<SelectionHighlight<'a>>,
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

        // Gather search match ranges for this row, if any.
        let row_match_ranges: &[_] = args
            .search
            .as_ref()
            .and_then(|s| s.row_matches.get(row_idx))
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let active_col_range: Option<(usize, usize)> = args.search.as_ref().and_then(|s| {
            let (ar, cs, ce) = s.active_row_match?;
            (ar == row_idx).then_some((cs, ce))
        });
        let row_sel_range: Option<(usize, usize)> = args
            .selection
            .as_ref()
            .and_then(|s| s.row_ranges.get(row_idx).copied().flatten());
        let sel_bg = args.selection.as_ref().map(|s| s.bg);

        let spans = build_row_spans(
            line_tokens,
            args.scroll_col,
            viewport_cols,
            row_match_ranges,
            active_col_range,
            args.palette.search_match_bg,
            args.palette.search_match_active_bg,
            row_sel_range,
            sel_bg.unwrap_or(args.palette.surface_bg),
        );

        let line = Line::from(spans);
        frame.render_widget(Paragraph::new(line).style(row_bg), content_rect);
    }
}

/// Build the visible `Span` list for a single row, applying scroll offset, viewport
/// clipping, and (optionally) search match and selection background highlights.
///
/// `match_ranges` and `active_col_range` are expressed as char-column offsets within
/// the row (not display-width offsets), because `find_matches` works in char space.
#[allow(clippy::too_many_arguments)]
fn build_row_spans(
    tokens: &HighlightedLine,
    scroll_col: usize,
    viewport_cols: usize,
    match_ranges: &[(usize, usize)],
    active_col_range: Option<(usize, usize)>,
    match_bg: Color,
    active_match_bg: Color,
    sel_range: Option<(usize, usize)>,
    sel_bg: Color,
) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    // `raw_cols`: display-width position (used for scroll / viewport clipping).
    // `char_col`: char count (used for search range lookup).
    let mut raw_cols = 0usize;
    let mut char_col = 0usize;
    let mut visible_cols = 0usize;

    for (fg_color, modifier, text) in tokens {
        // Compute char count and display width in one pass — no Vec<char> allocation.
        let (token_char_len, token_display_width) =
            text.chars().fold((0usize, 0usize), |(nc, dw), ch| {
                (nc + 1, dw + UnicodeWidthChar::width(ch).unwrap_or(0))
            });
        let token_end_display = raw_cols + token_display_width;

        if token_end_display <= scroll_col {
            raw_cols = token_end_display;
            char_col += token_char_len;
            continue;
        }

        // Walk the characters of this token individually so we can split at
        // search-match boundaries while respecting scroll and viewport limits.
        let skip_display = scroll_col.saturating_sub(raw_cols);
        let mut skipped_display = 0usize;
        let mut pending_text = String::new();
        let mut pending_bg: Option<Color> = None;

        for ch in text.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);

            // Skip chars still to the left of the scroll window (display-width skip).
            if skipped_display < skip_display {
                skipped_display += ch_width;
                raw_cols += ch_width;
                char_col += 1;
                continue;
            }

            // Viewport right-edge clip.
            if visible_cols >= viewport_cols {
                break;
            }

            // Determine the appropriate background override for this char.
            let char_bg = char_highlight_bg(
                char_col,
                sel_range,
                sel_bg,
                match_ranges,
                active_col_range,
                match_bg,
                active_match_bg,
            );

            // When the highlight state changes, flush the pending span.
            if char_bg != pending_bg && !pending_text.is_empty() {
                spans.push(styled_span(
                    std::mem::take(&mut pending_text),
                    *fg_color,
                    *modifier,
                    pending_bg,
                ));
            }
            pending_bg = char_bg;
            pending_text.push(ch);
            visible_cols += ch_width;
            raw_cols += ch_width;
            char_col += 1;
        }

        if !pending_text.is_empty() {
            spans.push(styled_span(
                std::mem::take(&mut pending_text),
                *fg_color,
                *modifier,
                pending_bg,
            ));
        }
    }

    if visible_cols < viewport_cols {
        spans.push(Span::raw(" ".repeat(viewport_cols - visible_cols)));
    }
    spans
}

/// Return the background override for the char at `char_col`, or `None` to use
/// the row default. Active match takes precedence over non-active match.
#[inline]
fn char_highlight_bg(
    char_col: usize,
    sel_range: Option<(usize, usize)>,
    sel_bg: Color,
    ranges: &[(usize, usize)],
    active: Option<(usize, usize)>,
    match_bg: Color,
    active_bg: Color,
) -> Option<Color> {
    // Active search match: highest priority.
    if let Some((s, e)) = active {
        if char_col >= s && char_col < e {
            return Some(active_bg);
        }
    }
    // Regular search match.
    for &(s, e) in ranges {
        if char_col >= s && char_col < e {
            return Some(match_bg);
        }
    }
    // Text selection: lowest priority.
    if let Some((s, e)) = sel_range {
        if char_col >= s && char_col < e {
            return Some(sel_bg);
        }
    }
    None
}

#[inline]
fn styled_span(
    text: String,
    fg: ratatui::style::Color,
    modifier: ratatui::style::Modifier,
    bg: Option<ratatui::style::Color>,
) -> Span<'static> {
    let mut style = Style::default().fg(fg).add_modifier(modifier);
    if let Some(bg_color) = bg {
        style = style.bg(bg_color);
    }
    Span::styled(text, style)
}
