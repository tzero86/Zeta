use std::fs as std_fs;
use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier};
use thiserror::Error;
use tui_textarea::{CursorMove, Input, Key, TextArea};

use crate::highlight::{highlight_text, normalize_preview_text, HighlightedLine};

// ---------------------------------------------------------------------------
// Public render state (unchanged from original)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorRenderState {
    pub visible_start: usize,
    pub visible_lines: Vec<String>,
    pub cursor_visible_row: Option<usize>,
    pub scroll_col: usize,
}

// ---------------------------------------------------------------------------
// EditorBuffer — backed by tui_textarea::TextArea<'static>
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct EditorBuffer {
    pub path: Option<PathBuf>,
    pub is_dirty: bool,
    pub search_query: String,
    pub search_active: bool,
    pub search_match_idx: usize,
    /// Horizontal scroll offset in columns — managed separately since
    /// tui-textarea doesn't expose horizontal scrolling for multi-line buffers.
    pub scroll_col: usize,
    inner: TextArea<'static>,
}

impl EditorBuffer {
    pub fn open(path: &Path) -> Result<Self, EditorError> {
        let bytes = std_fs::read(path).map_err(|source| EditorError::ReadFile {
            path: path.display().to_string(),
            source,
        })?;
        let text = String::from_utf8_lossy(&bytes);
        let mut lines: Vec<String> = text.lines().map(String::from).collect();
        // Preserve trailing newline as an extra empty line so `contents()` round-trips.
        if text.ends_with('\n') {
            lines.push(String::new());
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        let mut inner = TextArea::from(lines);
        inner.move_cursor(CursorMove::Top);

        Ok(Self {
            path: Some(path.to_path_buf()),
            is_dirty: false,
            search_query: String::new(),
            search_active: false,
            search_match_idx: 0,
            scroll_col: 0,
            inner,
        })
    }

    // -----------------------------------------------------------------------
    // Compatibility insert — places cursor at char_idx then inserts `text`.
    // Used by tests that construct buffer state without going through the
    // action system (e.g. state/mod.rs integration tests).
    // -----------------------------------------------------------------------

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        let (row, col) = self.char_idx_to_pos(char_idx);
        self.inner
            .move_cursor(CursorMove::Jump(row as u16, col as u16));
        self.inner.insert_str(text);
        self.is_dirty = true;
    }

    // -----------------------------------------------------------------------
    // Text mutation
    // -----------------------------------------------------------------------

    pub fn insert_char(&mut self, ch: char) {
        self.inner.input(Input { key: Key::Char(ch), ctrl: false, alt: false, shift: false });
        self.is_dirty = true;
    }

    pub fn insert_newline(&mut self) {
        self.inner.input(Input { key: Key::Enter, ctrl: false, alt: false, shift: false });
        self.is_dirty = true;
    }

    pub fn backspace(&mut self) {
        let (row, col) = self.inner.cursor();
        if row == 0 && col == 0 {
            return;
        }
        self.inner.input(Input { key: Key::Backspace, ctrl: false, alt: false, shift: false });
        self.is_dirty = true;
    }

    pub fn move_left(&mut self) {
        self.inner.move_cursor(CursorMove::Back);
    }

    pub fn move_right(&mut self) {
        self.inner.move_cursor(CursorMove::Forward);
    }

    pub fn move_up(&mut self) {
        self.inner.move_cursor(CursorMove::Up);
    }

    pub fn move_down(&mut self) {
        self.inner.move_cursor(CursorMove::Down);
    }

    /// Undo the most recent edit.
    pub fn undo(&mut self) {
        self.inner.undo();
        // If the buffer is back to the single-empty-line state, clear the dirty flag.
        self.is_dirty = self.inner.lines() != [String::new()];
    }

    /// Redo the most recently undone edit.
    pub fn redo(&mut self) {
        self.inner.redo();
        self.is_dirty = self.inner.lines() != [String::new()];
    }

    // -----------------------------------------------------------------------
    // Persistence
    // -----------------------------------------------------------------------

