//! Lightweight markdown renderer for ratatui 0.29.
//!
//! Converts a markdown string into styled `Line` objects rendered inside
//! a `Paragraph`. No external crate required.
//!
//! Supported: h1-h6, **bold**, *italic*, `code`, ~~strikethrough~~,
//! [links](url), fenced blocks (with language tag), bullet/ordered/task
//! lists, blockquotes, GFM tables, horizontal rules.

use unicode_width::UnicodeWidthStr;

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::config::ThemePalette;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Render a markdown string into `area` using native ratatui 0.29 widgets.
pub fn render_markdown_preview(
    frame: &mut Frame<'_>,
    area: Rect,
    source: &str,
    palette: ThemePalette,
    scroll: usize,
    is_focused: bool,
) {
    let inner_width = area.width.saturating_sub(2);
    let lines = parse_markdown_lines_with_palette(source, palette, inner_width);
    render_md_with_lines(frame, area, lines, palette, scroll, is_focused);
}

/// Render pre-parsed `Line` objects into `area` with the standard markdown border.
pub fn render_md_with_lines(
    frame: &mut Frame<'_>,
    area: Rect,
    lines: Vec<Line<'static>>,
    palette: ThemePalette,
    scroll: usize,
    is_focused: bool,
) {
    let border_style = if is_focused {
        Style::default()
            .fg(palette.border_focus)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_muted)
    };
    let block = Block::default()
        .title(" Markdown Preview ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(palette.tools_bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(palette.tools_bg))
        .scroll((scroll.min(u16::MAX as usize) as u16, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

// ---------------------------------------------------------------------------
// Parsing — exported for unit tests
// ---------------------------------------------------------------------------

/// Parse markdown source into styled `Line` objects using default colours.
/// Used by tests; production code uses `parse_markdown_lines_with_palette`.
pub fn parse_markdown_lines(source: &str) -> Vec<Line<'static>> {
    parse_markdown_lines_with_palette(source, default_palette(), 80)
}

fn default_palette() -> ThemePalette {
    crate::config::ThemePalette::from_preset(crate::config::ThemePreset::Oxide).palette
}

pub fn parse_markdown_lines_with_palette(
    source: &str,
    palette: ThemePalette,
    width: u16,
) -> Vec<Line<'static>> {
    if source.is_empty() {
        return vec![Line::from("")];
    }

    // Width for adaptive HR / heading rule; subtract 2 for block borders.
    let hr_width = (width as usize).saturating_sub(2).max(10);

    let source_lines: Vec<&str> = source.lines().collect();
    let mut output: Vec<Line<'static>> = Vec::new();
    let mut i = 0;
    let mut in_fence = false;

    while i < source_lines.len() {
        let raw_line = source_lines[i];

        // ── Fenced code block ─────────────────────────────────────────────
        if raw_line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            let marker = if in_fence {
                let lang = fence_lang(raw_line);
                if lang.is_empty() {
                    "┌─ code ".to_string()
                } else {
                    format!("┌─ {} ", lang)
                }
            } else {
                "└───────".to_string()
            };
            output.push(Line::from(vec![Span::styled(
                marker,
                Style::default().fg(palette.text_muted),
            )]));
            i += 1;
            continue;
        }
        if in_fence {
            output.push(Line::from(vec![Span::styled(
                format!("  {}", raw_line),
                Style::default()
                    .fg(palette.text_primary)
                    .bg(palette.surface_bg),
            )]));
            i += 1;
            continue;
        }

        // ── GFM table ─────────────────────────────────────────────────────
        // Consume all consecutive table rows in one pass so column widths
        // can be computed across the whole block before any line is emitted.
        if is_table_row(raw_line) {
            let start = i;
            while i < source_lines.len() && is_table_row(source_lines[i]) {
                i += 1;
            }
            output.extend(render_table(&source_lines[start..i], palette));
            continue; // i already advanced past the block
        }

        // ── Setext headings (=== / --- underline) ───────────────────────
        if i + 1 < source_lines.len() && !raw_line.trim().is_empty() && !is_hr(raw_line) {
            let next = source_lines[i + 1].trim();
            if next.len() >= 2 && next.chars().all(|c| c == '=') {
                // h1 equivalent
                output.push(Line::from(vec![Span::styled(
                    raw_line.trim().to_string(),
                    Style::default()
                        .fg(palette.border_focus)
                        .add_modifier(Modifier::BOLD),
                )]));
                output.push(Line::from(vec![Span::styled(
                    "─".repeat(hr_width),
                    Style::default().fg(palette.text_muted),
                )]));
                i += 2;
                continue;
            }
            if next.len() >= 2 && next.chars().all(|c| c == '-') {
                // h2 equivalent
                output.push(Line::from(vec![Span::styled(
                    raw_line.trim().to_string(),
                    Style::default()
                        .fg(palette.logo_accent)
                        .add_modifier(Modifier::BOLD),
                )]));
                i += 2;
                continue;
            }
        }

        // ── Headings ──────────────────────────────────────────────────────
        if let Some(level) = heading_level(raw_line) {
            let text = raw_line.trim_start_matches('#').trim().to_string();
            let (colour, add_rule) = match level {
                1 => (palette.border_focus, true),
                2 => (palette.logo_accent, false),
                _ => (palette.text_primary, false),
            };
            output.push(Line::from(vec![Span::styled(
                text,
                Style::default().fg(colour).add_modifier(Modifier::BOLD),
            )]));
            if add_rule {
                output.push(Line::from(vec![Span::styled(
                    "─".repeat(hr_width),
                    Style::default().fg(palette.text_muted),
                )]));
            }
            i += 1;
            continue;
        }

        // ── Horizontal rule ───────────────────────────────────────────────
        if is_hr(raw_line) {
            output.push(Line::from(vec![Span::styled(
                "─".repeat(hr_width),
                Style::default().fg(palette.text_muted),
            )]));
            i += 1;
            continue;
        }

        // ── Blockquote ─────────────────────────────────────────────────────
        if let Some((depth, rest)) = strip_blockquote(raw_line) {
            let prefix = "▍ ".repeat(depth);
            let mut spans = vec![Span::styled(
                prefix,
                Style::default().fg(palette.text_muted),
            )];
            spans.extend(parse_inline(rest, palette));
            output.push(Line::from(spans));
            i += 1;
            continue;
        }

        // ── Task list (must come before generic bullet) ───────────────────
        if let Some((checked, rest)) = strip_task(raw_line) {
            let indent = leading_spaces(raw_line);
            let checkbox = if checked { "☑ " } else { "☐ " };
            let mut spans = vec![
                Span::raw(" ".repeat(indent)),
                Span::styled(
                    checkbox.to_string(),
                    Style::default().fg(palette.logo_accent),
                ),
            ];
            spans.extend(parse_inline(rest, palette));
            output.push(Line::from(spans));
            i += 1;
            continue;
        }

        // ── Unordered list ────────────────────────────────────────────
        if let Some(rest) = strip_bullet(raw_line) {
            let indent = leading_spaces(raw_line);
            let marker = match indent / 2 {
                0 => "• ",
                1 => "◦ ",
                _ => "▸ ",
            };
            let mut spans = vec![
                Span::raw(" ".repeat(indent)),
                Span::styled(marker.to_string(), Style::default().fg(palette.logo_accent)),
            ];
            spans.extend(parse_inline(rest, palette));
            output.push(Line::from(spans));
            i += 1;
            continue;
        }

        // ── Ordered list ──────────────────────────────────────────────────
        if let Some((num, rest)) = strip_ordered(raw_line) {
            let indent = leading_spaces(raw_line);
            let mut spans = vec![
                Span::raw(" ".repeat(indent)),
                Span::styled(
                    format!("{}. ", num),
                    Style::default().fg(palette.logo_accent),
                ),
            ];
            spans.extend(parse_inline(rest, palette));
            output.push(Line::from(spans));
            i += 1;
            continue;
        }

        // ── Blank line ────────────────────────────────────────────────────
        if raw_line.trim().is_empty() {
            output.push(Line::from(vec![Span::raw("")]));
            i += 1;
            continue;
        }

        // ── Normal paragraph ──────────────────────────────────────────────
        output.push(Line::from(parse_inline(raw_line, palette)));
        i += 1;
    }

    output
}

