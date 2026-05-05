use ratatui::text::Line;
use std::path::PathBuf;
use tui_textarea::{CursorMove, Input, Key, TextArea};

#[derive(Debug, thiserror::Error)]
pub enum TextAreaError {
    #[error("no file path set")]
    NoPath,
    #[error("write failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug)]
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
    md_preview_cache: Option<MdPreviewCache>,
}

#[derive(Clone, Debug)]
pub struct MdPreviewCache {
    pub version: u64,
    pub panel_width: u16,
    pub theme: String,
    pub rendered: Vec<Line<'static>>,
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

    /// Mark the buffer as clean (not dirty). Used after external save operations.
    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
    }

    // Character insertion

    /// Insert a single character at the cursor.
    pub fn insert_char(&mut self, ch: char) {
        if self.inner.input(Input {
            key: Key::Char(ch),
            ctrl: false,
            alt: false,
            shift: false,
        }) {
            self.bump();
        }
    }

    /// Insert a newline at the cursor.
    pub fn insert_newline(&mut self) {
        if self.inner.input(Input {
            key: Key::Enter,
            ctrl: false,
            alt: false,
            shift: false,
        }) {
            self.bump();
        }
    }

    /// Insert a string at the cursor position. Normalizes CRLF to LF.
    pub fn insert_str_at_cursor(&mut self, text: &str) {
        let normalized: String = text.chars().filter(|&c| c != '\r').collect();
        if normalized.is_empty() {
            return;
        }
        for ch in normalized.chars() {
            if ch == '\n' {
                self.insert_newline();
            } else {
                self.insert_char(ch);
            }
        }
    }

    /// Get all lines as a Vec<String> (for test compatibility with EditorBuffer).
    pub fn visible_lines(&self) -> Vec<String> {
        self.inner.lines().iter().map(|s| s.to_string()).collect()
    }

    // Deletion

    /// Delete character before cursor (backspace).
    pub fn backspace(&mut self) {
        if self.inner.input(Input {
            key: Key::Backspace,
            ctrl: false,
            alt: false,
            shift: false,
        }) {
            self.bump();
        }
    }

    /// Delete character at cursor (delete key).
    pub fn delete_char(&mut self) {
        if self.inner.input(Input {
            key: Key::Delete,
            ctrl: false,
            alt: false,
            shift: false,
        }) {
            self.bump();
        }
    }

    // Cursor movement (non-mutating — do NOT call bump())

    pub fn move_right(&mut self) {
        self.inner.cancel_selection();
        self.inner.move_cursor(CursorMove::Forward);
    }

    pub fn move_left(&mut self) {
        self.inner.cancel_selection();
        self.inner.move_cursor(CursorMove::Back);
    }

    pub fn move_up(&mut self) {
        self.inner.cancel_selection();
        self.inner.move_cursor(CursorMove::Up);
    }

    pub fn move_down(&mut self) {
        self.inner.cancel_selection();
        self.inner.move_cursor(CursorMove::Down);
    }

    pub fn move_word_right(&mut self) {
        self.inner.cancel_selection();
        self.inner.move_cursor(CursorMove::WordForward);
    }

    pub fn move_word_left(&mut self) {
        self.inner.cancel_selection();
        self.inner.move_cursor(CursorMove::WordBack);
    }

    pub fn move_line_start(&mut self) {
        self.inner.cancel_selection();
        self.inner.move_cursor(CursorMove::Head);
    }

    pub fn move_line_end(&mut self) {
        self.inner.cancel_selection();
        self.inner.move_cursor(CursorMove::End);
    }

    pub fn move_doc_start(&mut self) {
        self.inner.cancel_selection();
        self.inner.move_cursor(CursorMove::Top);
    }

    pub fn move_doc_end(&mut self) {
        self.inner.cancel_selection();
        self.inner.move_cursor(CursorMove::Bottom);
    }

    /// Jump to exact (row, col) — clamp to valid range if out of bounds.
    pub fn jump_to(&mut self, row: usize, col: usize) {
        let row = row.min(self.line_count().saturating_sub(1));
        let col = col.min(
            self.lines()
                .get(row)
                .map(|l| l.chars().count())
                .unwrap_or(0),
        );
        let row = row.min(u16::MAX as usize);
        let col = col.min(u16::MAX as usize);
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

    // Selection methods (Task 4)

    /// Start a selection anchor at the current cursor position.
    pub fn start_selection(&mut self) {
        self.inner.start_selection();
    }

    /// Cancel/clear the current selection.
    pub fn cancel_selection(&mut self) {
        self.inner.cancel_selection();
    }

    /// Extend selection right by one char.
    pub fn extend_selection_right(&mut self) {
        if self.inner.selection_range().is_none() {
            self.inner.start_selection();
        }
        self.inner.move_cursor(CursorMove::Forward);
    }

    /// Extend selection left by one char.
    pub fn extend_selection_left(&mut self) {
        if self.inner.selection_range().is_none() {
            self.inner.start_selection();
        }
        self.inner.move_cursor(CursorMove::Back);
    }

    /// Extend selection down one line.
    pub fn extend_selection_down(&mut self) {
        if self.inner.selection_range().is_none() {
            self.inner.start_selection();
        }
        self.inner.move_cursor(CursorMove::Down);
    }

    /// Extend selection up one line.
    pub fn extend_selection_up(&mut self) {
        if self.inner.selection_range().is_none() {
            self.inner.start_selection();
        }
        self.inner.move_cursor(CursorMove::Up);
    }

    /// Select all text in the buffer.
    pub fn select_all(&mut self) {
        self.inner.select_all();
    }

    /// Delete the current selection (if any). Returns true if text was deleted.
    pub fn delete_selection(&mut self) -> bool {
        let deleted = self.inner.cut();
        if deleted {
            self.bump();
            self.inner.cancel_selection(); // Clear selection after cut
        }
        deleted
    }

    /// Converts a char-column index to a byte offset in `s`.
    fn char_col_to_byte(s: &str, char_col: usize) -> usize {
        s.char_indices()
            .nth(char_col)
            .map(|(b, _)| b)
            .unwrap_or(s.len())
    }

    /// Get the currently selected text. Returns empty string if nothing selected.
    pub fn selected_text(&self) -> String {
        if let Some(((start_row, start_col), (end_row, end_col))) = self.inner.selection_range() {
            let lines = self.inner.lines();

            if start_row == end_row {
                let s = &lines[start_row];
                let b_start = Self::char_col_to_byte(s, start_col);
                let b_end = Self::char_col_to_byte(s, end_col);
                return s[b_start..b_end].to_string();
            }

            let mut result = String::new();
            let first = &lines[start_row];
            result.push_str(&first[Self::char_col_to_byte(first, start_col)..]);
            result.push('\n');

            for line in &lines[start_row + 1..end_row] {
                result.push_str(line);
                result.push('\n');
            }

            let last = &lines[end_row];
            result.push_str(&last[..Self::char_col_to_byte(last, end_col)]);
            result
        } else {
            String::new()
        }
    }

    // Clipboard methods (Task 5)

    /// Copy selected text to OS clipboard. Returns the copied text, or empty string.
    pub fn copy_to_os_clipboard(&mut self) -> String {
        let text = self.selected_text();
        if !text.is_empty() {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&text);
            }
        }
        text
    }

    /// Cut selected text to OS clipboard. Returns the cut text, or empty string.
    pub fn cut_to_os_clipboard(&mut self) -> String {
        // Use cut() which both gets the text AND deletes it
        if self.inner.selection_range().is_some() {
            let deleted = self.inner.cut();
            if deleted {
                self.bump();
                let text = self.inner.yank_text().to_string();
                if !text.is_empty() {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(&text);
                    }
                }
                return text;
            }
        }
        String::new()
    }

    /// Paste text from OS clipboard at cursor. Returns true if paste succeeded.
    pub fn paste_from_os_clipboard(&mut self) -> bool {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                if !text.is_empty() {
                    let mut did_insert = false;
                    for ch in text.chars() {
                        if ch == '\n' || ch == '\r' {
                            self.inner.input(Input {
                                key: Key::Enter,
                                ctrl: false,
                                alt: false,
                                shift: false,
                            });
                            did_insert = true;
                        } else {
                            self.inner.input(Input {
                                key: Key::Char(ch),
                                ctrl: false,
                                alt: false,
                                shift: false,
                            });
                            did_insert = true;
                        }
                    }
                    if did_insert {
                        self.bump(); // single bump for entire paste
                    }
                    return did_insert;
                }
            }
        }
        false
    }

    // Search and replace methods (Task 6)

    /// Set the search query and find all matches. Resets match index.
    /// Each match is (row, col) of the start of the match (char positions, not byte offsets).
    pub fn set_search_query(&mut self, query: String) {
        self.search_matches.clear();
        self.search_match_idx = usize::MAX; // wraps to 0 on first search_next
        self.search_active = !query.is_empty();

        if !query.is_empty() {
            let query_chars: Vec<char> = query.chars().collect();
            let qlen = query_chars.len();

            for (row, line) in self.inner.lines().iter().enumerate() {
                let line_chars: Vec<char> = line.chars().collect();
                let llen = line_chars.len();
                if llen < qlen {
                    continue;
                }
                let mut col = 0usize;
                while col + qlen <= llen {
                    if line_chars[col..col + qlen] == query_chars[..] {
                        self.search_matches.push((row, col));
                        col += qlen; // non-overlapping: skip past this match
                    } else {
                        col += 1;
                    }
                }
            }
        }

        self.search_query = query;
    }

    /// Move to the next search match. Wraps around. Returns false if no matches.
    pub fn search_next(&mut self) -> bool {
        let len = self.search_matches.len();
        if len == 0 {
            return false;
        }
        self.search_match_idx = self.search_match_idx.wrapping_add(1) % len;
        let (row, col) = self.search_matches[self.search_match_idx];
        self.jump_to(row, col);
        true
    }

    /// Move to the previous search match. Wraps around. Returns false if no matches.
    pub fn search_prev(&mut self) -> bool {
        let len = self.search_matches.len();
        if len == 0 {
            return false;
        }
        self.search_match_idx = if self.search_match_idx == 0 || self.search_match_idx == usize::MAX
        {
            len - 1
        } else {
            self.search_match_idx - 1
        };
        let (row, col) = self.search_matches[self.search_match_idx];
        self.jump_to(row, col);
        true
    }

    /// Replace the current match with replacement text. Returns false if no current match.
    pub fn replace_current(&mut self, replacement: &str) -> bool {
        if self.search_matches.is_empty() {
            return false;
        }
        let idx = if self.search_match_idx == usize::MAX {
            0
        } else {
            self.search_match_idx
        };
        let (row, col) = self.search_matches[idx];
        let query_len = self.search_query.chars().count();

        // Position cursor at match start
        self.jump_to(row, col);

        // Select the match
        self.start_selection();
        for _ in 0..query_len {
            self.extend_selection_right();
        }

        // Delete the selection
        self.delete_selection();

        // Insert replacement
        for ch in replacement.chars() {
            if ch == '\n' {
                self.insert_newline();
            } else {
                self.insert_char(ch);
            }
        }

        // Refresh search matches
        let query = self.search_query.clone();
        self.set_search_query(query);

        true
    }

    /// Replace all matches with replacement text. Returns the count of replacements.
    pub fn replace_all(&mut self, replacement: &str) -> usize {
        if self.search_matches.is_empty() {
            return 0;
        }

        let count = self.search_matches.len();
        let query_len = self.search_query.chars().count();

        // Replace from last to first to preserve positions
        for i in (0..self.search_matches.len()).rev() {
            let (row, col) = self.search_matches[i];

            // Position cursor at match start
            self.jump_to(row, col);

            // Select the match
            self.start_selection();
            for _ in 0..query_len {
                self.extend_selection_right();
            }

            // Delete the selection
            self.delete_selection();

            // Insert replacement
            for ch in replacement.chars() {
                if ch == '\n' {
                    self.insert_newline();
                } else {
                    self.insert_char(ch);
                }
            }
        }

        // Refresh search matches
        let query = self.search_query.clone();
        self.set_search_query(query);

        count
    }

    /// Clear the search state.
    pub fn clear_search(&mut self) {
        self.search_active = false;
        self.search_query = String::new();
        self.search_matches.clear();
        self.search_match_idx = 0;
    }

    /// Get the total number of search matches.
    pub fn match_count(&self) -> usize {
        self.search_matches.len()
    }

    /// Get the current match index (0-based).
    pub fn current_match_idx(&self) -> usize {
        self.search_match_idx
    }

    // Mouse selection methods (for app.rs click handling)

    fn display_col_to_char_offset(line: &str, display_col: usize, tab_width: usize) -> usize {
        let mut col = 0usize;
        let mut char_offset = 0usize;
        for ch in line.chars() {
            let ch_width = if ch == '\t' {
                tab_width - (col % tab_width)
            } else {
                1
            };
            if col + ch_width > display_col {
                break;
            }
            col += ch_width;
            char_offset += 1;
        }
        char_offset
    }

    /// Move cursor to the nearest char position for the given logical line and display_col
    /// (tab-expanded column), without modifying the selection anchor.
    fn move_to_line_display_col(&mut self, line: usize, display_col: usize, tab_width: u8) {
        let lines = self.inner.lines();
        let clamped_line = line.min(lines.len().saturating_sub(1));
        let line_text = &lines[clamped_line];

        let char_offset =
            Self::display_col_to_char_offset(line_text, display_col, tab_width as usize);

        self.jump_to(clamped_line, char_offset);
        self.inner.cancel_selection();
    }

    /// Set the selection anchor to the nearest char position for the given logical line
    /// and display_col, then move the cursor there. This begins a new drag-selection.
    pub fn start_selection_at_line_display_col(
        &mut self,
        line: usize,
        display_col: usize,
        tab_width: u8,
    ) {
        self.move_to_line_display_col(line, display_col, tab_width);
        self.inner.start_selection();
    }

    /// Extend the selection to the nearest char position for the given logical line
    /// and display_col, keeping the existing anchor.
    pub fn extend_selection_to_line_display_col(
        &mut self,
        line: usize,
        display_col: usize,
        tab_width: u8,
    ) {
        let lines = self.inner.lines();
        let clamped_line = line.min(lines.len().saturating_sub(1));
        let line_text = &lines[clamped_line];

        let char_offset =
            Self::display_col_to_char_offset(line_text, display_col, tab_width as usize);

        // If no selection is active, start one
        if self.inner.selection_range().is_none() {
            self.inner.start_selection();
        }

        // Move cursor to the target position
        self.jump_to(clamped_line, char_offset);
    }

    // Markdown preview cache methods (Task 7)

    /// Returns a cached preview string if the cache is valid for the given parameters.
    /// Cache is valid when: version matches current edit_version, panel_width matches, theme matches.
    pub fn md_preview_cached(&self, panel_width: u16, theme: &str) -> Option<&Vec<Line<'static>>> {
        self.md_preview_cache.as_ref().and_then(|c| {
            if c.version == self.edit_version && c.panel_width == panel_width && c.theme == theme {
                Some(&c.rendered)
            } else {
                None
            }
        })
    }

    /// Store a newly rendered preview in the cache.
    pub fn set_md_preview_cache(
        &mut self,
        panel_width: u16,
        theme: &str,
        rendered: Vec<Line<'static>>,
    ) {
        self.md_preview_cache = Some(MdPreviewCache {
            version: self.edit_version,
            panel_width,
            theme: theme.to_string(),
            rendered,
        });
    }

    /// Render the textarea using tui-textarea's built-in widget.
    pub fn render(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        is_focused: bool,
    ) {
        use ratatui::style::{Color, Modifier, Style};

        self.inner
            .set_line_number_style(Style::default().fg(Color::DarkGray));

        if is_focused {
            self.inner.set_cursor_style(
                Style::default()
                    .bg(Color::White)
                    .fg(Color::Black)
                    .add_modifier(Modifier::REVERSED),
            );
            self.inner
                .set_cursor_line_style(Style::default().bg(Color::DarkGray));
        } else {
            self.inner.set_cursor_style(Style::default());
            self.inner.set_cursor_line_style(Style::default());
        }

        self.inner
            .set_selection_style(Style::default().bg(Color::Blue).fg(Color::White));

        frame.render_widget(&self.inner, area);
    }

    /// Forwards a crossterm mouse event to the tui-textarea input handler.
    /// Returns true if the event caused a content change (e.g. scroll wheel moves cursor).
    pub fn handle_mouse_input(&mut self, event: crossterm::event::MouseEvent) -> bool {
        let input = tui_textarea::Input::from(event);
        let changed = self.inner.input(input);
        if changed {
            self.bump();
        }
        changed
    }

    /// Forwards a crossterm key event to the tui-textarea input handler.
    /// Returns true if the event caused a content change.
    pub fn handle_key_input(&mut self, event: crossterm::event::KeyEvent) -> bool {
        let input = tui_textarea::Input::from(event);
        let changed = self.inner.input(input);
        if changed {
            self.bump();
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_compiles() {
        let adapter = TextAreaAdapter::new_empty();
        // Verify the inner TextArea field is accessible for rendering.
        let _w = &adapter.inner;
    }

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
        use ratatui::text::Line;
        let mut adapter = TextAreaAdapter::new_empty();
        adapter.md_preview_cache = Some(MdPreviewCache {
            version: 0,
            panel_width: 80,
            theme: "default".to_string(),
            rendered: vec![Line::from("test")],
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
        let mut adapter = TextAreaAdapter::from_text("test content").with_path(temp_file.clone());
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
        // tui-textarea WordForward lands after "hello " (col 6, including trailing space)
        assert_eq!(adapter.cursor(), (0, 6));

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

    // Task 4: Selection tests

    #[test]
    fn test_select_all_selects_content() {
        let mut adapter = TextAreaAdapter::from_text("hello\nworld");

        adapter.select_all();
        let selected = adapter.selected_text();

        assert_eq!(selected, "hello\nworld");
    }

    #[test]
    fn test_start_and_cancel_selection() {
        let mut adapter = TextAreaAdapter::from_text("hello");

        adapter.start_selection();
        adapter.extend_selection_right();
        adapter.extend_selection_right();

        // Should have selected "he"
        let selected = adapter.selected_text();
        assert_eq!(selected, "he");

        adapter.cancel_selection();
        let selected_after_cancel = adapter.selected_text();
        assert_eq!(selected_after_cancel, "");
    }

    #[test]
    fn test_delete_selection_removes_text() {
        let mut adapter = TextAreaAdapter::from_text("hello world");

        adapter.select_all();
        let deleted = adapter.delete_selection();

        assert!(deleted);
        assert_eq!(adapter.contents(), "");
        assert!(adapter.is_dirty());
    }

    #[test]
    fn test_extend_selection_right() {
        let mut adapter = TextAreaAdapter::from_text("hello");

        adapter.start_selection();
        adapter.extend_selection_right();
        adapter.extend_selection_right();
        adapter.extend_selection_right();

        let selected = adapter.selected_text();
        assert_eq!(selected, "hel");
    }

    #[test]
    fn test_selected_text_empty_when_no_selection() {
        let adapter = TextAreaAdapter::from_text("hello");

        let selected = adapter.selected_text();
        assert_eq!(selected, "");
    }

    #[test]
    fn test_selected_text_unicode() {
        let mut adapter = TextAreaAdapter::from_text("héllo wörld");
        adapter.move_line_start();
        adapter.start_selection();
        // select "hél" (3 chars, but 'é' is 2 bytes)
        adapter.extend_selection_right();
        adapter.extend_selection_right();
        adapter.extend_selection_right();
        assert_eq!(adapter.selected_text(), "hél");
    }

    #[test]
    fn test_extend_selection_down() {
        let mut adapter = TextAreaAdapter::from_text("line1\nline2\nline3");

        adapter.start_selection();
        adapter.extend_selection_down();

        let selected = adapter.selected_text();
        // Should select from (0,0) to (1,0)
        assert!(selected.contains("line1"));
    }

    #[test]
    fn test_extend_selection_left() {
        let mut adapter = TextAreaAdapter::from_text("hello");
        adapter.move_line_end(); // Move to end

        adapter.start_selection();
        adapter.extend_selection_left();
        adapter.extend_selection_left();

        let selected = adapter.selected_text();
        assert_eq!(selected, "lo");
    }

    #[test]
    fn test_delete_selection_returns_false_when_no_selection() {
        let mut adapter = TextAreaAdapter::from_text("hello");

        let deleted = adapter.delete_selection();

        assert!(!deleted);
    }

    // Task 5: Clipboard tests

    #[test]
    fn test_copy_returns_selected_text() {
        let mut adapter = TextAreaAdapter::from_text("hello world");

        adapter.select_all();
        let copied = adapter.copy_to_os_clipboard();

        assert_eq!(copied, "hello world");
    }

    #[test]
    fn test_cut_removes_and_returns_text() {
        let mut adapter = TextAreaAdapter::from_text("hello");

        adapter.select_all();
        let cut = adapter.cut_to_os_clipboard();

        assert_eq!(cut, "hello");
        assert_eq!(adapter.contents(), "");
    }

    #[test]
    #[ignore] // May not work in all test environments
    fn test_paste_from_os_clipboard() {
        let mut adapter = TextAreaAdapter::from_text("");

        // Try to set clipboard and paste
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text("pasted");
            let pasted = adapter.paste_from_os_clipboard();

            if pasted {
                assert_eq!(adapter.contents(), "pasted");
            }
        }
    }

    // Task 6: Search and replace tests

    #[test]
    fn test_set_search_query_finds_matches() {
        let mut adapter = TextAreaAdapter::from_text("hello world hello");

        adapter.set_search_query("hello".to_string());

        assert_eq!(adapter.match_count(), 2);
        assert!(adapter.search_active);
    }

    #[test]
    fn test_search_next_wraps_around() {
        let mut adapter = TextAreaAdapter::from_text("abc abc abc");

        adapter.set_search_query("abc".to_string());
        assert_eq!(adapter.match_count(), 3);
        // search_match_idx starts at usize::MAX; first next() lands at 0

        adapter.search_next();
        assert_eq!(adapter.current_match_idx(), 0);

        adapter.search_next();
        assert_eq!(adapter.current_match_idx(), 1);

        adapter.search_next();
        assert_eq!(adapter.current_match_idx(), 2);

        adapter.search_next(); // Should wrap to 0
        assert_eq!(adapter.current_match_idx(), 0);
    }

    #[test]
    fn test_search_prev_goes_backwards() {
        let mut adapter = TextAreaAdapter::from_text("abc abc abc");

        adapter.set_search_query("abc".to_string());
        // search_match_idx = usize::MAX; prev() wraps to last (2)

        adapter.search_prev();
        assert_eq!(adapter.current_match_idx(), 2);

        adapter.search_prev();
        assert_eq!(adapter.current_match_idx(), 1);
    }

    #[test]
    fn test_replace_current_replaces_text() {
        let mut adapter = TextAreaAdapter::from_text("hello world");

        adapter.set_search_query("hello".to_string());
        assert_eq!(adapter.match_count(), 1);

        let replaced = adapter.replace_current("goodbye");

        assert!(replaced);
        assert_eq!(adapter.contents(), "goodbye world");
    }

    #[test]
    fn test_replace_all_replaces_all_occurrences() {
        let mut adapter = TextAreaAdapter::from_text("foo bar foo baz foo");

        adapter.set_search_query("foo".to_string());
        assert_eq!(adapter.match_count(), 3);

        let count = adapter.replace_all("qux");

        assert_eq!(count, 3);
        assert_eq!(adapter.contents(), "qux bar qux baz qux");
    }

    #[test]
    fn test_clear_search_resets_state() {
        let mut adapter = TextAreaAdapter::from_text("hello world");

        adapter.set_search_query("hello".to_string());
        assert!(adapter.search_active);
        assert_eq!(adapter.match_count(), 1);

        adapter.clear_search();

        assert!(!adapter.search_active);
        assert_eq!(adapter.match_count(), 0);
        assert_eq!(adapter.search_query, "");
    }

    #[test]
    fn test_search_next_returns_false_when_no_matches() {
        let mut adapter = TextAreaAdapter::from_text("hello");

        adapter.set_search_query("xyz".to_string());

        let found = adapter.search_next();
        assert!(!found);
    }

    #[test]
    fn test_replace_current_returns_false_when_no_matches() {
        let mut adapter = TextAreaAdapter::from_text("hello");

        adapter.set_search_query("xyz".to_string());

        let replaced = adapter.replace_current("abc");
        assert!(!replaced);
    }

    #[test]
    fn test_selection_does_not_mark_dirty() {
        let mut adapter = TextAreaAdapter::from_text("hello");
        let version = adapter.edit_version();

        adapter.start_selection();
        adapter.extend_selection_right();
        adapter.cancel_selection();
        adapter.select_all();

        // Selection operations shouldn't bump version
        assert_eq!(adapter.edit_version(), version);
        assert!(!adapter.is_dirty());
    }

    // Unicode regression tests

    #[test]
    fn test_search_unicode_finds_matches() {
        // "Résumé café": R(0)é(1)s(2)u(3)m(4)é(5) (6)c(7)a(8)f(9)é(10)
        // "é" appears at char positions 1, 5, 10 → 3 matches
        let mut adapter = TextAreaAdapter::from_text("Résumé café");
        adapter.set_search_query("é".to_string());
        assert_eq!(adapter.match_count(), 3);
        assert!(adapter.search_active);
    }

    #[test]
    fn test_search_unicode_cursor_position() {
        // "héllo": h(0)é(1)l(2)l(3)o(4) → "l" at char positions 2 and 3
        let mut adapter = TextAreaAdapter::from_text("héllo");
        adapter.set_search_query("l".to_string());
        assert_eq!(adapter.match_count(), 2);

        adapter.search_next(); // first match: col 2
        assert_eq!(adapter.cursor().1, 2);

        adapter.search_next(); // second match: col 3
        assert_eq!(adapter.cursor().1, 3);
    }

    #[test]
    fn test_search_multibyte_query_no_panic() {
        // Ensures a multi-byte query like "é" (2 UTF-8 bytes) doesn't panic
        let mut adapter = TextAreaAdapter::from_text("naïve café résumé");
        adapter.set_search_query("é".to_string());
        // Just verify it runs without panic and finds matches
        assert!(adapter.match_count() > 0);
    }

    // Markdown preview cache tests (Task 7)

    #[test]
    fn test_md_preview_cache_miss_on_fresh_buffer() {
        let adapter = TextAreaAdapter::from_text("# Hello");
        let result = adapter.md_preview_cached(80, "dark");
        assert!(result.is_none());
    }

    #[test]
    fn test_md_preview_cache_hit() {
        use ratatui::text::Line;
        let mut adapter = TextAreaAdapter::from_text("# Hello");
        let rendered = vec![Line::from("<h1>Hello</h1>")];
        adapter.set_md_preview_cache(80, "dark", rendered.clone());

        let result = adapter.md_preview_cached(80, "dark");
        assert!(result.is_some());
        let cached = result.unwrap();
        assert_eq!(cached.len(), 1);
    }

    #[test]
    fn test_md_preview_cache_invalidated_after_edit() {
        use ratatui::text::Line;
        let mut adapter = TextAreaAdapter::from_text("# Hello");
        let rendered = vec![Line::from("<h1>Hello</h1>")];
        adapter.set_md_preview_cache(80, "dark", rendered);

        // Verify cache works before edit
        assert!(adapter.md_preview_cached(80, "dark").is_some());

        // Make an edit
        adapter.insert_char('x');

        // Cache should be cleared by bump()
        let result = adapter.md_preview_cached(80, "dark");
        assert!(result.is_none());
    }

    #[test]
    fn test_md_preview_cache_miss_on_wrong_width() {
        use ratatui::text::Line;
        let mut adapter = TextAreaAdapter::from_text("# Hello");
        let rendered = vec![Line::from("<h1>Hello</h1>")];
        adapter.set_md_preview_cache(80, "dark", rendered);

        // Query with different width
        let result = adapter.md_preview_cached(100, "dark");
        assert!(result.is_none());
    }

    #[test]
    fn test_md_preview_cache_miss_on_wrong_theme() {
        use ratatui::text::Line;
        let mut adapter = TextAreaAdapter::from_text("# Hello");
        let rendered = vec![Line::from("<h1>Hello</h1>")];
        adapter.set_md_preview_cache(80, "dark", rendered);

        // Query with different theme
        let result = adapter.md_preview_cached(80, "light");
        assert!(result.is_none());
    }
}
