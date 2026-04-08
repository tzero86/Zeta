use anyhow::Result;

use crate::action::{Action, Command};
use crate::editor::EditorBuffer;

/// Owns the optional editor buffer and routes editor actions.
#[derive(Clone, Debug, Default)]
pub struct EditorState {
    pub buffer: Option<EditorBuffer>,
}

impl EditorState {
    pub fn is_open(&self) -> bool {
        self.buffer.is_some()
    }

    pub fn is_dirty(&self) -> bool {
        self.buffer.as_ref().is_some_and(|e| e.is_dirty)
    }

    pub fn open(&mut self, editor: EditorBuffer) {
        self.buffer = Some(editor);
    }

    pub fn close(&mut self) {
        self.buffer = None;
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
                if let Some(editor) = &self.buffer {
                    if !editor.is_dirty {
                        self.buffer = None;
                    }
                }
            }
            Action::DiscardEditorChanges => {
                self.buffer = None;
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
}
