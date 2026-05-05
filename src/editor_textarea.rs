use std::path::PathBuf;
use tui_textarea::{CursorMove, Input, Key, TextArea};

#[derive(Debug, thiserror::Error)]
pub enum TextAreaError {
    #[error("no file path set")]
    NoPath,
    #[error("write failed: {0}")]
    Io(#[from] std::io::Error),
}

pub struct TextAreaAdapter {
    path: Option<PathBuf>,
    is_dirty: bool,
    pub search_query: String,
    pub search_active: bool,
    pub search_match_idx: usize,
    pub scroll_col: usize,
    inner: TextArea<'static>,
    edit_version: u64,
    #[allow(dead_code)]
    search_matches: Vec<(usize, usize)>,
    #[allow(dead_code)]
    md_preview_cache: Option<MdPreviewCache>,
}

#[derive(Clone)]
pub struct MdPreviewCache {
    pub version: u64,
    pub panel_width: u16,
    pub theme: String,
    pub rendered: String,
}

impl TextAreaAdapter {
    /// Create from a Vec<String> of lines (may be empty).
    pub fn from_lines(lines: Vec<String>) -> Self {
        Self {
            path: None,
            is_dirty: false,
            search_query: String::new(),
            search_active: false,
            search_match_idx: 0,
            scroll_col: 0,
            inner: TextArea::new(lines),
            edit_version: 0,
            search_matches: Vec::new(),
            md_preview_cache: None,
        }
    }

    /// Create from a raw text string (split on '\n').
    pub fn from_text(text: &str) -> Self {
        let lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
        Self::from_lines(lines)
    }

    /// Create an empty buffer.
    pub fn new_empty() -> Self {
        Self::from_lines(vec![String::new()])
    }

    /// Increment edit_version, set is_dirty = true, clear md_preview_cache.
    fn bump(&mut self) {
        self.edit_version += 1;
        self.is_dirty = true;
        self.md_preview_cache = None;
    }

    /// All lines as a slice.
    pub fn lines(&self) -> &[String] {
        self.inner.lines()
    }

    /// Full text joined by '\n'.
    pub fn contents(&self) -> String {
        self.inner.lines().join("\n")
    }

    /// Current cursor position as (row, col).
    pub fn cursor(&self) -> (usize, usize) {
        self.inner.cursor()
    }

    /// Total line count.
    pub fn line_count(&self) -> usize {
        self.inner.lines().len()
    }

    /// Whether any unsaved changes exist.
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// The associated file path.
    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }

    /// Current edit version (monotonically increasing).
    pub fn edit_version(&self) -> u64 {
        self.edit_version
    }

    /// Builder: set the file path on construction.
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Write contents() to self.path. Returns Err if path is None or write fails.
    pub fn save(&mut self) -> Result<(), TextAreaError> {
        let path = self.path.as_ref().ok_or(TextAreaError::NoPath)?;
        std::fs::write(path, self.contents())?;
        self.is_dirty = false;
        Ok(())
    }

    // Character insertion

    /// Insert a single character at the cursor.
    pub fn insert_char(&mut self, ch: char) {
        self.inner.input(Input {
            key: Key::Char(ch),
            ctrl: false,
            alt: false,
            shift: false,
        });
        self.bump();
    }

    /// Insert a newline at the cursor.
    pub fn insert_newline(&mut self) {
        self.inner.input(Input {
            key: Key::Enter,
            ctrl: false,
            alt: false,
            shift: false,
        });
        self.bump();
    }

    // Deletion

    /// Delete character before cursor (backspace).
    pub fn backspace(&mut self) {
        self.inner.input(Input {
            key: Key::Backspace,
            ctrl: false,
            alt: false,
            shift: false,
        });
        self.bump();
    }

    /// Delete character at cursor (delete key).
    pub fn delete_char(&mut self) {
        self.inner.input(Input {
            key: Key::Delete,
            ctrl: false,
            alt: false,
            shift: false,
        });
        self.bump();
    }

    // Cursor movement (non-mutating — do NOT call bump())

    pub fn move_right(&mut self) {
        self.inner.move_cursor(CursorMove::Forward);
    }

