use anyhow::Result;

use crate::action::{Action, Command};
use crate::editor::EditorBuffer;

/// Owns the optional editor buffer and routes editor actions.
#[derive(Clone, Debug, Default)]
pub struct EditorState {
    pub buffer: Option<EditorBuffer>,
    /// Scroll offset for the markdown preview panel (lines from top).
    pub markdown_preview_scroll: usize,
    /// Whether keyboard focus is currently on the markdown preview, not the editor.
    pub markdown_preview_focused: bool,
    /// Whether the markdown preview split is visible (auto-set when opening .md).
    pub markdown_preview_visible: bool,
}

impl EditorState {
    pub fn is_open(&self) -> bool {
        self.buffer.is_some()
    }

    pub fn is_dirty(&self) -> bool {
        self.buffer.as_ref().is_some_and(|e| e.is_dirty)
    }

    pub fn open(&mut self, editor: EditorBuffer) {
        let is_md = editor
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .is_some_and(|e| e.eq_ignore_ascii_case("md"));
        self.markdown_preview_visible = is_md;
        self.markdown_preview_focused = false;
        self.markdown_preview_scroll = 0;
        self.buffer = Some(editor);
    }

    pub fn close(&mut self) {
        self.buffer = None;
        self.markdown_preview_scroll = 0;
        self.markdown_preview_focused = false;
        self.markdown_preview_visible = false;
    }

    pub fn is_markdown_file(&self) -> bool {
        self.buffer.as_ref().is_some_and(|editor| {
            editor
                .path
                .as_ref()
                .and_then(|p| p.extension())
                .is_some_and(|e| e.eq_ignore_ascii_case("md"))
        })
    }

    pub fn sync_markdown_preview_to_cursor(&mut self, viewport_height: usize) {
        if !self.markdown_preview_visible || self.markdown_preview_focused {
            return;
        }
        let Some(editor) = self.buffer.as_ref() else {
            return;
        };
        let total_lines = editor.line_count().max(1);
        let cursor_line = editor.cursor_line_col().0;
        let scroll = cursor_line.saturating_sub(viewport_height / 3);
        let max_scroll = total_lines.saturating_sub(viewport_height.max(1));
        self.markdown_preview_scroll = scroll.min(max_scroll);
    }

