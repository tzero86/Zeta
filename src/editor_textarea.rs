use std::path::PathBuf;
use tui_textarea::TextArea;

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
    #[allow(dead_code)]
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
}
