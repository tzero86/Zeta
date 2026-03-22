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
                String::from(" ____      _        "),
                String::from("|_  / ___ | |_ __ _ "),
                String::from(" / / / _ \\| __/ _` |"),
                String::from("/___\\___/ \\__\\__,_|"),
                String::new(),
                String::from("Keyboard-first dual-pane file manager"),
                String::from("Version: 0.1.0-dev"),
                format!("Theme: {theme_name}"),
                format!("Config: {config_path}"),
                String::new(),
                String::from("Esc or Enter closes this window"),
            ],
        }
    }

    pub fn help() -> Self {
        Self {
            title: "Help",
            lines: vec![
                String::from("F1 help  Alt+F file  Alt+N navigate  Alt+V view  Alt+H help"),
                String::from("Enter open dir  Backspace parent  Tab switch pane  Ctrl+Q quit"),
                String::from("F4 edit  Ins new file  Shift+F7 new dir  F6 rename  F8 delete"),
                String::from("Ctrl+S save  Ctrl+D discard  arrows/jk move  Esc closes menus"),
                String::new(),
                String::from("Menus are keyboard-first and prompts use Enter/Esc."),
                String::from("Esc or Enter closes this window"),
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
            format!("Pending: {}", self.operation_label()),
            String::new(),
            String::from("O overwrite  R rename  S skip  Esc cancel"),
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
                "Delete",
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
            FileOperation::Move { destination, .. } => destination,
            FileOperation::Rename { destination, .. } => destination,
        }
    }
}
