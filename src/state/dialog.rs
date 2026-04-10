use std::path::Path;
use std::path::PathBuf;

use crate::action::FileOperation;
use crate::action::RefreshTarget;
use crate::fs::suggest_non_conflicting_path;

use super::prompt::prompt_base_path;
use super::PromptKind;
use super::PromptState;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DialogState {
    pub title: &'static str,
    pub lines: Vec<String>,
}

impl DialogState {
    pub fn about(theme_name: String, config_path: String) -> Self {
        Self {
            title: "About Zeta",
            lines: vec![
                String::from(" ____  ________  ____             __               "),
                String::from("|    \\|        \\|    \\           |  \\              "),
                String::from("| $$$$ \\$$$$$$$$ \\$$$$  ______  _| $$_     ______  "),
                String::from("| $$      /  $$   | $$ /      \\|   $$ \\   |      \\ "),
                String::from("| $$     /  $$    | $$|  $$$$$$\\\\$$$$$$    \\$$$$$$\\"),
                String::from("| $$    /  $$     | $$| $$    $$ | $$ __  /      $$"),
                String::from("| $$_  /  $$___  _| $$| $$$$$$$$ | $$|  \\|  $$$$$$$"),
                String::from("| $$ \\|  $$    \\|   $$ \\$$     \\  \\$$  $$ \\$$    $$"),
                String::from(" \\$$$$ \\$$$$$$$$ \\$$$$  \\$$$$$$$   \\$$$$   \\$$$$$$$"),
                String::new(),
                String::from("Zeta is a keyboard-first dual-pane file manager and lightweight editor."),
                format!("Version: {}", env!("CARGO_PKG_VERSION")),
                String::new(),
                String::from("By tzero86"),
                String::new(),
                format!("Theme: {theme_name}"),
                format!("Config: {config_path}"),
                String::from("Icons: Unicode by default with ASCII fallback"),
                String::from("Set icon_mode = \"ascii\" or \"custom\" in config.toml"),
                String::from("Settings: Ctrl+O opens the settings panel"),
                String::from("Core features: dual panes, stacked layouts, editor, menus, dialogs, theme switching, and file operations"),
                String::new(),
                String::from("Esc or Enter closes this window"),
            ],
        }
    }

