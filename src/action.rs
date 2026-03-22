use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::RuntimeKeymap;
use crate::pane::PaneId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    FocusNextPane,
    MoveSelectionDown,
    MoveSelectionUp,
    Refresh,
    Quit,
    Resize { width: u16, height: u16 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    ScanPane { pane: PaneId, path: PathBuf },
}

impl Action {
    pub fn from_key_event(key_event: KeyEvent, keymap: &RuntimeKeymap) -> Option<Self> {
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
            KeyCode::Down | KeyCode::Char('j') => Some(Self::MoveSelectionDown),
            KeyCode::Up | KeyCode::Char('k') => Some(Self::MoveSelectionUp),
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
    }
}