    pub fn move_left(&mut self) {
        self.inner.move_cursor(CursorMove::Back);
    }

    pub fn move_up(&mut self) {
        self.inner.move_cursor(CursorMove::Up);
    }

    pub fn move_down(&mut self) {
        self.inner.move_cursor(CursorMove::Down);
    }

    pub fn move_word_right(&mut self) {
        self.inner.move_cursor(CursorMove::WordForward);
    }

    pub fn move_word_left(&mut self) {
        self.inner.move_cursor(CursorMove::WordBack);
    }

    pub fn move_line_start(&mut self) {
        self.inner.move_cursor(CursorMove::Head);
    }

    pub fn move_line_end(&mut self) {
        self.inner.move_cursor(CursorMove::End);
    }

    pub fn move_doc_start(&mut self) {
        self.inner.move_cursor(CursorMove::Top);
    }

    pub fn move_doc_end(&mut self) {
        self.inner.move_cursor(CursorMove::Bottom);
    }

    /// Jump to exact (row, col) — clamp to valid range if out of bounds.
    pub fn jump_to(&mut self, row: usize, col: usize) {
        let row = row.min(self.line_count().saturating_sub(1));
        let col = col.min(self.lines().get(row).map(|l| l.len()).unwrap_or(0));
        self.inner
            .move_cursor(CursorMove::Jump(row as u16, col as u16));
    }

    // Undo/Redo

    /// Returns true if undo was applied.
    pub fn undo(&mut self) -> bool {
        let did_undo = self.inner.undo();
        if did_undo {
            self.bump();
        }
        did_undo
    }

    /// Returns true if redo was applied.
    pub fn redo(&mut self) -> bool {
        let did_redo = self.inner.redo();
        if did_redo {
            self.bump();
        }
        did_redo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_text_produces_correct_line_count_and_contents() {
        let text = "line1\nline2\nline3";
        let adapter = TextAreaAdapter::from_text(text);
        
        assert_eq!(adapter.line_count(), 3);
        assert_eq!(adapter.contents(), text);
        assert_eq!(adapter.lines(), &["line1", "line2", "line3"]);
    }

    #[test]
    fn test_from_text_single_line() {
        let text = "single line";
        let adapter = TextAreaAdapter::from_text(text);
        
        assert_eq!(adapter.line_count(), 1);
        assert_eq!(adapter.contents(), text);
    }

    #[test]
    fn test_from_text_empty() {
        let text = "";
        let adapter = TextAreaAdapter::from_text(text);
        
        assert_eq!(adapter.line_count(), 1);
        assert_eq!(adapter.contents(), "");
    }

    #[test]
    fn test_from_lines_round_trips_correctly() {
        let lines = vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ];
        let adapter = TextAreaAdapter::from_lines(lines.clone());
        
        assert_eq!(adapter.lines(), lines.as_slice());
        assert_eq!(adapter.contents(), "first\nsecond\nthird");
        assert_eq!(adapter.line_count(), 3);
    }

    #[test]
    fn test_from_lines_empty_vec() {
        let lines = vec![];
        let adapter = TextAreaAdapter::from_lines(lines);
        
        // TextArea::new with empty vec creates one empty line
        assert_eq!(adapter.line_count(), 1);
        assert_eq!(adapter.contents(), "");
    }

    #[test]
    fn test_edit_version_starts_at_zero() {
        let adapter = TextAreaAdapter::new_empty();
        assert_eq!(adapter.edit_version(), 0);
    }

    #[test]
    fn test_is_dirty_starts_false() {
        let adapter = TextAreaAdapter::from_text("hello world");
        assert!(!adapter.is_dirty());
    }

    #[test]
    fn test_cursor_starts_at_zero_zero() {
        let adapter = TextAreaAdapter::from_text("line1\nline2");
        assert_eq!(adapter.cursor(), (0, 0));
    }

    #[test]
    fn test_new_empty_creates_single_empty_line() {
        let adapter = TextAreaAdapter::new_empty();
        assert_eq!(adapter.line_count(), 1);
        assert_eq!(adapter.contents(), "");
    }

    #[test]
    fn test_path_starts_as_none() {
        let adapter = TextAreaAdapter::new_empty();
        assert!(adapter.path().is_none());
    }

