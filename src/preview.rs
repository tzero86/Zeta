use std::sync::Arc;

use ratatui::style::{Color, Modifier};

#[cfg(test)]
use crate::highlight::HighlightToken;
use crate::highlight::{normalize_preview_text, HighlightedLine};

/// A read-only scrollable view of syntax-highlighted file content.
/// Used by the preview panel. Scroll state is owned here.
///
/// `lines` is wrapped in `Arc<[_]>` so that `ViewBuffer::clone()` is O(1) —
/// the highlighted content is immutable after construction, and the preview
/// cache can hold a shared reference without paying a full deep-copy on every
/// cache hit during navigation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewBuffer {
    pub lines: Arc<[HighlightedLine]>,
    pub scroll_row: usize,
    pub total_lines: usize,
    /// Raw Markdown source. When `Some`, the buffer represents a markdown
    /// document to be rendered by `tui_markdown` at display time.
    /// `lines` is empty for markdown buffers.
    pub markdown_source: Option<String>,
}

impl ViewBuffer {
    /// Build from a raw Markdown string — rendered as wrapped plain text in the
    /// preview panel. Kept as a distinct variant so the renderer can apply
    /// markdown-specific layout (no gutter, word-wrap) and a future tui-markdown
    /// integration can be dropped in with a one-line change.
    pub fn from_markdown(source: String) -> Self {
        Self {
            lines: Arc::from([]),
            scroll_row: 0,
            total_lines: 0,
            markdown_source: Some(source),
        }
    }

    /// Returns `true` if this buffer holds raw Markdown source.
    pub fn is_markdown(&self) -> bool {
        self.markdown_source.is_some()
    }

    /// Returns the raw Markdown source, or `None` for non-markdown buffers.
    pub fn markdown_source(&self) -> Option<&str> {
        self.markdown_source.as_deref()
    }

    /// Build a sanitized read-only preview buffer from raw text.
    pub fn from_render_text(text: &str) -> Self {
        Self::from_plain(text)
    }

    /// Build from pre-highlighted lines (from syntect).
    pub fn from_highlighted(lines: Vec<HighlightedLine>) -> Self {
        let total_lines = lines.len();
        Self {
            lines: lines.into(), // Vec<HighlightedLine> → Arc<[HighlightedLine]>, O(1) clone
            scroll_row: 0,
            total_lines,
            markdown_source: None,
        }
    }

    /// Build from plain text — each line becomes a single unstyled token.
    pub fn from_plain(text: &str) -> Self {
        let text = normalize_preview_text(text);
        let lines: Vec<HighlightedLine> = text
            .lines()
            .map(|l| vec![(Color::Reset, Modifier::empty(), Box::from(l))])
            .collect();
        let total_lines = lines.len();
        Self {
            lines: lines.into(), // Vec → Arc<[_]>
            scroll_row: 0,
            total_lines,
            markdown_source: None,
        }
    }

    /// Returns the slice of lines visible starting at scroll_row, clamped to height.
    pub fn visible_window(&self, height: usize) -> (usize, &[HighlightedLine]) {
        if self.lines.is_empty() {
            return (0, &[]);
        }
        let start = self.scroll_row.min(self.total_lines.saturating_sub(1));
        let end = (start + height).min(self.total_lines);
        (start, &self.lines[start..end])
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll_row = self
            .scroll_row
            .saturating_add(n)
            .min(self.total_lines.saturating_sub(1));
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_row = self.scroll_row.saturating_sub(n);
    }

    pub fn reset_scroll(&mut self) {
        self.scroll_row = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_highlighted(n: usize) -> Vec<HighlightedLine> {
        (0..n)
            .map(|i| {
                vec![(
                    Color::White,
                    Modifier::empty(),
                    format!("line {i}").into_boxed_str(),
                )]
            })
            .collect()
    }

    #[test]
    fn markdown_variant_stores_raw_text() {
        let vb = ViewBuffer::from_markdown("# Hello\n\nWorld".to_string());
        assert!(vb.is_markdown());
        assert_eq!(vb.markdown_source(), Some("# Hello\n\nWorld"));
    }

    #[test]
    fn non_markdown_variant_returns_none_for_markdown_source() {
        let vb = ViewBuffer::from_plain("hello");
        assert!(!vb.is_markdown());
        assert_eq!(vb.markdown_source(), None);
    }

    #[test]
    fn from_plain_builds_correct_total() {
        let text = "alpha\nbeta\ngamma";
        let buf = ViewBuffer::from_render_text(text);
        assert_eq!(buf.total_lines, 3);
        assert_eq!(buf.scroll_row, 0);
        assert_eq!(buf.lines.len(), 3);

        // Each line should be a single unstyled token.
        let token: &HighlightToken = &buf.lines[1][0];
        assert_eq!(token.0, Color::Reset);
        assert_eq!(token.1, Modifier::empty());
        assert_eq!(token.2.as_ref(), "beta");
    }

    #[test]
    fn preview_prep_strips_control_chars_and_preserves_visible_width() {
        let buf = ViewBuffer::from_plain("alpha\r\nbeta\nchar\tlie\nwide: 測試\u{0007}");

        assert_eq!(buf.lines.len(), 4);
        assert!(buf
            .lines
            .iter()
            .all(|line| line.iter().all(|token| !token.2.contains('\r'))));
        assert!(buf
            .lines
            .iter()
            .all(|line| line.iter().all(|token| !token.2.contains('\u{0007}'))));
        assert_eq!(buf.lines[2][0].2.as_ref(), "char    lie");
        assert!(buf
            .lines
            .iter()
            .any(|line| line.iter().any(|token| token.2.contains("wide: 測試"))));
    }

    #[test]
    fn visible_window_clamps_to_available_lines() {
        let buf = ViewBuffer::from_highlighted(make_highlighted(5));

        // Asking for more lines than available should return all 5.
        let (start, window) = buf.visible_window(10);
        assert_eq!(start, 0);
        assert_eq!(window.len(), 5);

        // Asking for exactly 3 lines returns first 3.
        let (start, window) = buf.visible_window(3);
        assert_eq!(start, 0);
        assert_eq!(window.len(), 3);

        // Empty buffer always returns an empty slice.
        let empty = ViewBuffer::from_highlighted(vec![]);
        let (start, window) = empty.visible_window(10);
        assert_eq!(start, 0);
        assert_eq!(window.len(), 0);
    }

    #[test]
    fn scroll_down_clamps_at_last_line() {
        let mut buf = ViewBuffer::from_highlighted(make_highlighted(5));

        // Scroll far past the end — should clamp to last valid index (4).
        buf.scroll_down(100);
        assert_eq!(buf.scroll_row, 4);

        // Scrolling further should stay clamped.
        buf.scroll_down(1);
        assert_eq!(buf.scroll_row, 4);
    }

    #[test]
    fn scroll_up_clamps_at_zero() {
        let mut buf = ViewBuffer::from_highlighted(make_highlighted(5));
        buf.scroll_down(3);
        assert_eq!(buf.scroll_row, 3);

        buf.scroll_up(1);
        assert_eq!(buf.scroll_row, 2);

        // Scrolling past zero should clamp.
        buf.scroll_up(100);
        assert_eq!(buf.scroll_row, 0);

        // Scrolling up again at zero should stay at zero.
        buf.scroll_up(1);
        assert_eq!(buf.scroll_row, 0);
    }
}
