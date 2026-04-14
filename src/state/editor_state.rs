use anyhow::Result;

use crate::action::{Action, Command};
use crate::editor::EditorBuffer;

/// Owns the optional editor buffer and routes editor actions.
#[derive(Clone, Debug, Default)]
pub struct EditorState {
    pub buffer: Option<EditorBuffer>,
    pub loading: bool,
    pub replace_query: String,
    pub replace_active: bool,
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

    pub fn open_placeholder(&mut self, path: std::path::PathBuf) {
        let is_md = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"));
        let mut editor = EditorBuffer::default();
        editor.path = Some(path);
        self.buffer = Some(editor);
        self.loading = true;
        self.replace_query.clear();
        self.replace_active = false;
        self.markdown_preview_visible = is_md;
        self.markdown_preview_focused = false;
        self.markdown_preview_scroll = 0;
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
        self.loading = false;
        self.replace_query.clear();
        self.replace_active = false;
        self.markdown_preview_visible = is_md;
        self.markdown_preview_focused = false;
        self.markdown_preview_scroll = 0;
        self.buffer = Some(editor);
    }

    pub fn close(&mut self) {
        self.buffer = None;
        self.loading = false;
        self.replace_query.clear();
        self.replace_active = false;
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
                if self.replace_active {
                    self.replace_active = false;
                    self.replace_query.clear();
                    return Ok(commands);
                }
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
            Action::OpenEditorReplace => {
                if let Some(editor) = self.buffer.as_mut() {
                    if self.replace_active {
                        editor.replace_next(&self.replace_query);
                    } else {
                        editor.search_active = true;
                        self.replace_active = true;
                        self.replace_query.clear();
                    }
                }
            }
            Action::EditorCloseSearch => {
                if let Some(editor) = self.buffer.as_mut() {
                    editor.search_active = false;
                    editor.search_query.clear();
                }
                self.replace_active = false;
                self.replace_query.clear();
            }
            Action::EditorBackspace => {
                if let Some(editor) = self.buffer.as_mut() {
                    if self.replace_active {
                        self.replace_query.pop();
                        return Ok(commands);
                    }
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
                    if self.replace_active {
                        self.replace_query.push(*ch);
                        return Ok(commands);
                    }
                    if editor.search_active {
                        editor.search_query.push(*ch);
                        editor.search_next();
                        return Ok(commands);
                    }
                    // If a selection is active, typing replaces it.
                    editor.delete_selection();
                    editor.insert_char(*ch);
                }
            }
            Action::EditorPaste => {
                if let Some(editor) = self.buffer.as_mut() {
                    // Silently ignore clipboard errors (unavailable backend, empty clipboard, etc.)
                    if let Ok(text) = arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                        editor.insert_str_at_cursor(&text);
                    }
                }
            }
            Action::EditorUndo => {
                if let Some(editor) = self.buffer.as_mut() {
                    editor.undo();
                }
            }
            Action::EditorRedo => {
                if let Some(editor) = self.buffer.as_mut() {
                    editor.redo();
                }
            }
            Action::EditorSelectAll => {
                if let Some(editor) = self.buffer.as_mut() {
                    editor.select_all();
                }
            }
            Action::EditorCopy => {
                if let Some(editor) = self.buffer.as_ref() {
                    if let Some(text) = editor.selected_text() {
                        // Silently ignore clipboard errors.
                        let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text));
                    }
                }
            }
            Action::EditorCut => {
                if let Some(editor) = self.buffer.as_mut() {
                    if let Some(text) = editor.selected_text() {
                        let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text));
                        editor.delete_selection();
                    }
                }
            }
            Action::EditorMoveDown => {
                if let Some(e) = self.buffer.as_mut() {
                    e.move_down();
                }
            }
            Action::EditorExtendLeft => {
                if let Some(e) = self.buffer.as_mut() {
                    e.extend_left();
                }
            }
            Action::EditorExtendRight => {
                if let Some(e) = self.buffer.as_mut() {
                    e.extend_right();
                }
            }
            Action::EditorExtendUp => {
                if let Some(e) = self.buffer.as_mut() {
                    e.extend_up();
                }
            }
            Action::EditorExtendDown => {
                if let Some(e) = self.buffer.as_mut() {
                    e.extend_down();
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
            Action::EditorReplaceInput(ch) => {
                self.replace_query.push(*ch);
            }
            Action::EditorReplaceBackspace => {
                self.replace_query.pop();
            }
            Action::EditorReplaceNext => {
                if let Some(editor) = self.buffer.as_mut() {
                    editor.replace_next(&self.replace_query);
                }
            }
            Action::EditorReplaceAll => {
                if let Some(editor) = self.buffer.as_mut() {
                    editor.replace_all(&self.replace_query);
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
    #[test]
    fn undo_reverses_last_insert() {
        let mut state = EditorState::default();
        state.open(EditorBuffer::from_text(
            std::path::PathBuf::from("f.txt"),
            String::from("hello"),
        ));
        state
            .apply(&crate::action::Action::EditorInsert('!'))
            .unwrap();
        state.apply(&crate::action::Action::EditorUndo).unwrap();
        let buf = state.buffer.as_ref().unwrap();
        assert_eq!(buf.visible_lines()[0], "hello");
    }

    #[test]
    fn redo_reapplies_undone_insert() {
        let mut state = EditorState::default();
        state.open(EditorBuffer::from_text(
            std::path::PathBuf::from("f.txt"),
            String::from("hello"),
        ));
        state
            .apply(&crate::action::Action::EditorInsert('!'))
            .unwrap();
        state.apply(&crate::action::Action::EditorUndo).unwrap();
        state.apply(&crate::action::Action::EditorRedo).unwrap();
        let buf = state.buffer.as_ref().unwrap();
        // Cursor starts at position 0, so EditorInsert prepends the char.
        assert_eq!(buf.visible_lines()[0], "!hello");
    }
    #[test]
    fn select_all_covers_entire_buffer() {
        let mut state = EditorState::default();
        state.open(EditorBuffer::from_text(
            std::path::PathBuf::from("f.txt"),
            String::from("hello world"),
        ));
        state
            .apply(&crate::action::Action::EditorSelectAll)
            .unwrap();
        let buf = state.buffer.as_ref().unwrap();
        assert_eq!(buf.selected_text().as_deref(), Some("hello world"));
    }

    #[test]
    fn cut_removes_selected_text() {
        let mut state = EditorState::default();
        state.open(EditorBuffer::from_text(
            std::path::PathBuf::from("f.txt"),
            String::from("hello world"),
        ));
        state
            .apply(&crate::action::Action::EditorSelectAll)
            .unwrap();
        state.apply(&crate::action::Action::EditorCut).unwrap();
        let buf = state.buffer.as_ref().unwrap();
        // After cut, buffer should be empty and selection cleared.
        assert_eq!(buf.selected_text(), None);
        assert_eq!(buf.contents(), "");
    }
    #[test]
    fn shift_right_extends_selection_from_cursor() {
        let mut state = EditorState::default();
        state.open(EditorBuffer::from_text(
            std::path::PathBuf::from("f.txt"),
            String::from("hello"),
        ));
        // Cursor starts at 0; extend right 3 chars → selects "hel".
        state
            .apply(&crate::action::Action::EditorExtendRight)
            .unwrap();
        state
            .apply(&crate::action::Action::EditorExtendRight)
            .unwrap();
        state
            .apply(&crate::action::Action::EditorExtendRight)
            .unwrap();
        let buf = state.buffer.as_ref().unwrap();
        assert_eq!(buf.selected_text().as_deref(), Some("hel"));
    }

    #[test]
    fn shift_arrow_then_plain_arrow_clears_selection() {
        let mut state = EditorState::default();
        state.open(EditorBuffer::from_text(
            std::path::PathBuf::from("f.txt"),
            String::from("hello"),
        ));
        state
            .apply(&crate::action::Action::EditorExtendRight)
            .unwrap();
        state
            .apply(&crate::action::Action::EditorExtendRight)
            .unwrap();
        // Plain arrow clears selection.
        state
            .apply(&crate::action::Action::EditorMoveRight)
            .unwrap();
        let buf = state.buffer.as_ref().unwrap();
        assert_eq!(buf.selected_text(), None);
    }

    #[test]
    fn typing_with_selection_replaces_selected_text() {
        let mut state = EditorState::default();
        state.open(EditorBuffer::from_text(
            std::path::PathBuf::from("f.txt"),
            String::from("hello"),
        ));
        state
            .apply(&crate::action::Action::EditorSelectAll)
            .unwrap();
        state
            .apply(&crate::action::Action::EditorInsert('X'))
            .unwrap();
        let buf = state.buffer.as_ref().unwrap();
        assert_eq!(buf.contents(), "X");
        assert_eq!(buf.selected_text(), None);
    }
}
