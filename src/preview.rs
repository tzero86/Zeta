use ratatui::style::{Color, Modifier};

#[cfg(test)]
use crate::highlight::HighlightToken;
use crate::highlight::HighlightedLine;

/// A read-only scrollable view of syntax-highlighted file content.
/// Used by the preview panel. Scroll state is owned here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewBuffer {
    pub lines: Vec<HighlightedLine>,
    pub scroll_row: usize,
    pub total_lines: usize,
}

impl ViewBuffer {
    /// Build a sanitized read-only preview buffer from raw text.
    pub fn from_render_text(text: &str) -> Self {
        Self::from_plain(text)
    }

    /// Build from pre-highlighted lines (from syntect).
    pub fn from_highlighted(lines: Vec<HighlightedLine>) -> Self {
        let total_lines = lines.len();
        Self {
            lines,
            scroll_row: 0,
            total_lines,
        }
    }

    /// Build from plain text — each line becomes a single unstyled token.
    pub fn from_plain(text: &str) -> Self {
        let lines: Vec<HighlightedLine> = text
            .lines()
            .map(|l| vec![(Color::Reset, Modifier::empty(), l.to_string())])
            .collect();
        let total_lines = lines.len();
        Self {
            lines,
            scroll_row: 0,
            total_lines,
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
            .map(|i| vec![(Color::White, Modifier::empty(), format!("line {i}"))])
            .collect()
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
        assert_eq!(token.2, "beta");
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
