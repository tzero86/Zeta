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
    Edit,
    Search,
    View,
    Themes,
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
    CloseBookmarks,
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
    OpenEditorReplace,
    EditorCloseSearch,
    EditorSearchBackspace,
    EditorSearchNext,
    EditorSearchPrev,
    EditorReplaceInput(char),
    EditorReplaceBackspace,
    EditorReplaceNext,
    EditorReplaceAll,
    FocusNextPane,
    CycleFocus,
    FocusPreviewPanel,
    OpenShell,
    ToggleTerminal,
    TerminalInput(Vec<u8>),
    OpenArchive {
        path: std::path::PathBuf,
    },
    ExitArchive,
    AddBookmark,
    OpenBookmarks,
    BookmarkConfirm,
    BookmarkDeleteCurrent,
    BookmarkMoveDown,
    BookmarkMoveUp,
    BookmarkSelect(usize),
    DeleteBookmark(usize),
    OpenPaneFilter,
    PaneFilterInput(char),
    PaneFilterBackspace,
    ClosePaneFilter,
    ScrollPreviewDown,
    ScrollPreviewUp,
    ScrollPreviewPageDown,
    ScrollPreviewPageUp,
    ScrollDialogDown,
    ScrollDialogUp,
    ScrollDialogPageDown,
    ScrollDialogPageUp,
    MenuActivate,
    /// Mouse click on a menu item — set selection to `index` and activate.
    MenuClickItem(usize),
    /// Mouse hover over a menu item — update the highlighted selection.
    MenuSetSelection(usize),
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
    OpenPermanentDeletePrompt,
    OpenHelpDialog,
    OpenMovePrompt,
    OpenMenu(MenuId),
    OpenNewDirectoryPrompt,
    OpenNewFilePrompt,
    OpenRenamePrompt,
    /// Start inline (in-place) rename of the selected entry. Buffer pre-filled with current name.
    BeginInlineRename,
    /// Commit the inline rename buffer: perform the filesystem rename.
    ConfirmInlineRename,
    /// Discard the inline rename buffer without renaming.
    CancelInlineRename,
    /// Append a character to the inline rename buffer.
    InlineRenameType(char),
    /// Delete the last character from the inline rename buffer.
    InlineRenameBackspace,
    OpenSelectedInEditor,
    OpenSettingsPanel,
    PreviewFile {
        path: PathBuf,
    },
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
    ToggleEditorFullscreen,
    ToggleMarkdownPreview,
    FocusMarkdownPreview,
    ScrollMarkdownPreviewUp,
    ScrollMarkdownPreviewDown,
    ScrollMarkdownPreviewPageUp,
    ScrollMarkdownPreviewPageDown,
    Quit,
    Resize {
        width: u16,
        height: u16,
    },
    OpenCommandPalette,
    CloseCommandPalette,
    OpenFileFinder,
    CloseFileFinder,
    FileFinderInput(char),
    FileFinderBackspace,
    FileFinderConfirm,
    FileFinderMoveDown,
    FileFinderMoveUp,
    CloseSettingsPanel,
    OpenSshConnect,
    SshDialogInput(char),
    SshDialogBackspace,
    SshDialogToggleField,
    SshDialogToggleAuthMethod,
    SshConnectConfirm,
    SshDisconnect,
    CloseSshConnect,
    PaletteInput(char),
    PaletteBackspace,
    PaletteConfirm,
    PaletteMoveDown,
    PaletteMoveUp,
    SettingsMoveDown,
    SettingsMoveUp,
    SettingsToggleCurrent,
    CycleSortMode,
    ToggleDiffMode,
    DiffSyncToOther,
    /// Toggle between the compact name-only list and the detailed columns view.
    ToggleDetailsView,
    /// Mouse click on a pane entry row.
    PaneClick {
        left_pane: bool,
        row: usize,
    },
    /// Mouse double-click on a pane entry row (enter dir / open file).
    PaneDoubleClick {
        left_pane: bool,
        row: usize,
    },
    /// Open the selected file with the OS default application.
    OpenInDefaultApp,
    /// Extend the pane selection downward, marking each stepped-over entry.
    ExtendSelectionDown,
    /// Extend the pane selection upward, marking each stepped-over entry.
    ExtendSelectionUp,
    /// Copy the selected entry's path to the system clipboard.
    CopyPathToClipboard,
    /// Paste text from the system clipboard at the editor cursor.
    EditorPaste,
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
    SpawnTerminal {
        cwd: PathBuf,
    },
    WriteTerminal(Vec<u8>),
    ResizeTerminal {
        cols: u16,
        rows: u16,
    },
    ScanPane {
        pane: PaneId,
        path: PathBuf,
    },
    FindFiles {
        pane: PaneId,
        root: PathBuf,
        max_depth: usize,
    },
    OpenArchive {
        path: PathBuf,
        inner: PathBuf,
    },
    OpenShell {
        path: PathBuf,
    },
    ConnectSSH {
        address: String,
        auth_method: crate::state::ssh::SshAuthMethod,
        credential: String,
        pane: PaneId,
    },
    DisconnectSSH {
        pane: PaneId,
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
    Trash {
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
    /// Extract files or directories from an archive into a destination directory.
    ExtractArchive {
        archive: PathBuf,
        inner_path: PathBuf,
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
                KeyCode::Char('e') | KeyCode::Char('E') => Some(Self::OpenMenu(MenuId::Edit)),
                KeyCode::Char('s') | KeyCode::Char('S') => Some(Self::OpenMenu(MenuId::Search)),
                KeyCode::Char('v') | KeyCode::Char('V') => Some(Self::OpenMenu(MenuId::View)),
                KeyCode::Char('t') | KeyCode::Char('T') => Some(Self::OpenMenu(MenuId::Themes)),
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
            KeyCode::F(2) => Some(Self::ToggleTerminal),
            KeyCode::Char('P') if key_event.modifiers == KeyModifiers::SHIFT => {
                Some(Self::OpenCommandPalette)
            }
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
            KeyCode::F(8) if key_event.modifiers == KeyModifiers::SHIFT => {
                Some(Self::OpenPermanentDeletePrompt)
            }
            KeyCode::F(8) => Some(Self::OpenDeletePrompt),
            KeyCode::Insert => Some(Self::OpenNewFilePrompt),
            KeyCode::F(7) if key_event.modifiers == KeyModifiers::SHIFT => {
                Some(Self::OpenNewDirectoryPrompt)
            }
            KeyCode::F(10) => Some(Self::ToggleDiffMode),
            KeyCode::Char('d') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::DiffSyncToOther)
            }
            KeyCode::Enter | KeyCode::Right => Some(Self::EnterSelection),
            // Char('l') without Ctrl is the vim right/enter binding.
            KeyCode::Char('l') if key_event.modifiers == KeyModifiers::NONE => {
                Some(Self::EnterSelection)
            }
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => Some(Self::NavigateToParent),
            KeyCode::Char(' ') => Some(Self::ToggleMark),
            KeyCode::Char('M') if key_event.modifiers == KeyModifiers::SHIFT => {
                Some(Self::ClearMarks)
            }
            KeyCode::Char('s') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::SaveEditor)
            }
            KeyCode::Char('p') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::OpenFileFinder)
            }
            KeyCode::Char('p') | KeyCode::Char('P')
                if key_event.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) =>
            {
                Some(Self::OpenCommandPalette)
            }
            KeyCode::Char('o') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::OpenSettingsPanel)
            }
            KeyCode::Char('b') | KeyCode::Char('B')
                if key_event.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) =>
            {
                Some(Self::OpenBookmarks)
            }
            KeyCode::Char('/') => Some(Self::OpenPaneFilter),
            // Shift+arrow range-select must come before plain Down/Up (guards don't apply to OR patterns).
            KeyCode::Down if key_event.modifiers == KeyModifiers::SHIFT => {
                Some(Self::ExtendSelectionDown)
            }
            KeyCode::Up if key_event.modifiers == KeyModifiers::SHIFT => {
                Some(Self::ExtendSelectionUp)
            }
            KeyCode::Down | KeyCode::Char('j') => Some(Self::MoveSelectionDown),
            KeyCode::Up | KeyCode::Char('k') => Some(Self::MoveSelectionUp),
            KeyCode::Char('o') if key_event.modifiers == KeyModifiers::NONE => {
                Some(Self::OpenInDefaultApp)
            }
            KeyCode::Char('c') | KeyCode::Char('C')
                if key_event.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) =>
            {
                Some(Self::CopyPathToClipboard)
            }
            KeyCode::Char('s') | KeyCode::Char('S')
                if key_event.modifiers == KeyModifiers::NONE
                    || key_event.modifiers == KeyModifiers::SHIFT =>
            {
                Some(Self::CycleSortMode)
            }
            KeyCode::Char('l') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::ToggleDetailsView)
            }
            // 'r' for inline rename (intuitive, not otherwise bound in pane context).
            KeyCode::Char('r') if key_event.modifiers == KeyModifiers::NONE => {
                Some(Self::BeginInlineRename)
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

    /// Keys when the bookmarks modal is open. Consumes ALL input.
    pub fn from_bookmarks_key_event(key_event: KeyEvent) -> Option<Self> {
        match key_event.code {
            KeyCode::Esc => Some(Self::CloseBookmarks),
            KeyCode::Enter => Some(Self::BookmarkConfirm),
            KeyCode::Delete => Some(Self::BookmarkDeleteCurrent),
            KeyCode::Up => Some(Self::BookmarkMoveUp),
            KeyCode::Down => Some(Self::BookmarkMoveDown),
            _ => None,
        }
    }

    /// Keys when the active pane quick-filter is open. Consumes ALL input.
    pub fn from_pane_filter_key_event(key_event: KeyEvent) -> Option<Self> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Enter => Some(Self::ClosePaneFilter),
            KeyCode::Backspace => Some(Self::PaneFilterBackspace),
            KeyCode::Char(ch)
                if key_event.modifiers.is_empty() || key_event.modifiers == KeyModifiers::SHIFT =>
            {
                Some(Self::PaneFilterInput(ch))
            }
            _ => None,
        }
    }

    /// Keys when inline rename is active. Consumes ALL input.
    pub fn from_inline_rename_key_event(key_event: KeyEvent) -> Option<Self> {
        match key_event.code {
            KeyCode::Esc => Some(Self::CancelInlineRename),
            KeyCode::Enter => Some(Self::ConfirmInlineRename),
            KeyCode::Backspace => Some(Self::InlineRenameBackspace),
            KeyCode::Char(ch)
                if key_event.modifiers.is_empty() || key_event.modifiers == KeyModifiers::SHIFT =>
            {
                Some(Self::InlineRenameType(ch))
            }
            _ => None,
        }
    }

    /// Keys when the file finder modal is open. Consumes ALL input.
    pub fn from_file_finder_key_event(key_event: KeyEvent) -> Option<Self> {
        match key_event.code {
            KeyCode::Esc => Some(Self::CloseFileFinder),
            KeyCode::Enter => Some(Self::FileFinderConfirm),
            KeyCode::Up => Some(Self::FileFinderMoveUp),
            KeyCode::Down => Some(Self::FileFinderMoveDown),
            KeyCode::Backspace => Some(Self::FileFinderBackspace),
            KeyCode::Char(ch)
                if key_event.modifiers.is_empty() || key_event.modifiers == KeyModifiers::SHIFT =>
            {
                Some(Self::FileFinderInput(ch))
            }
            _ => None,
        }
    }

    pub fn from_terminal_key_event(key_event: KeyEvent) -> Option<Self> {
        // Toggle key: F2 or Ctrl+T or Ctrl+\
        if key_event.code == KeyCode::F(2)
            || (key_event.code == KeyCode::Char('\\')
                && key_event.modifiers == KeyModifiers::CONTROL)
        {
            return Some(Self::ToggleTerminal);
        }

        // Map some common keys to terminal sequences
        match key_event.code {
            KeyCode::Char(c) => {
                if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                    if c.is_ascii_lowercase() {
                        return Some(Self::TerminalInput(vec![c as u8 - b'a' + 1]));
                    }
                    if c.is_ascii_uppercase() {
                        return Some(Self::TerminalInput(vec![c as u8 - b'A' + 1]));
                    }
                    match c {
                        '[' => return Some(Self::TerminalInput(vec![27])),
                        '\\' => return Some(Self::TerminalInput(vec![28])),
                        ']' => return Some(Self::TerminalInput(vec![29])),
                        '^' => return Some(Self::TerminalInput(vec![30])),
                        '_' => return Some(Self::TerminalInput(vec![31])),
                        _ => {}
                    }
                }
                Some(Self::TerminalInput(c.to_string().into_bytes()))
            }
            KeyCode::Enter => {
                if cfg!(windows) {
                    Some(Self::TerminalInput(vec![b'\r', b'\n']))
                } else {
                    Some(Self::TerminalInput(vec![b'\r']))
                }
            }
            KeyCode::Backspace => Some(Self::TerminalInput(vec![127])),
            KeyCode::Tab => Some(Self::TerminalInput(vec![b'\t'])),
            KeyCode::Esc => Some(Self::TerminalInput(vec![27])),
            KeyCode::Up => Some(Self::TerminalInput(vec![27, b'[', b'A'])),
            KeyCode::Down => Some(Self::TerminalInput(vec![27, b'[', b'B'])),
            KeyCode::Right => Some(Self::TerminalInput(vec![27, b'[', b'C'])),
            KeyCode::Left => Some(Self::TerminalInput(vec![27, b'[', b'D'])),
            _ => None,
        }
    }

    /// Keys when the SSH connect dialog is open. Consumes ALL input.
    pub fn from_ssh_connect_key_event(key_event: KeyEvent) -> Option<Self> {
        match key_event.code {
            KeyCode::Esc => Some(Self::CloseSshConnect),
            KeyCode::Enter => Some(Self::SshConnectConfirm),
            KeyCode::Backspace => Some(Self::SshDialogBackspace),
            KeyCode::Tab => Some(Self::SshDialogToggleField),
            KeyCode::Char(' ')
                if key_event.modifiers.is_empty() || key_event.modifiers == KeyModifiers::SHIFT =>
            {
                Some(Self::SshDialogToggleAuthMethod)
            }
            KeyCode::Char(ch)
                if key_event.modifiers.is_empty() || key_event.modifiers == KeyModifiers::SHIFT =>
            {
                Some(Self::SshDialogInput(ch))
            }
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
            KeyCode::Char('b') | KeyCode::Char('B')
                if key_event.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) =>
            {
                Some(Self::OpenBookmarks)
            }
            _ => None,
        }
    }

    /// Keys when the markdown preview split has keyboard focus.
    pub fn from_markdown_preview_key_event(key_event: KeyEvent) -> Option<Self> {
        match key_event.code {
            KeyCode::Up => Some(Self::ScrollMarkdownPreviewUp),
            KeyCode::Down => Some(Self::ScrollMarkdownPreviewDown),
            KeyCode::PageUp => Some(Self::ScrollMarkdownPreviewPageUp),
            KeyCode::PageDown => Some(Self::ScrollMarkdownPreviewPageDown),
            // Esc or Tab returns focus to the editor.
            KeyCode::Esc | KeyCode::Tab => Some(Self::FocusMarkdownPreview),
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
                KeyCode::Char('e') | KeyCode::Char('E') => Some(Self::OpenMenu(MenuId::Edit)),
                KeyCode::Char('s') | KeyCode::Char('S') => Some(Self::OpenMenu(MenuId::Search)),
                KeyCode::Char('v') | KeyCode::Char('V') => Some(Self::OpenMenu(MenuId::View)),
                KeyCode::Char('t') | KeyCode::Char('T') => Some(Self::OpenMenu(MenuId::Themes)),
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
        if key_event.code == KeyCode::Char('b') && key_event.modifiers == KeyModifiers::CONTROL {
            return Some(Self::AddBookmark);
        }
        // Delegate remaining keys to the comprehensive fallback handler.
        Self::from_key_event_with_settings(key_event, keymap)
    }

    pub fn from_editor_key_event(key_event: KeyEvent) -> Option<Self> {
        if key_event.modifiers == KeyModifiers::ALT {
            return match key_event.code {
                KeyCode::Char('f') | KeyCode::Char('F') => Some(Self::OpenMenu(MenuId::File)),
                KeyCode::Char('n') | KeyCode::Char('N') => Some(Self::OpenMenu(MenuId::Navigate)),
                KeyCode::Char('e') | KeyCode::Char('E') => Some(Self::OpenMenu(MenuId::Edit)),
                KeyCode::Char('s') | KeyCode::Char('S') => Some(Self::OpenMenu(MenuId::Search)),
                KeyCode::Char('v') | KeyCode::Char('V') => Some(Self::OpenMenu(MenuId::View)),
                KeyCode::Char('t') | KeyCode::Char('T') => Some(Self::OpenMenu(MenuId::Themes)),
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
            KeyCode::Char('b') | KeyCode::Char('B')
                if key_event.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) =>
            {
                Some(Self::OpenBookmarks)
            }
            KeyCode::Char('d') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::DiscardEditorChanges)
            }
            KeyCode::Char('f') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::EditorOpenSearch)
            }
            KeyCode::Char('h') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::OpenEditorReplace)
            }
            KeyCode::Char('h') | KeyCode::Char('H')
                if key_event.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) =>
            {
                Some(Self::EditorReplaceAll)
            }
            KeyCode::F(11) if key_event.modifiers == KeyModifiers::SHIFT => {
                Some(Self::ToggleEditorFullscreen)
            }
            KeyCode::F(11) => Some(Self::ToggleEditorFullscreen),
            KeyCode::Char('m') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::ToggleMarkdownPreview)
            }
            KeyCode::Tab => Some(Self::FocusMarkdownPreview),
            KeyCode::Esc | KeyCode::F(4) => Some(Self::CloseEditor),
            KeyCode::Backspace => Some(Self::EditorBackspace),
            KeyCode::Enter => Some(Self::EditorNewline),
            KeyCode::Left => Some(Self::EditorMoveLeft),
            KeyCode::Right => Some(Self::EditorMoveRight),
            KeyCode::Up => Some(Self::EditorMoveUp),
            KeyCode::Down => Some(Self::EditorMoveDown),
            KeyCode::F(3) if key_event.modifiers == KeyModifiers::SHIFT => {
                Some(Self::EditorSearchPrev)
            }
            KeyCode::F(3) => Some(Self::EditorSearchNext),
            KeyCode::Char('s') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::SaveEditor)
            }
            KeyCode::Char('v') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(Self::EditorPaste)
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
                KeyCode::Char('e') | KeyCode::Char('E') => Some(Self::OpenMenu(MenuId::Edit)),
                KeyCode::Char('s') | KeyCode::Char('S') => Some(Self::OpenMenu(MenuId::Search)),
                KeyCode::Char('v') | KeyCode::Char('V') => Some(Self::OpenMenu(MenuId::View)),
                KeyCode::Char('t') | KeyCode::Char('T') => Some(Self::OpenMenu(MenuId::Themes)),
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
            KeyCode::Char('b') | KeyCode::Char('B')
                if key_event.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) =>
            {
                Some(Self::OpenBookmarks)
            }
            KeyCode::Down => Some(Self::ScrollDialogDown),
            KeyCode::Up => Some(Self::ScrollDialogUp),
            KeyCode::PageDown => Some(Self::ScrollDialogPageDown),
            KeyCode::PageUp => Some(Self::ScrollDialogPageUp),
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
        assert_eq!(
            Action::from_palette_key_event(key),
            Some(Action::CloseCommandPalette)
        );
    }

    #[test]
    fn from_pane_key_event_handles_quit() {
        let keymap = RuntimeKeymap::default();
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert_eq!(
            Action::from_pane_key_event(key, &keymap),
            Some(Action::Quit)
        );
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
            Action::from_pane_key_event(
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
                &keymap
            ),
            Some(Action::Quit)
        );
        assert_eq!(
            Action::from_pane_key_event(
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
                &keymap
            ),
            Some(Action::FocusNextPane)
        );
        assert_eq!(
            Action::from_pane_key_event(
                KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
                &keymap
            ),
            Some(Action::Refresh)
        );
    }

    #[test]
    fn movement_keys_remain_available() {
        let keymap = RuntimeKeymap::default();

        assert_eq!(
            Action::from_pane_key_event(
                KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
                &keymap
            ),
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
            Action::from_pane_key_event(
                KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
                &keymap
            ),
            Some(Action::ToggleMark)
        );
        assert_eq!(
            Action::from_pane_key_event(
                KeyEvent::new(KeyCode::Char('M'), KeyModifiers::SHIFT),
                &keymap
            ),
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
            Action::from_pane_key_event(KeyEvent::new(KeyCode::F(8), KeyModifiers::SHIFT), &keymap),
            Some(Action::OpenPermanentDeletePrompt)
        );
        assert_eq!(
            Action::from_pane_key_event(
                KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE),
                &keymap
            ),
            Some(Action::OpenNewFilePrompt)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::F(7), KeyModifiers::SHIFT), &keymap),
            Some(Action::OpenNewDirectoryPrompt)
        );
        assert_eq!(
            Action::from_pane_key_event(
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
                &keymap
            ),
            Some(Action::SaveEditor)
        );
        assert_eq!(
            Action::from_pane_key_event(
                KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
                &keymap
            ),
            Some(Action::AddBookmark)
        );
        assert_eq!(
            Action::from_pane_key_event(
                KeyEvent::new(
                    KeyCode::Char('B'),
                    KeyModifiers::CONTROL | KeyModifiers::SHIFT
                ),
                &keymap,
            ),
            Some(Action::OpenBookmarks)
        );
        assert_eq!(
            Action::from_pane_key_event(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE), &keymap),
            Some(Action::ToggleTerminal)
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
            Action::from_pane_key_event(
                KeyEvent::new(KeyCode::Char('f'), KeyModifiers::ALT),
                &keymap
            ),
            Some(Action::OpenMenu(MenuId::File))
        );
        assert_eq!(
            Action::from_pane_key_event(
                KeyEvent::new(KeyCode::Char('v'), KeyModifiers::ALT),
                &keymap
            ),
            Some(Action::OpenMenu(MenuId::View))
        );
        assert_eq!(
            Action::from_pane_key_event(
                KeyEvent::new(KeyCode::Char('h'), KeyModifiers::ALT),
                &keymap
            ),
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