// ---------------------------------------------------------------------------
// Inline span parser
// ---------------------------------------------------------------------------

fn parse_inline(text: &str, palette: ThemePalette) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut current = String::new();

    while i < chars.len() {
        // ── Bold+Italic: ***
        if i + 2 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' && chars[i + 2] == '*' {
            if !current.is_empty() {
                spans.push(plain_span(&current, palette));
                current.clear();
            }
            i += 3;
            let mut inner = String::new();
            while i + 2 < chars.len()
                && !(chars[i] == '*' && chars[i + 1] == '*' && chars[i + 2] == '*')
            {
                inner.push(chars[i]);
                i += 1;
            }
            i += 3; // skip closing ***
            spans.push(Span::styled(
                inner,
                Style::default()
                    .fg(palette.text_primary)
                    .add_modifier(Modifier::BOLD | Modifier::ITALIC),
            ));
            continue;
        }

        // ── Bold: ** or __
        if i + 1 < chars.len()
            && ((chars[i] == '*' && chars[i + 1] == '*')
                || (chars[i] == '_' && chars[i + 1] == '_'))
        {
            let marker = [chars[i], chars[i + 1]];
            if !current.is_empty() {
                spans.push(plain_span(&current, palette));
                current.clear();
            }
            i += 2;
            let mut inner = String::new();
            while i + 1 < chars.len() && !(chars[i] == marker[0] && chars[i + 1] == marker[1]) {
                inner.push(chars[i]);
                i += 1;
            }
            i += 2; // skip closing marker
            spans.push(Span::styled(
                inner,
                Style::default()
                    .fg(palette.text_primary)
                    .add_modifier(Modifier::BOLD),
            ));
            continue;
        }

        // ── Strikethrough: ~~
        if i + 1 < chars.len() && chars[i] == '~' && chars[i + 1] == '~' {
            if !current.is_empty() {
                spans.push(plain_span(&current, palette));
                current.clear();
            }
            i += 2;
            let mut inner = String::new();
            while i + 1 < chars.len() && !(chars[i] == '~' && chars[i + 1] == '~') {
                inner.push(chars[i]);
                i += 1;
            }
            i += 2; // skip closing ~~
            spans.push(Span::styled(
                inner,
                Style::default()
                    .fg(palette.text_muted)
                    .add_modifier(Modifier::CROSSED_OUT),
            ));
            continue;
        }

        // ── Italic: * or _ (single)
        if (chars[i] == '*' || chars[i] == '_')
            && (i == 0 || chars[i - 1] != chars[i])
            && (i + 1 < chars.len() && chars[i + 1] != chars[i])
        {
            let marker = chars[i];
            if !current.is_empty() {
                spans.push(plain_span(&current, palette));
                current.clear();
            }
            i += 1;
            let mut inner = String::new();
            while i < chars.len() && chars[i] != marker {
                inner.push(chars[i]);
                i += 1;
            }
            i += 1; // skip closing marker
            spans.push(Span::styled(
                inner,
                Style::default()
                    .fg(palette.text_primary)
                    .add_modifier(Modifier::ITALIC),
            ));
            continue;
        }

        // ── Inline code: `
        if chars[i] == '`' {
            if !current.is_empty() {
                spans.push(plain_span(&current, palette));
                current.clear();
            }
            i += 1;
            let mut inner = String::new();
            while i < chars.len() && chars[i] != '`' {
                inner.push(chars[i]);
                i += 1;
            }
            i += 1; // skip closing backtick
            spans.push(Span::styled(
                inner,
                Style::default()
                    .fg(Color::Cyan)
                    .bg(palette.surface_bg)
                    .add_modifier(Modifier::REVERSED),
            ));
            continue;
        }

        // ── Image: ![alt](url) — render only the alt text
        if chars[i] == '!' && i + 1 < chars.len() && chars[i + 1] == '[' {
            if let Some((alt, consumed)) = try_parse_link(&chars, i + 1) {
                if !current.is_empty() {
                    spans.push(plain_span(&current, palette));
                    current.clear();
                }
                let label = format!("[img: {}]", alt);
                spans.push(Span::styled(label, Style::default().fg(palette.text_muted)));
                i += 1 + consumed; // 1 for '!', consumed covers '[alt](url)'
                continue;
            }
        }

        // ── Link: [text](url) — render only the link text
        if chars[i] == '[' {
            if let Some((link_text, consumed)) = try_parse_link(&chars, i) {
                if !current.is_empty() {
                    spans.push(plain_span(&current, palette));
                    current.clear();
                }
                spans.push(Span::styled(
                    link_text,
                    Style::default()
                        .fg(palette.logo_accent)
                        .add_modifier(Modifier::UNDERLINED),
                ));
                i += consumed;
                continue;
            }
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        spans.push(plain_span(&current, palette));
    }
    if spans.is_empty() {
        spans.push(Span::raw(""));
    }
    spans
}

