use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::RuntimeKeymap;
use crate::pane::PaneId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    EnterSelection,
    CloseEditor,
    EditorBackspace,
    EditorInsert(char),
    EditorMoveDown,
    EditorMoveLeft,
    EditorMoveRight,
    EditorMoveUp,
    EditorNewline,
    FocusNextPane,
    MoveSelectionDown,
    MoveSelectionUp,
    NavigateToParent,
    OpenSelectedInEditor,
    Refresh,
    SaveEditor,
    Quit,
    Resize { width: u16, height: u16 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    OpenEditor { path: PathBuf },
    ScanPane { pane: PaneId, path: PathBuf },
    SaveEditor,
}

impl Action {
    pub fn from_key_event(key_event: KeyEvent, keymap: &RuntimeKeymap) -> Option<Self> {
        if key_event.code == KeyCode::Char('q') && key_event.modifiers == KeyModifiers::CONTROL {
            return Some(Self::Quit);
        }

        if keymap.switch_pane.matches(&key_event) {
            return Some(Self::FocusNextPane);
        }

        if keymap.refresh.matches(&key_event) {
            return Some(Self::Refresh);
        }

        if keymap.quit.matches(&key_event) {
            return Some(Self::Quit);
        }

        match key_event.code {
            KeyCode::F(4) => Some(Self::OpenSelectedInEditor),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => Some(Self::EnterSelection),
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => Some(Self::NavigateToParent),
            KeyCode::Char('s') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::SaveEditor)
            }
            KeyCode::Down | KeyCode::Char('j') => Some(Self::MoveSelectionDown),
            KeyCode::Up | KeyCode::Char('k') => Some(Self::MoveSelectionUp),
            _ => None,
        }
    }

    pub fn from_editor_key_event(key_event: KeyEvent) -> Option<Self> {
        match key_event.code {
            KeyCode::Char('q') if key_event.modifiers == KeyModifiers::CONTROL => Some(Self::Quit),
            KeyCode::Esc | KeyCode::F(4) => Some(Self::CloseEditor),
            KeyCode::Backspace => Some(Self::EditorBackspace),
            KeyCode::Enter => Some(Self::EditorNewline),
            KeyCode::Left => Some(Self::EditorMoveLeft),
            KeyCode::Right => Some(Self::EditorMoveRight),
            KeyCode::Up => Some(Self::EditorMoveUp),
            KeyCode::Down => Some(Self::EditorMoveDown),
            KeyCode::Tab => Some(Self::FocusNextPane),
            KeyCode::Char('s') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::SaveEditor)
            }
            KeyCode::Char(ch)
                if key_event.modifiers.is_empty() || key_event.modifiers == KeyModifiers::SHIFT =>
            {
                Some(Self::EditorInsert(ch))
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub fn matches(&self, key_event: &KeyEvent) -> bool {
        self.code == key_event.code && self.modifiers == key_event.modifiers
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::config::RuntimeKeymap;

    use super::{Action, KeyBinding};

    #[test]
    fn configured_keymap_drives_actions() {
        let keymap = RuntimeKeymap {
            quit: KeyBinding {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::NONE,
            },
            switch_pane: KeyBinding {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::NONE,
            },
            refresh: KeyBinding {
                code: KeyCode::Char('u'),
                modifiers: KeyModifiers::CONTROL,
            },
        };

        assert_eq!(
            Action::from_key_event(
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
                &keymap
            ),
            Some(Action::Quit)
        );
        assert_eq!(
            Action::from_key_event(
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
                &keymap
            ),
            Some(Action::FocusNextPane)
        );
        assert_eq!(
            Action::from_key_event(
                KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
                &keymap,
            ),
            Some(Action::Refresh)
        );
    }

    #[test]
    fn movement_keys_remain_available() {
        let keymap = RuntimeKeymap::default();

        assert_eq!(
            Action::from_key_event(
                KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
                &keymap
            ),
            Some(Action::MoveSelectionDown)
        );
        assert_eq!(
            Action::from_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), &keymap),
            Some(Action::MoveSelectionUp)
        );
        assert_eq!(
            Action::from_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &keymap),
            Some(Action::EnterSelection)
        );
        assert_eq!(
            Action::from_key_event(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), &keymap),
            Some(Action::NavigateToParent)
        );
    }

    #[test]
    fn editor_shortcuts_remain_available() {
        let keymap = RuntimeKeymap::default();

        assert_eq!(
            Action::from_key_event(KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE), &keymap),
            Some(Action::OpenSelectedInEditor)
        );
        assert_eq!(
            Action::from_key_event(
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
                &keymap,
            ),
            Some(Action::SaveEditor)
        );
    }

    #[test]
    fn editor_mode_prefers_text_entry() {
        assert_eq!(
            Action::from_editor_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(Action::EditorInsert('q'))
        );
        assert_eq!(
            Action::from_editor_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Some(Action::CloseEditor)
        );
        assert_eq!(
            Action::from_editor_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)),
            Some(Action::Quit)
        );
    }
}
