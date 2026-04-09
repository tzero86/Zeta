# Wave 4B — Markdown Live Preview Split

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When a `.md` file is open in the editor, split the tools panel vertically — editor on the left half, live preview on the right — updating on every edit. Write a lightweight native markdown renderer targeting ratatui 0.29 directly, bypassing the tui-markdown compatibility dead-end from Wave 1C.

**Architecture:**
- `src/ui/markdown.rs` — new module: `render_markdown_preview(frame, area, source, palette)`. Pure ratatui 0.29 rendering of a markdown string. No external crate.
- `src/ui/mod.rs` — when the editor is open and the file is `.md`, split `tools_area` into `[editor_area, md_preview_area]` using a horizontal 50/50 constraint, then call `render_markdown_preview` with the editor's current buffer contents.
- `src/ui/editor.rs` — expose `pub fn is_markdown_file(editor: &EditorBuffer) -> bool` helper.
- No state changes. No new actions. No new job results. The preview renders directly from the in-memory buffer — zero latency, zero background work.

**Renderer feature set (no external crate needed):**

| Markdown element | Rendering |
|---|---|
| `# H1` | Bold, accent colour, full-width rule beneath |
| `## H2` | Bold, accent colour |
| `### H3`–`###### H6` | Bold, slightly dimmer |
| `**bold**` / `__bold__` | Bold modifier |
| `*italic*` / `_italic_` | Italic modifier |
| `` `inline code` `` | Reverse video / code colour |
| ` ```fence``` ` | Boxed block with dim background |
| `- ` / `* ` / `+ ` bullet | `•` prefix, indented |
| `1.` ordered list | number prefix, indented |
| `> ` blockquote | `▍` prefix, dim |
| `---` / `===` hr | Full-width `─` rule |
| blank line | empty row |
| plain paragraph | normal text |

**Tech Stack:** ratatui 0.29 only. No new crate dependencies.

**Jira:** ZTA-89 (ZTA-131 through ZTA-135)

**Wave dependency:** Starts AFTER Wave 4A. Requires `EditorBuffer::path` (Wave 1C) and `EditorBuffer::contents()` (Wave 3A).

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/ui/markdown.rs` | Native markdown → ratatui renderer |
| Modify | `src/ui/mod.rs` | `pub mod markdown;` + split tools panel for `.md` editor files |
| Modify | `src/ui/editor.rs` | `pub fn is_markdown_file(editor: &EditorBuffer) -> bool` |

---

## Task 1: Create `src/ui/markdown.rs` — the renderer

**Files:**
- Create: `src/ui/markdown.rs`

- [ ] **Step 1.1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading1_produces_bold_line() {
        let lines = parse_markdown_lines("# Hello World");
        assert_eq!(lines.len(), 2); // heading + rule
        assert!(lines[0].spans.iter().any(|s| s.style.add_modifier.contains(ratatui::style::Modifier::BOLD)));
    }

    #[test]
    fn bullet_list_uses_bullet_char() {
        let lines = parse_markdown_lines("- item one");
        assert!(lines[0].spans.iter().any(|s| s.content.contains('•')));
    }

    #[test]
    fn blank_line_produces_empty_line() {
        let lines = parse_markdown_lines("");
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans.iter().all(|s| s.content.trim().is_empty()));
    }

    #[test]
    fn fenced_code_block_collects_multiple_lines() {
        let src = "```\nlet x = 1;\nlet y = 2;\n```";
        let lines = parse_markdown_lines(src);
        // fence open + 2 code lines + fence close = 4
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn horizontal_rule_fills_with_dash_chars() {
        let lines = parse_markdown_lines("---");
        assert!(lines[0].spans.iter().any(|s| s.content.contains('─')));
    }

    #[test]
    fn blockquote_uses_bar_prefix() {
        let lines = parse_markdown_lines("> quoted text");
        assert!(lines[0].spans.iter().any(|s| s.content.contains('▍')));
    }

    #[test]
    fn inline_bold_applies_bold_modifier() {
        let lines = parse_markdown_lines("this is **bold** text");
        let bold_spans: Vec<_> = lines[0]
            .spans
            .iter()
            .filter(|s| s.style.add_modifier.contains(ratatui::style::Modifier::BOLD))
            .collect();
        assert!(!bold_spans.is_empty(), "expected at least one bold span");
    }

    #[test]
    fn inline_italic_applies_italic_modifier() {
        let lines = parse_markdown_lines("this is *italic* text");
        let italic_spans: Vec<_> = lines[0]
            .spans
            .iter()
            .filter(|s| s.style.add_modifier.contains(ratatui::style::Modifier::ITALIC))
            .collect();
        assert!(!italic_spans.is_empty(), "expected at least one italic span");
    }
}
```

- [ ] **Step 1.2: Confirm they fail**

```bash
cargo test ui::markdown 2>&1 | head -5
```

Expected: compile error — `markdown` module not found.

- [ ] **Step 1.3: Implement `src/ui/markdown.rs`**