    pub fn save(&mut self) -> Result<(), EditorError> {
        let path = self.path.clone().ok_or(EditorError::MissingPath)?;
        let content = self.contents();
        std_fs::write(&path, content.as_bytes()).map_err(|source| EditorError::WriteFile {
            path: path.display().to_string(),
            source,
        })?;
        self.is_dirty = false;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Content accessors
    // -----------------------------------------------------------------------

    pub fn contents(&self) -> String {
        self.inner.lines().join("\n")
    }

    pub fn visible_lines(&self) -> Vec<String> {
        self.inner.lines().to_vec()
    }

    // -----------------------------------------------------------------------
    // Rendering support
    // -----------------------------------------------------------------------

    pub fn visible_highlighted_window(
        &self,
        height: usize,
        syntect_theme: &str,
        fallback_color: Color,
    ) -> (usize, Vec<HighlightedLine>) {
        if height == 0 {
            return (0, Vec::new());
        }
        let (start, window_lines) = self.visible_line_window(height);
        let text = normalize_preview_text(&window_lines.join("\n"));
        let ext = self
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str());
        if let Some(highlighted) = highlight_text(&text, ext, syntect_theme) {
            return (start, highlighted);
        }
        let plain: Vec<HighlightedLine> = window_lines
            .into_iter()
            .map(|line| vec![(fallback_color, Modifier::empty(), line)])
            .collect();
        (start, plain)
    }

    pub fn visible_line_window(&self, height: usize) -> (usize, Vec<String>) {
        if height == 0 {
            return (0, Vec::new());
        }
        let all_lines = self.inner.lines();
        let total = all_lines.len();
        let (cursor_row, _) = self.inner.cursor();
        let start = (cursor_row + 1).saturating_sub(height);
        let end = (start + height).min(total);
        (start, all_lines[start..end].to_vec())
    }

    pub fn cursor_line_col(&self) -> (usize, usize) {
        self.inner.cursor()
    }

    pub fn clamp_horizontal_scroll(&mut self, viewport_cols: usize) {
        let (_, col) = self.inner.cursor();
        if col < self.scroll_col {
            self.scroll_col = col;
        } else if viewport_cols > 0 && col >= self.scroll_col + viewport_cols {
            self.scroll_col = col.saturating_sub(viewport_cols) + 1;
        }
    }

    pub fn render_state(
        &mut self,
        viewport_rows: usize,
        viewport_cols: usize,
        is_active: bool,
    ) -> EditorRenderState {
        self.clamp_horizontal_scroll(viewport_cols);
        let (visible_start, visible_lines) = self.visible_line_window(viewport_rows);
        let (cursor_row, _) = self.inner.cursor();
        let cursor_visible_row = if is_active {
            Some(cursor_row.saturating_sub(visible_start))
        } else {
            None
        };
        EditorRenderState {
            visible_start,
            visible_lines,
            cursor_visible_row,
            scroll_col: self.scroll_col,
        }
    }

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------

    pub fn find_matches(&self, query: &str) -> Vec<(usize, usize)> {
        if query.is_empty() {
            return vec![];
        }
        let q = query.to_lowercase();
        let mut matches = Vec::new();
        let mut char_offset = 0usize;
        for line in self.inner.lines() {
            let lower = line.to_lowercase();
            let mut search_start = 0usize;
            while search_start <= lower.len() {
                match lower[search_start..].find(&q) {
                    Some(found) => {
                        let abs = char_offset + search_start + found;
                        matches.push((abs, abs + q.len()));
                        search_start += found + q.len().max(1);
                    }
                    None => break,
                }
            }
            char_offset += line.len() + 1; // +1 for the logical newline
        }
        matches
    }

    /// Jump forward to the next search match, wrapping at the end.
    pub fn search_next(&mut self) {
        let matches = self.find_matches(&self.search_query.clone());
        if matches.is_empty() {
            self.search_match_idx = 0;
            return;
        }
        let (cursor_row, cursor_col) = self.inner.cursor();
        // Compute a rough char offset for the cursor so we can pick the next match.
        let cursor_char = self.approx_char_offset(cursor_row, cursor_col);
        let next = matches
            .iter()
            .position(|(s, _)| *s > cursor_char)
            .unwrap_or(0);
        self.search_match_idx = next;
        let (row, col) = self.char_idx_to_pos(matches[next].0);
        self.inner.move_cursor(CursorMove::Jump(row as u16, col as u16));
    }