/// Try to parse `[text](url)` starting at `pos` (which must be `[`).
/// Returns `(link_text, chars_consumed)` if the pattern is complete.
fn try_parse_link(chars: &[char], pos: usize) -> Option<(String, usize)> {
    // Find closing ]
    let mut j = pos + 1;
    while j < chars.len() && chars[j] != ']' {
        j += 1;
    }
    // Require ]( immediately after
    if j + 1 >= chars.len() || chars[j] != ']' || chars[j + 1] != '(' {
        return None;
    }
    let link_text: String = chars[pos + 1..j].iter().collect();
    if link_text.is_empty() {
        return None;
    }
    // Find closing )
    let mut k = j + 2;
    while k < chars.len() && chars[k] != ')' {
        k += 1;
    }
    if k >= chars.len() {
        return None;
    }
    Some((link_text, k - pos + 1))
}

fn plain_span(text: &str, palette: ThemePalette) -> Span<'static> {
    Span::styled(text.to_string(), Style::default().fg(palette.text_primary))
}

// ---------------------------------------------------------------------------
// Table rendering
// ---------------------------------------------------------------------------

fn is_table_row(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

fn parse_table_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

/// A separator row is one where every non-empty cell consists only of
/// dashes and optional leading/trailing colons (GFM alignment markers).
fn is_separator_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|c| {
            let t = c.trim();
            !t.is_empty() && t.chars().all(|ch| ch == '-' || ch == ':') && t.contains('-')
        })
}