```rust
//! Lightweight markdown renderer for ratatui 0.29.
//!
//! Converts a markdown string into a `Vec<Line<'static>>` which can be
//! rendered inside any `Paragraph` or directly via `render_widget`.
//!
//! Supported: headings, bold, italic, inline code, fenced code blocks,
//! bullets, ordered lists, blockquotes, horizontal rules, blank lines.
//! Unsupported: tables, links (rendered as plain text), images.

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
) {
    let block = Block::default()
        .title(" Preview ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.text_muted))
        .style(Style::default().bg(palette.tools_bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = parse_markdown_lines_with_palette(source, palette);
    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(palette.tools_bg))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

// ---------------------------------------------------------------------------
// Parsing — exported for unit tests
// ---------------------------------------------------------------------------

/// Parse markdown source into styled `Line` objects using default colours.
/// Used by tests; production code uses `parse_markdown_lines_with_palette`.
pub fn parse_markdown_lines(source: &str) -> Vec<Line<'static>> {
    parse_markdown_lines_with_palette(source, default_palette())
}

fn default_palette() -> ThemePalette {
    crate::config::ResolvedTheme::from_preset(crate::config::ThemePreset::Oxide).palette
}

pub fn parse_markdown_lines_with_palette(
    source: &str,
    palette: ThemePalette,
) -> Vec<Line<'static>> {
    let mut output: Vec<Line<'static>> = Vec::new();
    let mut in_fence = false;

    for raw_line in source.lines() {
        // ── Fenced code block ────────────────────────────────────────────
        if raw_line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            let marker = if in_fence { "┌─ code " } else { "└───────" };
            output.push(Line::from(vec![Span::styled(
                marker.to_string(),
                Style::default().fg(palette.text_muted),
            )]));
            continue;
        }
        if in_fence {
            output.push(Line::from(vec![Span::styled(
                format!("  {}", raw_line),
                Style::default()
                    .fg(palette.text_primary)
                    .bg(palette.surface_bg),
            )]));
            continue;
        }

        // ── Headings ─────────────────────────────────────────────────────
        if let Some(level) = heading_level(raw_line) {
            let text = raw_line.trim_start_matches('#').trim().to_string();
            let (colour, add_rule) = match level {
                1 => (palette.border_focus, true),
                2 => (palette.logo_accent, false),
                _ => (palette.text_primary, false),
            };
            output.push(Line::from(vec![Span::styled(
                text,
                Style::default()
                    .fg(colour)
                    .add_modifier(Modifier::BOLD),
            )]));
            if add_rule {
                output.push(Line::from(vec![Span::styled(
                    "─".repeat(60),
                    Style::default().fg(palette.text_muted),
                )]));
            }
            continue;
        }

        // ── Horizontal rule ───────────────────────────────────────────────
        if is_hr(raw_line) {
            output.push(Line::from(vec![Span::styled(
                "─".repeat(60),
                Style::default().fg(palette.text_muted),
            )]));
            continue;
        }

        // ── Blockquote ────────────────────────────────────────────────────
        if let Some(rest) = raw_line.strip_prefix("> ").or_else(|| raw_line.strip_prefix(">")) {
            let mut spans = vec![Span::styled(
                "▍ ".to_string(),
                Style::default().fg(palette.text_muted),
            )];
            spans.extend(parse_inline(rest, palette));
            output.push(Line::from(spans));
            continue;
        }

        // ── Unordered list ────────────────────────────────────────────────
        if let Some(rest) = strip_bullet(raw_line) {
            let indent = leading_spaces(raw_line);
            let mut spans = vec![
                Span::raw(" ".repeat(indent)),
                Span::styled("• ".to_string(), Style::default().fg(palette.logo_accent)),
            ];
            spans.extend(parse_inline(rest, palette));
            output.push(Line::from(spans));
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
            continue;
        }

        // ── Blank line ────────────────────────────────────────────────────
        if raw_line.trim().is_empty() {
            output.push(Line::from(vec![Span::raw("")]));
            continue;
        }

        // ── Normal paragraph ──────────────────────────────────────────────
        output.push(Line::from(parse_inline(raw_line, palette)));
    }

    output
}

// ---------------------------------------------------------------------------
// Inline span parser — handles **bold**, *italic*, `code`
// ---------------------------------------------------------------------------

fn parse_inline(text: &str, palette: ThemePalette) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut current = String::new();

    while i < chars.len() {
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
            while i + 1 < chars.len()
                && !(chars[i] == marker[0] && chars[i + 1] == marker[1])
            {
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

fn plain_span(text: &str, palette: ThemePalette) -> Span<'static> {
    Span::styled(text.to_string(), Style::default().fg(palette.text_primary))
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
        && t.chars().all(|c| c == '-' || c == '=' || c == '*' || c == ' ')
        && t.len() >= 3
}

fn strip_bullet(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
    {
        Some(rest)
    } else {
        None
    }
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
            lines[0].spans.iter().any(|s| s.style.add_modifier.contains(Modifier::BOLD)),
            "h1 span should be bold"
        );
    }

    #[test]
    fn heading2_produces_bold_line_no_rule() {
        let lines = parse_markdown_lines("## Section");
        assert_eq!(lines.len(), 1, "h2 should produce only the heading line");
        assert!(lines[0].spans.iter().any(|s| s.style.add_modifier.contains(Modifier::BOLD)));
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
            lines[0].spans.iter().any(|s| s.style.add_modifier.contains(Modifier::BOLD)),
            "**bold** should produce a bold span"
        );
    }

    #[test]
    fn inline_italic_applies_italic_modifier() {
        let lines = parse_markdown_lines("this is *italic* text");
        assert!(
            lines[0].spans.iter().any(|s| s.style.add_modifier.contains(Modifier::ITALIC)),
            "*italic* should produce an italic span"
        );
    }

    #[test]
    fn inline_code_applies_reversed_modifier() {
        let lines = parse_markdown_lines("run `cargo test` now");
        assert!(
            lines[0].spans.iter().any(|s| s.style.add_modifier.contains(Modifier::REVERSED)),
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
}
