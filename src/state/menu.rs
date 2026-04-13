use crate::action::Action;
use crate::action::MenuId;
use crate::config::ThemePreset;

use super::MenuItem;
use super::PaneLayout;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MenuTab {
    pub id: MenuId,
    pub label: &'static str,
    pub mnemonic: char,
}

pub fn menu_tabs(editor_mode: bool) -> Vec<MenuTab> {
    if editor_mode {
        vec![
            MenuTab {
                id: MenuId::File,
                label: " File ",
                mnemonic: 'f',
            },
            MenuTab {
                id: MenuId::Edit,
                label: " Edit ",
                mnemonic: 'e',
            },
            MenuTab {
                id: MenuId::Search,
                label: " Search ",
                mnemonic: 's',
            },
            MenuTab {
                id: MenuId::View,
                label: " View ",
                mnemonic: 'v',
            },
            MenuTab {
                id: MenuId::Themes,
                label: " Themes ",
                mnemonic: 't',
            },
            MenuTab {
                id: MenuId::Help,
                label: " Help ",
                mnemonic: 'h',
            },
        ]
    } else {
        vec![
            MenuTab {
                id: MenuId::File,
                label: " File ",
                mnemonic: 'f',
            },
            MenuTab {
                id: MenuId::Navigate,
                label: " Navigate ",
                mnemonic: 'n',
            },
            MenuTab {
                id: MenuId::View,
                label: " View ",
                mnemonic: 'v',
            },
            MenuTab {
                id: MenuId::Themes,
                label: " Themes ",
                mnemonic: 't',
            },
            MenuTab {
                id: MenuId::Help,
                label: " Help ",
                mnemonic: 'h',
            },
        ]
    }
}