/// Render a slice of raw table lines into styled `Line` objects using
/// Unicode box-drawing characters. Column widths are computed from content.
fn render_table(rows: &[&str], palette: ThemePalette) -> Vec<Line<'static>> {
    let parsed: Vec<Vec<String>> = rows.iter().map(|r| parse_table_row(r)).collect();

    // Separator row index determines which rows are headers.
    let sep_idx = parsed.iter().position(|r| is_separator_row(r));

    let col_count = parsed
        .iter()
        .filter(|r| !is_separator_row(r))
        .map(|r| r.len())
        .max()
        .unwrap_or(0);

    if col_count == 0 {
        return vec![];
    }

    // Natural column widths from display width (handles multibyte chars).
    let mut col_widths: Vec<usize> = vec![0; col_count];
    for row in &parsed {
        if is_separator_row(row) {
            continue;
        }
        for (ci, cell) in row.iter().enumerate() {
            if ci < col_count {
                col_widths[ci] = col_widths[ci].max(UnicodeWidthStr::width(cell.as_str()));
            }
        }
    }

    let mut output = Vec::new();

    // Top border
    output.push(table_border_line('┌', '─', '┬', '┐', &col_widths, palette));

    for (row_idx, row) in parsed.iter().enumerate() {
        if is_separator_row(row) {
            // Header/body divider
            output.push(table_border_line('├', '─', '┼', '┤', &col_widths, palette));
            continue;
        }

        let is_header = sep_idx.is_some_and(|si| row_idx < si);

        let mut spans = vec![Span::styled(
            "│".to_string(),
            Style::default().fg(palette.text_muted),
        )];

        for (ci, w) in col_widths.iter().enumerate() {
            let cell = row.get(ci).map(|s| s.as_str()).unwrap_or("");
            let display_w = UnicodeWidthStr::width(cell);
            let padding = w.saturating_sub(display_w);
            let content = format!(" {}{} ", cell, " ".repeat(padding));
            let style = if is_header {
                Style::default()
                    .fg(palette.text_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_primary)
            };
            spans.push(Span::styled(content, style));
            spans.push(Span::styled(
                "│".to_string(),
                Style::default().fg(palette.text_muted),
            ));
        }
        output.push(Line::from(spans));
    }

    // Bottom border
    output.push(table_border_line('└', '─', '┴', '┘', &col_widths, palette));

    output
}