    pub fn apply(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        match action {
            Action::CloseEditor => {
                if let Some(editor) = self.buffer.as_mut() {
                    if editor.search_active {
                        editor.search_active = false;
                        editor.search_query.clear();
                        return Ok(commands);
                    }
                }
                if self.buffer.as_ref().is_some_and(|editor| !editor.is_dirty) {
                    self.close();
                }
            }
            Action::DiscardEditorChanges => {
                self.close();
            }
            Action::EditorOpenSearch => {
                if let Some(editor) = self.buffer.as_mut() {
                    editor.search_active = true;
                    editor.search_query.clear();
                    editor.search_match_idx = 0;
                }
            }
            Action::EditorCloseSearch => {
                if let Some(editor) = self.buffer.as_mut() {
                    editor.search_active = false;
                    editor.search_query.clear();
                }
            }
            Action::EditorBackspace => {
                if let Some(editor) = self.buffer.as_mut() {
                    if editor.search_active {
                        editor.search_query.pop();
                        if !editor.search_query.is_empty() {
                            editor.search_next();
                        }
                        return Ok(commands);
                    }
                    editor.backspace();
                }
            }
            Action::EditorInsert(ch) => {
                if let Some(editor) = self.buffer.as_mut() {
                    if editor.search_active {
                        editor.search_query.push(*ch);
                        editor.search_next();
                        return Ok(commands);
                    }
                    editor.insert_char(*ch);
                }
            }
            Action::EditorMoveDown => {
                if let Some(e) = self.buffer.as_mut() {
                    e.move_down();
                }
            }
            Action::EditorMoveLeft => {
                if let Some(e) = self.buffer.as_mut() {
                    e.move_left();
                }
            }
            Action::EditorMoveRight => {
                if let Some(e) = self.buffer.as_mut() {
                    e.move_right();
                }
            }
            Action::EditorMoveUp => {
                if let Some(e) = self.buffer.as_mut() {
                    e.move_up();
                }
            }
            Action::EditorNewline => {
                if let Some(editor) = self.buffer.as_mut() {
                    if editor.search_active {
                        editor.search_next();
                        return Ok(commands);
                    }
                    editor.insert_newline();
                }
            }
            Action::EditorSearchBackspace => {
                if let Some(editor) = self.buffer.as_mut() {
                    if editor.search_active {
                        editor.search_query.pop();
                        if !editor.search_query.is_empty() {
                            editor.search_next();
                        }
                    }
                }
            }
            Action::EditorSearchNext => {
                if let Some(e) = self.buffer.as_mut() {
                    e.search_next();
                }
            }
            Action::EditorSearchPrev => {
                if let Some(e) = self.buffer.as_mut() {
                    e.search_prev();
                }
            }
            Action::ToggleMarkdownPreview => {
                if self.is_markdown_file() {
                    self.markdown_preview_visible = !self.markdown_preview_visible;
                    if !self.markdown_preview_visible {
                        self.markdown_preview_focused = false;
                        self.markdown_preview_scroll = 0;
                    }
                }
            }
            Action::FocusMarkdownPreview => {
                if self.is_markdown_file() && self.markdown_preview_visible {
                    self.markdown_preview_focused = !self.markdown_preview_focused;
                }
            }
            Action::ScrollMarkdownPreviewUp => {
                if self.markdown_preview_visible && self.markdown_preview_focused {
                    self.markdown_preview_scroll = self.markdown_preview_scroll.saturating_sub(1);
                }
            }
            Action::ScrollMarkdownPreviewDown => {
                if self.markdown_preview_visible && self.markdown_preview_focused {
                    self.markdown_preview_scroll = self.markdown_preview_scroll.saturating_add(1);
                }
            }
            Action::ScrollMarkdownPreviewPageUp => {
                if self.markdown_preview_visible && self.markdown_preview_focused {
                    self.markdown_preview_scroll = self.markdown_preview_scroll.saturating_sub(20);
                }
            }
            Action::ScrollMarkdownPreviewPageDown => {
                if self.markdown_preview_visible && self.markdown_preview_focused {
                    self.markdown_preview_scroll = self.markdown_preview_scroll.saturating_add(20);
                }
            }
            Action::SaveEditor => {
                if let Some(editor) = &self.buffer {
                    if editor.is_dirty {
                        commands.push(Command::SaveEditor);
                    }
                }
            }
            _ => {}
        }
        Ok(commands)
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn editor_state_starts_closed() {
        let s = EditorState::default();
        assert!(!s.is_open());
        assert!(!s.is_dirty());
    }

    #[test]
    fn discard_closes_buffer() {
        let mut s = EditorState::default();
        s.buffer = Some(EditorBuffer::default());
        s.apply(&Action::DiscardEditorChanges).unwrap();
        assert!(!s.is_open());
    }

    #[test]
    fn close_editor_when_not_dirty_removes_buffer() {
        let mut s = EditorState::default();
        let mut buf = EditorBuffer::default();
        buf.is_dirty = false;
        s.buffer = Some(buf);
        s.apply(&Action::CloseEditor).unwrap();
        assert!(!s.is_open());
    }

    #[test]
    fn close_editor_when_dirty_keeps_buffer() {
        let mut s = EditorState::default();
        let mut buf = EditorBuffer::default();
        buf.is_dirty = true;
        s.buffer = Some(buf);
        s.apply(&Action::CloseEditor).unwrap();
        assert!(s.is_open(), "dirty editor should not be closed silently");
    }

    #[test]
    fn open_markdown_file_enables_live_preview() {
        let mut s = EditorState::default();
        let mut buf = EditorBuffer::default();
        buf.path = Some(std::path::PathBuf::from("note.md"));
        s.open(buf);
        assert!(s.markdown_preview_visible);
        assert!(!s.markdown_preview_focused);
        assert_eq!(s.markdown_preview_scroll, 0);
    }

    #[test]
    fn toggle_markdown_preview_changes_visibility_for_markdown_buffers() {
        let mut s = EditorState::default();
        let mut buf = EditorBuffer::default();
        buf.path = Some(std::path::PathBuf::from("note.md"));
        s.open(buf);
        s.apply(&Action::ToggleMarkdownPreview).unwrap();
        assert!(!s.markdown_preview_visible);
    }
}
