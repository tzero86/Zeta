use std::fs as std_fs;
use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier};
use ropey::Rope;
use thiserror::Error;

use crate::highlight::{highlight_text, normalize_preview_text, HighlightedLine};

// ---------------------------------------------------------------------------
// Public render state
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorRenderState {
    pub visible_start: usize,
    pub visible_lines: Vec<String>,
    pub cursor_visible_row: Option<usize>,
    pub scroll_col: usize,
}

// ---------------------------------------------------------------------------
// Undo stack — delta-based, O(delta_len) per entry regardless of file size
// ---------------------------------------------------------------------------

/// A single reversible edit.
#[derive(Clone, Debug)]
struct Edit {
    /// Char index where the change begins.
    char_start: usize,
    /// Text that was present at `char_start` before the edit (empty for
    /// pure insertions).
    removed: String,
    /// Text that replaced it (empty for pure deletions).
    inserted: String,
    /// Cursor position before this edit — restored on undo.
    cursor_before: usize,
    /// Cursor position after this edit — restored on redo.
    cursor_after: usize,
}

#[derive(Clone, Debug, Default)]
struct UndoStack {
    entries: Vec<Edit>,
    /// Next write slot. `head == entries.len()` means nothing to redo.
    head: usize,
}

impl UndoStack {
    /// Record a new edit, discarding any redo history above `head`.
    fn push(&mut self, edit: Edit) {
        self.entries.truncate(self.head);
        self.entries.push(edit);
        self.head = self.entries.len();
    }

    /// Return the edit to undo and decrement head, or `None` if at bottom.
    fn pop_undo(&mut self) -> Option<&Edit> {
        if self.head == 0 {
            return None;
        }
        self.head -= 1;
        Some(&self.entries[self.head])
    }

    /// Return the edit to redo and increment head, or `None` if at top.
    fn pop_redo(&mut self) -> Option<&Edit> {
        if self.head == self.entries.len() {
            return None;
        }
        let edit = &self.entries[self.head];
        self.head += 1;
        Some(edit)
    }
}

// ---------------------------------------------------------------------------
// EditorBuffer
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct EditorBuffer {
    pub path: Option<PathBuf>,
    pub is_dirty: bool,
    pub search_query: String,
    pub search_active: bool,
    pub search_match_idx: usize,
    pub scroll_col: usize,
    text: Rope,
    cursor_char_idx: usize,
    history: UndoStack,
    /// Increments on every text-changing operation (insert, delete, undo, redo).
    /// Used as the cache key for `highlight_cache` — scrolling alone never
    /// increments this, so highlighting is free during scroll.
    edit_version: usize,
    /// Full-file syntax-highlighted lines, keyed by `(edit_version, theme)`.
    /// `None` until first render. Recomputed only when text actually changes.
    highlight_cache: Option<(usize, String, Vec<HighlightedLine>)>,
}

impl EditorBuffer {
    pub fn from_text(path: PathBuf, contents: String) -> Self {
        Self {
            path: Some(path),
            text: Rope::from_str(&contents),
            ..Self::default()
        }
    }

    pub fn open(path: &Path) -> Result<Self, EditorError> {
        let bytes = std_fs::read(path).map_err(|source| EditorError::ReadFile {
            path: path.display().to_string(),
            source,
        })?;
        let contents = String::from_utf8_lossy(&bytes);
        Ok(Self::from_text(path.to_path_buf(), contents.into_owned()))
    }

