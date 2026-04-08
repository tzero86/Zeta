use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::RuntimeKeymap;
use crate::config::ThemePreset;
use crate::pane::PaneId;
use crate::state::PaneLayout;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuId {
    File,
    Navigate,
    View,
    Help,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    CollisionCancel,
    CollisionOverwrite,
    CollisionRename,
    CollisionSkip,
    CloseDialog,
    CloseMenu,
    EnterSelection,
    CloseEditor,
    ClearPreview,
    DiscardEditorChanges,
    EditorBackspace,
    EditorInsert(char),
    EditorMoveDown,
    EditorMoveLeft,
    EditorMoveRight,
    EditorMoveUp,
    EditorNewline,
    EditorOpenSearch,
    EditorCloseSearch,
    EditorSearchBackspace,
    EditorSearchNext,
    EditorSearchPrev,
    FocusNextPane,
    CycleFocus,
    FocusPreviewPanel,
    ScrollPreviewDown,
    ScrollPreviewUp,
    ScrollPreviewPageDown,
    ScrollPreviewPageUp,
    MenuActivate,
    MenuMnemonic(char),
    MenuMoveDown,
    MenuMoveUp,
    MenuNext,
    MenuPrevious,
    MoveSelectionDown,
    MoveSelectionUp,
    NavigateBack,
    NavigateForward,
    NavigateToParent,
    OpenAboutDialog,
    OpenCopyPrompt,
    OpenDeletePrompt,
    OpenHelpDialog,
    OpenMovePrompt,
    OpenMenu(MenuId),
    OpenNewDirectoryPrompt,
    OpenNewFilePrompt,
    OpenRenamePrompt,
    OpenSelectedInEditor,
    OpenSettingsPanel,
    PreviewFile { path: PathBuf },
    PromptBackspace,
    PromptCancel,
    PromptInput(char),
    PromptSubmit,
    Refresh,
    SaveEditor,
    SetPaneLayout(PaneLayout),
    SetTheme(ThemePreset),
    ToggleMark,
    ClearMarks,
    ToggleHiddenFiles,
    TogglePreviewPanel,
    Quit,
    Resize { width: u16, height: u16 },
    OpenCommandPalette,
    CloseCommandPalette,
    CloseSettingsPanel,
    PaletteInput(char),
    PaletteBackspace,
    PaletteConfirm,
    PaletteMoveDown,
    PaletteMoveUp,
    SettingsMoveDown,
    SettingsMoveUp,
    SettingsToggleCurrent,
    CycleSortMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    DispatchAction(Action),
    OpenEditor {
        path: PathBuf,
    },
    PreviewFile {
        path: PathBuf,
    },
    RunFileOperation {
        operation: FileOperation,
        refresh: Vec<RefreshTarget>,
        collision: CollisionPolicy,
    },
    ScanPane {
        pane: PaneId,
        path: PathBuf,
    },
    SaveEditor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollisionPolicy {
    Fail,
    Overwrite,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileOperation {
    Copy {
        source: PathBuf,
        destination: PathBuf,
    },
    CreateDirectory {
        path: PathBuf,
    },
    CreateFile {
        path: PathBuf,
    },
    Delete {
        path: PathBuf,
    },
    Move {
        source: PathBuf,
        destination: PathBuf,
    },
    Rename {
        source: PathBuf,
        destination: PathBuf,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefreshTarget {
    pub pane: PaneId,
    pub path: PathBuf,
}

impl Action {
    /// Low-priority fallback key handler for Pane and Editor contexts.
    /// Palette, settings, preview, collision, prompt, dialog, and menu
    /// are handled by their dedicated `from_*_key_event` helpers.
    pub fn from_key_event_with_settings(
        key_event: KeyEvent,
        keymap: &RuntimeKeymap,
    ) -> Option<Self> {
        if key_event.modifiers == KeyModifiers::ALT {
            return match key_event.code {
                KeyCode::Char('f') | KeyCode::Char('F') => Some(Self::OpenMenu(MenuId::File)),
                KeyCode::Char('n') | KeyCode::Char('N') => Some(Self::OpenMenu(MenuId::Navigate)),
                KeyCode::Char('v') | KeyCode::Char('V') => Some(Self::OpenMenu(MenuId::View)),
                KeyCode::Char('h') | KeyCode::Char('H') => Some(Self::OpenMenu(MenuId::Help)),
                KeyCode::Left => Some(Self::NavigateBack),
                KeyCode::Right => Some(Self::NavigateForward),
                _ => None,
            };
        }

        if key_event.code == KeyCode::Char('q') && key_event.modifiers == KeyModifiers::CONTROL {
            return Some(Self::Quit);
        }

        if key_event.code == KeyCode::Char('w') && key_event.modifiers == KeyModifiers::CONTROL {
            return Some(Self::CycleFocus);
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
            KeyCode::F(1) => Some(Self::OpenHelpDialog),
            KeyCode::F(3) if key_event.modifiers == KeyModifiers::ALT => {
                Some(Self::FocusPreviewPanel)
            }
            KeyCode::F(3) => Some(Self::TogglePreviewPanel),
            KeyCode::F(4) => Some(Self::OpenSelectedInEditor),
            KeyCode::F(5) => Some(Self::OpenCopyPrompt),
            KeyCode::F(6) if key_event.modifiers == KeyModifiers::SHIFT => {
                Some(Self::OpenMovePrompt)
            }
            KeyCode::F(6) => Some(Self::OpenRenamePrompt),
            KeyCode::F(8) => Some(Self::OpenDeletePrompt),
            KeyCode::Insert => Some(Self::OpenNewFilePrompt),
            KeyCode::F(7) if key_event.modifiers == KeyModifiers::SHIFT => {
                Some(Self::OpenNewDirectoryPrompt)
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => Some(Self::EnterSelection),
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => Some(Self::NavigateToParent),
            KeyCode::Char(' ') => Some(Self::ToggleMark),
            KeyCode::Char('M') if key_event.modifiers == KeyModifiers::SHIFT => {
                Some(Self::ClearMarks)
            }
            KeyCode::Char('s') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::SaveEditor)
            }
            KeyCode::Char('p') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::OpenCommandPalette)
            }
            KeyCode::Char('o') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::OpenSettingsPanel)
            }
            KeyCode::Down | KeyCode::Char('j') => Some(Self::MoveSelectionDown),
            KeyCode::Up | KeyCode::Char('k') => Some(Self::MoveSelectionUp),
            KeyCode::Char('s') | KeyCode::Char('S')
                if key_event.modifiers == KeyModifiers::NONE
                    || key_event.modifiers == KeyModifiers::SHIFT =>
            {
                Some(Self::CycleSortMode)
            }
            _ => None,
        }
    }

    // -----------------------------------------------------------------------
    // Focused dispatch helpers — one per FocusLayer arm in route_key_event
    // -----------------------------------------------------------------------

    /// Keys when the command palette is open. Consumes ALL input.
    pub fn from_palette_key_event(key_event: KeyEvent) -> Option<Self> {
        match key_event.code {
            KeyCode::Esc => Some(Self::CloseCommandPalette),
            KeyCode::Enter => Some(Self::PaletteConfirm),
            KeyCode::Up => Some(Self::PaletteMoveUp),
            KeyCode::Down => Some(Self::PaletteMoveDown),
            KeyCode::Backspace => Some(Self::PaletteBackspace),
            KeyCode::Char(c)
                if key_event.modifiers == KeyModifiers::NONE
                    || key_event.modifiers == KeyModifiers::SHIFT =>
            {
                Some(Self::PaletteInput(c))
            }
            _ => None,
        }
    }

    /// Keys when the settings panel is open. Consumes ALL input.
    pub fn from_settings_key_event(key_event: KeyEvent) -> Option<Self> {
        match key_event.code {
            KeyCode::Esc => Some(Self::CloseSettingsPanel),
            KeyCode::Enter | KeyCode::Char(' ') => Some(Self::SettingsToggleCurrent),
            KeyCode::Up => Some(Self::SettingsMoveUp),
            KeyCode::Down => Some(Self::SettingsMoveDown),
            _ => None,
        }
    }

    /// Keys when the preview panel has focus. Consumes ALL input.
    pub fn from_preview_key_event(key_event: KeyEvent) -> Option<Self> {
        match key_event.code {
            KeyCode::Up => Some(Self::ScrollPreviewUp),
            KeyCode::Down => Some(Self::ScrollPreviewDown),
            KeyCode::PageUp => Some(Self::ScrollPreviewPageUp),
            KeyCode::PageDown => Some(Self::ScrollPreviewPageDown),
            KeyCode::Esc => Some(Self::FocusPreviewPanel),
            KeyCode::Char('w') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::CycleFocus)
            }
            _ => None,
        }
    }

    /// Global keys available in Pane context (and as lower-priority fallback from Editor).
    pub fn from_pane_key_event(
        key_event: KeyEvent,
        keymap: &crate::config::RuntimeKeymap,
    ) -> Option<Self> {
        if key_event.modifiers == KeyModifiers::ALT {
            return match key_event.code {
                KeyCode::Char('f') | KeyCode::Char('F') => Some(Self::OpenMenu(MenuId::File)),
                KeyCode::Char('n') | KeyCode::Char('N') => Some(Self::OpenMenu(MenuId::Navigate)),
                KeyCode::Char('v') | KeyCode::Char('V') => Some(Self::OpenMenu(MenuId::View)),
                KeyCode::Char('h') | KeyCode::Char('H') => Some(Self::OpenMenu(MenuId::Help)),
                KeyCode::Left => Some(Self::NavigateBack),
                KeyCode::Right => Some(Self::NavigateForward),
                _ => None,
            };
        }
        if key_event.code == KeyCode::Char('q') && key_event.modifiers == KeyModifiers::CONTROL {
            return Some(Self::Quit);
        }
        if key_event.code == KeyCode::Char('w') && key_event.modifiers == KeyModifiers::CONTROL {
            return Some(Self::CycleFocus);
        }
        if keymap.switch_pane.matches(&key_event) {
            return Some(Self::FocusNextPane);
        }
        // Delegate remaining keys to the comprehensive fallback handler.
        Self::from_key_event_with_settings(key_event, keymap)
    }

    pub fn from_editor_key_event(key_event: KeyEvent) -> Option<Self> {
        if key_event.modifiers == KeyModifiers::ALT {
            return match key_event.code {
                KeyCode::Char('f') | KeyCode::Char('F') => Some(Self::OpenMenu(MenuId::File)),
                KeyCode::Char('n') | KeyCode::Char('N') => Some(Self::OpenMenu(MenuId::Navigate)),
                KeyCode::Char('v') | KeyCode::Char('V') => Some(Self::OpenMenu(MenuId::View)),
                KeyCode::Char('h') | KeyCode::Char('H') => Some(Self::OpenMenu(MenuId::Help)),
                _ => None,
            };
        }

        match key_event.code {
            KeyCode::F(1) => Some(Self::OpenHelpDialog),
            KeyCode::Char('q') if key_event.modifiers == KeyModifiers::CONTROL => Some(Self::Quit),
            KeyCode::Char('o') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::OpenSettingsPanel)
            }
            KeyCode::Char('d') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::DiscardEditorChanges)
            }
            KeyCode::Char('f') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::EditorOpenSearch)
            }
            KeyCode::Esc | KeyCode::F(4) => Some(Self::CloseEditor),
            KeyCode::Backspace => Some(Self::EditorBackspace),
            KeyCode::Enter => Some(Self::EditorNewline),
            KeyCode::Left => Some(Self::EditorMoveLeft),
            KeyCode::Right => Some(Self::EditorMoveRight),
            KeyCode::Up => Some(Self::EditorMoveUp),
            KeyCode::Down => Some(Self::EditorMoveDown),
            KeyCode::Tab => Some(Self::FocusNextPane),
            KeyCode::F(3) if key_event.modifiers == KeyModifiers::SHIFT => {
                Some(Self::EditorSearchPrev)
            }
            KeyCode::F(3) => Some(Self::EditorSearchNext),
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

    pub fn from_menu_key_event(key_event: KeyEvent) -> Option<Self> {
        if key_event.modifiers == KeyModifiers::ALT {
            return match key_event.code {
                KeyCode::Char('f') | KeyCode::Char('F') => Some(Self::OpenMenu(MenuId::File)),
                KeyCode::Char('n') | KeyCode::Char('N') => Some(Self::OpenMenu(MenuId::Navigate)),
                KeyCode::Char('v') | KeyCode::Char('V') => Some(Self::OpenMenu(MenuId::View)),
                KeyCode::Char('h') | KeyCode::Char('H') => Some(Self::OpenMenu(MenuId::Help)),
                _ => None,
            };
        }

        match key_event.code {
            KeyCode::Esc => Some(Self::CloseMenu),
            KeyCode::Enter => Some(Self::MenuActivate),
            KeyCode::Left => Some(Self::MenuPrevious),
            KeyCode::Right | KeyCode::Tab => Some(Self::MenuNext),
            KeyCode::Up => Some(Self::MenuMoveUp),
            KeyCode::Down => Some(Self::MenuMoveDown),
            KeyCode::Char('q') if key_event.modifiers == KeyModifiers::CONTROL => Some(Self::Quit),
            KeyCode::Char(ch) if key_event.modifiers.is_empty() => Some(Self::MenuMnemonic(ch)),
            _ => None,
        }
    }

    pub fn from_prompt_key_event(key_event: KeyEvent) -> Option<Self> {
        match key_event.code {
            KeyCode::Esc => Some(Self::PromptCancel),
            KeyCode::Enter => Some(Self::PromptSubmit),
            KeyCode::Backspace => Some(Self::PromptBackspace),
            KeyCode::Char('q') if key_event.modifiers == KeyModifiers::CONTROL => Some(Self::Quit),
            KeyCode::Char(ch)
                if key_event.modifiers.is_empty() || key_event.modifiers == KeyModifiers::SHIFT =>
            {
                Some(Self::PromptInput(ch))
            }
            _ => None,
        }
    }

    pub fn from_dialog_key_event(key_event: KeyEvent) -> Option<Self> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::F(1) => Some(Self::CloseDialog),
            KeyCode::Char('q') if key_event.modifiers == KeyModifiers::CONTROL => Some(Self::Quit),
            KeyCode::Char('o') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::OpenSettingsPanel)
            }
            _ => None,
        }
    }

    pub fn from_collision_key_event(key_event: KeyEvent) -> Option<Self> {
        match key_event.code {
            KeyCode::Esc => Some(Self::CollisionCancel),
            KeyCode::Char('o') | KeyCode::Char('O') => Some(Self::CollisionOverwrite),
            KeyCode::Char('r') | KeyCode::Char('R') => Some(Self::CollisionRename),
            KeyCode::Char('s') | KeyCode::Char('S') => Some(Self::CollisionSkip),
            KeyCode::Char('q') if key_event.modifiers == KeyModifiers::CONTROL => Some(Self::Quit),
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

    use super::{Action, KeyBinding, MenuId};

    #[test]
    fn from_palette_key_event_handles_esc() {
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(Action::from_palette_key_event(key), Some(Action::CloseCommandPalette));
    }

    #[test]
    fn from_pane_key_event_handles_quit() {
        let keymap = RuntimeKeymap::default();
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert_eq!(Action::from_pane_key_event(key, &keymap), Some(Action::Quit));
    }

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
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE), &keymap),
            Some(Action::Quit)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE), &keymap),
            Some(Action::FocusNextPane)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL), &keymap),
            Some(Action::Refresh)
        );
    }

    #[test]
    fn movement_keys_remain_available() {
        let keymap = RuntimeKeymap::default();

        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE), &keymap),
            Some(Action::MoveSelectionDown)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), &keymap),
            Some(Action::MoveSelectionUp)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &keymap),
            Some(Action::EnterSelection)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), &keymap),
            Some(Action::NavigateToParent)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE), &keymap),
            Some(Action::ToggleMark)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Char('M'), KeyModifiers::SHIFT), &keymap),
            Some(Action::ClearMarks)
        );
    }

    #[test]
    fn editor_shortcuts_remain_available() {
        let keymap = RuntimeKeymap::default();

        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE), &keymap),
            Some(Action::OpenSelectedInEditor)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE), &keymap),
            Some(Action::OpenCopyPrompt)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE), &keymap),
            Some(Action::OpenRenamePrompt)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::F(6), KeyModifiers::SHIFT), &keymap),
            Some(Action::OpenMovePrompt)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE), &keymap),
            Some(Action::OpenDeletePrompt)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE), &keymap),
            Some(Action::OpenNewFilePrompt)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::F(7), KeyModifiers::SHIFT), &keymap),
            Some(Action::OpenNewDirectoryPrompt)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL), &keymap),
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
            Action::from_editor_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)),
            Some(Action::DiscardEditorChanges)
        );
        assert_eq!(
            Action::from_editor_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)),
            Some(Action::Quit)
        );
    }

    #[test]
    fn alt_menu_shortcuts_are_available() {
        let keymap = RuntimeKeymap::default();

        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::ALT), &keymap),
            Some(Action::OpenMenu(MenuId::File))
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::ALT), &keymap),
            Some(Action::OpenMenu(MenuId::View))
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::ALT), &keymap),
            Some(Action::OpenMenu(MenuId::Help))
        );
        assert_eq!(
            Action::from_menu_key_event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            Some(Action::MenuNext)
        );
    }

    #[test]
    fn prompt_shortcuts_are_available() {
        assert_eq!(
            Action::from_prompt_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(Action::PromptInput('a'))
        );
        assert_eq!(
            Action::from_prompt_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Action::PromptSubmit)
        );
    }

    #[test]
    fn help_shortcuts_are_available() {
        let keymap = RuntimeKeymap::default();

        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE), &keymap),
            Some(Action::OpenHelpDialog)
        );
        assert_eq!(
            Action::from_dialog_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Some(Action::CloseDialog)
        );
    }

    #[test]
    fn collision_shortcuts_are_available() {
        assert_eq!(
            Action::from_collision_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)),
            Some(Action::CollisionOverwrite)
        );
        assert_eq!(
            Action::from_collision_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
            Some(Action::CollisionRename)
        );
        assert_eq!(
            Action::from_collision_key_event(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
            Some(Action::CollisionSkip)
        );
    }
}
