use std::fs as std_fs;
use std::path::{Path, PathBuf};

use ropey::Rope;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorRenderState {
    pub visible_start: usize,
    pub visible_lines: Vec<String>,
    pub cursor_visible_row: Option<usize>,
    pub scroll_col: usize,
}

#[derive(Clone, Debug)]
pub struct EditorBuffer {
    pub path: Option<PathBuf>,
    pub cursor_char_idx: usize,
    pub scroll_col: usize,
    pub text: Rope,
    pub is_dirty: bool,
    // Search
    pub search_query: String,
    pub search_active: bool,
    pub search_match_idx: usize,
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self {
            path: None,
            cursor_char_idx: 0,
            scroll_col: 0,
            text: Rope::new(),
            is_dirty: false,
            search_query: String::new(),
            search_active: false,
            search_match_idx: 0,
        }
    }
}

impl EditorBuffer {
    pub fn open(path: &Path) -> Result<Self, EditorError> {
        let bytes = std_fs::read(path).map_err(|source| EditorError::ReadFile {
            path: path.display().to_string(),
            source,
        })?;
        let contents = String::from_utf8_lossy(&bytes);

        Ok(Self {
            path: Some(path.to_path_buf()),
            cursor_char_idx: 0,
            scroll_col: 0,
            text: Rope::from_str(&contents),
            is_dirty: false,
            search_query: String::new(),
            search_active: false,
            search_match_idx: 0,
        })
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        self.text.insert(char_idx, text);
        self.is_dirty = true;
    }

