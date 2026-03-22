use std::fs as std_fs;
use std::path::{Path, PathBuf};

use ropey::Rope;
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct EditorBuffer {
    pub path: Option<PathBuf>,
    pub cursor_char_idx: usize,
    pub text: Rope,
    pub is_dirty: bool,
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self {
            path: None,
            cursor_char_idx: 0,
            text: Rope::new(),
            is_dirty: false,
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
            text: Rope::from_str(&contents),
            is_dirty: false,
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
}
