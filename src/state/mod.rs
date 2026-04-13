mod bookmarks;
mod dialog;
pub mod editor_state;
mod menu;
pub mod overlay;
pub mod pane_set;
pub mod preview_state;
mod prompt;
mod settings;
pub mod ssh;
pub mod terminal;
mod types;

pub use editor_state::EditorState;
pub use overlay::{ModalState, OverlayState};
pub use pane_set::PaneSetState;
pub use preview_state::PreviewState;

use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;

use crate::action::{Action, CollisionPolicy, Command, FileOperation, MenuId, RefreshTarget};
use crate::config::{AppConfig, IconMode, LoadedConfig, ResolvedTheme, ThemePalette, ThemePreset};
use crate::editor::EditorBuffer;
use crate::finder::FileFinderState;
use crate::fs;
use crate::fs::EntryKind;
use crate::jobs::{FileOperationStatus, JobResult};
use crate::pane::{InlineRenameState, PaneId, PaneState};
pub use ssh::*;

pub use bookmarks::BookmarksState;
pub use dialog::{CollisionState, DialogState};
pub use menu::{menu_tabs, MenuTab};
pub use prompt::{resolve_prompt_target, PromptKind, PromptState};
pub use settings::{SettingsEntry, SettingsField, SettingsState};
pub use types::{FocusLayer, MenuItem, ModalKind, PaneFocus, PaneLayout};

#[derive(Debug)]
struct PendingBatchOperation {
    pane: PaneId,
    queued_sources: VecDeque<PathBuf>,
    original_sources: BTreeSet<PathBuf>,
    failed_sources: BTreeSet<PathBuf>,
    settled_count: usize,
    total_count: usize,
}

#[derive(Debug)]
pub struct AppState {
    pub panes: PaneSetState,
    pub overlay: OverlayState,
    pub preview: PreviewState,
    pub editor: EditorState,
    pub terminal: crate::state::terminal::TerminalState,
    // Shared config/theme/status — not owned by any single sub-state
    config_path: String,
    config: AppConfig,
    icon_mode: IconMode,
    theme: ResolvedTheme,
    status_message: String,
    last_size: Option<(u16, u16)>,
    redraw_count: u64,
    startup_time_ms: u128,
    last_scan_time_ms: Option<u128>,
    file_operation_status: Option<FileOperationStatus>,
    should_quit: bool,
    /// Full-window editor mode hides the pane browser and lets the editor own
    /// the full content area.
    editor_fullscreen: bool,
    /// Cached git status for [Left=0, Right=1] pane working directories.
    /// Cached git status for [Left=0, Right=1] pane working directories.
    git: [Option<crate::git::RepoStatus>; 2],
    pending_reveal: Option<(PaneId, PathBuf)>,
    pub diff_mode: bool,
    pub diff_map: std::collections::HashMap<String, crate::diff::DiffStatus>,
    /// Tracks an in-flight batch prompt submission until all queued file-op results settle.
    pending_batch: Option<PendingBatchOperation>,
}

impl AppState {
    fn sync_editor_menu_mode(&mut self) {
        let enabled = self.editor_fullscreen && self.editor.is_open();
        self.overlay.set_editor_menu_mode(enabled);
    }

    pub fn bootstrap(loaded_config: LoadedConfig, started_at: Instant) -> Result<Self> {
        let cwd = fs::current_dir()?;
        let secondary = cwd
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| cwd.clone());
        let resolved_theme = loaded_config.config.resolve_theme();

        // Restore previous session if one exists alongside the config file.
        let session = crate::session::SessionState::load(
            &crate::session::SessionState::session_path(&loaded_config.path),
        );
        let left_cwd = session.left_cwd.filter(|p| p.is_dir()).unwrap_or(cwd);
        let right_cwd = session
            .right_cwd
            .filter(|p| p.is_dir())
            .unwrap_or(secondary);
        let layout = session.layout.unwrap_or_default();

        let mut left_pane = PaneState::empty("Left", left_cwd.clone());
        if let Some(sort) = session.left_sort {
            left_pane.sort_mode = sort;
        }
        left_pane.show_hidden = session.left_hidden;
        let mut right_pane = PaneState::empty("Right", right_cwd.clone());
        if let Some(sort) = session.right_sort {
            right_pane.sort_mode = sort;
        }
        right_pane.show_hidden = session.right_hidden;