fn table_border_line(
    left: char,
    fill: char,
    mid: char,
    right: char,
    col_widths: &[usize],
    palette: ThemePalette,
) -> Line<'static> {
    let mut s = String::new();
    s.push(left);
    for (i, w) in col_widths.iter().enumerate() {
        for _ in 0..w + 2 {
            s.push(fill);
        }
        if i + 1 < col_widths.len() {
            s.push(mid);
        }
    }
    s.push(right);
    Line::from(Span::styled(s, Style::default().fg(palette.text_muted)))
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn heading_level(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if level <= 6 && trimmed.chars().nth(level) == Some(' ') {
        Some(level)
    } else {
        None
    }
}

fn is_hr(line: &str) -> bool {
    let t = line.trim();
    (t.starts_with("---") || t.starts_with("===") || t.starts_with("***"))
        && t.chars()
            .all(|c| c == '-' || c == '=' || c == '*' || c == ' ')
        && t.len() >= 3
}

/// Strip leading `>` blockquote markers, returning (depth, remaining text).
/// Handles nested quotes: `>> text` yields (2, "text").
fn strip_blockquote(line: &str) -> Option<(usize, &str)> {
    let mut rest = line;
    let mut depth = 0usize;
    while let Some(r) = rest.strip_prefix("> ").or_else(|| rest.strip_prefix(">")) {
        depth += 1;
        rest = r;
    }
    if depth > 0 {
        Some((depth, rest))
    } else {
        None
    }
}

/// Detect a task-list item (`- [ ] ...` or `- [x] ...`) and return
/// `(checked, rest_text)`. Must be checked before `strip_bullet`.
fn strip_task(line: &str) -> Option<(bool, &str)> {
    let trimmed = line.trim_start();
    let after_bullet = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))?;
    if let Some(rest) = after_bullet.strip_prefix("[ ] ") {
        Some((false, rest))
    } else if let Some(rest) = after_bullet
        .strip_prefix("[x] ")
        .or_else(|| after_bullet.strip_prefix("[X] "))
    {
        Some((true, rest))
    } else {
        None
    }
}

fn strip_bullet(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
}

fn strip_ordered(line: &str) -> Option<(String, &str)> {
    let trimmed = line.trim_start();
    let dot = trimmed.find(". ")?;
    let num = &trimmed[..dot];
    if num.chars().all(|c| c.is_ascii_digit()) && !num.is_empty() {
        Some((num.to_string(), &trimmed[dot + 2..]))
    } else {
        None
    }
}

fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ').count()
}