    /// Jump backward to the previous search match, wrapping at the start.
    pub fn search_prev(&mut self) {
        let matches = self.find_matches(&self.search_query.clone());
        if matches.is_empty() {
            self.search_match_idx = 0;
            return;
        }
        let (cursor_row, cursor_col) = self.inner.cursor();
        let cursor_char = self.approx_char_offset(cursor_row, cursor_col);
        let prev = matches
            .iter()
            .rposition(|(s, _)| *s < cursor_char)
            .unwrap_or(matches.len() - 1);
        self.search_match_idx = prev;
        let (row, col) = self.char_idx_to_pos(matches[prev].0);
        self.inner.move_cursor(CursorMove::Jump(row as u16, col as u16));
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Convert a char-level offset to (row, col), clamping to buffer bounds.
    fn char_idx_to_pos(&self, char_idx: usize) -> (usize, usize) {
        let lines = self.inner.lines();
        let mut remaining = char_idx;
        for (row, line) in lines.iter().enumerate() {
            let line_chars = line.chars().count();
            if remaining <= line_chars {
                return (row, remaining);
            }
            remaining -= line_chars + 1; // +1 for the logical newline
        }
        let last_row = lines.len().saturating_sub(1);
        let last_col = lines.last().map(|l| l.chars().count()).unwrap_or(0);
        (last_row, last_col)
    }

    /// Approximate char offset for the given (row, col) — used for search
    /// positioning. Treats each line as `line.len() + 1` chars (including newline).
    fn approx_char_offset(&self, row: usize, col: usize) -> usize {
        let lines = self.inner.lines();
        let mut offset = 0usize;
        for (i, line) in lines.iter().enumerate() {
            if i == row {
                return offset + col;
            }
            offset += line.len() + 1;
        }
        offset
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("editor buffer has no file path")]
    MissingPath,
    #[error("failed to read editor file {path}: {source}")]
    ReadFile { path: String, source: std::io::Error },
    #[error("failed to write editor file {path}: {source}")]
    WriteFile { path: String, source: std::io::Error },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{EditorBuffer, EditorError};

    fn temp_file_path(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("zeta-editor-{name}-{unique}.txt"))
    }

    #[test]
    fn opens_existing_file_contents() {
        let path = temp_file_path("open");
        fs::write(&path, "hello editor").expect("temp file should be written");

        let buffer = EditorBuffer::open(&path).expect("editor should open file");

        assert_eq!(buffer.contents(), "hello editor");
        assert!(!buffer.is_dirty);

        fs::remove_file(path).expect("temp file should be removed");
    }

    #[test]
    fn open_preserves_trailing_newline() {
        let path = temp_file_path("trailing-nl");
        fs::write(&path, "hello\n").expect("temp file should be written");

        let buffer = EditorBuffer::open(&path).expect("editor should open file");
        // contents() should end with "\n" because we preserve the trailing empty line
        assert!(buffer.contents().ends_with('\n'), "trailing newline should be preserved");

        fs::remove_file(path).expect("temp file should be removed");
    }