    // -----------------------------------------------------------------------
    // Compatibility insert — used by integration tests in state/mod.rs
    // -----------------------------------------------------------------------

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        let idx = char_idx.min(self.text.len_chars());
        let edit = Edit {
            char_start: idx,
            removed: String::new(),
            inserted: text.to_string(),
            cursor_before: self.cursor_char_idx,
            cursor_after: idx + text.chars().count(),
        };
        self.text.insert(idx, text);
        self.cursor_char_idx = edit.cursor_after;
        self.history.push(edit);
        self.is_dirty = true;
        self.edit_version += 1;
    }

    // -----------------------------------------------------------------------
    // Text mutation
    // -----------------------------------------------------------------------

    pub fn insert_char(&mut self, ch: char) {
        let idx = self.cursor_char_idx.min(self.text.len_chars());
        let edit = Edit {
            char_start: idx,
            removed: String::new(),
            inserted: ch.to_string(),
            cursor_before: self.cursor_char_idx,
            cursor_after: idx + 1,
        };
        self.text.insert_char(idx, ch);
        self.cursor_char_idx = edit.cursor_after;
        self.history.push(edit);
        self.is_dirty = true;
        self.edit_version += 1;
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn backspace(&mut self) {
        if self.cursor_char_idx == 0 {
            return;
        }
        let start = self.cursor_char_idx - 1;
        let removed_char = self.text.char(start).to_string();
        let edit = Edit {
            char_start: start,
            removed: removed_char,
            inserted: String::new(),
            cursor_before: self.cursor_char_idx,
            cursor_after: start,
        };
        self.text.remove(start..self.cursor_char_idx);
        self.cursor_char_idx = start;
        self.history.push(edit);
        self.is_dirty = true;
        self.edit_version += 1;
    }

    pub fn move_left(&mut self) {
        self.cursor_char_idx = self.cursor_char_idx.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.cursor_char_idx =
            (self.cursor_char_idx + 1).min(self.text.len_chars());
    }

    pub fn move_up(&mut self) {
        let (line, col) = self.cursor_line_col();
        if line == 0 {
            return;
        }
        self.cursor_char_idx = self.line_col_to_char(line - 1, col);
    }

    pub fn move_down(&mut self) {
        let (line, col) = self.cursor_line_col();
        if line + 1 >= self.text.len_lines() {
            return;
        }
        self.cursor_char_idx = self.line_col_to_char(line + 1, col);
    }

    // -----------------------------------------------------------------------
    // Undo / redo
    // -----------------------------------------------------------------------

    /// Undo the most recent edit.
    pub fn undo(&mut self) {
        // Clone to avoid borrowing `self` while mutating it.
        let Some(edit) = self.history.pop_undo().cloned() else { return };
        let insert_end = edit.char_start + edit.inserted.chars().count();
        if insert_end > edit.char_start {
            self.text.remove(edit.char_start..insert_end);
        }
        if !edit.removed.is_empty() {
            self.text.insert(edit.char_start, &edit.removed);
        }
        self.cursor_char_idx = edit.cursor_before;
        self.is_dirty = self.text.len_chars() > 0;
    }

    /// Redo the most recently undone edit.
    pub fn redo(&mut self) {
        let Some(edit) = self.history.pop_redo().cloned() else { return };
        let removed_end = edit.char_start + edit.removed.chars().count();
        if removed_end > edit.char_start {
            self.text.remove(edit.char_start..removed_end);
        }
        if !edit.inserted.is_empty() {
            self.text.insert(edit.char_start, &edit.inserted);
        }
        self.cursor_char_idx = edit.cursor_after;
        self.is_dirty = true;
        self.edit_version += 1;
    }

    // -----------------------------------------------------------------------
    // Persistence
    // -----------------------------------------------------------------------

    pub fn save(&mut self) -> Result<(), EditorError> {
        let path = self.path.clone().ok_or(EditorError::MissingPath)?;
        std_fs::write(&path, self.text.to_string()).map_err(|source| {
            EditorError::WriteFile { path: path.display().to_string(), source }
        })?;
        self.is_dirty = false;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Content accessors
    // -----------------------------------------------------------------------

    pub fn contents(&self) -> String {
        self.text.to_string()
    }

    pub fn visible_lines(&self) -> Vec<String> {
        self.text
            .lines()
            .map(|l| {
                let s = normalize_preview_text(&l.to_string());
                s.strip_suffix('\n').unwrap_or(&s).to_string()
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    /// Returns syntax-highlighted lines for the visible viewport.
    ///
    /// The full-file highlight result is cached by `edit_version` so repeated
    /// calls during scrolling (no text change) cost only a slice — not a full
    /// syntect re-parse.
    pub fn visible_highlighted_window(
        &mut self,
        height: usize,
        syntect_theme: &str,
        fallback_color: Color,
    ) -> (usize, &[HighlightedLine]) {
        if height == 0 {
            return (0, &[]);
        }

        let ext = self
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .map(str::to_owned);

        // Recompute only when text has changed or theme/ext has changed.
        let cache_valid = self
            .highlight_cache
            .as_ref()
            .is_some_and(|(v, t, _)| *v == self.edit_version && t == syntect_theme);

        if !cache_valid {
            let text = normalize_preview_text(&self.text.to_string());
            let all_lines = match highlight_text(&text, ext.as_deref(), syntect_theme) {
                Some(lines) => lines,
                None => self
                    .text
                    .lines()
                    .map(|l| {
                        let s = normalize_preview_text(&l.to_string());
                        let s = s.strip_suffix('\n').unwrap_or(&s).to_string();
                        vec![(fallback_color, Modifier::empty(), s)]
                    })
                    .collect(),
            };
            self.highlight_cache =
                Some((self.edit_version, syntect_theme.to_string(), all_lines));
        }

        let (start, _) = self.visible_line_window(height);
        let all_lines = &self.highlight_cache.as_ref().unwrap().2;
        let end = (start + height).min(all_lines.len());
        (start, &all_lines[start..end])
    }

    /// Returns the visible line window as plain strings.
    ///
    /// Uses direct rope line access — O(height), not O(total_lines).
    pub fn visible_line_window(&self, height: usize) -> (usize, Vec<String>) {
        if height == 0 {
            return (0, Vec::new());
        }
        let total = self.text.len_lines();
        let (cursor_line, _) = self.cursor_line_col();
        let start = (cursor_line + 1).saturating_sub(height);
        let end = (start + height).min(total);
        let visible = (start..end)
            .map(|i| {
                let s = normalize_preview_text(&self.text.line(i).to_string());
                s.strip_suffix('\n').unwrap_or(&s).to_string()
            })
            .collect();
        (start, visible)
    }

    pub fn cursor_line_col(&self) -> (usize, usize) {
        let safe = self.cursor_char_idx.min(self.text.len_chars());
        let line = self.text.char_to_line(safe);
        let line_start = self.text.line_to_char(line);
        (line, safe.saturating_sub(line_start))
    }

    pub fn line_count(&self) -> usize {
        self.text.len_lines()
    }

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
        viewport_rows: usize,
        viewport_cols: usize,
        is_active: bool,
    ) -> EditorRenderState {
        self.clamp_horizontal_scroll(viewport_cols);
        let (visible_start, visible_lines) = self.visible_line_window(viewport_rows);
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

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------

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
            let Some(byte_pos) = text_lower[byte_start..].find(&query_lower) else {
                break;
            };
            let abs_byte = byte_start + byte_pos;
            let end_byte = abs_byte + query_lower.len();
            let start_char = self.text.byte_to_char(abs_byte);
            let end_char = self.text.byte_to_char(end_byte);
            matches.push((start_char, end_char));
            let next_char_len = text_lower[abs_byte..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            byte_start = abs_byte + next_char_len;
        }
        matches
    }

    pub fn search_next(&mut self) {
        let matches = self.find_matches(&self.search_query.clone());
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

    pub fn search_prev(&mut self) {
        let matches = self.find_matches(&self.search_query.clone());
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

    pub fn replace_next(&mut self, replacement: &str) -> bool {
        let matches = self.find_matches(&self.search_query.clone());
        if matches.is_empty() {
            return false;
        }
        let index = self.search_match_idx.min(matches.len() - 1);
        let (start, end) = matches[index];
        self.replace_span(start, end, replacement);
        self.search_match_idx = index;
        self.search_next();
        true
    }

    pub fn replace_all(&mut self, replacement: &str) -> usize {
        let matches = self.find_matches(&self.search_query.clone());
        let count = matches.len();
        for (start, end) in matches.into_iter().rev() {
            self.replace_span(start, end, replacement);
        }
        if count == 0 {
            self.search_match_idx = 0;
        }
        count
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn replace_span(&mut self, start: usize, end: usize, replacement: &str) {
        let removed = self.text.slice(start..end).to_string();
        let edit = Edit {
            char_start: start,
            removed,
            inserted: replacement.to_string(),
            cursor_before: self.cursor_char_idx,
            cursor_after: start + replacement.chars().count(),
        };
        self.text.remove(start..end);
        if !replacement.is_empty() {
            self.text.insert(start, replacement);
        }
        self.cursor_char_idx = edit.cursor_after;
        self.history.push(edit);
        self.is_dirty = true;
        self.edit_version += 1;
    }

    fn line_col_to_char(&self, line: usize, col: usize) -> usize {
        let line_start = self.text.line_to_char(line);
        let line_len = self.visible_line_len(line);
        line_start + col.min(line_len)
    }

    fn visible_line_len(&self, line: usize) -> usize {
        let slice = self.text.line(line);
        let len = slice.len_chars();
        if len > 0 && slice.char(len - 1) == '\n' {
            len - 1
        } else {
            len
        }
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
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    use super::{EditorBuffer, EditorError};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("zeta-{name}-{ts}.txt"))
    }

    // --- open / save -------------------------------------------------------

    #[test]
    fn opens_existing_file_contents() {
        let p = temp_path("open");
        fs::write(&p, "hello editor\n").unwrap();
        let buf = EditorBuffer::open(&p).unwrap();
        assert_eq!(buf.contents(), "hello editor\n");
        assert!(!buf.is_dirty);
        fs::remove_file(p).unwrap();
    }

    #[test]
    fn save_persists_changes_and_clears_dirty_flag() {
        let p = temp_path("save");
        fs::write(&p, "hello").unwrap();
        let mut buf = EditorBuffer::open(&p).unwrap();
        buf.insert(buf.text.len_chars(), " world");
        buf.save().unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "hello world");
        assert!(!buf.is_dirty);
        fs::remove_file(p).unwrap();
    }

    #[test]
    fn save_without_path_fails() {
        let mut buf = EditorBuffer::default();
        assert!(matches!(buf.save(), Err(EditorError::MissingPath)));
    }

    // --- cursor / mutation -------------------------------------------------

    #[test]
    fn typing_and_backspace_update_cursor() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.backspace();
        assert_eq!(buf.contents(), "a");
        assert_eq!(buf.cursor_line_col(), (0, 1));
        assert!(buf.is_dirty);
    }

    #[test]
    fn cursor_moves_between_lines() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('a');
        buf.insert_newline();
        buf.insert_char('b');
        buf.move_up();
        assert_eq!(buf.cursor_line_col(), (0, 1));
        buf.move_down();
        assert_eq!(buf.cursor_line_col(), (1, 1));
    }

    #[test]
    fn visible_window_follows_cursor() {
        let mut buf = EditorBuffer::default();
        for ch in ['a', '\n', 'b', '\n', 'c', '\n', 'd'] {
            if ch == '\n' {
                buf.insert_newline();
            } else {
                buf.insert_char(ch);
            }
        }
        let (start, visible) = buf.visible_line_window(2);
        assert_eq!(start, 2);
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn horizontal_scroll_advances_when_cursor_moves_right_past_viewport() {
        let mut buf = EditorBuffer::default();
        for _ in 0..20 {
            buf.insert_char('x');
        }
        buf.clamp_horizontal_scroll(10);
        let (_, col) = buf.cursor_line_col();
        assert!(
            col >= buf.scroll_col && col < buf.scroll_col + 10,
            "cursor col {col} should be inside [{}, {})",
            buf.scroll_col,
            buf.scroll_col + 10
        );
        assert!(buf.scroll_col > 0);
    }

    #[test]
    fn horizontal_scroll_retreats_when_cursor_moves_left_past_scroll_origin() {
        let mut buf = EditorBuffer::default();
        for _ in 0..20 {
            buf.insert_char('x');
        }
        buf.clamp_horizontal_scroll(10);
        assert!(buf.scroll_col > 0, "precondition: scroll_col > 0");
        for _ in 0..20 {
            buf.move_left();
        }
        buf.clamp_horizontal_scroll(10);
        assert_eq!(buf.scroll_col, 0);
    }

    // --- undo / redo -------------------------------------------------------

    #[test]
    fn undo_restores_previous_content() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.undo();
        assert_eq!(buf.contents(), "a", "undo should remove 'b'");
    }

    #[test]
    fn undo_twice_restores_empty() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.undo();
        buf.undo();
        assert_eq!(buf.contents(), "");
    }

    #[test]
    fn redo_reapplies_undone_change() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('x');
        buf.undo();
        buf.redo();
        assert!(buf.contents().contains('x'));
    }

    #[test]
    fn new_edit_after_undo_clears_redo_history() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.undo(); // removes 'b'
        buf.insert_char('c'); // branch — 'b' redo is gone
        buf.redo(); // no-op
        assert_eq!(buf.contents(), "ac");
    }

    #[test]
    fn undo_backspace_restores_deleted_char() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('h');
        buf.insert_char('i');
        buf.backspace();
        assert_eq!(buf.contents(), "h");
        buf.undo();
        assert_eq!(buf.contents(), "hi");
    }

    // --- search ------------------------------------------------------------

    #[test]
    fn find_matches_returns_all_occurrences() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo bar foo baz foo");
        assert_eq!(buf.find_matches("foo"), vec![(0, 3), (8, 11), (16, 19)]);
    }

    #[test]
    fn find_matches_is_case_insensitive() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "Hello hello HELLO");
        assert_eq!(buf.find_matches("hello").len(), 3);
    }

    #[test]
    fn find_matches_empty_query_returns_nothing() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('a');
        assert!(buf.find_matches("").is_empty());
    }

    #[test]
    fn search_next_jumps_to_next_match() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo bar foo");
        buf.search_query = String::from("foo");
        buf.cursor_char_idx = 0;
        buf.search_next();
        assert_eq!(buf.cursor_char_idx, 8);
    }

    #[test]
    fn search_next_wraps_around() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo bar foo");
        buf.search_query = String::from("foo");
        buf.cursor_char_idx = 9;
        buf.search_next();
        assert_eq!(buf.cursor_char_idx, 0);
    }

    #[test]
    fn search_prev_jumps_to_previous_match() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo bar foo");
        buf.search_query = String::from("foo");
        buf.cursor_char_idx = 8;
        buf.search_prev();
        assert_eq!(buf.cursor_char_idx, 0);
    }

    #[test]
    fn search_prev_wraps_around_to_last_match() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo bar foo");
        buf.search_query = String::from("foo");
        buf.cursor_char_idx = 0;
        buf.search_prev();
        assert_eq!(buf.cursor_char_idx, 8);
    }

    #[test]
    fn find_matches_returns_char_spans_for_unicode_text() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "café café");
        assert_eq!(buf.find_matches("café"), vec![(0, 4), (5, 9)]);
    }

    #[test]
    fn find_matches_returns_char_spans_for_japanese_text() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "ありがとう ありがとう");
        assert_eq!(buf.find_matches("ありがとう"), vec![(0, 5), (6, 11)]);
    }

    #[test]
    fn search_next_jumps_to_unicode_match() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "ありがとう ありがとう");
        buf.search_query = String::from("ありがとう");
        buf.cursor_char_idx = 0;
        buf.search_next();
        assert_eq!(buf.cursor_char_idx, 6);
        assert_eq!(buf.search_match_idx, 1);
    }

    #[test]
    fn replace_next_replaces_current_match_and_advances() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo foo");
        buf.search_query = String::from("foo");
        buf.search_next();
        assert!(buf.replace_next("bar"));
        assert_eq!(buf.contents(), "bar foo");
    }

    #[test]
    fn replace_all_replaces_every_occurrence() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo foo foo");
        buf.search_query = String::from("foo");
        let count = buf.replace_all("bar");
        assert_eq!(count, 3);
        assert_eq!(buf.contents(), "bar bar bar");
    }

    // --- performance -------------------------------------------------------

    #[test]
    fn large_file_insert_does_not_degrade_linearly() {
        let mut buf = EditorBuffer::default();
        let big = (0..10_000).map(|i| format!("line {i}\n")).collect::<String>();
        buf.insert(0, &big);
        let start = Instant::now();
        for ch in "hello world".chars() {
            buf.insert_char(ch);
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 50,
            "11 keystrokes took {}ms — expected < 50ms",
            elapsed.as_millis()
        );
    }
}
