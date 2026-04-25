use std::path::Path;
use std::path::PathBuf;

use crate::action::FileOperation;
use crate::action::RefreshTarget;
use crate::fs::suggest_non_conflicting_path;

use super::prompt::prompt_base_path;
use super::PromptKind;
use super::PromptState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DestructiveAction {
    Delete,
    PermanentDelete,
}

impl DestructiveAction {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Delete => "Delete",
            Self::PermanentDelete => "Delete Permanently",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DestructiveConfirmState {
    pub action: DestructiveAction,
    pub item_count: usize,
    pub item_sample: Vec<String>,
    pub operations: Vec<crate::action::FileOperation>,
    pub refresh_targets: Vec<crate::action::RefreshTarget>,
}

impl DestructiveConfirmState {
    pub fn new(
        action: DestructiveAction,
        items: &[std::path::PathBuf],
        refresh_targets: Vec<crate::action::RefreshTarget>,
    ) -> Self {
        let item_count = items.len();
        let item_sample = items
            .iter()
            .take(3)
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .collect();

        let operations = items
            .iter()
            .map(|item| match action {
                DestructiveAction::Delete => {
                    crate::action::FileOperation::Trash { path: item.clone() }
                }
                DestructiveAction::PermanentDelete => {
                    crate::action::FileOperation::Delete { path: item.clone() }
                }
            })
            .collect();

        Self {
            action,
            item_count,
            item_sample,
            operations,
            refresh_targets,
        }
    }

    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            String::from("⚠ WARNING: This action cannot be undone"),
            format!("Action: {}", self.action.label()),
            format!("Items: {}", self.item_count),
            String::new(),
        ];

        if !self.item_sample.is_empty() {
            lines.push(String::from("Sample:"));
            for item in &self.item_sample {
                lines.push(format!("  • {}", item));
            }
            if self.item_count > 3 {
                lines.push(format!("  ... and {} more", self.item_count - 3));
            }
            lines.push(String::new());
        }

        lines.extend(vec![
            String::from("Y/Enter  Confirm"),
            String::from("N/Esc    Cancel"),
        ]);

        lines
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DialogState {
    pub title: &'static str,
    pub lines: Vec<String>,
    pub scroll: usize,
}

impl DialogState {
    pub fn about(theme_name: String, config_path: String) -> Self {
        Self {
            title: "About Zeta",
            lines: vec![
                String::from("##LOGO ____  ________  ____             __               "),
                String::from("##LOGO |    \\|        \\|    \\           |  \\              "),
                String::from("##LOGO | $$$$ \\$$$$$$$$ \\$$$$  ______  _| $$_     ______  "),
                String::from("##LOGO | $$      /  $$   | $$ /      \\|   $$ \\   |      \\ "),
                String::from("##LOGO | $$     /  $$    | $$|  $$$$$$\\\\$$$$$$    \\$$$$$$\\"),
                String::from("##LOGO | $$    /  $$     | $$| $$    $$ | $$ __  /      $$"),
                String::from("##LOGO | $$_  /  $$___  _| $$| $$$$$$$$ | $$|  \\|  $$$$$$$"),
                String::from("##LOGO | $$ \\|  $$    \\|   $$ \\$$     \\  \\$$  $$ \\$$    $$"),
                String::from("##LOGO  \\$$$$ \\$$$$$$$$ \\$$$$  \\$$$$$$$   \\$$$$   \\$$$$$$$"),
                String::new(),
                String::from("## Overview"),
                String::from(
                    "Zeta is a keyboard-first dual-pane file manager and lightweight editor.",
                ),
                format!("  Version\t{} — Beta Release", env!("CARGO_PKG_VERSION")),
                String::from("  Author\ttzero86"),
                String::new(),
                String::from("## Appearance"),
                format!("  Theme\t{theme_name}"),
                String::from("  Defaults\tzeta (official dark)"),
                String::from("  Other\tfjord, sandbar, oxide, matrix, norton, dracula, neon, monochrome"),
                format!("  Config\t{config_path}"),
                String::from("  Icons\tNerdFont (recommended); Unicode and ASCII fallbacks available"),
                String::from("  Tip\tSet icon_mode = \"nerd_font\" in config.toml"),
                String::new(),
                String::from("## Features"),
                String::from("  Dual panes\tBrowse side-by-side or stacked"),
                String::from(
                    "  Workspaces	4 isolated desktops with independent pane/editor/preview/terminal state and top-bar indicators",
                ),
                String::from(
                    "  Sessions	Restores active workspace and pane locations on restart",
                ),
                String::from(
                    "  SSH/SFTP\tConnect to remote servers (command palette or Navigate menu)",
                ),
                String::from("  Archive browsing\tOpen .zip/.tar/.gz/.bz2/.xz like folders"),
                String::from("  Diff mode\tF9 highlights unique/changed entries"),
                String::from("  Mouse\tClick selects, double-click opens, wheel scrolls"),
                String::from("  Sync\tCtrl+D copies diff entries to the other pane"),
                String::from("  Parent entry\t'..' stays pinned at the top of directories"),
                String::new(),
                String::from("## Usage tips"),
                String::from(
                    "  [1] [2] [3] [4]	Top bar workspace pills; the highlighted box is active",
                ),
                String::from(
                    "  Alt+1..Alt+4\tSwitch workspaces (Shift+1..4 also works)",
                ),
                String::from("  ws:N/4	Status bar also shows the active workspace"),
                String::from("  Ctrl+O	Open settings, including theme chooser"),
                String::from("  ssh\tConnect to SSH server from the command palette"),
                String::from("  F1\tOpen help for shortcuts and workflows"),
                String::from("  Esc / Enter\tClose this window"),
            ],
            scroll: 0,
        }
    }