    pub fn insert_char(&mut self, ch: char) {
        self.text.insert_char(self.cursor_char_idx, ch);
        self.cursor_char_idx += 1;
        self.is_dirty = true;
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn backspace(&mut self) {
        if self.cursor_char_idx == 0 {
            return;
        }

        let start = self.cursor_char_idx - 1;
        self.text.remove(start..self.cursor_char_idx);
        self.cursor_char_idx = start;
        self.is_dirty = true;
    }

    pub fn move_left(&mut self) {
        self.cursor_char_idx = self.cursor_char_idx.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.cursor_char_idx = (self.cursor_char_idx + 1).min(self.text.len_chars());
    }

    pub fn move_up(&mut self) {
        let (line, column) = self.cursor_line_col();
        if line == 0 {
            return;
        }

        let target_line = line - 1;
        self.cursor_char_idx = self.line_to_char_with_column(target_line, column);
    }

    pub fn move_down(&mut self) {
        let (line, column) = self.cursor_line_col();
        let total_lines = self.text.len_lines();
        if line + 1 >= total_lines {
            return;
        }

        let target_line = line + 1;
        self.cursor_char_idx = self.line_to_char_with_column(target_line, column);
    }

    pub fn save(&mut self) -> Result<(), EditorError> {
        let path = self.path.clone().ok_or(EditorError::MissingPath)?;
        std_fs::write(&path, self.text.to_string()).map_err(|source| EditorError::WriteFile {
            path: path.display().to_string(),
            source,
        })?;
        self.is_dirty = false;
        Ok(())
    }

    pub fn contents(&self) -> String {
        self.text.to_string()
    }

    pub fn visible_lines(&self) -> Vec<String> {
        self.text.lines().map(|line| line.to_string()).collect()
    }

    pub fn visible_line_window(&self, height: usize) -> (usize, Vec<String>) {
        if height == 0 {
            return (0, Vec::new());
        }

        let lines = self.visible_lines();
        let (cursor_line, _) = self.cursor_line_col();
        let start = if cursor_line >= height {
            cursor_line + 1 - height
        } else {
            0
        };

        let visible = lines.into_iter().skip(start).take(height).collect();
        (start, visible)
    }

    pub fn cursor_line_col(&self) -> (usize, usize) {
        let safe_idx = self.cursor_char_idx.min(self.text.len_chars());
        let line = self.text.char_to_line(safe_idx);
        let line_start = self.text.line_to_char(line);
        (line, safe_idx.saturating_sub(line_start))
    }

    /// Returns all `(char_idx_start, char_idx_end)` pairs for case-insensitive
    /// query matches within the buffer text.
    pub fn find_matches(&self, query: &str) -> Vec<(usize, usize)> {
        if query.is_empty() {
            return vec![];
        }
        let text = self.text.to_string();
        let query_lower = query.to_lowercase();
        let text_lower = text.to_lowercase();
        let mut matches = Vec::new();
        let mut byte_start = 0;

        while byte_start < text_lower.len() {
            let slice = &text_lower[byte_start..];
            let Some(byte_pos) = slice.find(&query_lower) else {
                break;
            };

            let abs_byte = byte_start + byte_pos;
            let end_byte = abs_byte + query_lower.len();

            // Convert byte offsets from the lowered text into Ropey char indices.
            let start_char = self.text.byte_to_char(abs_byte);
            let end_char = self.text.byte_to_char(end_byte);
            matches.push((start_char, end_char));

            let next_char_len = text_lower[abs_byte..]
                .chars()
                .next()
                .map(|ch| ch.len_utf8())
                .unwrap_or(1);
            byte_start = abs_byte + next_char_len;
        }
        matches
    }

    /// Jump the cursor to the next match after the current cursor position,
    /// wrapping around to the first match when the end is reached.
    pub fn search_next(&mut self) {
        let matches = self.find_matches(&self.search_query);
        if matches.is_empty() {
            self.search_match_idx = 0;
            return;
        }
        let next = matches
            .iter()
            .position(|(s, _)| *s > self.cursor_char_idx)
            .unwrap_or(0);
        self.search_match_idx = next;
        self.cursor_char_idx = matches[next].0;
    }

    /// Jump the cursor to the previous match before the current cursor position,
    /// wrapping around to the last match when the beginning is reached.
    pub fn search_prev(&mut self) {
        let matches = self.find_matches(&self.search_query);
        if matches.is_empty() {
            self.search_match_idx = 0;
            return;
        }
        let prev = matches
            .iter()
            .rposition(|(s, _)| *s < self.cursor_char_idx)
            .unwrap_or(matches.len() - 1);
        self.search_match_idx = prev;
        self.cursor_char_idx = matches[prev].0;
    }

    /// Called by the renderer after layout is known.
    /// Adjusts `scroll_col` so the cursor column is always visible within
    /// the given `viewport_cols` wide content area.
    pub fn clamp_horizontal_scroll(&mut self, viewport_cols: usize) {
        let (_, col) = self.cursor_line_col();
        if col < self.scroll_col {
            self.scroll_col = col;
        } else if viewport_cols > 0 && col >= self.scroll_col + viewport_cols {
            self.scroll_col = col.saturating_sub(viewport_cols) + 1;
        }
    }

    pub fn render_state(
        &mut self,
        height: usize,
        viewport_cols: usize,
        is_active: bool,
    ) -> EditorRenderState {
        self.clamp_horizontal_scroll(viewport_cols);
        let (visible_start, visible_lines) = self.visible_line_window(height);

        EditorRenderState {
            visible_start,
            visible_lines,
            cursor_visible_row: if is_active {
                Some(self.cursor_line_col().0.saturating_sub(visible_start))
            } else {
                None
            },
            scroll_col: self.scroll_col,
        }
    }

    fn line_to_char_with_column(&self, line: usize, column: usize) -> usize {
        let line_start = self.text.line_to_char(line);
        let line_len = self.visible_line_len(line);
        line_start + column.min(line_len)
    }

    fn visible_line_len(&self, line: usize) -> usize {
        let line_slice = self.text.line(line);
        let len = line_slice.len_chars();
        if len > 0 && line_slice.char(len - 1) == '\n' {
            len - 1
        } else {
            len
        }
    }
}

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("editor buffer has no file path")]
    MissingPath,
    #[error("failed to read editor file {path}: {source}")]
    ReadFile {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to write editor file {path}: {source}")]
    WriteFile {
        path: String,
        source: std::io::Error,
    },
}

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
        fs::write(&path, "hello editor\n").expect("temp file should be written");

        let buffer = EditorBuffer::open(&path).expect("editor should open file");

        assert_eq!(buffer.contents(), "hello editor\n");
        assert!(!buffer.is_dirty);

        fs::remove_file(path).expect("temp file should be removed");
    }

    #[test]
    fn save_persists_changes_and_clears_dirty_flag() {
        let path = temp_file_path("save");
        fs::write(&path, "hello").expect("temp file should be written");

        let mut buffer = EditorBuffer::open(&path).expect("editor should open file");
        buffer.insert(buffer.text.len_chars(), " world");
        buffer.save().expect("editor should save file");

        let saved = fs::read_to_string(&path).expect("saved file should be readable");
        assert_eq!(saved, "hello world");
        assert!(!buffer.is_dirty);

        fs::remove_file(path).expect("temp file should be removed");
    }

    #[test]
    fn save_without_path_fails() {
        let mut buffer = EditorBuffer::default();

        let error = buffer.save().expect_err("save should fail without path");
        assert!(matches!(error, EditorError::MissingPath));
    }

    #[test]
    fn typing_and_backspace_update_cursor() {
        let mut buffer = EditorBuffer::default();

        buffer.insert_char('a');
        buffer.insert_char('b');
        buffer.backspace();

        assert_eq!(buffer.contents(), "a");
        assert_eq!(buffer.cursor_line_col(), (0, 1));
        assert!(buffer.is_dirty);
    }

    #[test]
    fn cursor_moves_between_lines() {
        let mut buffer = EditorBuffer::default();
        buffer.insert_char('a');
        buffer.insert_newline();
        buffer.insert_char('b');
        buffer.move_up();

        assert_eq!(buffer.cursor_line_col(), (0, 1));

        buffer.move_down();
        assert_eq!(buffer.cursor_line_col(), (1, 1));
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
        // Write 20 characters on one line.
        for _ in 0..20 {
            buffer.insert_char('x');
        }
        // Cursor is now at column 20; viewport is 10 wide.
        // scroll_col should pan so the cursor stays visible.
        buffer.clamp_horizontal_scroll(10);
        let (_, col) = buffer.cursor_line_col();
        assert!(
            col >= buffer.scroll_col && col < buffer.scroll_col + 10,
            "cursor col {col} should be inside scroll window [{}, {})",
            buffer.scroll_col,
            buffer.scroll_col + 10
        );
        assert!(
            buffer.scroll_col > 0,
            "scroll_col should have advanced beyond 0"
        );
    }

    #[test]
    fn find_matches_returns_all_occurrences() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "foo bar foo baz foo");

        let matches = buffer.find_matches("foo");

        assert_eq!(matches, vec![(0, 3), (8, 11), (16, 19)]);
    }

    #[test]
    fn find_matches_is_case_insensitive() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "Hello hello HELLO");

        let matches = buffer.find_matches("hello");

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].0, 0);
        assert_eq!(matches[1].0, 6);
        assert_eq!(matches[2].0, 12);
    }

    #[test]
    fn find_matches_empty_query_returns_nothing() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "some text");

        let matches = buffer.find_matches("");

        assert!(matches.is_empty());
    }

    #[test]
    fn search_next_jumps_to_next_match() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "foo bar foo");
        buffer.search_query = String::from("foo");
        // cursor starts at 0 (on first match); search_next should find the second
        buffer.cursor_char_idx = 0;
        buffer.search_next();

        // The second "foo" starts at byte/char index 8
        assert_eq!(buffer.cursor_char_idx, 8);
    }

    #[test]
    fn search_next_wraps_around() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "foo bar foo");
        buffer.search_query = String::from("foo");
        // Place cursor past the last match so wrap-around triggers
        buffer.cursor_char_idx = 9;
        buffer.search_next();

        // Wraps to the first match at 0
        assert_eq!(buffer.cursor_char_idx, 0);
    }

    #[test]
    fn search_prev_jumps_to_previous_match() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "foo bar foo");
        buffer.search_query = String::from("foo");
        // Place cursor AT the second match (index 8); prev should find the
        // last match whose start < 8, which is the first "foo" at index 0.
        buffer.cursor_char_idx = 8;
        buffer.search_prev();

        // The first "foo" starts at 0
        assert_eq!(buffer.cursor_char_idx, 0);
    }

    #[test]
    fn search_prev_wraps_around_to_last_match() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "foo bar foo");
        buffer.search_query = String::from("foo");
        // Place cursor before the first match so wrap-around triggers
        buffer.cursor_char_idx = 0;
        buffer.search_prev();

        // Wraps to the last match at index 8
        assert_eq!(buffer.cursor_char_idx, 8);
    }

    #[test]
    fn find_matches_returns_char_spans_for_unicode_text() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "café café");

        let matches = buffer.find_matches("café");

        assert_eq!(matches, vec![(0, 4), (5, 9)]);
    }

    #[test]
    fn search_next_jumps_to_unicode_match() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "ありがとう ありがとう");
        buffer.search_query = String::from("ありがとう");
        buffer.cursor_char_idx = 0;

        buffer.search_next();

        assert_eq!(buffer.cursor_char_idx, 6);
        assert_eq!(buffer.search_match_idx, 1);
    }

    #[test]
    fn find_matches_returns_char_spans_for_japanese_text() {
        let mut buffer = EditorBuffer::default();
        buffer.insert(0, "ありがとう ありがとう");

        let matches = buffer.find_matches("ありがとう");

        assert_eq!(matches, vec![(0, 5), (6, 11)]);
    }

    #[test]
    fn horizontal_scroll_retreats_when_cursor_moves_left_past_scroll_origin() {
        let mut buffer = EditorBuffer::default();
        // Write 20 characters, then scroll right.
        for _ in 0..20 {
            buffer.insert_char('x');
        }
        buffer.clamp_horizontal_scroll(10);
        let scroll_after_right = buffer.scroll_col;
        assert!(scroll_after_right > 0, "precondition: scroll_col > 0");

        // Move cursor all the way back to column 0.
        for _ in 0..20 {
            buffer.move_left();
        }
        buffer.clamp_horizontal_scroll(10);

        assert_eq!(
            buffer.scroll_col, 0,
            "scroll_col should retreat to 0 when cursor is at column 0"
        );
    }
}