        Ok(Self {
            panes: PaneSetState::new(left_pane, right_pane).with_layout(layout),
            overlay: OverlayState::default(),
            preview: PreviewState::new(
                loaded_config.config.preview_panel_open,
                loaded_config.config.preview_on_selection,
            ),
            editor: EditorState::default(),
            terminal: crate::state::terminal::TerminalState::default(),
            config_path: loaded_config.path.display().to_string(),
            config: loaded_config.config.clone(),
            icon_mode: loaded_config.config.icon_mode,
            theme: resolved_theme.clone(),
            status_message: resolved_theme.warning.unwrap_or_else(|| {
                format!(
                    "loading panes | config {} ({})",
                    loaded_config.path.display(),
                    loaded_config.source.label()
                )
            }),
            last_size: None,
            redraw_count: 0,
            startup_time_ms: started_at.elapsed().as_millis(),
            last_scan_time_ms: None,
            file_operation_status: None,
            should_quit: false,
            editor_fullscreen: false,
            git: [None, None],
            pending_reveal: None,
            diff_mode: false,
            diff_map: std::collections::HashMap::new(),
            pending_batch: None,
        })
    }

    pub fn config_path(&self) -> &str {
        &self.config_path
    }

    /// Apply a freshly loaded config without a full restart.
    /// Only updates fields that can change at runtime (theme, icon mode, editor prefs).
    pub fn apply_config_reload(&mut self, new_config: AppConfig) {
        self.icon_mode = new_config.icon_mode;
        self.theme = new_config.resolve_theme();
        self.config = new_config;
    }

    pub fn initial_commands(&self) -> Vec<Command> {
        vec![
            Command::ScanPane {
                pane: PaneId::Left,
                path: self.panes.left.cwd.clone(),
            },
            Command::ScanPane {
                pane: PaneId::Right,
                path: self.panes.right.cwd.clone(),
            },
        ]
    }

    pub fn apply(&mut self, action: Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        commands.extend(self.overlay.apply(&action)?);
        commands.extend(self.panes.apply(&action)?);
        commands.extend(self.editor.apply(&action)?);
        commands.extend(self.preview.apply(&action, &self.panes.focus)?);
        match action {
            Action::ToggleTerminal => {
                let was_open = self.terminal.is_open();
                commands.extend(
                    self.terminal
                        .apply(&action, self.panes.active_pane().cwd.clone())?,
                );
                if !was_open && self.terminal.is_open() {
                    self.status_message = String::from("terminal opened");
                } else if was_open && !self.terminal.is_open() {
                    self.status_message = String::from("terminal closed");
                }
            }
            _ => {
                commands.extend(
                    self.terminal
                        .apply(&action, self.panes.active_pane().cwd.clone())?,
                );
            }
        }
        commands.extend(self.apply_view(&action)?);
        Ok(commands)
    }

    fn apply_view(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        match action {
            Action::OpenAboutDialog => {
                self.overlay.open_about(DialogState::about(
                    self.theme.preset.clone(),
                    self.config_path.clone(),
                ));
                self.status_message = String::from("opened about");
            }
            Action::SetPaneLayout(layout) => {
                self.panes.pane_layout = *layout;
                self.config.pane_layout = *layout;
                self.status_message = match layout {
                    PaneLayout::SideBySide => String::from("layout set to side-by-side"),
                    PaneLayout::Stacked => String::from("layout set to stacked"),
                };
                let _ = self.config.save(Path::new(&self.config_path));
            }
            Action::SetTheme(preset) => {
                self.theme = ThemePalette::from_preset(*preset);
                self.config.theme.preset = preset.as_str().to_string();
                self.status_message = format!("theme set to {}", preset.as_str());
                let _ = self.config.save(Path::new(&self.config_path));
            }
            Action::TogglePreviewPanel => {
                self.config.preview_panel_open = self.preview.panel_open;
                if self.preview.panel_open {
                    if let Some(entry) = self.panes.active_pane().selected_entry() {
                        if entry.kind == EntryKind::File {
                            self.preview.request_debounced_preview(entry.path.clone());
                        }
                    }
                }
                let _ = self.config.save(Path::new(&self.config_path));
            }
            Action::ToggleEditorFullscreen => {
                if self.editor.is_open() {
                    self.editor_fullscreen = !self.editor_fullscreen;
                    self.sync_editor_menu_mode();
                    self.status_message = if self.editor_fullscreen {
                        String::from("editor fullscreen enabled")
                    } else {
                        String::from("editor fullscreen disabled")
                    };
                }
            }
            Action::ToggleMarkdownPreview => {
                if self.editor.is_markdown_file() {
                    self.status_message = if self.editor.markdown_preview_visible {
                        String::from("markdown preview shown")
                    } else {
                        String::from("markdown preview hidden")
                    };
                }
            }
            Action::ToggleHiddenFiles => {
                self.status_message = if self.panes.active_pane().show_hidden {
                    String::from("showing hidden files")
                } else {
                    String::from("hiding hidden files")
                };
            }
            Action::OpenPaneFilter => {
                let pane = self.panes.active_pane_mut();
                pane.filter_active = true;
                pane.filter_query.clear();
                pane.selection = 0;
                pane.scroll_offset = 0;
                self.status_message = String::from("type to filter pane entries");
            }
            Action::PaneFilterInput(ch) => {
                let pane = self.panes.active_pane_mut();
                pane.filter_query.push(*ch);
                pane.selection = 0;
                pane.scroll_offset = 0;
                pane.refresh_filter();
            }
            Action::PaneFilterBackspace => {
                let pane = self.panes.active_pane_mut();
                pane.filter_query.pop();
                pane.selection = 0;
                pane.scroll_offset = 0;
                pane.refresh_filter();
            }
            Action::ClosePaneFilter => {
                self.panes.active_pane_mut().clear_filter();
                self.status_message = String::from("pane filter closed");
            }
            Action::OpenFileFinder => {
                let pane = self.panes.focused_pane_id();
                let root = self.panes.active_pane().cwd.clone();
                self.overlay
                    .open_file_finder(FileFinderState::new(pane, root.clone()));
                commands.push(Command::FindFiles {
                    pane,
                    root: root.clone(),
                    max_depth: 5,
                });
                self.status_message = format!("searching under {}", root.display());
            }
            Action::FileFinderInput(ch) => {
                if let Some(finder) = self.overlay.file_finder_mut() {
                    finder.input(*ch);
                }
            }
            Action::FileFinderBackspace => {
                if let Some(finder) = self.overlay.file_finder_mut() {
                    finder.backspace();
                }
            }
            Action::FileFinderMoveDown => {
                if let Some(finder) = self.overlay.file_finder_mut() {
                    finder.move_down();
                }
            }
            Action::FileFinderMoveUp => {
                if let Some(finder) = self.overlay.file_finder_mut() {
                    finder.move_up();
                }
            }
            Action::CloseFileFinder => {
                self.overlay.close_all();
                self.status_message = String::from("file finder closed");
            }
            Action::FileFinderConfirm => {
                if let Some(finder) = self.overlay.file_finder() {
                    if let Some(path) = finder.selected().cloned() {
                        if let Some(parent) = path.parent() {
                            let pane = finder.pane;
                            self.pending_reveal = Some((pane, path.clone()));
                            commands.push(Command::ScanPane {
                                pane,
                                path: parent.to_path_buf(),
                            });
                            self.status_message = format!("jumping to {}", path.display());
                        }
                    }
                }
                self.overlay.close_all();
            }
            Action::CycleFocus => {
                self.editor.markdown_preview_focused = false;
                let preview_available = self.preview.panel_open && self.preview.view.is_some();
                let terminal_open = self.terminal.is_open();

                // If terminal is open and focused, focus Left pane
                if terminal_open && self.terminal.focused {
                    self.terminal.focused = false;
                    self.panes.focus = PaneFocus::Left;
                    self.status_message = String::from("focus returned to left pane");
                } else if terminal_open
                    && !self.terminal.focused
                    && self.panes.focus == PaneFocus::Right
                {
                    // If we're on Right pane and terminal is open, focus terminal next
                    self.terminal.focused = true;
                    self.status_message =
                        String::from("terminal focused (Ctrl+W to cycle, Esc to return)");
                } else {
                    self.panes.focus = match self.panes.focus {
                        PaneFocus::Left => {
                            if preview_available {
                                self.status_message = String::from(
                                    "preview panel focused (Ctrl+W to cycle, Esc to return)",
                                );
                                PaneFocus::Preview
                            } else {
                                PaneFocus::Right
                            }
                        }
                        PaneFocus::Right => PaneFocus::Left,
                        PaneFocus::Preview => {
                            self.status_message = String::from("focus returned to left pane");
                            PaneFocus::Left
                        }
                    };
                }
            }
            Action::FocusNextPane => {
                self.editor.markdown_preview_focused = false;
            }
            Action::FocusPreviewPanel => {
                if self.preview.panel_open {
                    self.editor.markdown_preview_focused = false;
                    self.panes.focus = if self.panes.focus == PaneFocus::Preview {
                        self.status_message = String::from("preview focus returned to file pane");
                        PaneFocus::Left
                    } else {
                        self.status_message = String::from("preview panel focused");
                        PaneFocus::Preview
                    };
                }
            }
            Action::FocusMarkdownPreview => {
                if self.editor.is_markdown_file() && self.editor.markdown_preview_visible {
                    if self.editor.markdown_preview_focused {
                        self.status_message = String::from(
                            "markdown preview focused  (Tab/Esc to return, Ctrl+M to hide)",
                        );
                    } else {
                        self.status_message =
                            String::from("editor focused  (Tab to focus markdown preview)");
                    }
                    if self.panes.focus == PaneFocus::Preview {
                        self.panes.focus = PaneFocus::Left;
                    }
                }
            }
            Action::Quit => {
                if self.editor.is_dirty() {
                    self.status_message =
                        String::from("unsaved changes: Ctrl+S save, Ctrl+D discard, Esc cancel");
                } else {
                    self.should_quit = true;
                }
            }
            Action::Resize { width, height } => {
                self.last_size = Some((*width, *height));
                self.status_message = format!("resized to {width}x{height}");
            }
            Action::OpenSelectedInEditor => {
                if let Some(entry) = self.panes.active_pane().selected_entry() {
                    if entry.kind == EntryKind::File {
                        commands.push(Command::OpenEditor {
                            path: entry.path.clone(),
                        });
                        self.status_message = format!("opening {}", entry.path.display());
                    } else if entry.kind == EntryKind::Archive {
                        commands.push(Command::OpenArchive {
                            path: entry.path.clone(),
                            inner: std::path::PathBuf::new(),
                        });
                        self.status_message = format!("opening archive {}", entry.path.display());
                    } else {
                        self.status_message = String::from("only files and archives can be opened");
                    }
                } else {
                    self.status_message = String::from("no file selected for editor or archive");
                }
            }
            Action::OpenInDefaultApp => {
                if let Some(path) = self.panes.active_pane().selected_path() {
                    match open::that(&path) {
                        Ok(()) => {
                            self.status_message =
                                format!("opened {} with system default", path.display());
                        }
                        Err(e) => {
                            self.status_message = format!("could not open file: {e}");
                        }
                    }
                }
            }
            Action::ToggleDiffMode => {
                self.diff_mode = !self.diff_mode;
                if self.diff_mode {
                    self.diff_map = crate::diff::compute_diff(
                        &self.panes.left.entries,
                        &self.panes.right.entries,
                    );
                    self.status_message =
                        format!("diff mode — {}", crate::diff::diff_summary(&self.diff_map));
                } else {
                    self.diff_map.clear();
                    self.status_message = String::from("diff mode off");
                }
            }
            Action::DiffSyncToOther => {
                if self.diff_mode {
                    let is_left_active = matches!(
                        self.panes.focus,
                        crate::state::types::PaneFocus::Left
                            | crate::state::types::PaneFocus::Preview
                    );
                    let src_cwd = self.panes.active_pane().cwd.clone();
                    let dst_cwd = self.panes.inactive_pane().cwd.clone();
                    let entries_to_sync: Vec<_> = self
                        .panes
                        .active_pane()
                        .entries
                        .iter()
                        .filter(|e| match self.diff_map.get(&e.name) {
                            Some(crate::diff::DiffStatus::LeftOnly) => is_left_active,
                            Some(crate::diff::DiffStatus::RightOnly) => !is_left_active,
                            Some(crate::diff::DiffStatus::Different) => true,
                            _ => false,
                        })
                        .map(|e| e.path.clone())
                        .collect();
                    let count = entries_to_sync.len();
                    let pane = self.panes.focused_pane_id();
                    let inactive_pane = if is_left_active {
                        crate::pane::PaneId::Right
                    } else {
                        crate::pane::PaneId::Left
                    };
                    for src_path in entries_to_sync {
                        let name = src_path
                            .file_name()
                            .map(|n| n.to_os_string())
                            .unwrap_or_default();
                        let dst_path = dst_cwd.join(name);
                        commands.push(Command::RunFileOperation {
                            operation: crate::action::FileOperation::Copy {
                                source: src_path,
                                destination: dst_path,
                            },
                            refresh: vec![
                                crate::action::RefreshTarget {
                                    pane,
                                    path: src_cwd.clone(),
                                },
                                crate::action::RefreshTarget {
                                    pane: inactive_pane,
                                    path: dst_cwd.clone(),
                                },
                            ],
                            collision: CollisionPolicy::Fail,
                        });
                    }
                    self.status_message = format!("queued {count} file(s) to sync");
                } else {
                    self.status_message = String::from("enable diff mode (F10) first");
                }
            }
            Action::OpenShell => {
                let path = self.panes.active_pane().cwd.clone();
                commands.push(Command::OpenShell { path: path.clone() });
                self.status_message = format!("opening shell in {}", path.display());
            }
            Action::OpenArchive { path } => {
                commands.push(Command::OpenArchive {
                    path: path.clone(),
                    inner: std::path::PathBuf::new(),
                });
                self.status_message = format!("opening archive {}", path.display());
            }
            Action::ExitArchive => {
                self.panes.active_pane_mut().mode = crate::pane::PaneMode::Real;
                commands.push(Command::ScanPane {
                    pane: self.panes.focused_pane_id(),
                    path: self.panes.active_pane().cwd.clone(),
                });
                self.status_message = String::from("exited archive");
            }
            Action::AddBookmark => {
                let cwd = self.panes.active_pane().cwd.clone();
                if self.config.bookmarks.contains(&cwd) {
                    self.status_message = String::from("bookmark already exists");
                } else {
                    self.config.bookmarks.push(cwd.clone());
                    let _ = self.config.save(Path::new(&self.config_path));
                    self.status_message = format!("bookmark added: {}", cwd.display());
                }
            }
            Action::OpenBookmarks => {
                self.overlay
                    .open_bookmarks(crate::state::BookmarksState::new());
                self.status_message = if self.config.bookmarks.is_empty() {
                    String::from("no bookmarks saved yet")
                } else {
                    String::from("bookmarks opened")
                };
            }
            Action::BookmarkSelect(index) => {
                if let Some(path) = self.config.bookmarks.get(*index).cloned() {
                    let pane = self.panes.focused_pane_id();
                    self.overlay.close_all();
                    commands.push(Command::ScanPane {
                        pane,
                        path: path.clone(),
                    });
                    self.status_message = format!("jumping to bookmark: {}", path.display());
                }
            }
            Action::DeleteBookmark(index) => {
                if *index < self.config.bookmarks.len() {
                    let removed = self.config.bookmarks.remove(*index);
                    let _ = self.config.save(Path::new(&self.config_path));
                    if let Some(bookmarks) = self.overlay.bookmarks_mut() {
                        bookmarks.selection = bookmarks
                            .selection
                            .min(self.config.bookmarks.len().saturating_sub(1));
                    }
                    self.status_message = format!("bookmark removed: {}", removed.display());
                }
            }
            Action::OpenCopyPrompt => {
                let marks: Vec<PathBuf> = {
                    let m = &self.panes.active_pane().marked;
                    if !m.is_empty() {
                        let mut v: Vec<PathBuf> = m.iter().cloned().collect();
                        v.sort();
                        v
                    } else {
                        Vec::new()
                    }
                };
                let target_dir = self.panes.inactive_pane().cwd.clone();
                if !marks.is_empty() {
                    let mut prompt = PromptState::with_value(
                        PromptKind::Copy,
                        "Copy Marked Items",
                        target_dir.clone(),
                        None,
                        target_dir.display().to_string(),
                    );
                    prompt.source_paths = marks;
                    self.overlay.open_prompt(prompt);
                    self.status_message =
                        String::from("enter destination directory for marked items");
                } else if let Some(entry) = self.panes.active_pane().selected_entry() {
                    let suggested = target_dir.join(&entry.name);
                    self.overlay.open_prompt(PromptState::with_value(
                        PromptKind::Copy,
                        "Copy",
                        target_dir,
                        Some(entry.path.clone()),
                        suggested.display().to_string(),
                    ));
                    self.status_message = String::from("enter copy destination");
                } else {
                    self.status_message = String::from("no item selected to copy");
                }
            }
            Action::OpenDeletePrompt => {
                let marks: Vec<PathBuf> = {
                    let m = &self.panes.active_pane().marked;
                    if !m.is_empty() {
                        let mut v: Vec<PathBuf> = m.iter().cloned().collect();
                        v.sort();
                        v
                    } else {
                        Vec::new()
                    }
                };
                let cwd = self.panes.active_pane().cwd.clone();
                if !marks.is_empty() {
                    let count = marks.len();
                    let preview = Self::summarize_paths(&marks);
                    let mut prompt = PromptState::with_value(
                        PromptKind::Trash,
                        "Trash Marked Items",
                        cwd,
                        None,
                        String::new(),
                    );
                    prompt.source_paths = marks;
                    self.overlay.open_prompt(prompt);
                    self.status_message =
                        format!("confirm trash for {count} marked items: {preview}");
                } else if let Some(entry) = self.panes.active_pane().selected_entry() {
                    let entry_name = entry.name.clone();
                    let entry_path = entry.path.clone();
                    self.overlay.open_prompt(PromptState::with_value(
                        PromptKind::Trash,
                        "Move to Trash",
                        cwd,
                        Some(entry_path),
                        String::new(),
                    ));
                    self.status_message = format!("confirm trash for {entry_name}");
                } else {
                    self.status_message = String::from("no item selected to trash");
                }
            }
            Action::OpenPermanentDeletePrompt => {
                let marks: Vec<PathBuf> = {
                    let m = &self.panes.active_pane().marked;
                    if !m.is_empty() {
                        let mut v: Vec<PathBuf> = m.iter().cloned().collect();
                        v.sort();
                        v
                    } else {
                        Vec::new()
                    }
                };
                let cwd = self.panes.active_pane().cwd.clone();
                if !marks.is_empty() {
                    let count = marks.len();
                    let preview = Self::summarize_paths(&marks);
                    let mut prompt = PromptState::with_value(
                        PromptKind::Delete,
                        "Delete Marked Items Permanently",
                        cwd,
                        None,
                        String::new(),
                    );
                    prompt.source_paths = marks;
                    self.overlay.open_prompt(prompt);
                    self.status_message =
                        format!("confirm permanent delete for {count} marked items: {preview}");
                } else if let Some(entry) = self.panes.active_pane().selected_entry() {
                    let entry_name = entry.name.clone();
                    let entry_path = entry.path.clone();
                    self.overlay.open_prompt(PromptState::with_value(
                        PromptKind::Delete,
                        "Delete Permanently",
                        cwd,
                        Some(entry_path),
                        String::new(),
                    ));
                    self.status_message = format!("confirm permanent delete for {entry_name}");
                } else {
                    self.status_message = String::from("no item selected to delete");
                }
            }
            Action::OpenMovePrompt => {
                let marks: Vec<PathBuf> = {
                    let m = &self.panes.active_pane().marked;
                    if !m.is_empty() {
                        let mut v: Vec<PathBuf> = m.iter().cloned().collect();
                        v.sort();
                        v
                    } else {
                        Vec::new()
                    }
                };
                let target_dir = self.panes.inactive_pane().cwd.clone();
                if !marks.is_empty() {
                    let mut prompt = PromptState::with_value(
                        PromptKind::Move,
                        "Move Marked Items",
                        target_dir.clone(),
                        None,
                        target_dir.display().to_string(),
                    );
                    prompt.source_paths = marks;
                    self.overlay.open_prompt(prompt);
                    self.status_message =
                        String::from("enter destination directory for marked items");
                } else if let Some(entry) = self.panes.active_pane().selected_entry() {
                    let suggested = target_dir.join(&entry.name);
                    self.overlay.open_prompt(PromptState::with_value(
                        PromptKind::Move,
                        "Move",
                        target_dir,
                        Some(entry.path.clone()),
                        suggested.display().to_string(),
                    ));
                    self.status_message = String::from("enter move destination");
                } else {
                    self.status_message = String::from("no item selected to move");
                }
            }
            Action::OpenNewDirectoryPrompt => {
                let cwd = self.panes.active_pane().cwd.clone();
                self.overlay.open_prompt(PromptState::new(
                    PromptKind::NewDirectory,
                    "New Directory",
                    cwd,
                ));
                self.status_message = String::from("enter directory name");
            }
            Action::OpenNewFilePrompt => {
                let cwd = self.panes.active_pane().cwd.clone();
                self.overlay
                    .open_prompt(PromptState::new(PromptKind::NewFile, "New File", cwd));
                self.status_message = String::from("enter file name");
            }
            Action::OpenRenamePrompt => {
                if let Some(entry) = self.panes.active_pane().selected_entry() {
                    let cwd = self.panes.active_pane().cwd.clone();
                    let path = entry.path.clone();
                    let name = entry.name.clone();
                    self.overlay.open_prompt(PromptState::with_value(
                        PromptKind::Rename,
                        "Rename",
                        cwd,
                        Some(path),
                        name,
                    ));
                    self.status_message = String::from("edit the new name");
                } else {
                    self.status_message = String::from("no item selected to rename");
                }
            }
            Action::PromptSubmit => {
                if let Some(ModalState::Prompt(prompt)) = &self.overlay.modal {
                    let prompt = prompt.clone();
                    if !prompt.kind.is_confirmation_only() && prompt.value.trim().is_empty() {
                        self.status_message = String::from("name cannot be empty");
                    } else {
                        // --- Batch mode: source_paths non-empty ---
                        if !prompt.source_paths.is_empty() {
                            let kind = prompt.kind;
                            let value = prompt.value.trim().to_string();
                            let dest_dir = {
                                let p = PathBuf::from(&value);
                                if p.is_absolute() {
                                    p
                                } else {
                                    prompt.base_path.join(p)
                                }
                            };
                            let count = prompt.source_paths.len();
                            let queued_sources: VecDeque<PathBuf> =
                                prompt.source_paths.iter().cloned().collect();
                            let batch_sources: BTreeSet<PathBuf> =
                                queued_sources.iter().cloned().collect();
                            for source in &prompt.source_paths {
                                let (operation, refresh_path) = match kind {
                                    PromptKind::Copy => {
                                        let target_path = source
                                            .file_name()
                                            .map(|n| dest_dir.join(n))
                                            .unwrap_or_else(|| dest_dir.clone());
                                        let copy_target =
                                            if Self::archive_member_source(source).is_some() {
                                                dest_dir.clone()
                                            } else {
                                                target_path.clone()
                                            };
                                        let operation =
                                            self.copy_operation_for_source(source, &copy_target);
                                        let refresh_path = self
                                            .refresh_target_path_for_transfer(source, &copy_target);
                                        (Some(operation), refresh_path)
                                    }
                                    PromptKind::Move => {
                                        let target_path = source
                                            .file_name()
                                            .map(|n| dest_dir.join(n))
                                            .unwrap_or_else(|| dest_dir.clone());
                                        let operation = FileOperation::Move {
                                            source: source.clone(),
                                            destination: target_path.clone(),
                                        };
                                        let refresh_path = self
                                            .refresh_target_path_for_transfer(source, &target_path);
                                        (Some(operation), refresh_path)
                                    }
                                    PromptKind::Trash => (
                                        Some(FileOperation::Trash {
                                            path: source.clone(),
                                        }),
                                        self.panes.active_pane().cwd.clone(),
                                    ),
                                    PromptKind::Delete => (
                                        Some(FileOperation::Delete {
                                            path: source.clone(),
                                        }),
                                        self.panes.active_pane().cwd.clone(),
                                    ),
                                    _ => (None, self.panes.active_pane().cwd.clone()),
                                };
                                if let Some(op) = operation {
                                    commands.push(Command::RunFileOperation {
                                        operation: op,
                                        refresh: self
                                            .refresh_targets_for_prompt(kind, &refresh_path),
                                        collision: CollisionPolicy::Fail,
                                    });
                                }
                            }
                            self.pending_batch = Some(PendingBatchOperation {
                                pane: self.panes.focused_pane_id(),
                                queued_sources,
                                original_sources: batch_sources,
                                failed_sources: BTreeSet::new(),
                                settled_count: 0,
                                total_count: count,
                            });
                            self.overlay.close_all();
                            self.status_message = match kind {
                                PromptKind::Copy => format!("copying {count} items"),
                                PromptKind::Move => format!("moving {count} items"),
                                PromptKind::Trash => format!("trashing {count} items"),
                                PromptKind::Delete => {
                                    format!("deleting {count} items permanently")
                                }
                                _ => String::from("processing items"),
                            };
                        } else {
                            let kind = prompt.kind;
                            let value = prompt.value.trim().to_string();
                            let target_path = resolve_prompt_target(&prompt, &value);
                            let operation = match kind {
                                PromptKind::Copy => prompt
                                    .source_path
                                    .as_ref()
                                    .map(|s| self.copy_operation_for_source(s, &target_path)),

                                PromptKind::Trash => prompt
                                    .source_path
                                    .as_ref()
                                    .map(|p| FileOperation::Trash { path: p.clone() }),
                                PromptKind::Delete => prompt
                                    .source_path
                                    .as_ref()
                                    .map(|p| FileOperation::Delete { path: p.clone() }),
                                PromptKind::Move => {
                                    prompt.source_path.as_ref().map(|s| FileOperation::Move {
                                        source: s.clone(),
                                        destination: target_path.clone(),
                                    })
                                }
                                PromptKind::NewDirectory => Some(FileOperation::CreateDirectory {
                                    path: target_path.clone(),
                                }),
                                PromptKind::NewFile => Some(FileOperation::CreateFile {
                                    path: target_path.clone(),
                                }),
                                PromptKind::Rename => prompt.source_path.as_ref().and_then(|s| {
                                    match Self::validate_rename_target(s, &value) {
                                        Err(msg) => {
                                            self.status_message = msg;
                                            None
                                        }
                                        Ok(None) => {
                                            self.status_message = String::from("rename unchanged");
                                            None
                                        }
                                        Ok(Some(destination)) => Some(FileOperation::Rename {
                                            source: s.clone(),
                                            destination,
                                        }),
                                    }
                                }),
                            };
                            let should_close_overlay = if let Some(operation) = operation {
                                let refresh_path = match &operation {
                                    FileOperation::Copy {
                                        source,
                                        destination,
                                    }
                                    | FileOperation::Move {
                                        source,
                                        destination,
                                    } => self.refresh_target_path_for_transfer(source, destination),
                                    FileOperation::ExtractArchive { destination, .. } => {
                                        destination.clone()
                                    }
                                    _ => target_path.clone(),
                                };
                                let refresh = self.refresh_targets_for_prompt(kind, &refresh_path);
                                commands.push(Command::RunFileOperation {
                                    operation,
                                    refresh,
                                    collision: CollisionPolicy::Fail,
                                });
                                self.status_message = match kind {
                                    PromptKind::Copy => String::from("copying item"),
                                    PromptKind::Trash => String::from("moving item to trash"),
                                    PromptKind::Delete => String::from("deleting item permanently"),
                                    PromptKind::Move => String::from("moving item"),
                                    PromptKind::NewDirectory => String::from("creating directory"),
                                    PromptKind::NewFile => String::from("creating file"),
                                    PromptKind::Rename => String::from("renaming item"),
                                };
                                true
                            } else if !(matches!(kind, PromptKind::Rename)
                                && prompt.source_path.is_some())
                            {
                                self.status_message = String::from("missing source for operation");
                                true
                            } else {
                                false
                            };
                            if should_close_overlay {
                                self.overlay.close_all();
                            }
                        } // end single-file path
                    }
                }
            }
            Action::CloseEditor => {
                if self.editor.is_dirty() {
                    self.status_message =
                        String::from("unsaved changes: Ctrl+S save, Ctrl+D discard, Esc cancel");
                } else if !self.editor.is_open() {
                    self.editor_fullscreen = false;
                    self.sync_editor_menu_mode();
                }
            }
            Action::DiscardEditorChanges => {
                self.editor_fullscreen = false;
                self.sync_editor_menu_mode();
                self.status_message = String::from("discarded editor changes");
            }
            Action::SettingsToggleCurrent => {
                if let Some(ModalState::Settings(s)) = &self.overlay.modal {
                    let selection = s.selection;
                    let entries = self.settings_entries();
                    if let Some(entry) = entries.get(selection).cloned() {
                        self.apply_settings_entry(entry);
                    }
                }
            }
            // Auto-preview after navigation
            Action::MoveSelectionDown | Action::MoveSelectionUp | Action::EnterSelection => {
                // Non-extend movement clears the range anchor.
                self.panes.active_pane_mut().reset_mark_anchor();
                if self.preview.should_auto_preview() {
                    if let Some(entry) = self.panes.active_pane().selected_entry() {
                        if entry.kind == EntryKind::File {
                            self.preview.request_debounced_preview(entry.path.clone());
                        } else {
                            self.preview.view = None;
                        }
                    } else {
                        self.preview.view = None;
                    }
                }
            }
            Action::ExtendSelectionDown => {
                self.panes.active_pane_mut().extend_selection_down();
            }
            Action::ExtendSelectionUp => {
                self.panes.active_pane_mut().extend_selection_up();
            }
            Action::ToggleDetailsView => {
                let pane = self.panes.active_pane_mut();
                pane.details_view = !pane.details_view;
                self.status_message = if self.panes.active_pane().details_view {
                    String::from("details view on")
                } else {
                    String::from("details view off")
                };
            }
            Action::BeginInlineRename => {
                // Pre-fill buffer with the current entry name (without trailing slash).
                if let Some(entry) = self.panes.active_pane().selected_entry() {
                    let original_path = entry.path.clone();
                    let name = entry.name.trim_end_matches('/').to_string();
                    self.panes.active_pane_mut().rename_state = Some(InlineRenameState {
                        buffer: name,
                        original_path,
                    });
                }
            }
            Action::CancelInlineRename => {
                self.panes.active_pane_mut().rename_state = None;
            }
            Action::ConfirmInlineRename => {
                let refresh = vec![RefreshTarget {
                    pane: self.panes.focused_pane_id(),
                    path: self.panes.active_pane().cwd.clone(),
                }];
                if let Some(state) = self.panes.active_pane().rename_state.clone() {
                    match Self::validate_rename_target(&state.original_path, &state.buffer) {
                        Err(msg) => {
                            self.status_message = msg;
                        }
                        Ok(None) => {
                            self.panes.active_pane_mut().rename_state = None;
                            self.status_message = String::from("rename unchanged");
                        }
                        Ok(Some(destination)) => {
                            self.panes.active_pane_mut().rename_state = None;
                            commands.push(Command::RunFileOperation {
                                operation: crate::action::FileOperation::Rename {
                                    source: state.original_path,
                                    destination,
                                },
                                refresh,
                                collision: CollisionPolicy::Fail,
                            });
                        }
                    }
                }
            }
            Action::InlineRenameType(ch) => {
                if let Some(ref mut rs) = self.panes.active_pane_mut().rename_state {
                    rs.buffer.push(*ch);
                }
            }
            Action::InlineRenameBackspace => {
                if let Some(ref mut rs) = self.panes.active_pane_mut().rename_state {
                    rs.buffer.pop();
                }
            }
            Action::CopyPathToClipboard => {
                if let Some(path) = self.panes.active_pane().selected_path() {
                    match arboard::Clipboard::new()
                        .and_then(|mut cb| cb.set_text(path.display().to_string()))
                    {
                        Ok(()) => {
                            self.status_message =
                                format!("copied to clipboard: {}", path.display());
                        }
                        Err(e) => {
                            self.status_message = format!("clipboard error: {e}");
                        }
                    }
                }
            }
            Action::OpenSshConnect => {
                self.overlay.open_ssh_connect(Default::default());
                self.status_message = String::from("enter SSH connection details");
            }
            Action::SshConnectConfirm => {
                if let Some(ModalState::SshConnect(state)) = &self.overlay.modal {
                    let address = state.address.clone();
                    let auth_method = state.auth_method;
                    let credential = state.credential.clone();
                    let pane = self.panes.focused_pane_id();
                    commands.push(Command::ConnectSSH {
                        address: address.clone(),
                        auth_method,
                        credential,
                        pane,
                    });
                    self.overlay.close_all();
                    self.status_message = format!("connecting to {}", address);
                }
            }
            Action::SshDialogInput(ch) => {
                if let Some(state) = self.overlay.ssh_connect_mut() {
                    match state.focused_field {
                        crate::state::ssh::SshDialogField::Address => state.address.push(*ch),
                        crate::state::ssh::SshDialogField::Credential => state.credential.push(*ch),
                    }
                    state.error = None;
                }
            }
            Action::SshDialogBackspace => {
                if let Some(state) = self.overlay.ssh_connect_mut() {
                    match state.focused_field {
                        crate::state::ssh::SshDialogField::Address => {
                            state.address.pop();
                        }
                        crate::state::ssh::SshDialogField::Credential => {
                            state.credential.pop();
                        }
                    }
                    state.error = None;
                }
            }
            Action::SshDialogToggleField => {
                if let Some(state) = self.overlay.ssh_connect_mut() {
                    state.focused_field = match state.focused_field {
                        crate::state::ssh::SshDialogField::Address => {
                            crate::state::ssh::SshDialogField::Credential
                        }
                        crate::state::ssh::SshDialogField::Credential => {
                            crate::state::ssh::SshDialogField::Address
                        }
                    };
                }
            }
            Action::SshDialogToggleAuthMethod => {
                if let Some(state) = self.overlay.ssh_connect_mut() {
                    state.auth_method = match state.auth_method {
                        crate::state::ssh::SshAuthMethod::Password => {
                            crate::state::ssh::SshAuthMethod::KeyFile
                        }
                        crate::state::ssh::SshAuthMethod::KeyFile => {
                            crate::state::ssh::SshAuthMethod::Agent
                        }
                        crate::state::ssh::SshAuthMethod::Agent => {
                            crate::state::ssh::SshAuthMethod::Password
                        }
                    };
                }
            }
            Action::CloseSshConnect => {
                self.overlay.close_all();
                self.status_message = String::from("SSH connection cancelled");
            }
            _ => {}
        }

        if matches!(
            action,
            Action::EditorMoveUp
                | Action::EditorMoveDown
                | Action::EditorMoveLeft
                | Action::EditorMoveRight
                | Action::EditorInsert(_)
                | Action::EditorPaste
                | Action::EditorBackspace
                | Action::EditorNewline
                | Action::EditorSearchNext
                | Action::EditorSearchPrev
                | Action::EditorReplaceNext
                | Action::EditorReplaceAll
                | Action::ToggleMarkdownPreview
        ) {
            let preview_height = self
                .last_size
                .map(|(_, h)| usize::from(h.saturating_sub(6) / 2).max(1))
                .unwrap_or(12);
            self.editor.sync_markdown_preview_to_cursor(preview_height);
        }

        Ok(commands)
    }

    pub fn apply_job_result(&mut self, result: JobResult) {
        match result {
            JobResult::DirectoryScanned {
                pane,
                path,
                entries,
                elapsed_ms,
            } => {
                self.panes.pane_mut(pane).cwd = path.clone();
                // Prepend a ".." entry so users can click to navigate to the parent.
                let mut all_entries = entries;
                if let Some(parent) = path.parent() {
                    let parent_path: PathBuf = parent.to_path_buf();
                    all_entries.insert(
                        0,
                        crate::fs::EntryInfo {
                            name: String::from(".."),
                            path: parent_path,
                            kind: EntryKind::Directory,
                            size_bytes: None,
                            modified: None,
                        },
                    );
                }
                self.panes.pane_mut(pane).set_entries(all_entries);
                self.panes.pane_mut(pane).refresh_filter();
                if let Some((pending_pane, pending_path)) = self.pending_reveal.clone() {
                    if pending_pane == pane && pending_path.parent() == Some(path.as_path()) {
                        self.panes.pane_mut(pane).select_path(&pending_path);
                        self.pending_reveal = None;
                    }
                }
                self.status_message = format!("refreshed {} in {elapsed_ms} ms", path.display());
                self.last_scan_time_ms = Some(elapsed_ms);
                // Recompute diff when diff mode is active.
                if self.diff_mode {
                    self.diff_map = crate::diff::compute_diff(
                        &self.panes.left.entries,
                        &self.panes.right.entries,
                    );
                }
            }
            JobResult::FileOperationCompleted {
                message,
                refreshed,
                elapsed_ms,
            } => {
                self.overlay.close_all();
                self.file_operation_status = None;
                for pane in refreshed {
                    self.panes.pane_mut(pane.pane).cwd = pane.path;
                    self.panes.pane_mut(pane.pane).set_entries(pane.entries);
                }
                if self.pending_batch.is_some() {
                    self.note_batch_settled(None, false);
                    if self.pending_batch.is_some() {
                        self.status_message = format!("{message} in {elapsed_ms} ms");
                    }
                } else {
                    self.status_message = format!("{message} in {elapsed_ms} ms");
                }
                self.last_scan_time_ms = Some(elapsed_ms);
            }
            JobResult::FileOperationCollision {
                operation,
                refresh,
                path,
                elapsed_ms,
            } => {
                self.file_operation_status = None;
                let source_path = Self::file_operation_source_path(&operation);
                self.note_batch_settled(Some(source_path), true);
                self.overlay.set_collision(CollisionState {
                    operation,
                    refresh,
                    path: path.clone(),
                });
                if self.pending_batch.is_some() {
                    self.status_message = format!(
                        "destination exists after {elapsed_ms} ms: {}",
                        path.display()
                    );
                }
                self.last_scan_time_ms = Some(elapsed_ms);
            }
            JobResult::FileOperationProgress { status } => {
                self.file_operation_status = Some(status);
            }
            JobResult::JobFailed {
                path,
                message,
                elapsed_ms,
                ..
            } => {
                self.file_operation_status = None;
                if self.pending_batch.is_some() {
                    self.note_batch_settled(Some(path.clone()), true);
                    if self.pending_batch.is_some() {
                        self.status_message = format!(
                            "job failed for {} after {elapsed_ms} ms: {message}",
                            path.display()
                        );
                    }
                } else {
                    self.status_message = format!(
                        "job failed for {} after {elapsed_ms} ms: {message}",
                        path.display()
                    );
                }
                self.last_scan_time_ms = Some(elapsed_ms);
            }
            JobResult::PreviewLoaded { path, view } => {
                self.preview.apply_job_loaded(path, view);
            }
            JobResult::EditorLoaded { path, contents } => {
                let is_expected = self
                    .editor
                    .buffer
                    .as_ref()
                    .and_then(|current| current.path.as_ref())
                    == Some(&path);
                if is_expected {
                    self.open_editor(EditorBuffer::from_text(path, contents));
                }
            }
            JobResult::EditorLoadFailed { path, message } => {
                let is_expected = self
                    .editor
                    .buffer
                    .as_ref()
                    .and_then(|current| current.path.as_ref())
                    == Some(&path);
                if is_expected {
                    self.editor.close();
                    self.set_error_status(format!("failed to open editor buffer: {message}"));
                }
            }
            JobResult::GitStatusLoaded { pane, status } => {
                self.git[pane as usize] = Some(status);
            }
            JobResult::GitStatusAbsent { pane } => {
                self.git[pane as usize] = None;
            }
            JobResult::FindResults {
                pane,
                root,
                entries,
            } => {
                if let Some(finder) = self.overlay.file_finder_mut() {
                    if finder.pane == pane && finder.root == root {
                        finder.set_results(entries);
                        self.status_message = format!(
                            "file finder loaded {} entries from {}",
                            finder.all_entries.len(),
                            root.display()
                        );
                    }
                }
            }
            JobResult::DirectoryChanged { path } => {
                self.status_message = format!("filesystem changed: {}", path.display());
            }
            JobResult::TerminalOutput(bytes) => {
                self.terminal.process_output(&bytes);
            }
            JobResult::TerminalDiagnostic(msg) => {
                self.status_message = format!("[Terminal] {}", msg);
            }
            JobResult::TerminalExited => {
                self.terminal.close();
                self.status_message = String::from("terminal session ended");
            }
            JobResult::DirSizeCalculated { pane, path, bytes } => {
                let p = self.panes.pane_mut(pane);
                if let Some(entry) = p.entries.iter_mut().find(|e| e.path == path) {
                    entry.size_bytes = Some(bytes);
                    p.cache_dirty.set(true);
                }
            }
            JobResult::ArchiveListed {
                pane,
                archive_path,
                inner_path,
                entries,
                elapsed_ms,
            } => {
                // Enter archive mode for the pane and populate entries
                let pane_mut = self.panes.pane_mut(pane);
                pane_mut.mode = crate::pane::PaneMode::Archive {
                    source: archive_path.clone(),
                    inner_path: inner_path.clone(),
                };
                pane_mut.set_entries(entries);
                pane_mut.refresh_filter();
                self.status_message = format!(
                    "opened archive {} in {elapsed_ms} ms",
                    archive_path.display()
                );
                self.last_scan_time_ms = Some(elapsed_ms);
            }
            JobResult::ConfigChanged => {
                // Consumed at the app layer; no state reducer work required here.
            }
        }
    }

    fn apply_settings_entry(&mut self, entry: SettingsEntry) {
        match entry.field {
            SettingsField::Theme(current) => {
                let next = match current {
                    ThemePreset::Zeta => ThemePreset::Neon,
                    ThemePreset::Neon => ThemePreset::Monochrome,
                    ThemePreset::Monochrome => ThemePreset::Fjord,
                    ThemePreset::Fjord => ThemePreset::Sandbar,
                    ThemePreset::Sandbar => ThemePreset::Oxide,
                    ThemePreset::Oxide => ThemePreset::Matrix,
                    ThemePreset::Matrix => ThemePreset::Norton,
                    ThemePreset::Norton => ThemePreset::Dracula,
                    ThemePreset::Dracula => ThemePreset::Zeta,
                };
                self.theme = ThemePalette::from_preset(next);
                self.config.theme.preset = next.as_str().to_string();
                self.status_message = format!("theme set to {}", next.as_str());
                let _ = self.config.save(Path::new(&self.config_path));
            }
            SettingsField::IconMode(current) => {
                let next = match current {
                    IconMode::Unicode => IconMode::Ascii,
                    IconMode::Ascii => IconMode::Unicode,
                    IconMode::Custom => IconMode::Unicode,
                };
                self.icon_mode = next;
                self.config.icon_mode = next;
                self.status_message = match next {
                    IconMode::Unicode => String::from("icons set to unicode"),
                    IconMode::Ascii => String::from("icons set to ASCII"),
                    IconMode::Custom => String::from("icons set to custom"),
                };
                let _ = self.config.save(Path::new(&self.config_path));
            }
            SettingsField::PaneLayout(current) => {
                let next = match current {
                    PaneLayout::SideBySide => PaneLayout::Stacked,
                    PaneLayout::Stacked => PaneLayout::SideBySide,
                };
                self.panes.pane_layout = next;
                self.config.pane_layout = next;
                self.status_message = match next {
                    PaneLayout::SideBySide => String::from("layout set to side-by-side"),
                    PaneLayout::Stacked => String::from("layout set to stacked"),
                };
                let _ = self.config.save(Path::new(&self.config_path));
            }
            SettingsField::PreviewPanel => {
                self.preview.panel_open = !self.preview.panel_open;
                self.config.preview_panel_open = self.preview.panel_open;
                self.status_message = if self.preview.panel_open {
                    String::from("preview panel enabled")
                } else {
                    String::from("preview panel disabled")
                };
                let _ = self.config.save(Path::new(&self.config_path));
            }
            SettingsField::PreviewOnSelection => {
                self.preview.preview_on_selection = !self.preview.preview_on_selection;
                self.config.preview_on_selection = self.preview.preview_on_selection;
                self.status_message = if self.preview.preview_on_selection {
                    String::from("preview on selection enabled")
                } else {
                    String::from("preview on selection disabled")
                };
                let _ = self.config.save(Path::new(&self.config_path));
            }
            SettingsField::KeymapPlaceholder => {
                self.status_message = String::from("keymap settings coming soon");
            }
        }
    }

    // =========================================================================
    // Public Accessors
    // =========================================================================

    // Pane accessors — delegate to PaneSetState
    pub fn left_pane(&self) -> &PaneState {
        &self.panes.left
    }
    pub fn right_pane(&self) -> &PaneState {
        &self.panes.right
    }
    pub fn focus(&self) -> PaneId {
        self.panes.focused_pane_id()
    }
    pub fn active_pane_title(&self) -> &str {
        self.panes
            .active_pane()
            .selected_entry()
            .map(|e| e.name.as_str())
            .unwrap_or("")
    }
    pub fn pane_layout(&self) -> PaneLayout {
        self.panes.pane_layout
    }
    pub fn is_editor_focused(&self) -> bool {
        self.editor.is_open()
            && self.panes.focus != PaneFocus::Preview
            && !self.editor.markdown_preview_focused
    }

    pub fn is_markdown_preview_focused(&self) -> bool {
        self.editor.is_open()
            && self.editor.markdown_preview_visible
            && self.editor.markdown_preview_focused
    }

    pub fn is_editor_fullscreen(&self) -> bool {
        self.editor_fullscreen
    }
    pub fn is_editor_loading(&self) -> bool {
        self.editor.loading
    }

    /// Returns the cached git status for the given pane, if available.
    pub fn git_status(&self, pane: crate::pane::PaneId) -> Option<&crate::git::RepoStatus> {
        self.git[pane as usize].as_ref()
    }

    /// Derive the current input focus layer from state.
    /// Priority (highest → lowest): Palette > FileFinder > Collision > Prompt > Dialog > Menu > Settings > Bookmarks > PaneFilter > MarkdownPreview > Editor > Preview > Pane.
    pub fn focus_layer(&self) -> FocusLayer {
        if self.is_palette_open() {
            return FocusLayer::Modal(ModalKind::Palette);
        }
        if self.file_finder().is_some() {
            return FocusLayer::Modal(ModalKind::FileFinder);
        }
        if self.is_collision_open() {
            return FocusLayer::Modal(ModalKind::Collision);
        }
        if self.is_prompt_open() {
            return FocusLayer::Modal(ModalKind::Prompt);
        }
        if self.is_dialog_open() {
            return FocusLayer::Modal(ModalKind::Dialog);
        }
        if self.is_menu_open() {
            return FocusLayer::Modal(ModalKind::Menu);
        }
        if self.is_settings_open() {
            return FocusLayer::Modal(ModalKind::Settings);
        }
        if self.ssh_connect().is_some() {
            return FocusLayer::Modal(ModalKind::SshConnect);
        }
        if self.bookmarks().is_some() {
            return FocusLayer::Modal(ModalKind::Bookmarks);
        }
        if self.terminal.is_open() && self.terminal.focused {
            return FocusLayer::Terminal;
        }
        if self.panes.active_pane().rename_state.is_some() {
            return FocusLayer::PaneInlineRename;
        }
        if self.panes.active_pane().filter_active {
            return FocusLayer::PaneFilter;
        }
        if self.is_markdown_preview_focused() {
            return FocusLayer::MarkdownPreview;
        }
        if self.is_editor_focused() {
            return FocusLayer::Editor;
        }
        if self.is_preview_focused() {
            return FocusLayer::Preview;
        }
        FocusLayer::Pane
    }

    // Overlay accessors — delegate to OverlayState
    pub fn active_menu(&self) -> Option<MenuId> {
        self.overlay.active_menu()
    }
    pub fn menu_items(&self) -> Vec<MenuItem> {
        self.overlay.menu_items()
    }
    pub fn menu_selection(&self) -> usize {
        self.overlay.menu_selection()
    }
    pub fn prompt(&self) -> Option<&PromptState> {
        self.overlay.prompt()
    }
    pub fn dialog(&self) -> Option<&DialogState> {
        self.overlay.dialog()
    }
    pub fn collision(&self) -> Option<&CollisionState> {
        self.overlay.collision()
    }
    pub fn palette(&self) -> Option<&crate::palette::PaletteState> {
        self.overlay.palette()
    }
    pub fn settings(&self) -> Option<&SettingsState> {
        self.overlay.settings()
    }
    pub fn bookmarks(&self) -> Option<&BookmarksState> {
        self.overlay.bookmarks()
    }
    pub fn file_finder(&self) -> Option<&FileFinderState> {
        self.overlay.file_finder()
    }
    pub fn ssh_connect(&self) -> Option<&crate::state::ssh::SshConnectionState> {
        self.overlay.ssh_connect()
    }
    pub fn is_menu_open(&self) -> bool {
        self.overlay.is_menu_open()
    }
    pub fn is_prompt_open(&self) -> bool {
        self.overlay.prompt().is_some()
    }
    pub fn is_dialog_open(&self) -> bool {
        self.overlay.dialog().is_some()
    }
    pub fn is_collision_open(&self) -> bool {
        self.overlay.collision().is_some()
    }
    pub fn is_palette_open(&self) -> bool {
        self.overlay.palette().is_some()
    }
    pub fn is_settings_open(&self) -> bool {
        self.overlay.settings().is_some()
    }

    // Preview accessors — delegate to PreviewState
    pub fn preview_view(&self) -> Option<&(PathBuf, crate::preview::ViewBuffer)> {
        self.preview.view.as_ref()
    }
    pub fn preview_command_due(&mut self) -> Option<Command> {
        self.preview.preview_command_due()
    }
    pub fn is_preview_panel_open(&self) -> bool {
        self.preview.panel_open
    }
    pub fn is_preview_focused(&self) -> bool {
        self.panes.focus == PaneFocus::Preview
    }

    // Editor accessor — delegate to EditorState
    pub fn editor(&self) -> Option<&EditorBuffer> {
        self.editor.buffer.as_ref()
    }
    pub fn editor_mut(&mut self) -> Option<&mut EditorBuffer> {
        self.editor.buffer.as_mut()
    }
    pub fn is_markdown_preview_visible(&self) -> bool {
        self.editor.is_markdown_file() && self.editor.markdown_preview_visible
    }
    pub fn markdown_preview_scroll(&self) -> usize {
        self.editor.markdown_preview_scroll
    }

    pub fn begin_open_editor(&mut self, path: PathBuf) {
        if self.panes.focus == PaneFocus::Preview {
            self.panes.focus = PaneFocus::Left;
        }
        self.editor_fullscreen = false;
        let display = path.display().to_string();
        self.editor.open_placeholder(path);
        self.sync_editor_menu_mode();
        self.status_message = format!("opening {display}...");
    }

    pub fn open_editor(&mut self, buffer: EditorBuffer) {
        let path = buffer
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| String::from("<unnamed>"));
        if self.panes.focus == PaneFocus::Preview {
            self.panes.focus = PaneFocus::Left;
        }
        self.editor_fullscreen = false;
        self.editor.open(buffer);
        self.sync_editor_menu_mode();
        self.editor.sync_markdown_preview_to_cursor(12);
        self.status_message = format!("opened editor for {path}");
    }

    pub fn mark_editor_saved(&mut self) {
        let message = self
            .editor
            .buffer
            .as_ref()
            .and_then(|e| e.path.as_ref())
            .map(|p| format!("saved editor buffer {}", p.display()))
            .unwrap_or_else(|| String::from("saved editor buffer"));
        self.status_message = message;
        if let Some(e) = self.editor.buffer.as_mut() {
            e.is_dirty = false;
        }
    }

    pub fn set_error_status(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
    }

    // Theme/config
    pub fn theme(&self) -> &ResolvedTheme {
        &self.theme
    }
    pub fn config(&self) -> &AppConfig {
        &self.config
    }
    pub fn icon_mode(&self) -> IconMode {
        self.icon_mode
    }
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn mark_drawn(&mut self) {
        self.redraw_count += 1;
    }

    pub fn status_line(&self) -> String {
        let mark_count = self.panes.active_pane().marked_count();
        let scan = self
            .last_scan_time_ms
            .map(|value| format!("scan:{value}ms"))
            .unwrap_or_else(|| String::from("scan:-"));
        let marks = if mark_count > 0 {
            format!(" | marks:{mark_count}")
        } else {
            String::new()
        };
        let progress = self
            .file_operation_status
            .as_ref()
            .map(|status| {
                let current = status
                    .current_path
                    .file_name()
                    .and_then(|value: &std::ffi::OsStr| value.to_str())
                    .unwrap_or(".");
                format!(
                    " | {}:{}/{} {}",
                    status.operation, status.completed, status.total, current
                )
            })
            .unwrap_or_default();
        let branch = {
            let active_pane_id = match self.panes.focus {
                PaneFocus::Left | PaneFocus::Preview => crate::pane::PaneId::Left,
                PaneFocus::Right => crate::pane::PaneId::Right,
            };
            self.git_status(active_pane_id)
                .map(|g| format!(" ⎇ {}", g.branch))
                .unwrap_or_default()
        };
        format!(
            "{} | {}{} | {} | up:{}ms {}{}{} | d:{}",
            self.config.theme.status_bar_label,
            self.status_message,
            branch,
            self.theme.preset,
            self.startup_time_ms,
            scan,
            marks,
            progress,
            self.redraw_count
        )
    }

    pub fn settings_entries(&self) -> Vec<SettingsEntry> {
        vec![
            SettingsEntry {
                label: "Theme",
                value: self.theme.preset.clone(),
                hint: "Enter",
                field: SettingsField::Theme(match self.theme.preset.as_str() {
                    "fjord" => ThemePreset::Fjord,
                    "sandbar" => ThemePreset::Sandbar,
                    "oxide" => ThemePreset::Oxide,
                    "matrix" => ThemePreset::Matrix,
                    "norton" => ThemePreset::Norton,
                    "neon" => ThemePreset::Neon,
                    "monochrome" => ThemePreset::Monochrome,
                    "dracula" => ThemePreset::Dracula,
                    _ => ThemePreset::Neon, // Fallback to Neon if an unknown theme string is somehow loaded
                }),
            },
            SettingsEntry {
                label: "Icon mode",
                value: match self.icon_mode {
                    IconMode::Unicode => String::from("unicode"),
                    IconMode::Ascii => String::from("ascii"),
                    IconMode::Custom => String::from("custom"),
                },
                hint: "Space",
                field: SettingsField::IconMode(self.icon_mode),
            },
            SettingsEntry {
                label: "Pane layout",
                value: match self.panes.pane_layout {
                    PaneLayout::SideBySide => String::from("side by side"),
                    PaneLayout::Stacked => String::from("stacked"),
                },
                hint: "Enter",
                field: SettingsField::PaneLayout(self.panes.pane_layout),
            },
            SettingsEntry {
                label: "Preview panel",
                value: if self.preview.panel_open {
                    String::from("enabled")
                } else {
                    String::from("disabled")
                },
                hint: "Space",
                field: SettingsField::PreviewPanel,
            },
            SettingsEntry {
                label: "Preview on selection",
                value: if self.preview.preview_on_selection {
                    String::from("enabled")
                } else {
                    String::from("disabled")
                },
                hint: "Space",
                field: SettingsField::PreviewOnSelection,
            },
            SettingsEntry {
                label: "Keymap",
                value: String::from("coming soon"),
                hint: "-",
                field: SettingsField::KeymapPlaceholder,
            },
        ]
    }

    // =========================================================================
    // Private Helpers
    // =========================================================================

    fn summarize_paths(paths: &[PathBuf]) -> String {
        let names: Vec<String> = paths
            .iter()
            .take(3)
            .map(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string()
            })
            .collect();
        let mut summary = names.join(", ");
        if paths.len() > 3 {
            summary.push_str(&format!(", +{} more", paths.len() - 3));
        }
        summary
    }

    fn validate_rename_target(source: &Path, raw_name: &str) -> Result<Option<PathBuf>, String> {
        let name = raw_name.trim();
        if name.is_empty() {
            return Err(String::from("name cannot be empty"));
        }
        if name.contains('/') || name.contains('\\') {
            return Err(String::from("rename target must be a name, not a path"));
        }
        let current = source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name == current {
            return Ok(None);
        }
        let parent = source.parent().map(Path::to_path_buf).unwrap_or_default();
        Ok(Some(parent.join(name)))
    }

    fn file_operation_source_path(operation: &FileOperation) -> PathBuf {
        match operation {
            FileOperation::Copy { source, .. } => source.clone(),
            FileOperation::Move { source, .. } => source.clone(),
            FileOperation::Rename { source, .. } => source.clone(),
            FileOperation::Trash { path } => path.clone(),
            FileOperation::Delete { path } => path.clone(),
            FileOperation::ExtractArchive {
                archive,
                inner_path,
                ..
            } => archive.join(inner_path),
            FileOperation::CreateDirectory { path } | FileOperation::CreateFile { path } => {
                path.clone()
            }
        }
    }

    fn note_batch_settled(&mut self, source_path: Option<PathBuf>, failed: bool) {
        let mut finalize: Option<(PaneId, BTreeSet<PathBuf>, BTreeSet<PathBuf>, usize)> = None;
        if let Some(batch) = self.pending_batch.as_mut() {
            let settled_source = match source_path {
                Some(path) => {
                    if batch.queued_sources.front() == Some(&path) {
                        batch.queued_sources.pop_front()
                    } else if let Some(index) = batch
                        .queued_sources
                        .iter()
                        .position(|candidate| candidate == &path)
                    {
                        batch.queued_sources.remove(index)
                    } else {
                        None
                    }
                }
                None => batch.queued_sources.pop_front(),
            };
            if let Some(source_path) = settled_source {
                batch.settled_count += 1;
                if failed {
                    batch.failed_sources.insert(source_path);
                }
                if batch.settled_count >= batch.total_count {
                    finalize = Some((
                        batch.pane,
                        batch.original_sources.clone(),
                        batch.failed_sources.clone(),
                        batch.total_count,
                    ));
                }
            }
        }
        if let Some((pane_id, originals, failed_sources, total_count)) = finalize {
            let succeeded = total_count.saturating_sub(failed_sources.len());
            let pane = self.panes.pane_mut(pane_id);
            for source in originals {
                if !failed_sources.contains(&source) {
                    pane.marked.remove(&source);
                }
            }
            self.status_message = if failed_sources.is_empty() {
                format!("completed {succeeded} items")
            } else if succeeded == 0 {
                format!("failed {total_count} items")
            } else {
                format!(
                    "partially completed: {succeeded} succeeded, {} failed",
                    failed_sources.len()
                )
            };
            self.pending_batch = None;
        }
    }

    fn copy_operation_for_source(&self, source: &Path, target_path: &Path) -> FileOperation {
        if let Some((archive_path, inner_path)) = Self::archive_member_source(source) {
            FileOperation::ExtractArchive {
                archive: archive_path,
                inner_path,
                destination: target_path.to_path_buf(),
            }
        } else {
            FileOperation::Copy {
                source: source.to_path_buf(),
                destination: target_path.to_path_buf(),
            }
        }
    }

    fn archive_member_source(source: &Path) -> Option<(PathBuf, PathBuf)> {
        let mut candidate = source.to_path_buf();
        loop {
            if candidate.exists() && candidate.is_file() {
                if let Some(name) = candidate.file_name().and_then(|n| n.to_str()) {
                    let lower = name.to_lowercase();
                    if lower.ends_with(".zip")
                        || lower.ends_with(".tar")
                        || lower.ends_with(".tar.gz")
                        || lower.ends_with(".tgz")
                        || lower.ends_with(".tar.bz2")
                        || lower.ends_with(".tbz2")
                        || lower.ends_with(".tar.xz")
                        || lower.ends_with(".txz")
                    {
                        let inner = source
                            .strip_prefix(&candidate)
                            .map(Path::to_path_buf)
                            .unwrap_or_default();
                        return Some((candidate, inner));
                    }
                }
            }
            if !candidate.pop() {
                break;
            }
        }
        None
    }
    fn refresh_target_path_for_transfer(&self, source: &Path, destination: &Path) -> PathBuf {
        if Self::archive_member_source(source).is_some()
            || source.is_dir()
            || destination == self.panes.inactive_pane().cwd
            || destination.is_dir()
        {
            destination.to_path_buf()
        } else {
            destination
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| self.panes.inactive_pane().cwd.clone())
        }
    }

    fn refresh_targets_for_prompt(
        &self,
        kind: PromptKind,
        target_path: &Path,
    ) -> Vec<RefreshTarget> {
        let mut refresh = vec![RefreshTarget {
            pane: self.panes.focused_pane_id(),
            path: self.panes.active_pane().cwd.clone(),
        }];

        let target_dir = match kind {
            PromptKind::Copy | PromptKind::Move => target_path.to_path_buf(),
            _ => self.panes.active_pane().cwd.clone(),
        };

        if target_dir != self.panes.active_pane().cwd
            && target_dir.starts_with(&self.panes.inactive_pane().cwd)
        {
            refresh.push(RefreshTarget {
                pane: match self.panes.focus {
                    PaneFocus::Left | PaneFocus::Preview => PaneId::Right,
                    PaneFocus::Right => PaneId::Left,
                },
                // Refresh the directory currently shown to the user; do not navigate
                // the inactive pane into a destination subpath as a side effect.
                path: self.panes.inactive_pane().cwd.clone(),
            });
        }

        refresh
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::action::{Action, CollisionPolicy, Command, FileOperation, MenuId, RefreshTarget};
    use crate::config::{ResolvedTheme, ThemePalette, ThemePreset};
    use crate::editor::EditorBuffer;
    use crate::fs::{EntryInfo, EntryKind};
    use crate::jobs::{FileOperationStatus, JobResult};
    use crate::pane::{InlineRenameState, PaneId, PaneState, SortMode};

    use super::{
        resolve_prompt_target, AppState, CollisionState, EditorState, FocusLayer, ModalKind,
        ModalState, OverlayState, PaneFocus, PaneLayout, PaneSetState, PreviewState, PromptKind,
        PromptState,
    };

    fn pane_with_file(path: &str) -> PaneState {
        PaneState {
            title: String::from("left"),
            cwd: PathBuf::from("."),
            entries: vec![EntryInfo {
                name: String::from("note.txt"),
                path: PathBuf::from(path),
                kind: EntryKind::File,
                size_bytes: Some(1024),
                modified: None,
            }],
            selection: 0,
            scroll_offset: 0,
            show_hidden: false,
            sort_mode: SortMode::Name,
            marked: std::collections::BTreeSet::new(),
            filter_query: String::new(),
            filter_active: false,
            history_back: Vec::new(),
            history_forward: Vec::new(),
            filtered_indices: std::cell::RefCell::new(Vec::new()),
            cache_dirty: std::cell::Cell::new(true),
            cache_entry_count: std::cell::Cell::new(0),
            cache_sort_mode: std::cell::Cell::new(SortMode::Name),
            cache_filter_active: std::cell::Cell::new(false),
            cache_filter_query: std::cell::RefCell::new(String::new()),
            mode: crate::pane::PaneMode::Real,
            mark_anchor: None,
            details_view: false,
            rename_state: None,
        }
    }

    fn temp_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("zeta-state-test-{unique}"))
    }

    #[test]
    fn git_status_defaults_to_none_for_both_panes() {
        let state = test_state();
        assert!(state.git_status(crate::pane::PaneId::Left).is_none());
        assert!(state.git_status(crate::pane::PaneId::Right).is_none());
    }

    #[test]
    fn git_status_loaded_result_stores_status_for_correct_pane() {
        use crate::git::RepoStatus;
        use std::collections::HashMap;
        let mut state = test_state();
        state.apply_job_result(JobResult::GitStatusLoaded {
            pane: crate::pane::PaneId::Left,
            status: RepoStatus::new(
                PathBuf::from("/tmp/repo"),
                String::from("main"),
                HashMap::new(),
            ),
        });
        assert!(state.git_status(crate::pane::PaneId::Left).is_some());
        assert_eq!(
            state.git_status(crate::pane::PaneId::Left).unwrap().branch,
            "main"
        );
        assert!(state.git_status(crate::pane::PaneId::Right).is_none());
    }

    #[test]
    fn git_status_absent_clears_status() {
        use crate::git::RepoStatus;
        use std::collections::HashMap;
        let mut state = test_state();
        state.apply_job_result(JobResult::GitStatusLoaded {
            pane: crate::pane::PaneId::Left,
            status: RepoStatus::new(
                PathBuf::from("/tmp/repo"),
                String::from("main"),
                HashMap::new(),
            ),
        });
        state.apply_job_result(JobResult::GitStatusAbsent {
            pane: crate::pane::PaneId::Left,
        });
        assert!(state.git_status(crate::pane::PaneId::Left).is_none());
    }

    #[test]
    fn status_line_includes_branch_name_when_git_loaded() {
        use crate::git::RepoStatus;
        use std::collections::HashMap;
        let mut state = test_state();
        state.apply_job_result(JobResult::GitStatusLoaded {
            pane: crate::pane::PaneId::Left,
            status: RepoStatus::new(
                PathBuf::from("/tmp/repo"),
                String::from("feature/cool"),
                HashMap::new(),
            ),
        });
        assert!(
            state.status_line().contains("feature/cool"),
            "status line should contain branch name, got: {}",
            state.status_line()
        );
    }

    #[test]
    fn focus_layer_returns_pane_when_nothing_open() {
        let state = test_state();
        assert!(matches!(state.focus_layer(), FocusLayer::Pane));
    }

    #[test]
    fn focus_layer_returns_palette_when_palette_open() {
        let mut state = test_state();
        state.apply(Action::OpenCommandPalette).unwrap();
        assert!(matches!(
            state.focus_layer(),
            FocusLayer::Modal(ModalKind::Palette)
        ));
    }

    #[test]
    fn focus_layer_returns_markdown_preview_when_split_preview_is_focused() {
        let mut state = test_state();
        let mut editor = EditorBuffer::default();
        editor.path = Some(PathBuf::from("note.md"));
        state.open_editor(editor);
        state.apply(Action::FocusMarkdownPreview).unwrap();
        assert!(matches!(state.focus_layer(), FocusLayer::MarkdownPreview));
    }

    #[test]
    fn open_pane_filter_switches_focus_layer() {
        let mut state = test_state();
        state.apply(Action::OpenPaneFilter).unwrap();
        assert!(matches!(state.focus_layer(), FocusLayer::PaneFilter));
    }

    #[test]
    fn open_file_finder_enqueues_background_search() {
        let mut state = test_state();
        let commands = state.apply(Action::OpenFileFinder).unwrap();
        assert!(matches!(commands.first(), Some(Command::FindFiles { .. })));
        assert!(matches!(
            state.focus_layer(),
            FocusLayer::Modal(ModalKind::FileFinder)
        ));
    }

    #[test]
    fn open_bookmarks_switches_focus_layer() {
        let mut state = test_state();
        state.apply(Action::OpenBookmarks).unwrap();
        assert!(matches!(
            state.focus_layer(),
            FocusLayer::Modal(ModalKind::Bookmarks)
        ));
    }

    fn test_state() -> AppState {
        let right = PaneState {
            title: String::from("right"),
            cwd: PathBuf::from("."),
            entries: Vec::new(),
            selection: 0,
            scroll_offset: 0,
            show_hidden: false,
            sort_mode: SortMode::Name,
            marked: std::collections::BTreeSet::new(),
            filter_query: String::new(),
            filter_active: false,
            history_back: Vec::new(),
            history_forward: Vec::new(),
            filtered_indices: std::cell::RefCell::new(Vec::new()),
            cache_dirty: std::cell::Cell::new(true),
            cache_entry_count: std::cell::Cell::new(0),
            cache_sort_mode: std::cell::Cell::new(SortMode::Name),
            cache_filter_active: std::cell::Cell::new(false),
            cache_filter_query: std::cell::RefCell::new(String::new()),
            mode: crate::pane::PaneMode::Real,
            mark_anchor: None,
            details_view: false,
            rename_state: None,
        };
        AppState {
            panes: PaneSetState::new(pane_with_file("./note.txt"), right),
            overlay: OverlayState::default(),
            preview: PreviewState::new(false, true),
            editor: EditorState::default(),
            config_path: String::from("/tmp/zeta/config.toml"),
            config: crate::config::AppConfig::default(),
            icon_mode: crate::config::IconMode::Unicode,
            theme: ResolvedTheme {
                palette: ThemePalette::resolve(&crate::config::ThemeConfig::default()).palette,
                preset: String::from("fjord"),
                warning: None,
            },
            status_message: String::from("ready"),
            last_size: None,
            redraw_count: 0,
            startup_time_ms: 0,
            last_scan_time_ms: None,
            file_operation_status: None,
            should_quit: false,
            editor_fullscreen: false,
            git: [None, None],
            pending_reveal: None,
            diff_mode: false,
            diff_map: std::collections::HashMap::new(),
            terminal: crate::state::terminal::TerminalState::default(),
            pending_batch: None,
        }
    }

    #[test]
    fn bootstrap_initial_commands_queue_both_pane_scans() {
        let state = test_state();

        assert_eq!(
            state.initial_commands(),
            vec![
                Command::ScanPane {
                    pane: PaneId::Left,
                    path: PathBuf::from("."),
                },
                Command::ScanPane {
                    pane: PaneId::Right,
                    path: PathBuf::from("."),
                },
            ]
        );
    }

    #[test]
    fn add_bookmark_persists_to_config() {
        let mut state = test_state();
        let root = temp_root();
        std::fs::create_dir_all(&root).expect("temp dir should exist");
        state.config_path = root.join("config.toml").display().to_string();
        state.panes.left.cwd = root.join("work");

        state
            .apply(Action::AddBookmark)
            .expect("bookmark add should succeed");

        assert_eq!(state.config.bookmarks, vec![root.join("work")]);
        assert!(root.join("config.toml").exists());
        std::fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn delete_bookmark_removes_from_config() {
        let mut state = test_state();
        let root = temp_root();
        std::fs::create_dir_all(&root).expect("temp dir should exist");
        state.config_path = root.join("config.toml").display().to_string();
        state.config.bookmarks = vec![root.join("one"), root.join("two")];
        state
            .overlay
            .open_bookmarks(crate::state::BookmarksState::new());

        state
            .apply(Action::DeleteBookmark(0))
            .expect("bookmark delete should succeed");

        assert_eq!(state.config.bookmarks, vec![root.join("two")]);
        std::fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn prompt_submit_dispatches_trash_operation() {
        let mut state = test_state();
        state
            .apply(Action::OpenDeletePrompt)
            .expect("trash prompt should open");

        let commands = state
            .apply(Action::PromptSubmit)
            .expect("submit should succeed");

        assert!(matches!(
            commands.first(),
            Some(Command::RunFileOperation {
                operation: FileOperation::Trash { .. },
                ..
            })
        ));
    }

    #[test]
    fn prompt_submit_dispatches_permanent_delete_operation() {
        let mut state = test_state();
        state
            .apply(Action::OpenPermanentDeletePrompt)
            .expect("delete prompt should open");

        let commands = state
            .apply(Action::PromptSubmit)
            .expect("submit should succeed");

        assert!(matches!(
            commands.first(),
            Some(Command::RunFileOperation {
                operation: FileOperation::Delete { .. },
                ..
            })
        ));
    }

    #[test]
    fn open_selected_file_enqueues_editor_command() {
        let mut state = test_state();

        let commands = state
            .apply(Action::OpenSelectedInEditor)
            .expect("action should succeed");

        assert_eq!(
            commands,
            vec![Command::OpenEditor {
                path: PathBuf::from("./note.txt")
            }]
        );
    }

    #[test]
    fn save_editor_enqueues_save_when_dirty() {
        let mut state = test_state();
        let mut editor = EditorBuffer::default();
        editor.path = Some(PathBuf::from("./note.txt"));
        editor.insert(0, "hello");
        state.editor.buffer = Some(editor);

        let commands = state
            .apply(Action::SaveEditor)
            .expect("action should succeed");

        assert_eq!(commands, vec![Command::SaveEditor]);
    }

    #[test]
    fn close_editor_is_guarded_when_dirty() {
        let mut state = test_state();
        let mut editor = EditorBuffer::default();
        editor.path = Some(PathBuf::from("./note.txt"));
        editor.insert_char('x');
        state.editor.buffer = Some(editor);

        let commands = state
            .apply(Action::CloseEditor)
            .expect("action should succeed");

        assert!(commands.is_empty());
        assert!(state.editor.is_open());
    }

    #[test]
    fn discard_editor_changes_closes_dirty_buffer() {
        let mut state = test_state();
        let mut editor = EditorBuffer::default();
        editor.path = Some(PathBuf::from("./note.txt"));
        editor.insert_char('x');
        state.editor.buffer = Some(editor);

        let commands = state
            .apply(Action::DiscardEditorChanges)
            .expect("action should succeed");

        assert!(commands.is_empty());
        assert!(!state.editor.is_open());
    }

    #[test]
    fn enter_selection_enqueues_directory_scan() {
        let mut state = test_state();
        state.panes.left.entries[0].kind = EntryKind::Directory;

        let commands = state
            .apply(Action::EnterSelection)
            .expect("action should succeed");

        assert_eq!(
            commands,
            vec![Command::ScanPane {
                pane: PaneId::Left,
                path: PathBuf::from("./note.txt"),
            }]
        );
    }

    #[test]
    fn navigate_to_parent_enqueues_scan() {
        let mut state = test_state();
        state.panes.left.cwd = PathBuf::from("/tmp/example");

        let commands = state
            .apply(Action::NavigateToParent)
            .expect("action should succeed");

        assert_eq!(
            commands,
            vec![Command::ScanPane {
                pane: PaneId::Left,
                path: PathBuf::from("/tmp"),
            }]
        );
    }

    #[test]
    fn menu_activation_dispatches_selected_action() {
        let mut state = test_state();
        state.overlay.modal = Some(ModalState::Menu {
            id: MenuId::Navigate,
            selection: 1,
        });

        let commands = state
            .apply(Action::MenuActivate)
            .expect("action should succeed");

        assert_eq!(
            commands,
            vec![Command::DispatchAction(Action::NavigateToParent)]
        );
    }

    #[test]
    fn batch_prompt_submit_does_not_clear_marks_at_dispatch() {
        let mut state = test_state();
        state.panes.right.cwd = PathBuf::from("/tmp/target");
        state.panes.left.marked.insert(PathBuf::from("./note.txt"));
        let mut prompt = PromptState::with_value(
            PromptKind::Copy,
            "Copy Marked Items",
            PathBuf::from("/tmp/target"),
            None,
            String::from("/tmp/target"),
        );
        prompt.source_paths = vec![PathBuf::from("./note.txt")];
        state.overlay.open_prompt(prompt);

        let commands = state
            .apply(Action::PromptSubmit)
            .expect("submit should work");

        assert_eq!(commands.len(), 1);
        assert_eq!(state.panes.left.marked_count(), 1);
        assert!(state.pending_batch.is_some());
    }

    #[test]
    fn batch_move_success_clears_marks_after_completed_result() {
        let mut state = test_state();
        state.panes.right.cwd = PathBuf::from("/tmp/target");
        state.panes.left.marked.insert(PathBuf::from("./note.txt"));
        let mut prompt = PromptState::with_value(
            PromptKind::Move,
            "Move Marked Items",
            PathBuf::from("/tmp/target"),
            None,
            String::from("/tmp/target"),
        );
        prompt.source_paths = vec![PathBuf::from("./note.txt")];
        state.overlay.open_prompt(prompt);
        state
            .apply(Action::PromptSubmit)
            .expect("submit should work");

        state.file_operation_status = Some(FileOperationStatus {
            operation: "move",
            completed: 1,
            total: 1,
            current_path: PathBuf::from("/tmp/target/note.txt"),
        });
        state.apply_job_result(JobResult::FileOperationCompleted {
            message: String::from("moved"),
            refreshed: Vec::new(),
            elapsed_ms: 1,
        });

        assert_eq!(state.panes.left.marked_count(), 0);
        assert!(state.pending_batch.is_none());
        assert_eq!(state.status_message, "completed 1 items");
    }

    #[test]
    fn batch_archive_extract_success_clears_marks_after_completed_result() {
        let root = temp_root();
        fs::create_dir_all(&root).expect("temp root should be created");
        let archive_path = root.join("bundle.zip");
        fs::write(&archive_path, b"placeholder").expect("archive placeholder should be created");

        let mut state = test_state();
        state.panes.left.cwd = root.clone();
        state.panes.right.cwd = root.join("target");
        let archive_member = archive_path.join("nested").join("note.txt");
        state.panes.left.marked.insert(archive_member.clone());
        let mut prompt = PromptState::with_value(
            PromptKind::Copy,
            "Copy Marked Items",
            state.panes.right.cwd.clone(),
            None,
            state.panes.right.cwd.display().to_string(),
        );
        prompt.source_paths = vec![archive_member];
        state.overlay.open_prompt(prompt);
        state
            .apply(Action::PromptSubmit)
            .expect("submit should work");

        state.file_operation_status = Some(FileOperationStatus {
            operation: "extract",
            completed: 1,
            total: 1,
            current_path: state.panes.right.cwd.join("note.txt"),
        });
        state.apply_job_result(JobResult::FileOperationCompleted {
            message: String::from("extracted"),
            refreshed: Vec::new(),
            elapsed_ms: 1,
        });

        assert_eq!(state.panes.left.marked_count(), 0);
        assert!(state.pending_batch.is_none());
        assert_eq!(state.status_message, "completed 1 items");

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn batch_full_success_clears_marks_and_sets_completed_status() {
        let mut state = test_state();
        state.panes.right.cwd = PathBuf::from("/tmp/target");
        state.panes.left.marked.insert(PathBuf::from("./note.txt"));
        let mut prompt = PromptState::with_value(
            PromptKind::Copy,
            "Copy Marked Items",
            PathBuf::from("/tmp/target"),
            None,
            String::from("/tmp/target"),
        );
        prompt.source_paths = vec![PathBuf::from("./note.txt")];
        state.overlay.open_prompt(prompt);
        state
            .apply(Action::PromptSubmit)
            .expect("submit should work");

        state.file_operation_status = Some(FileOperationStatus {
            operation: "copy",
            completed: 1,
            total: 1,
            current_path: PathBuf::from("./note.txt"),
        });
        state.apply_job_result(JobResult::FileOperationCompleted {
            message: String::from("copied"),
            refreshed: Vec::new(),
            elapsed_ms: 1,
        });

        assert_eq!(state.panes.left.marked_count(), 0);
        assert!(state.pending_batch.is_none());
        assert_eq!(state.status_message, "completed 1 items");
    }

    #[test]
    fn batch_partial_failure_keeps_failed_marks_only() {
        let mut state = test_state();
        state.panes.right.cwd = PathBuf::from("/tmp/target");
        state.panes.left.entries.push(EntryInfo {
            name: String::from("two.txt"),
            path: PathBuf::from("./two.txt"),
            kind: EntryKind::File,
            size_bytes: Some(64),
            modified: None,
        });
        state.panes.left.marked.insert(PathBuf::from("./note.txt"));
        state.panes.left.marked.insert(PathBuf::from("./two.txt"));
        let mut prompt = PromptState::with_value(
            PromptKind::Copy,
            "Copy Marked Items",
            PathBuf::from("/tmp/target"),
            None,
            String::from("/tmp/target"),
        );
        prompt.source_paths = vec![PathBuf::from("./note.txt"), PathBuf::from("./two.txt")];
        state.overlay.open_prompt(prompt);
        state
            .apply(Action::PromptSubmit)
            .expect("submit should work");

        state.file_operation_status = Some(FileOperationStatus {
            operation: "copy",
            completed: 1,
            total: 2,
            current_path: PathBuf::from("./note.txt"),
        });
        state.apply_job_result(JobResult::FileOperationCompleted {
            message: String::from("copied"),
            refreshed: Vec::new(),
            elapsed_ms: 1,
        });
        assert_eq!(state.panes.left.marked_count(), 2);
        assert!(state.pending_batch.is_some());

        state.apply_job_result(JobResult::JobFailed {
            pane: PaneId::Left,
            path: PathBuf::from("./two.txt"),
            message: String::from("permission denied"),
            elapsed_ms: 2,
        });

        assert!(!state
            .panes
            .left
            .marked
            .contains(&PathBuf::from("./note.txt")));
        assert!(state
            .panes
            .left
            .marked
            .contains(&PathBuf::from("./two.txt")));
        assert!(state.pending_batch.is_none());
        assert_eq!(
            state.status_message,
            "partially completed: 1 succeeded, 1 failed"
        );
    }

    #[test]
    fn batch_full_failure_keeps_marks_and_reports_failed_status() {
        let mut state = test_state();
        state.panes.right.cwd = PathBuf::from("/tmp/target");
        state.panes.left.marked.insert(PathBuf::from("./note.txt"));
        let mut prompt = PromptState::with_value(
            PromptKind::Delete,
            "Delete Marked Items Permanently",
            PathBuf::from("."),
            None,
            String::new(),
        );
        prompt.source_paths = vec![PathBuf::from("./note.txt")];
        state.overlay.open_prompt(prompt);
        state
            .apply(Action::PromptSubmit)
            .expect("submit should work");

        state.apply_job_result(JobResult::JobFailed {
            pane: PaneId::Left,
            path: PathBuf::from("./note.txt"),
            message: String::from("permission denied"),
            elapsed_ms: 2,
        });

        assert!(state
            .panes
            .left
            .marked
            .contains(&PathBuf::from("./note.txt")));
        assert!(state.pending_batch.is_none());
        assert_eq!(state.status_message, "failed 1 items");
    }

    #[test]
    fn toggle_hidden_files_flips_active_pane_flag() {
        let mut state = test_state();

        state
            .apply(Action::ToggleHiddenFiles)
            .expect("toggle hidden should succeed");

        assert!(state.panes.left.show_hidden);
    }

    #[test]
    fn open_new_file_prompt_sets_prompt_state() {
        let mut state = test_state();

        state
            .apply(Action::OpenNewFilePrompt)
            .expect("prompt should open");

        assert!(state.overlay.prompt().is_some());
    }

    #[test]
    fn open_copy_prompt_prefills_inactive_pane_destination() {
        let mut state = test_state();
        state.panes.right.cwd = PathBuf::from("/tmp/target");

        state
            .apply(Action::OpenCopyPrompt)
            .expect("copy prompt should open");

        let expected = PathBuf::from("/tmp/target")
            .join("note.txt")
            .display()
            .to_string();
        let prompt = state.overlay.prompt().expect("prompt should exist");
        assert_eq!(prompt.title, "Copy");
        assert_eq!(prompt.value, expected);
    }

    #[test]
    fn status_line_includes_file_operation_progress() {
        let mut state = test_state();
        state.file_operation_status = Some(FileOperationStatus {
            operation: "copy",
            completed: 2,
            total: 5,
            current_path: PathBuf::from("/tmp/target/note.txt"),
        });

        let status = state.status_line();

        assert!(status.contains("copy:2/5 note.txt"));
    }

    #[test]
    fn status_line_includes_mark_count() {
        let mut state = test_state();
        state.panes.left.marked.insert(PathBuf::from("./note.txt"));

        let status = state.status_line();

        assert!(status.contains("marks:1"));
    }

    #[test]
    fn toggle_mark_action_updates_active_pane_marks() {
        let mut state = test_state();

        state.apply(Action::ToggleMark).expect("toggle should work");

        assert_eq!(state.panes.left.marked_count(), 1);
    }

    #[test]
    fn clear_marks_action_clears_active_pane_marks() {
        let mut state = test_state();
        state.panes.left.marked.insert(PathBuf::from("./note.txt"));

        state.apply(Action::ClearMarks).expect("clear should work");

        assert_eq!(state.panes.left.marked_count(), 0);
    }

    #[test]
    fn file_operation_completion_clears_progress_state() {
        let mut state = test_state();
        state.file_operation_status = Some(FileOperationStatus {
            operation: "copy",
            completed: 2,
            total: 5,
            current_path: PathBuf::from("/tmp/target/note.txt"),
        });

        state.apply_job_result(JobResult::FileOperationCompleted {
            message: String::from("copied to /tmp/target/note.txt"),
            refreshed: Vec::new(),
            elapsed_ms: 42,
        });

        assert!(state.file_operation_status.is_none());
    }

    #[test]
    fn open_move_prompt_prefills_inactive_pane_destination() {
        let mut state = test_state();
        state.panes.right.cwd = PathBuf::from("/tmp/target");

        state
            .apply(Action::OpenMovePrompt)
            .expect("move prompt should open");

        let expected = PathBuf::from("/tmp/target")
            .join("note.txt")
            .display()
            .to_string();
        let prompt = state.overlay.prompt().expect("prompt should exist");
        assert_eq!(prompt.title, "Move");
        assert_eq!(prompt.value, expected);
    }

    #[test]
    fn set_pane_layout_updates_runtime_layout() {
        let mut state = test_state();

        state
            .apply(Action::SetPaneLayout(PaneLayout::Stacked))
            .expect("layout change should succeed");

        assert_eq!(state.pane_layout(), PaneLayout::Stacked);
    }

    #[test]
    fn prompt_submit_enqueues_copy_job_instead_of_mutating_directly() {
        let mut state = test_state();
        state.panes.right.cwd = PathBuf::from("/tmp/target");
        state.overlay.open_prompt(PromptState::with_value(
            PromptKind::Copy,
            "Copy",
            PathBuf::from("/tmp/target"),
            Some(PathBuf::from("./note.txt")),
            String::from("/tmp/target/note.txt"),
        ));

        let commands = state
            .apply(Action::PromptSubmit)
            .expect("submit should work");

        assert_eq!(
            commands,
            vec![Command::RunFileOperation {
                operation: FileOperation::Copy {
                    source: PathBuf::from("./note.txt"),
                    destination: PathBuf::from("/tmp/target/note.txt"),
                },
                refresh: vec![
                    RefreshTarget {
                        pane: PaneId::Left,
                        path: PathBuf::from("."),
                    },
                    RefreshTarget {
                        pane: PaneId::Right,
                        path: PathBuf::from("/tmp/target"),
                    },
                ],
                collision: CollisionPolicy::Fail,
            }]
        );
        assert!(state.overlay.prompt().is_none());
    }

    #[test]
    fn refresh_targets_for_prompt_batch_copy_uses_actual_directory_target() {
        let root = temp_root();
        let source_root = root.join("source");
        let source_dir = source_root.join("photos");
        let destination_dir = root.join("target");

        fs::create_dir_all(&source_dir).expect("source directory should be created");
        fs::create_dir_all(&destination_dir).expect("destination directory should be created");

        let mut state = test_state();
        state.panes.left.cwd = source_root.clone();
        state.panes.right.cwd = destination_dir.clone();

        let mut prompt = PromptState::with_value(
            PromptKind::Copy,
            "Copy Marked Items",
            destination_dir.clone(),
            None,
            destination_dir.display().to_string(),
        );
        prompt.source_paths = vec![source_dir.clone()];
        state.overlay.open_prompt(prompt);

        let commands = state
            .apply(Action::PromptSubmit)
            .expect("submit should work");

        assert_eq!(
            commands,
            vec![Command::RunFileOperation {
                operation: FileOperation::Copy {
                    source: source_dir.clone(),
                    destination: destination_dir.join("photos"),
                },
                refresh: vec![
                    RefreshTarget {
                        pane: PaneId::Left,
                        path: source_root.clone(),
                    },
                    RefreshTarget {
                        pane: PaneId::Right,
                        path: destination_dir.clone(),
                    },
                ],
                collision: CollisionPolicy::Fail,
            }]
        );

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn refresh_targets_for_prompt_batch_move_uses_actual_directory_target() {
        let root = temp_root();
        let source_root = root.join("source");
        let source_dir = source_root.join("photos");
        let destination_dir = root.join("target");

        fs::create_dir_all(&source_dir).expect("source directory should be created");
        fs::create_dir_all(&destination_dir).expect("destination directory should be created");

        let mut state = test_state();
        state.panes.left.cwd = source_root.clone();
        state.panes.right.cwd = destination_dir.clone();

        let mut prompt = PromptState::with_value(
            PromptKind::Move,
            "Move Marked Items",
            destination_dir.clone(),
            None,
            destination_dir.display().to_string(),
        );
        prompt.source_paths = vec![source_dir.clone()];
        state.overlay.open_prompt(prompt);

        let commands = state
            .apply(Action::PromptSubmit)
            .expect("submit should work");

        assert_eq!(
            commands,
            vec![Command::RunFileOperation {
                operation: FileOperation::Move {
                    source: source_dir.clone(),
                    destination: destination_dir.join("photos"),
                },
                refresh: vec![
                    RefreshTarget {
                        pane: PaneId::Left,
                        path: source_root.clone(),
                    },
                    RefreshTarget {
                        pane: PaneId::Right,
                        path: destination_dir.clone(),
                    },
                ],
                collision: CollisionPolicy::Fail,
            }]
        );

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn collision_job_result_opens_resolution_dialog() {
        let mut state = test_state();

        state.apply_job_result(JobResult::FileOperationCollision {
            operation: FileOperation::Copy {
                source: PathBuf::from("./note.txt"),
                destination: PathBuf::from("/tmp/target/note.txt"),
            },
            refresh: vec![RefreshTarget {
                pane: PaneId::Left,
                path: PathBuf::from("."),
            }],
            path: PathBuf::from("/tmp/target/note.txt"),
            elapsed_ms: 4,
        });

        assert!(state.is_collision_open());
        assert_eq!(
            state.collision().map(|collision| collision.path.clone()),
            Some(PathBuf::from("/tmp/target/note.txt"))
        );
    }

    #[test]
    fn collision_overwrite_requeues_job_with_overwrite_policy() {
        let mut state = test_state();
        state.overlay.modal = Some(ModalState::Collision(CollisionState {
            operation: FileOperation::Copy {
                source: PathBuf::from("./note.txt"),
                destination: PathBuf::from("/tmp/target/note.txt"),
            },
            refresh: vec![RefreshTarget {
                pane: PaneId::Right,
                path: PathBuf::from("/tmp/target"),
            }],
            path: PathBuf::from("/tmp/target/note.txt"),
        }));

        let commands = state
            .apply(Action::CollisionOverwrite)
            .expect("overwrite should enqueue work");

        assert_eq!(
            commands,
            vec![Command::RunFileOperation {
                operation: FileOperation::Copy {
                    source: PathBuf::from("./note.txt"),
                    destination: PathBuf::from("/tmp/target/note.txt"),
                },
                refresh: vec![RefreshTarget {
                    pane: PaneId::Right,
                    path: PathBuf::from("/tmp/target"),
                }],
                collision: CollisionPolicy::Overwrite,
            }]
        );
        assert!(!state.is_collision_open());
    }

    #[test]
    fn collision_rename_reopens_prompt_with_new_destination() {
        let root = temp_root();
        fs::create_dir_all(&root).expect("temp dir should exist");
        fs::write(root.join("note.txt"), "demo").expect("collision target should exist");
        let mut state = test_state();
        state.overlay.modal = Some(ModalState::Collision(CollisionState {
            operation: FileOperation::Rename {
                source: root.join("source.txt"),
                destination: root.join("note.txt"),
            },
            refresh: vec![],
            path: root.join("note.txt"),
        }));

        state
            .apply(Action::CollisionRename)
            .expect("rename should reopen prompt");

        let prompt = state.overlay.prompt().expect("prompt should reopen");
        assert_eq!(prompt.kind, PromptKind::Rename);
        assert_eq!(prompt.value, root.join("note-1.txt").display().to_string());
        assert!(!state.is_collision_open());

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn prompt_submit_rename_rejects_path_like_targets() {
        let mut state = test_state();
        state.overlay.open_prompt(PromptState::with_value(
            PromptKind::Rename,
            "Rename",
            PathBuf::from("/tmp/base"),
            Some(PathBuf::from("/tmp/base/old.txt")),
            String::from("nested/new.txt"),
        ));

        let commands = state
            .apply(Action::PromptSubmit)
            .expect("submit should work");

        assert!(commands.is_empty());
        assert_eq!(
            state.status_message,
            "rename target must be a name, not a path"
        );
        assert!(state.overlay.prompt().is_some());
    }

    #[test]
    fn prompt_submit_rename_same_name_is_no_op() {
        let mut state = test_state();
        state.overlay.open_prompt(PromptState::with_value(
            PromptKind::Rename,
            "Rename",
            PathBuf::from("/tmp/base"),
            Some(PathBuf::from("/tmp/base/old.txt")),
            String::from("old.txt"),
        ));

        let commands = state
            .apply(Action::PromptSubmit)
            .expect("submit should work");

        assert!(commands.is_empty());
        assert_eq!(state.status_message, "rename unchanged");
        assert!(state.overlay.prompt().is_some());
    }

    #[test]
    fn inline_rename_empty_name_keeps_editor_open_and_sets_status() {
        let mut state = test_state();
        state.panes.left.rename_state = Some(InlineRenameState {
            buffer: String::from("   "),
            original_path: PathBuf::from("./note.txt"),
        });

        let commands = state
            .apply(Action::ConfirmInlineRename)
            .expect("inline rename should validate");

        assert!(commands.is_empty());
        assert_eq!(state.status_message, "name cannot be empty");
        assert!(state.panes.left.rename_state.is_some());
    }

    #[test]
    fn inline_rename_same_name_is_no_op() {
        let mut state = test_state();
        state.panes.left.rename_state = Some(InlineRenameState {
            buffer: String::from("note.txt"),
            original_path: PathBuf::from("./note.txt"),
        });

        let commands = state
            .apply(Action::ConfirmInlineRename)
            .expect("inline rename should validate");

        assert!(commands.is_empty());
        assert_eq!(state.status_message, "rename unchanged");
        assert!(state.panes.left.rename_state.is_none());
    }

    #[test]
    fn resolve_prompt_target_joins_relative_values_to_base_path() {
        let prompt = PromptState::with_value(
            PromptKind::Rename,
            "Rename",
            PathBuf::from("/tmp/base"),
            Some(PathBuf::from("/tmp/base/old.txt")),
            String::new(),
        );

        assert_eq!(
            resolve_prompt_target(&prompt, "new.txt"),
            PathBuf::from("/tmp/base/new.txt")
        );
        assert_eq!(
            resolve_prompt_target(&prompt, "/tmp/elsewhere/new.txt"),
            PathBuf::from("/tmp/elsewhere/new.txt")
        );
    }

    #[test]
    fn copy_operation_for_source_selects_extract_archive_for_archive_members() {
        let root = temp_root();
        let archive_path = root.join("bundle.zip");
        let archive_member = PathBuf::from("nested").join("note.txt");
        let regular_source = root.join("note.txt");
        let destination_dir = root.join("target");

        fs::create_dir_all(&destination_dir).expect("destination directory should be created");
        fs::write(&archive_path, b"placeholder").expect("archive placeholder should be created");
        fs::write(&regular_source, b"note").expect("regular source should be created");

        let state = test_state();

        assert_eq!(
            state.copy_operation_for_source(&archive_path.join(&archive_member), &destination_dir),
            FileOperation::ExtractArchive {
                archive: archive_path.clone(),
                inner_path: archive_member.clone(),
                destination: destination_dir.clone(),
            }
        );
        assert_eq!(
            state.copy_operation_for_source(&regular_source, &destination_dir.join("note.txt")),
            FileOperation::Copy {
                source: regular_source.clone(),
                destination: destination_dir.join("note.txt"),
            }
        );

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn open_delete_prompt_summarizes_marked_items() {
        let mut state = test_state();
        state.panes.left.marked.insert(PathBuf::from("./beta.txt"));
        state.panes.left.marked.insert(PathBuf::from("./alpha.txt"));

        state
            .apply(Action::OpenDeletePrompt)
            .expect("trash prompt should open");

        assert!(state.overlay.prompt().is_some());
        assert!(state.status_message.contains("2 marked items"));
        assert!(
            state.status_message.contains("alpha.txt") || state.status_message.contains("beta.txt")
        );
    }

    #[test]
    fn open_permanent_delete_prompt_summarizes_marked_items() {
        let mut state = test_state();
        state.panes.left.marked.insert(PathBuf::from("./beta.txt"));
        state.panes.left.marked.insert(PathBuf::from("./alpha.txt"));

        state
            .apply(Action::OpenPermanentDeletePrompt)
            .expect("delete prompt should open");

        assert!(state.overlay.prompt().is_some());
        assert!(state.status_message.contains("2 marked items"));
        assert!(
            state.status_message.contains("alpha.txt") || state.status_message.contains("beta.txt")
        );
    }

    #[test]
    fn open_delete_prompt_sets_trash_confirmation_message() {
        let mut state = test_state();

        state
            .apply(Action::OpenDeletePrompt)
            .expect("trash prompt should open");

        assert!(state.overlay.prompt().is_some());
        assert_eq!(
            state.overlay.prompt().map(|prompt| prompt.kind),
            Some(PromptKind::Trash)
        );
        assert_eq!(
            state.overlay.prompt().map(|prompt| prompt.title),
            Some("Move to Trash")
        );
    }

    #[test]
    fn open_permanent_delete_prompt_sets_delete_confirmation_message() {
        let mut state = test_state();

        state
            .apply(Action::OpenPermanentDeletePrompt)
            .expect("delete prompt should open");

        assert!(state.overlay.prompt().is_some());
        assert_eq!(
            state.overlay.prompt().map(|prompt| prompt.kind),
            Some(PromptKind::Delete)
        );
        assert_eq!(
            state.overlay.prompt().map(|prompt| prompt.title),
            Some("Delete Permanently")
        );
    }

    #[test]
    fn open_help_dialog_sets_dialog_state() {
        let mut state = test_state();

        state
            .apply(Action::OpenHelpDialog)
            .expect("help dialog should open");

        assert_eq!(
            state.overlay.dialog().map(|dialog| dialog.title),
            Some(" Help ")
        );
    }

    #[test]
    fn open_about_dialog_uses_runtime_details() {
        let mut state = test_state();
        state.theme = ThemePalette::from_preset(ThemePreset::Sandbar);

        state
            .apply(Action::OpenAboutDialog)
            .expect("about dialog should open");

        let dialog = state.overlay.dialog().expect("dialog should exist");
        assert_eq!(dialog.title, "About Zeta");
        assert!(dialog.lines.iter().any(|line| line.contains("sandbar")));
    }

    #[test]
    fn focus_preview_panel_toggles_even_when_editor_exists() {
        let mut state = test_state();
        state.preview.panel_open = true;
        state.editor.buffer = Some(EditorBuffer::default());

        state
            .apply(Action::FocusPreviewPanel)
            .expect("focus preview should succeed");

        assert_eq!(state.panes.focus, PaneFocus::Preview);
    }

    #[test]
    fn set_theme_updates_runtime_palette() {
        let mut state = test_state();

        state
            .apply(Action::SetTheme(ThemePreset::Oxide))
            .expect("theme change should succeed");

        assert_eq!(state.theme.preset, "oxide");
    }
}