    #[test]
    fn test_bump_increments_version_and_sets_dirty() {
        let mut adapter = TextAreaAdapter::new_empty();
        assert_eq!(adapter.edit_version(), 0);
        assert!(!adapter.is_dirty());
        
        adapter.bump();
        
        assert_eq!(adapter.edit_version(), 1);
        assert!(adapter.is_dirty());
    }

    #[test]
    fn test_bump_clears_md_preview_cache() {
        let mut adapter = TextAreaAdapter::new_empty();
        adapter.md_preview_cache = Some(MdPreviewCache {
            version: 0,
            panel_width: 80,
            theme: "default".to_string(),
            rendered: "test".to_string(),
        });
        
        adapter.bump();
        
        assert!(adapter.md_preview_cache.is_none());
    }

    #[test]
    fn test_save_returns_error_when_no_path() {
        let mut adapter = TextAreaAdapter::from_text("content");
        let result = adapter.save();
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "no file path set");
    }

    #[test]
    fn test_save_clears_dirty_flag_on_success() {
        let temp_file = std::path::PathBuf::from("target/test_save_file.txt");
        let mut adapter = TextAreaAdapter::from_text("test content")
            .with_path(temp_file.clone());
        adapter.bump();

        let result = adapter.save();

        assert!(result.is_ok());
        assert!(!adapter.is_dirty());

        // Cleanup
        let _ = std::fs::remove_file(&temp_file);
    }

    // Movement and editing tests

    #[test]
    fn test_insert_char_adds_character_and_marks_dirty() {
        let mut adapter = TextAreaAdapter::from_text("hello");
        assert_eq!(adapter.edit_version(), 0);
        assert!(!adapter.is_dirty());
        
        adapter.insert_char('x');
        
        assert_eq!(adapter.contents(), "xhello");
        assert_eq!(adapter.edit_version(), 1);
        assert!(adapter.is_dirty());
    }

    #[test]
    fn test_insert_newline_increases_line_count() {
        let mut adapter = TextAreaAdapter::from_text("hello");
        assert_eq!(adapter.line_count(), 1);
        
        adapter.insert_newline();
        
        assert_eq!(adapter.line_count(), 2);
        assert_eq!(adapter.contents(), "\nhello");
        assert!(adapter.is_dirty());
    }

    #[test]
    fn test_backspace_removes_character() {
        let mut adapter = TextAreaAdapter::from_text("hello");
        adapter.move_right(); // Move cursor to position 1
        
        adapter.backspace();
        
        assert_eq!(adapter.contents(), "ello");
        assert!(adapter.is_dirty());
    }

    #[test]
    fn test_delete_char_removes_character_at_cursor() {
        let mut adapter = TextAreaAdapter::from_text("hello");
        // Cursor at (0, 0) - start of line
        
        adapter.delete_char();
        
        assert_eq!(adapter.contents(), "ello");
        assert!(adapter.is_dirty());
    }

    #[test]
    fn test_move_right_changes_cursor_position() {
        let mut adapter = TextAreaAdapter::from_text("hello");
        assert_eq!(adapter.cursor(), (0, 0));
        
        adapter.move_right();
        
        assert_eq!(adapter.cursor(), (0, 1));
        assert!(!adapter.is_dirty()); // Movement doesn't mark dirty
    }

    #[test]
    fn test_move_left_changes_cursor_position() {
        let mut adapter = TextAreaAdapter::from_text("hello");
        adapter.move_right();
        adapter.move_right();
        assert_eq!(adapter.cursor(), (0, 2));
        
        adapter.move_left();
        
        assert_eq!(adapter.cursor(), (0, 1));
        assert!(!adapter.is_dirty());
    }

    #[test]
    fn test_move_up_and_down_change_cursor_row() {
        let mut adapter = TextAreaAdapter::from_text("line1\nline2\nline3");
        adapter.move_down();
        assert_eq!(adapter.cursor().0, 1);
        
        adapter.move_down();
        assert_eq!(adapter.cursor().0, 2);
        
        adapter.move_up();
        assert_eq!(adapter.cursor().0, 1);
        
        assert!(!adapter.is_dirty());
    }

    #[test]
    fn test_move_word_right_and_left() {
        let mut adapter = TextAreaAdapter::from_text("hello world test");
        assert_eq!(adapter.cursor(), (0, 0));
        
        adapter.move_word_right();
        // Cursor should move to next word boundary
        let pos_after_first_word = adapter.cursor();
        assert!(pos_after_first_word.1 > 0);
        
        adapter.move_word_left();
        assert_eq!(adapter.cursor(), (0, 0));
        
        assert!(!adapter.is_dirty());
    }

    #[test]
    fn test_move_line_start_and_end() {
        let mut adapter = TextAreaAdapter::from_text("hello world");
        adapter.move_line_end();
        
        assert_eq!(adapter.cursor(), (0, 11)); // End of "hello world"
        
        adapter.move_line_start();
        assert_eq!(adapter.cursor(), (0, 0));
        
        assert!(!adapter.is_dirty());
    }

    #[test]
    fn test_move_doc_start_and_end() {
        let mut adapter = TextAreaAdapter::from_text("line1\nline2\nline3");
        adapter.move_doc_end();
        
        assert_eq!(adapter.cursor().0, 2); // Last line
        
        adapter.move_doc_start();
        assert_eq!(adapter.cursor(), (0, 0));
        
        assert!(!adapter.is_dirty());
    }

    #[test]
    fn test_jump_to_positions_cursor_correctly() {
        let mut adapter = TextAreaAdapter::from_text("line1\nline2\nline3");
        
        adapter.jump_to(1, 3);
        assert_eq!(adapter.cursor(), (1, 3));
        
        adapter.jump_to(2, 0);
        assert_eq!(adapter.cursor(), (2, 0));
        
        assert!(!adapter.is_dirty());
    }

    #[test]
    fn test_jump_to_clamps_out_of_bounds() {
        let mut adapter = TextAreaAdapter::from_text("abc\ndef");
        
        // Try to jump beyond last line
        adapter.jump_to(10, 0);
        assert_eq!(adapter.cursor().0, 1); // Should clamp to last line (row 1)
        
        // Try to jump beyond line length
        adapter.jump_to(0, 100);
        assert_eq!(adapter.cursor(), (0, 3)); // Should clamp to line length
    }

    #[test]
    fn test_undo_redo_round_trip() {
        let mut adapter = TextAreaAdapter::from_text("hello");
        let original_content = adapter.contents();
        
        // Make a change
        adapter.insert_char('x');
        assert_eq!(adapter.contents(), "xhello");
        assert_eq!(adapter.edit_version(), 1);
        
        // Undo the change
        let did_undo = adapter.undo();
        assert!(did_undo);
        assert_eq!(adapter.contents(), original_content);
        assert_eq!(adapter.edit_version(), 2); // Undo also bumps version
        
        // Redo the change
        let did_redo = adapter.redo();
        assert!(did_redo);
        assert_eq!(adapter.contents(), "xhello");
        assert_eq!(adapter.edit_version(), 3);
    }

    #[test]
    fn test_undo_returns_false_when_nothing_to_undo() {
        let mut adapter = TextAreaAdapter::from_text("hello");
        
        let did_undo = adapter.undo();
        
        assert!(!did_undo);
        assert_eq!(adapter.edit_version(), 0); // Version unchanged
    }

    #[test]
    fn test_redo_returns_false_when_nothing_to_redo() {
        let mut adapter = TextAreaAdapter::from_text("hello");
        
        let did_redo = adapter.redo();
        
        assert!(!did_redo);
        assert_eq!(adapter.edit_version(), 0); // Version unchanged
    }

    #[test]
    fn test_movement_does_not_call_bump() {
        let mut adapter = TextAreaAdapter::from_text("hello\nworld");
        let initial_version = adapter.edit_version();
        
        // Perform various movements
        adapter.move_right();
        adapter.move_down();
        adapter.move_left();
        adapter.move_up();
        adapter.move_line_end();
        adapter.move_line_start();
        adapter.jump_to(1, 2);
        
        // Version should remain unchanged
        assert_eq!(adapter.edit_version(), initial_version);
        assert!(!adapter.is_dirty());
    }
}
