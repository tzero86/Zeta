use crate::action::Action;
use crate::action::MenuId;
use crate::config::ThemePreset;

use super::MenuItem;
use super::PaneLayout;

pub fn menu_items_for(menu: MenuId) -> Vec<MenuItem> {
    match menu {
        MenuId::File => vec![
            MenuItem {
                label: "Open in Editor",
                shortcut: "F4",
                mnemonic: 'o',
                action: Action::OpenSelectedInEditor,
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
                label: "Delete",
                shortcut: "F8",
                mnemonic: 'd',
                action: Action::OpenDeletePrompt,
            },
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
                label: "Theme: Fjord",
                shortcut: "1",
                mnemonic: 'f',
                action: Action::SetTheme(ThemePreset::Fjord),
            },
            MenuItem {
                label: "Theme: Sandbar",
                shortcut: "2",
                mnemonic: 's',
                action: Action::SetTheme(ThemePreset::Sandbar),
            },
            MenuItem {
                label: "Theme: Oxide",
                shortcut: "3",
                mnemonic: 'o',
                action: Action::SetTheme(ThemePreset::Oxide),
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
    }
}