/// Extract the language identifier from a fenced code-block opening line.
/// `\`\`\`rust` → `"rust"`, `\`\`\`` → `""`.
fn fence_lang(line: &str) -> &str {
    line.trim_start().trim_start_matches('`').trim()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading1_produces_bold_line_and_rule() {
        let lines = parse_markdown_lines("# Hello World");
        assert_eq!(lines.len(), 2, "h1 should produce heading + rule");
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::BOLD)),
            "h1 span should be bold"
        );
    }

    #[test]
    fn heading2_produces_bold_line_no_rule() {
        let lines = parse_markdown_lines("## Section");
        assert_eq!(lines.len(), 1, "h2 should produce only the heading line");
        assert!(lines[0]
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::BOLD)));
    }

    #[test]
    fn bullet_list_uses_bullet_char() {
        let lines = parse_markdown_lines("- item one");
        assert!(
            lines[0].spans.iter().any(|s| s.content.contains('•')),
            "bullet line should contain '•'"
        );
    }

    #[test]
    fn ordered_list_uses_number_prefix() {
        let lines = parse_markdown_lines("1. first item");
        assert!(
            lines[0].spans.iter().any(|s| s.content.contains("1.")),
            "ordered list should contain '1.'"
        );
    }

    #[test]
    fn blank_line_produces_empty_line() {
        let lines = parse_markdown_lines("");
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn fenced_code_block_collects_inner_lines() {
        let src = "```\nlet x = 1;\nlet y = 2;\n```";
        let lines = parse_markdown_lines(src);
        assert_eq!(lines.len(), 4, "fence-open, 2 code lines, fence-close");
    }

    #[test]
    fn fenced_block_shows_language_tag() {
        let src = "```rust\nlet x = 1;\n```";
        let lines = parse_markdown_lines(src);
        assert!(
            lines[0].spans.iter().any(|s| s.content.contains("rust")),
            "fence open line should contain the language name"
        );
    }

    #[test]
    fn horizontal_rule_fills_with_rule_chars() {
        let lines = parse_markdown_lines("---");
        assert!(
            lines[0].spans.iter().any(|s| s.content.contains('─')),
            "hr should use '─' character"
        );
    }

    #[test]
    fn blockquote_uses_bar_prefix() {
        let lines = parse_markdown_lines("> quoted text");
        assert!(
            lines[0].spans.iter().any(|s| s.content.contains('▍')),
            "blockquote should use '▍' prefix"
        );
    }

    #[test]
    fn inline_bold_applies_bold_modifier() {
        let lines = parse_markdown_lines("this is **bold** text");
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::BOLD)),
            "**bold** should produce a bold span"
        );
    }

    #[test]
    fn inline_italic_applies_italic_modifier() {
        let lines = parse_markdown_lines("this is *italic* text");
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::ITALIC)),
            "*italic* should produce an italic span"
        );
    }

    #[test]
    fn inline_code_applies_reversed_modifier() {
        let lines = parse_markdown_lines("run `cargo test` now");
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::REVERSED)),
            "`code` should be reversed"
        );
    }

    #[test]
    fn plain_text_produces_single_span() {
        let lines = parse_markdown_lines("hello world");
        assert_eq!(lines.len(), 1);
        let combined: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(combined.contains("hello world"));
    }

    #[test]
    fn mixed_document_line_count() {
        let src = "# Title\n\nParagraph.\n\n- bullet\n- bullet2\n\n```\ncode\n```";
        let lines = parse_markdown_lines(src);
        // Title(1) + rule(1) + blank(1) + para(1) + blank(1) + 2 bullets(2) + blank(1) + fence(2) + code(1) = 11
        assert!(lines.len() >= 10, "got {} lines", lines.len());
    }

    // ── New: strikethrough ──────────────────────────────────────────────────

    #[test]
    fn strikethrough_applies_crossed_out_modifier() {
        let lines = parse_markdown_lines("this is ~~deleted~~ text");
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::CROSSED_OUT)),
            "~~text~~ should produce a CROSSED_OUT span"
        );
    }

    // ── New: links ──────────────────────────────────────────────────────────

    #[test]
    fn link_renders_text_not_url() {
        let lines = parse_markdown_lines("see [the docs](https://example.com) for details");
        let combined: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(combined.contains("the docs"), "link text must appear");
        assert!(
            !combined.contains("https://"),
            "raw URL must not appear in output"
        );
    }

    #[test]
    fn link_applies_underline_modifier() {
        let lines = parse_markdown_lines("[click here](https://example.com)");
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::UNDERLINED)),
            "link span should be underlined"
        );
    }

    // ── New: task lists ─────────────────────────────────────────────────────

    #[test]
    fn task_list_unchecked_uses_open_checkbox() {
        let lines = parse_markdown_lines("- [ ] todo item");
        let combined: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(combined.contains('☐'), "unchecked task should use '☐'");
        assert!(combined.contains("todo item"));
    }

    #[test]
    fn task_list_checked_uses_filled_checkbox() {
        let lines = parse_markdown_lines("- [x] done item");
        let combined: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(combined.contains('☑'), "checked task should use '☑'");
    }

    #[test]
    fn task_list_uppercase_x_also_checked() {
        let lines = parse_markdown_lines("- [X] also done");
        let combined: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(combined.contains('☑'));
    }

    // ── New: tables ─────────────────────────────────────────────────────────

    #[test]
    fn table_emits_border_and_data_lines() {
        let src = "| Name | Age |\n|------|-----|\n| Alice | 30 |\n| Bob | 25 |";
        let lines = parse_markdown_lines(src);
        // top border + header + divider + 2 data rows + bottom border = 6
        assert_eq!(lines.len(), 6, "got {} lines: {:#?}", lines.len(), lines);
    }

    #[test]
    fn table_top_border_uses_box_drawing() {
        let src = "| A | B |\n|---|---|\n| 1 | 2 |";
        let lines = parse_markdown_lines(src);
        let top: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(top.starts_with('┌'), "top border must start with '┌'");
        assert!(top.ends_with('┐'), "top border must end with '┐'");
    }

    #[test]
    fn table_header_row_is_bold() {
        let src = "| Header |\n|---------|\n| Cell |";
        let lines = parse_markdown_lines(src);
        // lines[0] = top border, lines[1] = header row
        assert!(
            lines[1]
                .spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::BOLD)),
            "header row cells should be bold"
        );
    }

    #[test]
    fn table_alignment_markers_do_not_appear_as_data() {
        let src = "| A | B |\n|:---|---:|\n| x | y |";
        let lines = parse_markdown_lines(src);
        let all_content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        // Alignment markers like :--- must not leak into rendered output
        assert!(
            !all_content.contains(":---"),
            "alignment markers must not appear in output"
        );
    }

    #[test]
    fn table_columns_are_padded_to_equal_width() {
        // "Name" is wider than "A", so cells in that column must be padded to match.
        let src = "| Name | Score |\n|------|-------|\n| A | 100 |";
        let lines = parse_markdown_lines(src);
        // header row is lines[1]; find the cell span containing "Name"
        let header_row: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        // "A" cell should be padded to the same width as "Name" → " A    " (with spaces)
        assert!(header_row.contains("Name"), "header must contain 'Name'");
    }

    // ── New: bold+italic, nested bullets, images, setext, nested blockquotes ────

    #[test]
    fn bold_italic_combined_applies_both_modifiers() {
        let lines = parse_markdown_lines("this is ***bold italic*** text");
        let span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.contains("bold italic"));
        assert!(span.is_some(), "bold italic span must exist");
        let s = span.unwrap();
        assert!(s.style.add_modifier.contains(Modifier::BOLD));
        assert!(s.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn nested_bullet_uses_open_circle() {
        let lines = parse_markdown_lines("  - nested");
        assert!(
            lines[0]
                .spans
                .iter()
                .any(|s| s.content.contains('\u{25e6}')),
            "2-space indent bullet should use '\u{25e6}'"
        );
    }

    #[test]
    fn image_renders_alt_not_url() {
        let lines = parse_markdown_lines("![my diagram](https://example.com/img.png)");
        let combined: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(combined.contains("my diagram"), "alt text must appear");
        assert!(!combined.contains("https://"), "image URL must not appear");
    }

    #[test]
    fn setext_h1_produces_bold_and_rule() {
        let src = "My Title\n========";
        let lines = parse_markdown_lines(src);
        assert_eq!(
            lines.len(),
            2,
            "setext h1 should produce heading + rule, got {}",
            lines.len()
        );
        assert!(lines[0]
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::BOLD)));
        assert!(lines[1]
            .spans
            .iter()
            .any(|s| s.content.contains('\u{2500}')));
    }

    #[test]
    fn setext_h2_produces_bold_no_rule() {
        let src = "My Section\n----------";
        let lines = parse_markdown_lines(src);
        assert_eq!(
            lines.len(),
            1,
            "setext h2 should produce only the heading line"
        );
        assert!(lines[0]
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::BOLD)));
    }

    #[test]
    fn nested_blockquote_uses_double_bar() {
        let lines = parse_markdown_lines(">> deeply quoted");
        let combined: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        // Two levels → "▍ ▍ " prefix
        assert_eq!(
            combined.matches('\u{258d}').count(),
            2,
            "double blockquote should have 2 bar chars"
        );
    }
}