    pub fn help() -> Self {
        Self {
            title: " Help ",
            lines: vec![
                // LEFT column (Navigation + Files)
                String::from("##COLSTART"),
                String::from("## Navigation"),
                String::from("  ↑/↓ j/k\tMove selection"),
                String::from("  Enter\tOpen file or directory"),
                String::from("  Backspace\tGo to parent"),
                String::from("  Tab\tSwitch pane"),
                String::from("  PgUp/PgDn\tScroll page"),
                String::from("  Alt+1..4\tSwitch workspace"),
                String::from("  Click\tSelect with mouse"),
                String::new(),
                String::from("## Files"),
                String::from("  F5\tCopy"),
                String::from("  F6/Shift+F6\tRename / Move"),
                String::from("  F8\tDelete (trash)"),
                String::from("  Ins\tNew file"),
                String::from("  Shift+F7\tNew directory"),
                String::from("  Ctrl+D\tDiff-sync to other pane"),
                String::from("  /\tFilter pane entries"),
                String::new(),
                // RIGHT column (Editor + System)
                String::from("##COLBREAK"),
                String::from("## Editor"),
                String::from("  F4\tOpen in editor"),
                String::from("  Ctrl+S\tSave"),
                String::from("  Ctrl+D\tDiscard changes"),
                String::from("  Ctrl+F\tFind in file"),
                String::from("  F3/Shift+F3\tNext / prev match"),
                String::from("  Esc\tClose search or editor"),
                String::new(),
                String::from("## System"),
                String::from("  F1 / Ctrl+O\tHelp / Settings"),
                String::from("  F2\tToggle terminal"),
                String::from("  F3 / Ctrl+W\tToggle preview"),
                String::from("  F9\tDiff mode"),
                String::from("  F10 / q\tQuit"),
                String::from("  Shift+P\tCommand palette"),
                String::from("  Esc / Enter\tClose dialogs"),
                String::from("##COLEND"),
            ],
            scroll: 0,
        }
    }

    pub fn scroll_down(&mut self, step: usize) {
        self.scroll = self.scroll.saturating_add(step);
    }

    pub fn scroll_up(&mut self, step: usize) {
        self.scroll = self.scroll.saturating_sub(step);
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
            FileOperation::ExtractArchive { archive, .. } => PromptState::with_value(
                PromptKind::Copy,
                "Copy",
                prompt_base_path(&suggested),
                Some(archive.clone()),
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
            FileOperation::ExtractArchive { archive, .. } => {
                format!("extract {}", archive.display())
            }
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
            FileOperation::ExtractArchive { destination, .. } => destination,
        }
    }
}
