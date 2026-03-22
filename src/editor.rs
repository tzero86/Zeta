use std::fs as std_fs;
use std::path::{Path, PathBuf};

use ropey::Rope;
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct EditorBuffer {
    pub path: Option<PathBuf>,
    pub text: Rope,
    pub is_dirty: bool,
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self {
            path: None,
            text: Rope::new(),
            is_dirty: false,
        }
    }
}

impl EditorBuffer {
    pub fn open(path: &Path) -> Result<Self, EditorError> {
        let contents = std_fs::read_to_string(path).map_err(|source| EditorError::ReadFile {
            path: path.display().to_string(),
            source,
        })?;

        Ok(Self {
            path: Some(path.to_path_buf()),
            text: Rope::from_str(&contents),
            is_dirty: false,
        })
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        self.text.insert(char_idx, text);
        self.is_dirty = true;
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
}
