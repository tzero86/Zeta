use ratatui::layout::Rect;

/// Rects computed during each render frame.
/// Stored on `App` so the event loop can route mouse events
/// without re-running the layout algorithm.
#[derive(Clone, Copy, Debug, Default)]
pub struct LayoutCache {
    pub menu_bar: Rect,
    pub left_pane: Rect,
    pub right_pane: Rect,
    /// Present when the editor or preview panel is visible.
    pub tools_panel: Option<Rect>,
    pub status_bar: Rect,
}

/// Returns `true` if the terminal cell at (`col`, `row`) falls inside `rect`.
pub fn rect_contains(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_contains_returns_true_for_inner_cell() {
        let r = Rect {
            x: 5,
            y: 3,
            width: 10,
            height: 4,
        };
        assert!(rect_contains(r, 5, 3));
        assert!(rect_contains(r, 14, 6));
        assert!(rect_contains(r, 10, 5));
    }

    #[test]
    fn rect_contains_returns_false_for_border_outside() {
        let r = Rect {
            x: 5,
            y: 3,
            width: 10,
            height: 4,
        };
        assert!(!rect_contains(r, 15, 5)); // x == x + width (exclusive)
        assert!(!rect_contains(r, 10, 7)); // y == y + height (exclusive)
        assert!(!rect_contains(r, 4, 5)); // x < x
        assert!(!rect_contains(r, 10, 2)); // y < y
    }

    #[test]
    fn rect_contains_zero_size_rect_never_matches() {
        let r = Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };
        assert!(!rect_contains(r, 0, 0));
    }
}