    #[test]
    fn save_persists_changes_and_clears_dirty_flag() {
        let path = temp_file_path("save");
        let mut buffer = EditorBuffer { path: Some(path.clone()), ..Default::default() };
        buffer.insert_char('h');
        buffer.insert_char('i');
        assert!(buffer.is_dirty);
        buffer.save().expect("editor should save file");
        assert!(!buffer.is_dirty);
        let saved = fs::read_to_string(&path).expect("saved file should be readable");
        assert!(saved.contains("hi"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn save_without_path_fails() {
        let mut buffer = EditorBuffer::default();
        buffer.insert_char('x');
        assert!(matches!(buffer.save(), Err(EditorError::MissingPath)));
    }

    #[test]
    fn typing_and_backspace_update_cursor() {
        let mut buffer = EditorBuffer::default();

        buffer.insert_char('a');
        buffer.insert_char('b');
        buffer.backspace();

        assert_eq!(buffer.contents(), "a");
        let (_, col) = buffer.cursor_line_col();
        assert_eq!(col, 1);
        assert!(buffer.is_dirty);
    }

    #[test]
    fn cursor_moves_between_lines() {
        let mut buffer = EditorBuffer::default();
        buffer.insert_char('a');
        buffer.insert_newline();
        buffer.insert_char('b');
        buffer.move_up();

        let (row, _) = buffer.cursor_line_col();
        assert_eq!(row, 0);

        buffer.move_down();
        let (row, _) = buffer.cursor_line_col();
        assert_eq!(row, 1);
    }

    #[test]
    fn visible_window_follows_cursor() {
        let mut buffer = EditorBuffer::default();
        for ch in ['a', '\n', 'b', '\n', 'c', '\n', 'd'] {
            if ch == '\n' {
                buffer.insert_newline();
            } else {
                buffer.insert_char(ch);
            }
        }
        let (start, visible) = buffer.visible_line_window(2);
        assert_eq!(start, 2);
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn horizontal_scroll_advances_when_cursor_moves_right_past_viewport() {
        let mut buffer = EditorBuffer::default();
        for _ in 0..20 {
            buffer.insert_char('x');
        }
        buffer.clamp_horizontal_scroll(10);
        let (_, col) = buffer.cursor_line_col();
        assert!(
            col >= buffer.scroll_col && col < buffer.scroll_col + 10,
            "cursor col {col} should be inside scroll window [{}, {})",
            buffer.scroll_col,
            buffer.scroll_col + 10
        );
        assert!(buffer.scroll_col > 0, "scroll_col should have advanced beyond 0");
    }

    #[test]
    fn horizontal_scroll_retreats_when_cursor_moves_left_past_scroll_origin() {
        let mut buffer = EditorBuffer::default();
        for _ in 0..20 {
            buffer.insert_char('x');
        }
        buffer.clamp_horizontal_scroll(10);
        assert!(buffer.scroll_col > 0, "precondition: scroll_col > 0");

        for _ in 0..20 {
            buffer.move_left();
        }
        buffer.clamp_horizontal_scroll(10);
        assert_eq!(buffer.scroll_col, 0, "scroll_col should retreat to 0");
    }

    #[test]
    fn undo_restores_previous_content() {
        let mut buffer = EditorBuffer::default();
        buffer.insert_char('a');
        buffer.insert_char('b');
        buffer.undo();
        let contents = buffer.contents();
        assert!(
            contents == "a" || contents.is_empty(),
            "unexpected after undo: {contents:?}"
        );
    }

    #[test]
    fn redo_reapplies_undone_change() {
        let mut buffer = EditorBuffer::default();
        buffer.insert_char('x');
        buffer.undo();
        buffer.redo();
        assert!(buffer.contents().contains('x'));
    }

    #[test]
    fn find_matches_returns_all_occurrences() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "foo bar foo baz foo");
        let matches = buffer.find_matches("foo");
        assert!(matches.len() >= 2, "expected >= 2 matches, got {}", matches.len());
    }

    #[test]
    fn find_matches_is_case_insensitive() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "Hello hello HELLO");
        let matches = buffer.find_matches("hello");
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn find_matches_empty_query_returns_nothing() {
        let mut buffer = EditorBuffer::default();
        buffer.insert_char('a');
        assert!(buffer.find_matches("").is_empty());
    }

    #[test]
    fn search_next_jumps_to_next_match() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "foo bar foo");
        buffer.search_query = String::from("foo");
        // Reset cursor to beginning of line so search_next finds the match *after* col 0.
        buffer.inner.move_cursor(tui_textarea::CursorMove::Head);
        buffer.search_next();
        let (row, col) = buffer.cursor_line_col();
        // Second "foo" starts at column 8 on row 0.
        assert_eq!((row, col), (0, 8), "should jump to second 'foo'");
    }

    #[test]
    fn search_next_wraps_around() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "foo bar foo");
        buffer.search_query = String::from("foo");
        // Place cursor past the last match — wrap should go to first match.
        buffer.inner.move_cursor(tui_textarea::CursorMove::Jump(0, 10));
        buffer.search_next();
        let (_, col) = buffer.cursor_line_col();
        assert_eq!(col, 0, "should wrap to first 'foo'");
    }

    #[test]
    fn search_prev_wraps_around_to_last_match() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "foo bar foo");
        buffer.search_query = String::from("foo");
        // Cursor at start — prev should wrap to last match.
        buffer.inner.move_cursor(tui_textarea::CursorMove::Top);
        buffer.search_prev();
        let (_, col) = buffer.cursor_line_col();
        assert_eq!(col, 8, "should wrap to last 'foo' at col 8");
    }
}