pub fn menu_items_for(menu: MenuId, editor_mode: bool) -> Vec<MenuItem> {
    if editor_mode {
        match menu {
            MenuId::File => vec![
                MenuItem {
                    label: "Save",
                    shortcut: "Ctrl+S",
                    mnemonic: 's',
                    action: Action::SaveEditor,
                },
                MenuItem {
                    label: "Discard Changes",
                    shortcut: "Ctrl+D",
                    mnemonic: 'd',
                    action: Action::DiscardEditorChanges,
                },
                MenuItem {
                    label: "Close Editor",
                    shortcut: "Esc",
                    mnemonic: 'c',
                    action: Action::CloseEditor,
                },
                MenuItem {
                    label: "Quit",
                    shortcut: "Ctrl+Q",
                    mnemonic: 'q',
                    action: Action::Quit,
                },
            ],
            MenuId::Edit => vec![
                MenuItem {
                    label: "Find",
                    shortcut: "Ctrl+F",
                    mnemonic: 'f',
                    action: Action::EditorOpenSearch,
                },
                MenuItem {
                    label: "Replace",
                    shortcut: "Ctrl+H",
                    mnemonic: 'r',
                    action: Action::OpenEditorReplace,
                },
                MenuItem {
                    label: "Replace All",
                    shortcut: "Ctrl+Shift+H",
                    mnemonic: 'a',
                    action: Action::EditorReplaceAll,
                },
            ],
            MenuId::Search => vec![
                MenuItem {
                    label: "Next Match",
                    shortcut: "F3",
                    mnemonic: 'n',
                    action: Action::EditorSearchNext,
                },
                MenuItem {
                    label: "Previous Match",
                    shortcut: "Shift+F3",
                    mnemonic: 'p',
                    action: Action::EditorSearchPrev,
                },
                MenuItem {
                    label: "Toggle Markdown Preview",
                    shortcut: "Ctrl+M",
                    mnemonic: 'm',
                    action: Action::ToggleMarkdownPreview,
                },
            ],
            MenuId::View => vec![
                MenuItem {
                    label: "Fullscreen Editor",
                    shortcut: "Shift+F11",
                    mnemonic: 'f',
                    action: Action::ToggleEditorFullscreen,
                },
                MenuItem {
                    label: "Settings",
                    shortcut: "Ctrl+O",
                    mnemonic: 's',
                    action: Action::OpenSettingsPanel,
                },
                MenuItem {
                    label: "Themes...",
                    shortcut: "T",
                    mnemonic: 't',
                    action: Action::OpenMenu(MenuId::Themes),
                },
            ],
            MenuId::Themes => vec![
                MenuItem {
                    label: "Theme: Zeta (default)",
                    shortcut: "Z",
                    mnemonic: 'z',
                    action: Action::SetTheme(ThemePreset::Zeta),
                },
                MenuItem {
                    label: "Theme: Neon",
                    shortcut: "E",
                    mnemonic: 'e',
                    action: Action::SetTheme(ThemePreset::Neon),
                },
                MenuItem {
                    label: "Theme: Monochrome",
                    shortcut: "O",
                    mnemonic: 'o',
                    action: Action::SetTheme(ThemePreset::Monochrome),
                },
                MenuItem {
                    label: "Theme: Matrix",
                    shortcut: "M",
                    mnemonic: 'm',
                    action: Action::SetTheme(ThemePreset::Matrix),
                },
                MenuItem {
                    label: "Theme: Norton Commander",
                    shortcut: "N",
                    mnemonic: 'n',
                    action: Action::SetTheme(ThemePreset::Norton),
                },
                MenuItem {
                    label: "Theme: Fjord",
                    shortcut: "F",
                    mnemonic: 'f',
                    action: Action::SetTheme(ThemePreset::Fjord),
                },
                MenuItem {
                    label: "Theme: Sandbar",
                    shortcut: "S",
                    mnemonic: 's',
                    action: Action::SetTheme(ThemePreset::Sandbar),
                },
                MenuItem {
                    label: "Theme: Oxide",
                    shortcut: "X",
                    mnemonic: 'x',
                    action: Action::SetTheme(ThemePreset::Oxide),
                },
                MenuItem {
                    label: "Theme: Dracula",
                    shortcut: "D",
                    mnemonic: 'd',
                    action: Action::SetTheme(ThemePreset::Dracula),
                },
            ],
            MenuId::Help => vec![
                MenuItem {
                    label: "Help",
                    shortcut: "F1",
                    mnemonic: 'h',
                    action: Action::OpenHelpDialog,
                },
                MenuItem {
                    label: "About Zeta",
                    shortcut: "Enter",
                    mnemonic: 'a',
                    action: Action::OpenAboutDialog,
                },
            ],
            _ => vec![],
        }
    } else {
        match menu {
            MenuId::File => vec![
                MenuItem {
                    label: "Open Shell",
                    shortcut: "F2",
                    mnemonic: 's',
                    action: Action::OpenShell,
                },
                MenuItem {
                    label: "Open in Editor",
                    shortcut: "F4",
                    mnemonic: 'o',
                    action: Action::OpenSelectedInEditor,
                },
                MenuItem {
                    label: "Open Externally",
                    shortcut: "o",
                    mnemonic: 'e',
                    action: Action::OpenInDefaultApp,
                },
                MenuItem {
                    label: "Copy",
                    shortcut: "F5",
                    mnemonic: 'c',
                    action: Action::OpenCopyPrompt,
                },
                MenuItem {
                    label: "Move",
                    shortcut: "Shift+F6",
                    mnemonic: 'v',
                    action: Action::OpenMovePrompt,
                },
                MenuItem {
                    label: "New File",
                    shortcut: "Ins",
                    mnemonic: 'n',
                    action: Action::OpenNewFilePrompt,
                },
                MenuItem {
                    label: "New Directory",
                    shortcut: "Shift+F7",
                    mnemonic: 'm',
                    action: Action::OpenNewDirectoryPrompt,
                },
                MenuItem {
                    label: "Rename",
                    shortcut: "F6",
                    mnemonic: 'r',
                    action: Action::OpenRenamePrompt,
                },
                MenuItem {
                    label: "Move to Trash",
                    shortcut: "F8",
                    mnemonic: 't',
                    action: Action::OpenDeletePrompt,
                },
                MenuItem {
                    label: "Delete Permanently",
                    shortcut: "Shift+F8",
                    mnemonic: 'd',
                    action: Action::OpenPermanentDeletePrompt,
                },
                MenuItem {
                    label: "Quit",
                    shortcut: "Ctrl+Q",
                    mnemonic: 'q',
                    action: Action::Quit,
                },
            ],
            MenuId::Navigate => vec![
                MenuItem {
                    label: "Open Directory",
                    shortcut: "Enter",
                    mnemonic: 'o',
                    action: Action::EnterSelection,
                },
                MenuItem {
                    label: "Parent Directory",
                    shortcut: "Backspace",
                    mnemonic: 'p',
                    action: Action::NavigateToParent,
                },
                MenuItem {
                    label: "Connect SSH...",
                    shortcut: "Ctrl+R",
                    mnemonic: 'c',
                    action: Action::OpenSshConnect,
                },
                MenuItem {
                    label: "Add Bookmark",
                    shortcut: "Ctrl+B",
                    mnemonic: 'b',
                    action: Action::AddBookmark,
                },
                MenuItem {
                    label: "Show Bookmarks",
                    shortcut: "Ctrl+Shift+B",
                    mnemonic: 'k',
                    action: Action::OpenBookmarks,
                },
                MenuItem {
                    label: "Refresh",
                    shortcut: "r",
                    mnemonic: 'r',
                    action: Action::Refresh,
                },
                MenuItem {
                    label: "Switch Pane",
                    shortcut: "Tab",
                    mnemonic: 's',
                    action: Action::FocusNextPane,
                },
            ],
            MenuId::View => vec![
                MenuItem {
                    label: "Toggle Hidden Files",
                    shortcut: ".",
                    mnemonic: 'h',
                    action: Action::ToggleHiddenFiles,
                },
                MenuItem {
                    label: "Settings",
                    shortcut: "Ctrl+O",
                    mnemonic: 's',
                    action: Action::OpenSettingsPanel,
                },
                MenuItem {
                    label: "Layout: Side by Side",
                    shortcut: "4",
                    mnemonic: 'l',
                    action: Action::SetPaneLayout(PaneLayout::SideBySide),
                },
                MenuItem {
                    label: "Layout: Stacked",
                    shortcut: "5",
                    mnemonic: 'k',
                    action: Action::SetPaneLayout(PaneLayout::Stacked),
                },
                MenuItem {
                    label: "Themes...",
                    shortcut: "T",
                    mnemonic: 't',
                    action: Action::OpenMenu(MenuId::Themes),
                },
                MenuItem {
                    label: "Toggle Details View",
                    shortcut: "Ctrl+L",
                    mnemonic: 'd',
                    action: Action::ToggleDetailsView,
                },
            ],
            MenuId::Themes => vec![
                MenuItem {
                    label: "Theme: Zeta (default)",
                    shortcut: "Z",
                    mnemonic: 'z',
                    action: Action::SetTheme(ThemePreset::Zeta),
                },
                MenuItem {
                    label: "Theme: Neon",
                    shortcut: "E",
                    mnemonic: 'e',
                    action: Action::SetTheme(ThemePreset::Neon),
                },
                MenuItem {
                    label: "Theme: Monochrome",
                    shortcut: "O",
                    mnemonic: 'o',
                    action: Action::SetTheme(ThemePreset::Monochrome),
                },
                MenuItem {
                    label: "Theme: Matrix",
                    shortcut: "M",
                    mnemonic: 'm',
                    action: Action::SetTheme(ThemePreset::Matrix),
                },
                MenuItem {
                    label: "Theme: Norton Commander",
                    shortcut: "N",
                    mnemonic: 'n',
                    action: Action::SetTheme(ThemePreset::Norton),
                },
                MenuItem {
                    label: "Theme: Fjord",
                    shortcut: "F",
                    mnemonic: 'f',
                    action: Action::SetTheme(ThemePreset::Fjord),
                },
                MenuItem {
                    label: "Theme: Sandbar",
                    shortcut: "S",
                    mnemonic: 's',
                    action: Action::SetTheme(ThemePreset::Sandbar),
                },
                MenuItem {
                    label: "Theme: Oxide",
                    shortcut: "X",
                    mnemonic: 'x',
                    action: Action::SetTheme(ThemePreset::Oxide),
                },
                MenuItem {
                    label: "Theme: Dracula",
                    shortcut: "D",
                    mnemonic: 'd',
                    action: Action::SetTheme(ThemePreset::Dracula),
                },
            ],
            MenuId::Help => vec![
                MenuItem {
                    label: "Help",
                    shortcut: "F1",
                    mnemonic: 'h',
                    action: Action::OpenHelpDialog,
                },
                MenuItem {
                    label: "About Zeta",
                    shortcut: "Enter",
                    mnemonic: 'a',
                    action: Action::OpenAboutDialog,
                },
            ],
            _ => vec![],
        }
    }
}