    pub fn help() -> Self {
        Self {
            title: " Help ",
            lines: vec![
                String::from("##Navigation"),
                String::from("  Enter\tOpen directory"),
                String::from("  Backspace\tGo to parent"),
                String::from("  Tab\tSwitch pane"),
                String::from("  Alt+Left\tNavigate back"),
                String::from("  Alt+Right\tNavigate forward"),
                String::from("  Up/Down  j/k\tMove selection"),
                String::from("  Space\tToggle mark on file"),
                String::from("  Shift+M\tClear marks"),
                String::from("  S\tCycle sort mode"),
                String::from("  Ctrl+P\tOpen command palette"),
                String::from("  Ctrl+O\tOpen settings panel"),
                String::new(),
                String::from("##File Operations"),
                String::from("  F5\tCopy"),
                String::from("  Shift+F6\tMove"),
                String::from("  F6\tRename"),
                String::from("  F8\tDelete"),
                String::from("  Ins\tNew file"),
                String::from("  Shift+F7\tNew directory"),
                String::new(),
                String::from("##Editor"),
                String::from("  F4\tOpen file in editor"),
                String::from("  Ctrl+S\tSave"),
                String::from("  Ctrl+D\tDiscard changes"),
                String::from("  Ctrl+F\tFind in editor"),
                String::from("  F3\tNext match"),
                String::from("  Shift+F3\tPrevious match"),
                String::from("  Esc\tClose search or editor"),
                String::new(),
                String::from("##Preview"),
                String::from("  F3\tToggle preview panel"),
                String::from("  Ctrl+W\tCycle focus: left → right → preview"),
                String::from("  Up/Down\tScroll preview (when focused)"),
                String::from("  PageUp/PageDown\tScroll preview fast"),
                String::from("  Esc\tReturn focus to file pane"),
                String::new(),
                String::from("##Menus & System"),
                String::from("  Alt+F\tFile menu"),
                String::from("  Alt+N\tNavigate menu"),
                String::from("  Alt+V\tView menu"),
                String::from("  Alt+H\tHelp menu"),
                String::from("##Appearance"),
                String::from("  icon_mode\tUnicode icons by default; set to ascii for fallback"),
                String::from("  Ctrl+O\tOpen settings panel"),
                String::from("  F1\tThis help"),
                String::from("  Ctrl+Q\tQuit"),
                String::from("  Esc\tClose menu / cancel"),
                String::new(),
                String::from("  Enter\tClose this window"),
            ],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollisionState {
    pub operation: FileOperation,
    pub refresh: Vec<RefreshTarget>,
    pub path: PathBuf,
}

impl CollisionState {
    pub fn lines(&self) -> Vec<String> {
        vec![
            format!("Destination exists: {}", self.path.display()),
            format!("Operation: {}", self.operation_label()),
            String::new(),
            String::from("O\tOverwrite"),
            String::from("R\tRename to new name"),
            String::from("S\tSkip this file"),
            String::from("Esc\tCancel"),
        ]
    }

    pub(crate) fn rename_prompt(self) -> PromptState {
        let suggested = suggest_non_conflicting_path(self.destination_path());
        let value = suggested.display().to_string();

        match self.operation {
            FileOperation::Copy { source, .. } => PromptState::with_value(
                PromptKind::Copy,
                "Copy",
                prompt_base_path(&suggested),
                Some(source),
                value,
            ),
            FileOperation::CreateDirectory { .. } => PromptState::with_value(
                PromptKind::NewDirectory,
                "New Directory",
                prompt_base_path(&suggested),
                None,
                value,
            ),
            FileOperation::CreateFile { .. } => PromptState::with_value(
                PromptKind::NewFile,
                "New File",
                prompt_base_path(&suggested),
                None,
                value,
            ),
            FileOperation::Delete { path } => PromptState::with_value(
                PromptKind::Delete,
                "Delete Permanently",
                prompt_base_path(&path),
                Some(path),
                String::new(),
            ),
            FileOperation::Trash { path } => PromptState::with_value(
                PromptKind::Trash,
                "Move to Trash",
                prompt_base_path(&path),
                Some(path),
                String::new(),
            ),
            FileOperation::Move { source, .. } => PromptState::with_value(
                PromptKind::Move,
                "Move",
                prompt_base_path(&suggested),
                Some(source),
                value,
            ),
            FileOperation::Rename { source, .. } => PromptState::with_value(
                PromptKind::Rename,
                "Rename",
                prompt_base_path(&suggested),
                Some(source),
                value,
            ),
        }
    }

    fn operation_label(&self) -> String {
        match &self.operation {
            FileOperation::Copy { source, .. } => format!("copy {}", source.display()),
            FileOperation::CreateDirectory { .. } => String::from("create directory"),
            FileOperation::CreateFile { .. } => String::from("create file"),
            FileOperation::Delete { path } => format!("delete {}", path.display()),
            FileOperation::Trash { path } => format!("trash {}", path.display()),
            FileOperation::Move { source, .. } => format!("move {}", source.display()),
            FileOperation::Rename { source, .. } => format!("rename {}", source.display()),
        }
    }

    fn destination_path(&self) -> &Path {
        match &self.operation {
            FileOperation::Copy { destination, .. } => destination,
            FileOperation::CreateDirectory { path } => path,
            FileOperation::CreateFile { path } => path,
            FileOperation::Delete { path } => path,
            FileOperation::Trash { path } => path,
            FileOperation::Move { destination, .. } => destination,
            FileOperation::Rename { destination, .. } => destination,
        }
    }
}
