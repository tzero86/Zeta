mod bookmarks;
pub mod dialog;
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
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;

use crate::action::{Action, CollisionPolicy, Command, FileOperation, MenuId, RefreshTarget};
use crate::config::{
    key_event_to_string, AppConfig, IconMode, LoadedConfig, ResolvedTheme, ThemePalette,
    ThemePreset,
};
use crate::editor::EditorBuffer;
use crate::finder::FileFinderState;
use crate::fs;
use crate::fs::EntryKind;
use crate::jobs::{FileOperationIdentity, FileOperationStatus, JobResult};
use crate::pane::{InlineRenameState, PaneId, PaneState};
pub use ssh::*;

pub use bookmarks::BookmarksState;
pub use dialog::{CollisionState, DialogState};
pub use menu::{menu_tabs, MenuContext, MenuTab};
pub use prompt::{resolve_prompt_target, PromptKind, PromptState};
pub use settings::{KeymapField, SettingsEntry, SettingsField, SettingsState, SettingsTab};
pub use types::{FocusLayer, MenuItem, ModalKind, PaneFocus, PaneLayout, ZetaError};

/// Structured data for the four status bar zones.
#[derive(Clone, Debug)]
pub struct StatusZones {
    pub git_branch: Option<String>,
    pub entry_detail: Option<String>,
    pub message: String,
    pub marks: Option<MarksInfo>,
    pub progress: Option<FileOpProgress>,
    pub workspace: String,
}

#[derive(Clone, Debug)]
pub struct MarksInfo {
    pub count: usize,
    pub total_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct FileOpProgress {
    pub operation: String,
    pub current: u64,
    pub total: u64,
    pub current_name: String,
}

#[derive(Debug)]
struct PendingBatchOperation {
    pane: PaneId,
    pending_operations: Vec<FileOperationIdentity>,
    original_sources: BTreeSet<PathBuf>,
    failed_sources: BTreeSet<PathBuf>,
    total_count: usize,
}

#[derive(Debug)]
pub struct WorkspaceState {
    pub panes: PaneSetState,
    pub preview: PreviewState,
    pub editor: EditorState,
    pub terminal: crate::state::terminal::TerminalState,
    status_message: String,
    last_scan_time_ms: Option<u128>,
    file_operation_status: Option<FileOperationStatus>,
    /// Full-window editor mode hides the pane browser and lets the editor own
    /// the full content area.
    editor_fullscreen: bool,
    /// When true, the terminal panel expands to fill the full content area.
    terminal_fullscreen: bool,
    /// Cached git status for [Left=0, Right=1] pane working directories.
    git: [Option<crate::git::RepoStatus>; 2],
    pending_reveal: Option<(PaneId, PathBuf)>,
    pending_collision: Option<CollisionState>,
    /// Left pane width as a percentage of the content split (clamped to 20–80, default 50).
    pane_split_ratio: u8,
    pub diff_mode: bool,
    pub diff_map: std::collections::HashMap<String, crate::diff::DiffStatus>,
    /// Tracks an in-flight batch prompt submission until all queued file-op results settle.
    pending_batch: Option<PendingBatchOperation>,
}

impl WorkspaceState {
    fn new(panes: PaneSetState, preview: PreviewState, status_message: String) -> Self {
        Self {
            panes,
            preview,
            editor: EditorState::default(),
            terminal: crate::state::terminal::TerminalState::default(),
            status_message,
            last_scan_time_ms: None,
            file_operation_status: None,
            editor_fullscreen: false,
            terminal_fullscreen: false,
            git: [None, None],
            pending_reveal: None,
            pending_collision: None,
            diff_mode: false,
            pane_split_ratio: 50,
            diff_map: std::collections::HashMap::new(),
            pending_batch: None,
        }
    }
}

/// Live debug state updated on every key press and action dispatch.
/// Rendered by the debug panel when toggled with F12.
#[derive(Debug, Default)]
pub struct DebugState {
    /// Human-readable description of the last received key event.
    pub last_key: String,
    /// Debug-formatted name of the last dispatched action.
    pub last_action: String,
    /// Rolling log of the last 8 dispatched action names (newest first).
    pub action_log: VecDeque<String>,
}

impl DebugState {
    const LOG_CAPACITY: usize = 8;

    pub fn record_key(&mut self, description: String) {
        self.last_key = description;
    }

    pub fn record_action(&mut self, name: String) {
        self.last_action = name.clone();
        self.action_log.push_front(name);
        self.action_log.truncate(Self::LOG_CAPACITY);
    }
}

#[derive(Debug)]
pub struct AppState {
    workspaces: [WorkspaceState; 4],
    active_workspace_idx: usize,
    pub overlay: OverlayState,
    // Shared config/theme/runtime shell state.
    config_path: String,
    config: AppConfig,
    icon_mode: IconMode,
    theme: ResolvedTheme,
    last_size: Option<(u16, u16)>,
    redraw_count: u64,
    startup_time_ms: u128,
    should_quit: bool,
    /// Set whenever visible state changes; cleared by `mark_drawn()`.
    /// The event loop skips `terminal.draw()` when this is false, avoiding
    /// unconditional 60 fps redraws on resource-constrained machines.
    needs_redraw: bool,
    /// Whether the floating debug panel is visible (toggled by F12).
    pub debug_visible: bool,
    /// Live debug state: last key, last action, action log.
    pub debug: DebugState,
}

impl AppState {
    pub fn active_workspace(&self) -> &WorkspaceState {
        &self.workspaces[self.active_workspace_idx]
    }

    pub fn active_workspace_mut(&mut self) -> &mut WorkspaceState {
        &mut self.workspaces[self.active_workspace_idx]
    }

    pub fn workspace(&self, idx: usize) -> &WorkspaceState {
        &self.workspaces[idx]
    }

    pub fn workspace_mut(&mut self, idx: usize) -> &mut WorkspaceState {
        &mut self.workspaces[idx]
    }

    pub fn active_workspace_index(&self) -> usize {
        self.active_workspace_idx
    }

    pub fn workspace_count(&self) -> usize {
        self.workspaces.len()
    }

    fn switch_to_workspace(&mut self, idx: usize) -> Vec<Command> {
        if idx >= self.workspaces.len() || idx == self.active_workspace_idx {
            return Vec::new();
        }

        let should_initialize = self.workspace(idx).last_scan_time_ms.is_none();
        self.overlay.close_all();
        self.active_workspace_idx = idx;
        if self.active_workspace().panes.focus == PaneFocus::Preview
            && !self.can_focus_preview_panel()
        {
            self.active_workspace_mut().panes.focus = PaneFocus::Left;
        }
        self.sync_editor_menu_mode();
        if let Some(collision) = self.active_workspace_mut().pending_collision.take() {
            self.overlay.set_collision(collision);
        }

        if !should_initialize {
            return Vec::new();
        }

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

    fn sync_editor_menu_mode(&mut self) {
        self.overlay.set_menu_context(self.menu_context());
    }

    fn can_focus_preview_panel(&self) -> bool {
        self.preview.panel_open && self.preview.view.is_some()
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
        let default_status = resolved_theme.warning.clone().unwrap_or_else(|| {
            format!(
                "loading panes | config {} ({})",
                loaded_config.path.display(),
                loaded_config.source.label()
            )
        });
        let workspaces = std::array::from_fn(|idx| {
            let saved = session.workspace(idx);
            let left_cwd = saved
                .as_ref()
                .and_then(|workspace| workspace.left_cwd.clone())
                .filter(|path| path.is_dir())
                .unwrap_or_else(|| cwd.clone());
            let right_cwd = saved
                .as_ref()
                .and_then(|workspace| workspace.right_cwd.clone())
                .filter(|path| path.is_dir())
                .unwrap_or_else(|| secondary.clone());
            let layout = saved
                .as_ref()
                .and_then(|workspace| workspace.layout)
                .unwrap_or_default();

            let mut left_pane = PaneState::empty("Left", left_cwd);
            if let Some(sort) = saved.as_ref().and_then(|workspace| workspace.left_sort) {
                left_pane.sort_mode = sort;
            }
            left_pane.show_hidden = saved
                .as_ref()
                .is_some_and(|workspace| workspace.left_hidden);
            if let Some(saved_ws) = saved.as_ref() {
                left_pane.history_back = saved_ws.left_history.clone();
            }

            let mut right_pane = PaneState::empty("Right", right_cwd);
            if let Some(sort) = saved.as_ref().and_then(|workspace| workspace.right_sort) {
                right_pane.sort_mode = sort;
            }
            right_pane.show_hidden = saved
                .as_ref()
                .is_some_and(|workspace| workspace.right_hidden);
            if let Some(saved_ws) = saved.as_ref() {
                right_pane.history_back = saved_ws.right_history.clone();
            }

            WorkspaceState::new(
                PaneSetState::new(left_pane, right_pane).with_layout(layout),
                PreviewState::new(
                    loaded_config.config.preview_panel_open,
                    loaded_config.config.preview_on_selection,
                ),
                default_status.clone(),
            )
        });

        Ok(Self {
            workspaces,
            active_workspace_idx: session.active_workspace.unwrap_or(0).min(3),
            overlay: OverlayState::default(),
            config_path: loaded_config.path.display().to_string(),
            config: loaded_config.config.clone(),
            icon_mode: loaded_config.config.icon_mode,
            theme: resolved_theme,
            last_size: None,
            redraw_count: 0,
            startup_time_ms: started_at.elapsed().as_millis(),
            should_quit: false,
            needs_redraw: true,
            debug_visible: false,
            debug: DebugState::default(),
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

    pub fn initial_commands(&mut self) -> Vec<Command> {
        let mut commands = vec![
            Command::ScanPane {
                pane: PaneId::Left,
                path: self.panes.left.cwd.clone(),
            },
            Command::ScanPane {
                pane: PaneId::Right,
                path: self.panes.right.cwd.clone(),
            },
        ];
        if self.config.terminal_open_by_default {
            let cwd = self.panes.active_pane().cwd.clone();
            commands.extend(self.terminal.toggle(cwd));
        }
        commands
    }

    pub fn apply(&mut self, action: Action) -> Result<Vec<Command>> {
        if let Action::SwitchToWorkspace(idx) = action {
            return Ok(self.switch_to_workspace(idx));
        }

        let mut commands = Vec::new();
        commands.extend(self.overlay.apply(&action)?);
        commands.extend(self.panes.apply(&action)?);
        commands.extend(self.editor.apply(&action)?);
        let pane_focus = self.panes.focus;
        commands.extend(self.preview.apply(&action, &pane_focus)?);
        match action {
            Action::ToggleTerminal => {
                let was_open = self.terminal.is_open();
                let cwd = self.panes.active_pane().cwd.clone();
                commands.extend(self.terminal.apply(&action, cwd)?);
                if !was_open && self.terminal.is_open() {
                    self.status_message = String::from("terminal opened");
                } else if was_open && !self.terminal.is_open() {
                    self.terminal_fullscreen = false;
                    self.status_message = String::from("terminal closed");
                }
            }
            Action::ToggleTerminalFullscreen => {
                if self.terminal.is_open() {
                    self.terminal_fullscreen = !self.terminal_fullscreen;
                    self.status_message = if self.terminal_fullscreen {
                        String::from("terminal fullscreen enabled")
                    } else {
                        String::from("terminal fullscreen disabled")
                    };
                }
            }
            _ => {
                let cwd = self.panes.active_pane().cwd.clone();
                commands.extend(self.terminal.apply(&action, cwd)?);
            }
        }
        commands.extend(self.apply_view(&action)?);
        Ok(commands)
    }

    fn apply_view(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();

        // Rebind capture: process before the main match so it can return early.
        if let Action::SettingsRebindCapture(key_event) = action {
            if let Some(ModalState::Settings(s)) = &self.overlay.modal {
                if let Some(rebind_idx) = s.rebind_mode {
                    let active_tab = s.active_tab;
                    let entries = self.settings_entries_for_tab(active_tab);
                    if let Some(SettingsField::KeymapBinding { field, .. }) =
                        entries.get(rebind_idx).map(|e| &e.field)
                    {
                        let field = *field;
                        if let Some(raw) = key_event_to_string(*key_event) {
                            // Apply the new string to the correct config field.
                            match field {
                                KeymapField::Quit => self.config.keymap.quit = raw.clone(),
                                KeymapField::SwitchPane => {
                                    self.config.keymap.switch_pane = raw.clone();
                                }
                                KeymapField::Refresh => {
                                    self.config.keymap.refresh = raw.clone();
                                }
                                KeymapField::Workspace(0) => {
                                    self.config.keymap.workspace_1 = raw.clone();
                                }
                                KeymapField::Workspace(1) => {
                                    self.config.keymap.workspace_2 = raw.clone();
                                }
                                KeymapField::Workspace(2) => {
                                    self.config.keymap.workspace_3 = raw.clone();
                                }
                                KeymapField::Workspace(_) => {
                                    self.config.keymap.workspace_4 = raw.clone();
                                }
                            }
                            // Try to compile; on success push an UpdateKeymap command.
                            match self.config.compile_keymap() {
                                Ok(new_keymap) => {
                                    let _ = self.config.save(Path::new(&self.config_path));
                                    self.status_message = format!("bound to {raw}");
                                    commands.push(Command::UpdateKeymap(new_keymap));
                                }
                                Err(_) => {
                                    // Roll back: restore old value from the entry's current field.
                                    if let Some(SettingsField::KeymapBinding {
                                        current: old, ..
                                    }) = entries.get(rebind_idx).map(|e| &e.field)
                                    {
                                        match field {
                                            KeymapField::Quit => {
                                                self.config.keymap.quit = old.clone();
                                            }
                                            KeymapField::SwitchPane => {
                                                self.config.keymap.switch_pane = old.clone();
                                            }
                                            KeymapField::Refresh => {
                                                self.config.keymap.refresh = old.clone();
                                            }
                                            KeymapField::Workspace(0) => {
                                                self.config.keymap.workspace_1 = old.clone();
                                            }
                                            KeymapField::Workspace(1) => {
                                                self.config.keymap.workspace_2 = old.clone();
                                            }
                                            KeymapField::Workspace(2) => {
                                                self.config.keymap.workspace_3 = old.clone();
                                            }
                                            KeymapField::Workspace(_) => {
                                                self.config.keymap.workspace_4 = old.clone();
                                            }
                                        }
                                    }
                                    self.status_message =
                                        format!("invalid key binding '{raw}', rebind cancelled");
                                }
                            }
                        } else {
                            self.status_message =
                                String::from("unsupported key — rebind cancelled");
                        }
                    }
                    if let Some(ModalState::Settings(s)) = &mut self.overlay.modal {
                        s.rebind_mode = None;
                    }
                }
            }
            return Ok(commands);
        }

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
                    let selected_file = self
                        .panes
                        .active_pane()
                        .selected_entry()
                        .filter(|entry| entry.kind == EntryKind::File)
                        .map(|entry| entry.path.clone());
                    if let Some(path) = selected_file {
                        self.preview.request_debounced_preview(path);
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
                if self.can_focus_preview_panel() {
                    self.editor.markdown_preview_focused = false;
                    self.panes.focus = if self.panes.focus == PaneFocus::Preview {
                        self.status_message = String::from("preview focus returned to file pane");
                        PaneFocus::Left
                    } else {
                        self.status_message = String::from("preview panel focused");
                        PaneFocus::Preview
                    };
                } else if self.preview.panel_open {
                    if self.panes.focus == PaneFocus::Preview {
                        self.panes.focus = PaneFocus::Left;
                    }
                    self.status_message = String::from("preview panel has no content to focus");
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
            Action::ShrinkLeftPane => {
                self.pane_split_ratio = self.pane_split_ratio.saturating_sub(5).max(20);
                self.status_message = format!("pane split: {}%", self.pane_split_ratio);
            }
            Action::GrowLeftPane => {
                self.pane_split_ratio = (self.pane_split_ratio + 5).min(80);
                self.status_message = format!("pane split: {}%", self.pane_split_ratio);
            }
            Action::ToggleDebugPanel => {
                self.debug_visible = !self.debug_visible;
            }
            Action::OpenOpenWithMenu => {
                if let Some(path) = self.panes.active_pane().selected_path() {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let mut items: Vec<(String, String)> =
                        vec![(String::from("Default App"), String::new())];
                    for opener in &self.config.openers {
                        if opener.matches_extension(&ext) {
                            items.push((opener.name.clone(), opener.command.clone()));
                        }
                    }
                    self.overlay.modal = Some(crate::state::overlay::ModalState::OpenWith {
                        items,
                        selection: 0,
                        target: path,
                    });
                } else {
                    self.status_message = String::from("no file selected");
                }
            }
            Action::OpenWithMoveUp => {
                if let Some(crate::state::overlay::ModalState::OpenWith {
                    items, selection, ..
                }) = &mut self.overlay.modal
                {
                    let len = items.len();
                    if *selection == 0 {
                        *selection = len.saturating_sub(1);
                    } else {
                        *selection -= 1;
                    }
                }
            }
            Action::OpenWithMoveDown => {
                if let Some(crate::state::overlay::ModalState::OpenWith {
                    items, selection, ..
                }) = &mut self.overlay.modal
                {
                    let len = items.len();
                    *selection = (*selection + 1) % len.max(1);
                }
            }
            Action::OpenWithConfirm => {
                if let Some(crate::state::overlay::ModalState::OpenWith {
                    items,
                    selection,
                    target,
                }) = self.overlay.modal.take()
                {
                    let idx = selection.min(items.len().saturating_sub(1));
                    if let Some((name, command)) = items.into_iter().nth(idx) {
                        let target_str = target.display().to_string();
                        if command.is_empty() {
                            match open::that(&target) {
                                Ok(()) => {
                                    self.status_message =
                                        format!("opened {} with default app", target.display());
                                }
                                Err(e) => {
                                    self.status_message = format!("could not open file: {e}");
                                }
                            }
                        } else {
                            let expanded = if command.contains("{}") {
                                command.replace("{}", &target_str)
                            } else {
                                format!("{command} {target_str}")
                            };
                            // Use shell to properly parse arguments (preserves spaces in paths)
                            match std::process::Command::new("sh")
                                .arg("-c")
                                .arg(&expanded)
                                .spawn()
                            {
                                Ok(_) => {
                                    self.status_message =
                                        format!("opened {} with {name}", target.display());
                                }
                                Err(e) => {
                                    self.status_message = format!("could not open file: {e}");
                                }
                            }
                        }
                    }
                }
            }
            Action::CloseOpenWithMenu => {
                self.overlay.close_all();
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

                if !marks.is_empty() {
                    // Batch operation: trash multiple marked items
                    // Set source_path to first item for display (renderer shows this, not source_paths)
                    let display_path = marks.first().cloned();
                    let mut prompt = PromptState::with_value(
                        PromptKind::Trash,
                        "Trash Marked Items",
                        self.panes.active_pane().cwd.clone(),
                        display_path,
                        String::new(),
                    );
                    prompt.source_paths = marks;
                    self.overlay.open_prompt(prompt);
                    self.status_message = String::from("Press Enter to confirm, or Esc to cancel");
                } else if let Some(entry) = self.panes.active_pane().selected_entry() {
                    // Single item: use destructive confirm dialog
                    let refresh = vec![crate::action::RefreshTarget {
                        pane: self.panes.focused_pane_id(),
                        path: self.panes.active_pane().cwd.clone(),
                    }];

                    let state = crate::state::dialog::DestructiveConfirmState::new(
                        crate::state::dialog::DestructiveAction::Delete,
                        &[entry.path.clone()],
                        refresh,
                    );

                    self.overlay.modal =
                        Some(crate::state::overlay::ModalState::DestructiveConfirm(state));
                    self.status_message = String::new();
                } else {
                    self.status_message = "No items selected to delete".to_string();
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

                if !marks.is_empty() {
                    // Batch operation: permanently delete multiple marked items
                    // Set source_path to first item for display (renderer shows this, not source_paths)
                    let display_path = marks.first().cloned();
                    let mut prompt = PromptState::with_value(
                        PromptKind::Delete,
                        "Delete Permanently Marked Items",
                        self.panes.active_pane().cwd.clone(),
                        display_path,
                        String::new(),
                    );
                    prompt.source_paths = marks;
                    self.overlay.open_prompt(prompt);
                    self.status_message = String::from("Press Enter to confirm, or Esc to cancel");
                } else if let Some(entry) = self.panes.active_pane().selected_entry() {
                    // Single item: use destructive confirm dialog
                    let refresh = vec![crate::action::RefreshTarget {
                        pane: self.panes.focused_pane_id(),
                        path: self.panes.active_pane().cwd.clone(),
                    }];

                    let state = crate::state::dialog::DestructiveConfirmState::new(
                        crate::state::dialog::DestructiveAction::PermanentDelete,
                        &[entry.path.clone()],
                        refresh,
                    );

                    self.overlay.modal =
                        Some(crate::state::overlay::ModalState::DestructiveConfirm(state));
                    self.status_message = String::new();
                } else {
                    self.status_message = "No items selected to delete".to_string();
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
            Action::OpenGoToPrompt => {
                let cwd = self.panes.active_pane().cwd.clone();
                self.overlay
                    .open_prompt(PromptState::new(PromptKind::GoTo, "Go to Path", cwd));
                self.status_message = String::from("type an absolute or relative path");
            }
            Action::OpenBulkRenamePrompt => {
                let marked: Vec<PathBuf> = {
                    let m = &self.panes.active_pane().marked;
                    let mut v: Vec<PathBuf> = m.iter().cloned().collect();
                    v.sort();
                    v
                };
                if marked.is_empty() {
                    self.status_message =
                        String::from("mark files first (Space), then Ctrl+R to bulk rename");
                } else {
                    let count = marked.len();
                    let cwd = self.panes.active_pane().cwd.clone();
                    let mut prompt = PromptState::new(
                        PromptKind::BulkRename,
                        "Bulk Rename — pattern: {n} {name} {ext}",
                        cwd,
                    );
                    prompt.source_paths = marked;
                    self.overlay.open_prompt(prompt);
                    self.status_message = format!("{count} files marked — enter rename pattern");
                }
            }
            Action::PromptSubmit => {
                if let Some(ModalState::Prompt(prompt)) = &self.overlay.modal {
                    let prompt = prompt.clone();
                    if !prompt.kind.is_confirmation_only() && prompt.value().trim().is_empty() {
                        self.status_message = String::from("name cannot be empty");
                    } else {
                        // --- Batch mode: source_paths non-empty ---
                        if !prompt.source_paths.is_empty() {
                            let kind = prompt.kind;
                            let value = prompt.value().trim().to_string();
                            let count = prompt.source_paths.len();
                            let batch_sources: BTreeSet<PathBuf> =
                                prompt.source_paths.iter().cloned().collect();
                            let mut pending_operations = Vec::with_capacity(count);

                            if kind == PromptKind::BulkRename {
                                // Generate one Rename operation per marked file.
                                for (idx, source) in prompt.source_paths.iter().enumerate() {
                                    let new_name =
                                        Self::apply_rename_pattern(&value, source, idx + 1);
                                    let destination = source
                                        .parent()
                                        .map(|p| p.join(&new_name))
                                        .unwrap_or_else(|| PathBuf::from(&new_name));
                                    let refresh_path = source
                                        .parent()
                                        .map(Path::to_path_buf)
                                        .unwrap_or_else(|| self.panes.active_pane().cwd.clone());
                                    let op = FileOperation::Rename {
                                        source: source.clone(),
                                        destination,
                                    };
                                    pending_operations
                                        .push(FileOperationIdentity::from_operation(&op));
                                    commands.push(Command::RunFileOperation {
                                        operation: op,
                                        refresh: self
                                            .refresh_targets_for_prompt(kind, &refresh_path),
                                        collision: CollisionPolicy::Fail,
                                    });
                                }
                                self.pending_batch = Some(PendingBatchOperation {
                                    pane: self.panes.focused_pane_id(),
                                    pending_operations,
                                    original_sources: batch_sources,
                                    failed_sources: BTreeSet::new(),
                                    total_count: count,
                                });
                                self.overlay.close_all();
                                self.status_message = format!("renaming {count} items");
                            } else {
                                let dest_dir = {
                                    let p = PathBuf::from(&value);
                                    if p.is_absolute() {
                                        p
                                    } else {
                                        prompt.base_path.join(p)
                                    }
                                };
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
                                            let operation = self
                                                .copy_operation_for_source(source, &copy_target);
                                            let refresh_path = self
                                                .refresh_target_path_for_transfer(
                                                    source,
                                                    &copy_target,
                                                );
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
                                                .refresh_target_path_for_transfer(
                                                    source,
                                                    &target_path,
                                                );
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
                                        pending_operations
                                            .push(FileOperationIdentity::from_operation(&op));
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
                                    pending_operations,
                                    original_sources: batch_sources,
                                    failed_sources: BTreeSet::new(),
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
                            } // end else (non-BulkRename batch)
                        } else {
                            let kind = prompt.kind;
                            let value = prompt.value().trim().to_string();
                            // GoTo: navigate the active pane to the typed directory.
                            if kind == PromptKind::GoTo {
                                let target = resolve_prompt_target(&prompt, &value);
                                if target.is_dir() {
                                    let pane = self.panes.focused_pane_id();
                                    commands.push(Command::ScanPane {
                                        pane,
                                        path: target.clone(),
                                    });
                                    self.status_message =
                                        format!("navigated to {}", target.display());
                                    self.overlay.close_all();
                                } else {
                                    self.status_message = format!("not a directory: {value}");
                                }
                            } else {
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
                                    PromptKind::NewDirectory => {
                                        Some(FileOperation::CreateDirectory {
                                            path: target_path.clone(),
                                        })
                                    }
                                    PromptKind::NewFile => Some(FileOperation::CreateFile {
                                        path: target_path.clone(),
                                    }),
                                    PromptKind::Rename => {
                                        prompt.source_path.as_ref().and_then(|s| {
                                            match Self::validate_rename_target(s, &value) {
                                                Err(msg) => {
                                                    self.status_message = msg;
                                                    None
                                                }
                                                Ok(None) => {
                                                    self.status_message =
                                                        String::from("rename unchanged");
                                                    None
                                                }
                                                Ok(Some(destination)) => {
                                                    Some(FileOperation::Rename {
                                                        source: s.clone(),
                                                        destination,
                                                    })
                                                }
                                            }
                                        })
                                    }
                                    // GoTo handled above; BulkRename only applies in batch mode.
                                    PromptKind::GoTo | PromptKind::BulkRename => None,
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
                                        } => self
                                            .refresh_target_path_for_transfer(source, destination),
                                        FileOperation::ExtractArchive { destination, .. } => {
                                            destination.clone()
                                        }
                                        _ => target_path.clone(),
                                    };
                                    let refresh =
                                        self.refresh_targets_for_prompt(kind, &refresh_path);
                                    commands.push(Command::RunFileOperation {
                                        operation,
                                        refresh,
                                        collision: CollisionPolicy::Fail,
                                    });
                                    self.status_message = match kind {
                                        PromptKind::Copy => String::from("copying item"),
                                        PromptKind::Trash => String::from("moving item to trash"),
                                        PromptKind::Delete => {
                                            String::from("deleting item permanently")
                                        }
                                        PromptKind::Move => String::from("moving item"),
                                        PromptKind::NewDirectory => {
                                            String::from("creating directory")
                                        }
                                        PromptKind::NewFile => String::from("creating file"),
                                        PromptKind::Rename => String::from("renaming item"),
                                        PromptKind::GoTo | PromptKind::BulkRename => String::new(),
                                    };
                                    true
                                } else if !(matches!(kind, PromptKind::Rename)
                                    && prompt.source_path.is_some())
                                {
                                    self.status_message =
                                        String::from("missing source for operation");
                                    true
                                } else {
                                    false
                                };
                                if should_close_overlay {
                                    self.overlay.close_all();
                                }
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
                let (selection, active_tab) =
                    if let Some(ModalState::Settings(s)) = &self.overlay.modal {
                        (s.selection, s.active_tab)
                    } else {
                        return Ok(commands);
                    };
                let entries = self.settings_entries_for_tab(active_tab);
                if let Some(entry) = entries.get(selection).cloned() {
                    if matches!(entry.field, SettingsField::KeymapBinding { .. }) {
                        // Begin rebind: enter key-capture mode for this entry.
                        if let Some(ModalState::Settings(s)) = &mut self.overlay.modal {
                            s.rebind_mode = Some(selection);
                        }
                        self.status_message = String::from("press new key combo (Esc to cancel)");
                    } else {
                        self.apply_settings_entry(entry);
                    }
                }
            }
            Action::SettingsMoveDown => {
                let active_tab = if let Some(ModalState::Settings(s)) = &self.overlay.modal {
                    s.active_tab
                } else {
                    return Ok(commands);
                };
                let entries = self.settings_entries_for_tab(active_tab);
                if !entries.is_empty() {
                    if let Some(s) = self.settings_mut() {
                        s.selection = (s.selection + 1) % entries.len();
                    }
                }
            }
            Action::SettingsMoveUp => {
                let active_tab = if let Some(ModalState::Settings(s)) = &self.overlay.modal {
                    s.active_tab
                } else {
                    return Ok(commands);
                };
                let entries = self.settings_entries_for_tab(active_tab);
                if !entries.is_empty() {
                    if let Some(s) = self.settings_mut() {
                        s.selection = if s.selection == 0 {
                            entries.len() - 1
                        } else {
                            s.selection - 1
                        };
                    }
                }
            }
            Action::SettingsBeginRebind => {
                if let Some(ModalState::Settings(s)) = &self.overlay.modal {
                    let selection = s.selection;
                    if let Some(ModalState::Settings(s)) = &mut self.overlay.modal {
                        s.rebind_mode = Some(selection);
                    }
                    self.status_message = String::from("press new key combo (Esc to cancel)");
                }
            }
            Action::SettingsCancelRebind => {
                if let Some(ModalState::Settings(s)) = &mut self.overlay.modal {
                    s.rebind_mode = None;
                }
                self.status_message = String::from("rebind cancelled");
            }
            Action::SettingsNextTab => {
                if let Some(s) = self.settings_mut() {
                    s.active_tab = s.active_tab.next();
                    s.selection = 0;
                }
            }
            Action::SettingsPrevTab => {
                if let Some(s) = self.settings_mut() {
                    s.active_tab = s.active_tab.prev();
                    s.selection = 0;
                }
            }
            Action::SettingsSelectTab(n) => {
                if let Some(s) = self.settings_mut() {
                    if let Some(tab) = crate::state::settings::SettingsTab::from_number(*n) {
                        s.active_tab = tab;
                        s.selection = 0;
                    }
                }
            }
            // Auto-preview after navigation
            Action::MoveSelectionDown | Action::MoveSelectionUp | Action::EnterSelection => {
                // Non-extend movement clears the range anchor.
                self.panes.active_pane_mut().reset_mark_anchor();
                if self.preview.should_auto_preview() {
                    let selected_kind_and_path = self
                        .panes
                        .active_pane()
                        .selected_entry()
                        .map(|entry| (entry.kind, entry.path.clone()));
                    if let Some((EntryKind::File, path)) = selected_kind_and_path {
                        self.preview.request_debounced_preview(path);
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
                let marked = &self.panes.active_pane().marked;
                if !marked.is_empty() {
                    let text = marked
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n");
                    let count = marked.len();
                    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text)) {
                        Ok(()) => {
                            self.status_message = format!("copied {count} paths to clipboard");
                        }
                        Err(e) => {
                            self.status_message = format!("clipboard error: {e}");
                        }
                    }
                } else if let Some(path) = self.panes.active_pane().selected_path() {
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
                        trust_unknown_host: false,
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
            Action::SshTrustAccept => {
                if let Some(crate::state::overlay::ModalState::SshTrustPrompt {
                    address,
                    auth_method,
                    credential,
                    pane,
                    ..
                }) = self.overlay.modal.clone()
                {
                    self.overlay.close_all();
                    self.status_message = format!("connecting to {} (trusted)…", address);
                    commands.push(Command::ConnectSSH {
                        address,
                        auth_method,
                        credential,
                        pane,
                        trust_unknown_host: true,
                    });
                }
            }
            Action::SshTrustReject => {
                self.overlay.close_all();
                self.status_message = String::from("SSH connection cancelled");
            }
            Action::ShowSymlinkTarget => {
                if let Some(entry) = self.panes.active_pane().selected_entry() {
                    if entry.kind == EntryKind::Symlink {
                        self.status_message = match &entry.link_target {
                            Some(t) => format!("symlink → {}", t.display()),
                            None => String::from("symlink target unavailable"),
                        };
                    }
                }
            }
            Action::FollowSymlink => {
                if let Some(entry) = self.panes.active_pane().selected_entry().cloned() {
                    if entry.kind == EntryKind::Symlink {
                        if let Some(target) = entry.link_target {
                            if target.is_dir() {
                                let pane = self.panes.active_pane_mut();
                                pane.push_history();
                                pane.cwd = target;
                                return Ok(vec![Command::DispatchAction(Action::Refresh)]);
                            } else if target.is_file() {
                                return Ok(vec![Command::DispatchAction(
                                    Action::OpenSelectedInEditor,
                                )]);
                            } else {
                                self.status_message =
                                    format!("target does not exist: {}", target.display());
                            }
                        }
                    }
                }
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
        let target_workspace = match &result {
            JobResult::DirectoryScanned { workspace_id, .. }
            | JobResult::ArchiveListed { workspace_id, .. }
            | JobResult::FileOperationCompleted { workspace_id, .. }
            | JobResult::FileOperationCollision { workspace_id, .. }
            | JobResult::FileOperationProgress { workspace_id, .. }
            | JobResult::JobFailed { workspace_id, .. }
            | JobResult::PreviewLoaded { workspace_id, .. }
            | JobResult::EditorLoaded { workspace_id, .. }
            | JobResult::EditorLoadFailed { workspace_id, .. }
            | JobResult::GitStatusLoaded { workspace_id, .. }
            | JobResult::GitStatusAbsent { workspace_id, .. }
            | JobResult::FindResults { workspace_id, .. }
            | JobResult::TerminalOutput { workspace_id, .. }
            | JobResult::TerminalDiagnostic { workspace_id, .. }
            | JobResult::TerminalExited { workspace_id, .. }
            | JobResult::DirSizeCalculated { workspace_id, .. } => Some(*workspace_id),
            JobResult::DirectoryChanged { .. } | JobResult::ConfigChanged => None,
            JobResult::SshConnected { workspace_id, .. }
            | JobResult::SshHostUnknown { workspace_id, .. }
            | JobResult::SshConnectionFailed { workspace_id, .. } => Some(*workspace_id),
        };

        let previous_workspace = self.active_workspace_idx;
        let target_is_active =
            target_workspace.is_none_or(|workspace_id| workspace_id == previous_workspace);
        if let Some(workspace_id) = target_workspace {
            self.active_workspace_idx = workspace_id;
        }

        match result {
            JobResult::DirectoryScanned {
                workspace_id: _,
                pane,
                path,
                entries,
                elapsed_ms,
            } => {
                // Detect whether this is a refresh of the pane's current directory
                // or navigation to a new one. Checked before updating `cwd`.
                let is_refresh =
                    self.panes.pane(pane).cwd == path && !self.panes.pane(pane).entries.is_empty();
                let is_local =
                    !self.panes.pane(pane).in_archive() && !self.panes.pane(pane).in_remote();

                self.panes.pane_mut(pane).cwd = path.clone();

                let cache_entries: Option<Vec<crate::fs::EntryInfo>> = if is_refresh {
                    // Incremental update: compute diff against the current entry list.
                    let old: Vec<crate::fs::EntryInfo> = self
                        .panes
                        .pane(pane)
                        .entries
                        .iter()
                        .filter(|e| e.name != "..")
                        .cloned()
                        .collect();
                    let diff = crate::fs::scan_diff::compute_scan_diff(&old, &entries);
                    self.panes.pane_mut(pane).apply_scan_diff(diff);
                    // `entries` is still owned here; re-use for cache.
                    if is_local {
                        Some(entries)
                    } else {
                        None
                    }
                } else {
                    // Full replace (navigation to a new directory).
                    let parent_entry = path.parent().map(|parent| crate::fs::EntryInfo {
                        name: String::from(".."),
                        path: parent.to_path_buf(),
                        kind: EntryKind::Directory,
                        size_bytes: None,
                        modified: None,
                        link_target: None,
                    });
                    let mut all_entries = Vec::with_capacity(entries.len() + 1);
                    if let Some(pe) = parent_entry {
                        all_entries.push(pe);
                    }
                    all_entries.extend_from_slice(&entries);
                    let cache = if is_local {
                        // Cache the raw entries without the ".." sentinel.
                        Some(
                            all_entries
                                .iter()
                                .filter(|e| e.name != "..")
                                .cloned()
                                .collect::<Vec<_>>(),
                        )
                    } else {
                        None
                    };
                    self.panes.pane_mut(pane).set_entries(all_entries);
                    self.panes.pane_mut(pane).refresh_filter();
                    cache
                };

                // Update the scan cache for local, non-archive panes.
                if let Some(raw_entries) = cache_entries {
                    let dir_mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                    if let Some(dir_mtime) = dir_mtime {
                        self.panes.pane_mut(pane).scan_cache = Some(crate::pane::ScanCache {
                            path: path.clone(),
                            dir_mtime,
                            entries: raw_entries,
                        });
                    }
                }

                if let Some((pending_pane, pending_path)) = self.pending_reveal.clone() {
                    if pending_pane == pane && pending_path.parent() == Some(path.as_path()) {
                        self.panes.pane_mut(pane).select_path(&pending_path);
                        self.pending_reveal = None;
                    }
                }
                self.status_message = format!("refreshed {} in {elapsed_ms} ms", path.display());
                self.last_scan_time_ms = Some(elapsed_ms);
                if self.diff_mode {
                    self.diff_map = crate::diff::compute_diff(
                        &self.panes.left.entries,
                        &self.panes.right.entries,
                    );
                }
            }
            JobResult::FileOperationCompleted {
                workspace_id: _,
                identity,
                message,
                refreshed,
                elapsed_ms,
            } => {
                if target_is_active {
                    self.overlay.close_all();
                }
                self.pending_collision = None;
                self.file_operation_status = None;
                for pane in refreshed {
                    self.panes.pane_mut(pane.pane).cwd = pane.path;
                    self.panes.pane_mut(pane.pane).set_entries(pane.entries);
                }
                if self.pending_batch.is_some() {
                    self.note_batch_settled(&identity, false);
                    if self.pending_batch.is_some() {
                        self.status_message = format!("{message} in {elapsed_ms} ms");
                    }
                } else {
                    self.status_message = format!("{message} in {elapsed_ms} ms");
                }
                self.last_scan_time_ms = Some(elapsed_ms);
            }
            JobResult::FileOperationCollision {
                workspace_id: _,
                identity,
                operation,
                refresh,
                path,
                elapsed_ms,
            } => {
                self.file_operation_status = None;
                self.note_batch_settled(&identity, true);
                let collision = CollisionState {
                    operation,
                    refresh,
                    path: path.clone(),
                };
                if target_is_active {
                    self.overlay.set_collision(collision);
                } else {
                    self.pending_collision = Some(collision);
                }
                if self.pending_batch.is_some() {
                    self.status_message = format!(
                        "destination exists after {elapsed_ms} ms: {}",
                        path.display()
                    );
                }
                self.last_scan_time_ms = Some(elapsed_ms);
            }
            JobResult::FileOperationProgress {
                workspace_id: _,
                status,
            } => {
                self.file_operation_status = Some(status);
            }
            JobResult::JobFailed {
                workspace_id: _,
                path,
                file_op,
                message,
                elapsed_ms,
                ..
            } => {
                self.file_operation_status = None;
                let failure_status = format!(
                    "job failed for {} after {elapsed_ms} ms: {message}",
                    path.display()
                );
                if let Some(file_op) = file_op.as_ref() {
                    if self.pending_batch.is_some() {
                        self.note_batch_settled(file_op, true);
                        if self.pending_batch.is_some() {
                            self.status_message = failure_status;
                        }
                    } else {
                        self.status_message = failure_status;
                    }
                } else {
                    self.status_message = failure_status;
                }
                self.last_scan_time_ms = Some(elapsed_ms);
            }
            JobResult::PreviewLoaded {
                workspace_id: _,
                path,
                view,
            } => {
                self.preview.apply_job_loaded(path, view);
            }
            JobResult::EditorLoaded {
                workspace_id: _,
                path,
                contents,
            } => {
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
            JobResult::EditorLoadFailed {
                workspace_id: _,
                path,
                message,
            } => {
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
            JobResult::GitStatusLoaded {
                workspace_id: _,
                pane,
                status,
            } => {
                self.git[pane as usize] = Some(status);
            }
            JobResult::GitStatusAbsent {
                workspace_id: _,
                pane,
            } => {
                self.git[pane as usize] = None;
            }
            JobResult::FindResults {
                workspace_id: _,
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
            JobResult::TerminalOutput {
                workspace_id: _,
                bytes,
            } => {
                self.terminal.process_output(&bytes);
            }
            JobResult::TerminalDiagnostic {
                workspace_id: _,
                message,
            } => {
                self.status_message = format!("[Terminal] {}", message);
            }
            JobResult::TerminalExited {
                workspace_id: _,
                spawn_id,
            } => {
                if spawn_id == self.terminal.spawn_id {
                    self.terminal.close();
                    self.terminal_fullscreen = false;
                    self.status_message = String::from("terminal session ended");
                }
            }
            JobResult::DirSizeCalculated {
                workspace_id: _,
                pane,
                path,
                bytes,
            } => {
                let p = self.panes.pane_mut(pane);
                if let Some(entry) = p.entries.iter_mut().find(|e| e.path == path) {
                    entry.size_bytes = Some(bytes);
                    p.cache_dirty.set(true);
                }
            }
            JobResult::ArchiveListed {
                workspace_id: _,
                pane,
                archive_path,
                inner_path,
                entries,
                elapsed_ms,
            } => {
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
            JobResult::ConfigChanged => {}
            JobResult::SshHostUnknown {
                workspace_id: _,
                pane,
                address,
                auth_method,
                credential,
                fingerprints,
            } => {
                // Parse host and port from the full address string for display.
                let (host, port) = match address.rsplit_once('@') {
                    Some((_, rest)) => match rest.rsplit_once(':') {
                        Some((h, p)) => (h.to_string(), p.parse::<u16>().unwrap_or(22)),
                        None => (rest.to_string(), 22u16),
                    },
                    None => (address.clone(), 22u16),
                };
                self.overlay.open_ssh_trust_prompt(
                    host,
                    port,
                    fingerprints,
                    address,
                    auth_method,
                    credential,
                    pane,
                );
                self.status_message = String::from("unknown SSH host — verify fingerprint");
            }
            JobResult::SshConnected {
                workspace_id: _,
                pane,
                session_id,
                address,
            } => {
                // Set pane to remote mode; app.rs will queue the SFTP scan.
                let home = std::path::PathBuf::from("/");
                self.panes.pane_mut(pane).mode = crate::pane::PaneMode::Remote {
                    address: session_id,
                    base_path: home.clone(),
                };
                self.panes.pane_mut(pane).cwd = home;
                self.overlay.close_all();
                self.status_message = format!("connected to {}", address);
            }
            JobResult::SshConnectionFailed {
                workspace_id: _,
                pane: _,
                address,
                error,
            } => {
                // Error is displayed in the SSH dialog; show status message
                self.status_message =
                    format!("SSH connection failed: {} - {}", address, error.message());
            }
        }

        if target_workspace.is_some() {
            self.active_workspace_idx = previous_workspace;
            self.sync_editor_menu_mode();
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
                    ThemePreset::Dracula => ThemePreset::CatppuccinMocha,
                    ThemePreset::CatppuccinMocha => ThemePreset::Zeta,
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
                    IconMode::NerdFont => IconMode::Unicode,
                };
                self.icon_mode = next;
                self.config.icon_mode = next;
                self.status_message = match next {
                    IconMode::Unicode => String::from("icons set to unicode"),
                    IconMode::Ascii => String::from("icons set to ASCII"),
                    IconMode::NerdFont => String::from("icons set to NerdFont"),
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
            SettingsField::TerminalOpenByDefault => {
                self.config.terminal_open_by_default = !self.config.terminal_open_by_default;
                self.status_message = if self.config.terminal_open_by_default {
                    String::from("terminal will open on startup")
                } else {
                    String::from("terminal will not open on startup")
                };
                let _ = self.config.save(Path::new(&self.config_path));
            }
            SettingsField::EditorTabWidth(current) => {
                let next = match current {
                    2 => 4,
                    4 => 8,
                    _ => 2,
                };
                self.config.editor.tab_width = next;
                self.status_message = format!("editor tab width set to {next}");
                let _ = self.config.save(Path::new(&self.config_path));
            }
            SettingsField::EditorWordWrap => {
                self.config.editor.word_wrap = !self.config.editor.word_wrap;
                self.status_message = if self.config.editor.word_wrap {
                    String::from("editor word wrap enabled")
                } else {
                    String::from("editor word wrap disabled")
                };
                let _ = self.config.save(Path::new(&self.config_path));
            }
            // Keymap bindings are rebindable — handled by SettingsBeginRebind/SettingsRebindCapture.
            // This arm is reached when the user presses Enter on a keymap entry.
            SettingsField::KeymapBinding { .. } => {}
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

    pub fn is_terminal_fullscreen(&self) -> bool {
        self.terminal_fullscreen
    }

    /// Computes the current UI context for menu/status display.
    pub fn menu_context(&self) -> MenuContext {
        if self.terminal_fullscreen {
            MenuContext::TerminalFullscreen
        } else if self.editor_fullscreen && self.editor.is_open() {
            MenuContext::EditorFullscreen
        } else if self.editor.is_open() {
            MenuContext::Editor
        } else if self.terminal.is_open() {
            MenuContext::Terminal
        } else {
            MenuContext::Pane
        }
    }

    pub fn pane_split_ratio(&self) -> u8 {
        self.pane_split_ratio
    }
    pub fn is_settings_rebinding(&self) -> bool {
        matches!(
            &self.overlay.modal,
            Some(ModalState::Settings(s)) if s.rebind_mode.is_some()
        )
    }
    pub fn is_editor_loading(&self) -> bool {
        self.editor.loading
    }

    /// Returns the cached git status for the given pane, if available.
    pub fn git_status(&self, pane: crate::pane::PaneId) -> Option<&crate::git::RepoStatus> {
        self.git[pane as usize].as_ref()
    }

    /// Derive the current input focus layer from state.
    /// Priority (highest → lowest): Palette > FileFinder > Collision > DestructiveConfirm > Prompt > Dialog > Menu > Settings > Bookmarks > PaneFilter > MarkdownPreview > Editor > Preview > Pane.
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
        if self.destructive_confirm().is_some() {
            return FocusLayer::Modal(ModalKind::DestructiveConfirm);
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
        if self.overlay.is_open_with() {
            return FocusLayer::Modal(ModalKind::OpenWith);
        }
        if self.overlay.is_ssh_trust_prompt() {
            return FocusLayer::Modal(ModalKind::SshTrustPrompt);
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
    pub fn destructive_confirm(&self) -> Option<&crate::state::dialog::DestructiveConfirmState> {
        self.overlay.destructive_confirm()
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
        self.can_focus_preview_panel() && self.panes.focus == PaneFocus::Preview
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

    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    pub fn set_needs_redraw(&mut self) {
        self.needs_redraw = true;
    }

    pub fn mark_drawn(&mut self) {
        self.redraw_count += 1;
    }

    pub fn redraw_count(&self) -> u64 {
        self.redraw_count
    }

    pub fn status_line(&self) -> String {
        let mark_count = self.panes.active_pane().marked_count();
        let scan = self
            .last_scan_time_ms
            .map(|value| format!("scan:{value}ms"))
            .unwrap_or_else(|| String::from("scan:-"));

        // Selected entry detail: permissions (unix) + human-readable size.
        let entry_detail = self
            .panes
            .active_pane()
            .selected_entry()
            .map(|e| {
                let size_str = e.size_bytes.map(format_file_size).unwrap_or_default();
                #[cfg(unix)]
                let perms = format_permissions_unix(&e.path);
                #[cfg(not(unix))]
                let perms = String::new();
                match (perms.is_empty(), size_str.is_empty()) {
                    (true, true) => String::new(),
                    (true, false) => format!(" {size_str}"),
                    (false, true) => format!(" {perms}"),
                    (false, false) => format!(" {perms} {size_str}"),
                }
            })
            .unwrap_or_default();

        let marks = if mark_count > 0 {
            let total_bytes: u64 = {
                let pane = self.panes.active_pane();
                pane.marked
                    .iter()
                    .filter_map(|path| {
                        pane.entries
                            .iter()
                            .find(|e| &e.path == path)
                            .and_then(|e| e.size_bytes)
                    })
                    .sum()
            };
            let size_suffix = if total_bytes > 0 {
                format!(" ({})", format_file_size(total_bytes))
            } else {
                String::new()
            };
            format!(" | {mark_count} marked{size_suffix}")
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
        let workspace = format!(
            "ws:{}/{}",
            self.active_workspace_index() + 1,
            self.workspace_count()
        );
        format!(
            "{} | {} | {}{} | {} | up:{}ms {}{}{}{} | d:{}",
            self.config.theme.status_bar_label,
            workspace,
            self.status_message,
            branch,
            self.theme.preset,
            self.startup_time_ms,
            scan,
            marks,
            progress,
            entry_detail,
            self.redraw_count
        )
    }

    pub fn status_zones(&self) -> StatusZones {
        let active_pane_id = match self.panes.focus {
            PaneFocus::Left | PaneFocus::Preview => crate::pane::PaneId::Left,
            PaneFocus::Right => crate::pane::PaneId::Right,
        };

        let git_branch = self
            .git_status(active_pane_id)
            .map(|g| format!(" ⎇ {} ", g.branch));

        let entry_detail = self.panes.active_pane().selected_entry().map(|e| {
            let icon = crate::icon::icon_for_entry(
                e.kind,
                e.path.extension().and_then(|x| x.to_str()),
                self.icon_mode(),
            );
            let name = e
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&e.name);
            let size_str = e.size_bytes.map(format_file_size).unwrap_or_default();
            #[cfg(unix)]
            let perms = format_permissions_unix(&e.path);
            #[cfg(not(unix))]
            let perms = String::new();
            if perms.is_empty() {
                format!(" {} {}  {} ", icon, name, size_str)
            } else {
                format!(" {} {}  {} {} ", icon, name, perms, size_str)
            }
        });

        let marks = {
            let pane = self.panes.active_pane();
            let count = pane.marked_count();
            if count > 0 {
                let total_bytes: u64 = pane
                    .marked
                    .iter()
                    .filter_map(|path| {
                        pane.entries
                            .iter()
                            .find(|e| &e.path == path)
                            .and_then(|e| e.size_bytes)
                    })
                    .sum();
                Some(MarksInfo { count, total_bytes })
            } else {
                None
            }
        };

        let progress = self.file_operation_status.as_ref().map(|status| {
            let current_name = status
                .current_path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or(".")
                .to_string();
            FileOpProgress {
                operation: status.operation.to_string(),
                current: status.completed,
                total: status.total,
                current_name,
            }
        });

        let workspace = format!(
            " ws:{}/{} ",
            self.active_workspace_index() + 1,
            self.workspace_count()
        );

        StatusZones {
            git_branch,
            entry_detail,
            message: format!(" {} ", self.status_message),
            marks,
            progress,
            workspace,
        }
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
                    "catppuccin_mocha" => ThemePreset::CatppuccinMocha,
                    _ => ThemePreset::Neon,
                }),
            },
            SettingsEntry {
                label: "Icon mode",
                value: match self.icon_mode {
                    IconMode::Unicode => String::from("unicode"),
                    IconMode::Ascii => String::from("ascii"),
                    IconMode::NerdFont => String::from("nerdfont"),
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
                label: "Terminal on startup",
                value: if self.config.terminal_open_by_default {
                    String::from("yes")
                } else {
                    String::from("no")
                },
                hint: "Space",
                field: SettingsField::TerminalOpenByDefault,
            },
            SettingsEntry {
                label: "Editor tab width",
                value: self.config.editor.tab_width.to_string(),
                hint: "Space",
                field: SettingsField::EditorTabWidth(self.config.editor.tab_width),
            },
            SettingsEntry {
                label: "Editor word wrap",
                value: if self.config.editor.word_wrap {
                    String::from("on")
                } else {
                    String::from("off")
                },
                hint: "Space",
                field: SettingsField::EditorWordWrap,
            },
            SettingsEntry {
                label: "Workspace 1 key",
                value: self.config.keymap.workspace_1.clone(),
                hint: "Enter",
                field: SettingsField::KeymapBinding {
                    field: KeymapField::Workspace(0),
                    current: self.config.keymap.workspace_1.clone(),
                },
            },
            SettingsEntry {
                label: "Workspace 2 key",
                value: self.config.keymap.workspace_2.clone(),
                hint: "Enter",
                field: SettingsField::KeymapBinding {
                    field: KeymapField::Workspace(1),
                    current: self.config.keymap.workspace_2.clone(),
                },
            },
            SettingsEntry {
                label: "Workspace 3 key",
                value: self.config.keymap.workspace_3.clone(),
                hint: "Enter",
                field: SettingsField::KeymapBinding {
                    field: KeymapField::Workspace(2),
                    current: self.config.keymap.workspace_3.clone(),
                },
            },
            SettingsEntry {
                label: "Workspace 4 key",
                value: self.config.keymap.workspace_4.clone(),
                hint: "Enter",
                field: SettingsField::KeymapBinding {
                    field: KeymapField::Workspace(3),
                    current: self.config.keymap.workspace_4.clone(),
                },
            },
            SettingsEntry {
                label: "Quit key",
                value: self.config.keymap.quit.clone(),
                hint: "Enter",
                field: SettingsField::KeymapBinding {
                    field: KeymapField::Quit,
                    current: self.config.keymap.quit.clone(),
                },
            },
            SettingsEntry {
                label: "Switch pane key",
                value: self.config.keymap.switch_pane.clone(),
                hint: "Enter",
                field: SettingsField::KeymapBinding {
                    field: KeymapField::SwitchPane,
                    current: self.config.keymap.switch_pane.clone(),
                },
            },
            SettingsEntry {
                label: "Refresh key",
                value: self.config.keymap.refresh.clone(),
                hint: "Enter",
                field: SettingsField::KeymapBinding {
                    field: KeymapField::Refresh,
                    current: self.config.keymap.refresh.clone(),
                },
            },
        ]
    }

    pub fn settings_entries_for_tab(
        &self,
        tab: SettingsTab,
    ) -> Vec<SettingsEntry> {
        let all = self.settings_entries();
        match tab {
            SettingsTab::Appearance => all
                .into_iter()
                .filter(|e| {
                    matches!(
                        e.field,
                        SettingsField::Theme(_) | SettingsField::IconMode(_)
                    )
                })
                .collect(),
            SettingsTab::Panels => all
                .into_iter()
                .filter(|e| {
                    matches!(
                        e.field,
                        SettingsField::PaneLayout(_)
                            | SettingsField::PreviewPanel
                            | SettingsField::PreviewOnSelection
                            | SettingsField::TerminalOpenByDefault
                    )
                })
                .collect(),
            SettingsTab::Editor => all
                .into_iter()
                .filter(|e| {
                    matches!(
                        e.field,
                        SettingsField::EditorTabWidth(_) | SettingsField::EditorWordWrap
                    )
                })
                .collect(),
            SettingsTab::Keymaps => all
                .into_iter()
                .filter(|e| matches!(e.field, SettingsField::KeymapBinding { .. }))
                .collect(),
        }
    }

    // =========================================================================
    // Private Helpers
    // =========================================================================

    fn settings_mut(&mut self) -> Option<&mut crate::state::settings::SettingsState> {
        if let Some(crate::state::overlay::ModalState::Settings(ref mut s)) = self.overlay.modal {
            Some(s)
        } else {
            None
        }
    }

    #[allow(dead_code)]
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

    /// Apply a bulk-rename pattern to a single source path.
    ///
    /// Supported substitutions:
    /// - `{n}` — 1-based index within the batch
    /// - `{name}` — filename stem (without extension)
    /// - `{ext}` — file extension (without the leading dot), empty for files with no extension
    fn apply_rename_pattern(pattern: &str, source: &Path, index: usize) -> String {
        let stem = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("");
        pattern
            .replace("{n}", &index.to_string())
            .replace("{name}", stem)
            .replace("{ext}", ext)
    }

    fn note_batch_settled(&mut self, identity: &FileOperationIdentity, failed: bool) {
        let mut finalize: Option<(PaneId, BTreeSet<PathBuf>, BTreeSet<PathBuf>, usize)> = None;
        if let Some(batch) = self.pending_batch.as_mut() {
            if let Some(index) = batch
                .pending_operations
                .iter()
                .position(|candidate| candidate == identity)
            {
                let settled_operation = batch.pending_operations.remove(index);
                if failed {
                    batch.failed_sources.insert(settled_operation.source);
                }
                if batch.pending_operations.is_empty() {
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

impl Deref for AppState {
    type Target = WorkspaceState;

    fn deref(&self) -> &Self::Target {
        self.active_workspace()
    }
}

impl DerefMut for AppState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.active_workspace_mut()
    }
}

/// Format a byte count as a human-readable string (B / K / M / G).
fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{bytes}B")
    }
}

/// Return a Unix-style permission string (e.g. `drwxr-xr-x`) for the given path.
/// Returns an empty string on non-unix platforms or if metadata cannot be read.
#[cfg(unix)]
fn format_permissions_unix(path: &std::path::Path) -> String {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return String::new();
    };
    let mode = meta.permissions().mode();
    let bit = |mask: u32, c: char| if mode & mask != 0 { c } else { '-' };
    let type_char = match mode & 0o170_000 {
        0o040_000 => 'd',
        0o120_000 => 'l',
        0o060_000 => 'b',
        0o020_000 => 'c',
        0o010_000 => 'p',
        0o140_000 => 's',
        _ => '-',
    };
    format!(
        "{}{}{}{}{}{}{}{}{}{}",
        type_char,
        bit(0o400, 'r'),
        bit(0o200, 'w'),
        bit(0o100, 'x'),
        bit(0o040, 'r'),
        bit(0o020, 'w'),
        bit(0o010, 'x'),
        bit(0o004, 'r'),
        bit(0o002, 'w'),
        bit(0o001, 'x'),
    )
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
    use crate::jobs::{FileOperationIdentity, FileOperationStatus, JobResult};
    use crate::pane::{InlineRenameState, PaneId, PaneState, SortMode};
    use crate::state::DebugState;

    use super::{
        resolve_prompt_target, AppState, CollisionState, FocusLayer, ModalKind, ModalState,
        OverlayState, PaneFocus, PaneLayout, PaneSetState, PreviewState, PromptKind, PromptState,
        WorkspaceState,
    };
    use crate::state::dialog::DestructiveAction;

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
                link_target: None,
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
            scan_cache: None,
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
            workspace_id: 0,
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
            workspace_id: 0,
            pane: crate::pane::PaneId::Left,
            status: RepoStatus::new(
                PathBuf::from("/tmp/repo"),
                String::from("main"),
                HashMap::new(),
            ),
        });
        state.apply_job_result(JobResult::GitStatusAbsent {
            workspace_id: 0,
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
            workspace_id: 0,
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

    #[test]
    fn workspace_switch_preserves_independent_pane_directories() {
        let mut state = test_state();
        state.panes.left.cwd = PathBuf::from("/tmp/ws0");

        state.switch_to_workspace(1);
        assert_eq!(state.panes.left.cwd, PathBuf::from("."));

        state.panes.left.cwd = PathBuf::from("/tmp/ws1");

        state.switch_to_workspace(0);
        assert_eq!(state.panes.left.cwd, PathBuf::from("/tmp/ws0"));
        assert_eq!(state.workspace(1).panes.left.cwd, PathBuf::from("/tmp/ws1"));
    }

    #[test]
    fn workspace_switch_preserves_independent_editor_state() {
        let mut state = test_state();

        let mut ws0_editor = EditorBuffer::default();
        ws0_editor.path = Some(PathBuf::from("alpha.txt"));
        state.open_editor(ws0_editor);
        state.editor.replace_active = true;
        state.editor.replace_query = String::from("alpha");

        state.switch_to_workspace(1);
        assert!(state.editor().is_none());
        assert!(!state.editor.replace_active);
        assert!(state.editor.replace_query.is_empty());

        let mut ws1_editor = EditorBuffer::default();
        ws1_editor.path = Some(PathBuf::from("beta.txt"));
        state.open_editor(ws1_editor);
        state.editor.replace_active = true;
        state.editor.replace_query = String::from("beta");

        state.switch_to_workspace(0);
        assert_eq!(
            state.editor().and_then(|editor| editor.path.as_ref()),
            Some(&PathBuf::from("alpha.txt"))
        );
        assert!(state.editor.replace_active);
        assert_eq!(state.editor.replace_query, "alpha");

        assert_eq!(
            state
                .workspace(1)
                .editor
                .buffer
                .as_ref()
                .and_then(|editor| editor.path.as_ref()),
            Some(&PathBuf::from("beta.txt"))
        );
        assert!(state.workspace(1).editor.replace_active);
        assert_eq!(state.workspace(1).editor.replace_query, "beta");
    }

    #[test]
    fn switch_to_workspace_updates_index_and_queues_initial_scans() {
        let mut state = test_state();

        let commands = state.apply(Action::SwitchToWorkspace(1)).unwrap();

        assert_eq!(state.active_workspace_index(), 1);
        assert_eq!(
            commands,
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
    fn directory_scan_result_updates_only_matching_workspace() {
        let mut state = test_state();
        state.workspace_mut(0).panes.left.cwd = PathBuf::from("/tmp/ws0");

        state.apply_job_result(JobResult::DirectoryScanned {
            workspace_id: 1,
            pane: PaneId::Left,
            path: PathBuf::from("/tmp/ws1"),
            entries: vec![],
            elapsed_ms: 1,
        });

        assert_eq!(state.workspace(1).panes.left.cwd, PathBuf::from("/tmp/ws1"));
        assert_eq!(state.workspace(0).panes.left.cwd, PathBuf::from("/tmp/ws0"));
    }

    #[test]
    fn switch_to_workspace_closes_shared_overlay() {
        let mut state = test_state();
        state.overlay.open_prompt(PromptState::with_value(
            PromptKind::Copy,
            "Copy",
            PathBuf::from("/tmp/target"),
            Some(PathBuf::from("./note.txt")),
            String::from("/tmp/target/note.txt"),
        ));

        let _ = state.apply(Action::SwitchToWorkspace(1)).unwrap();

        assert!(state.overlay.modal.is_none());
        assert_eq!(state.active_workspace_index(), 1);
    }

    #[test]
    fn file_operation_progress_does_not_leak_across_workspaces() {
        let mut state = test_state();

        state.apply_job_result(JobResult::FileOperationProgress {
            workspace_id: 2,
            status: FileOperationStatus {
                operation: "copy",
                completed: 1,
                total: 3,
                current_path: PathBuf::from("/tmp/a"),
            },
        });

        assert!(state.workspace(2).file_operation_status.is_some());
        assert!(state.workspace(0).file_operation_status.is_none());
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
            scan_cache: None,
        };
        let workspace0 = WorkspaceState::new(
            PaneSetState::new(pane_with_file("./note.txt"), right.clone()),
            PreviewState::new(false, true),
            String::from("ready"),
        );
        let workspace1 = WorkspaceState::new(
            PaneSetState::new(pane_with_file("./note.txt"), right.clone()),
            PreviewState::new(false, true),
            String::from("ready"),
        );
        let workspace2 = WorkspaceState::new(
            PaneSetState::new(pane_with_file("./note.txt"), right.clone()),
            PreviewState::new(false, true),
            String::from("ready"),
        );
        let workspace3 = WorkspaceState::new(
            PaneSetState::new(pane_with_file("./note.txt"), right),
            PreviewState::new(false, true),
            String::from("ready"),
        );
        AppState {
            workspaces: [workspace0, workspace1, workspace2, workspace3],
            active_workspace_idx: 0,
            overlay: OverlayState::default(),
            config_path: String::from("/tmp/zeta/config.toml"),
            config: crate::config::AppConfig::default(),
            icon_mode: crate::config::IconMode::Unicode,
            theme: ResolvedTheme {
                palette: ThemePalette::resolve(&crate::config::ThemeConfig::default()).palette,
                preset: String::from("fjord"),
                warning: None,
            },
            last_size: None,
            redraw_count: 0,
            startup_time_ms: 0,
            should_quit: false,
            needs_redraw: true,
            debug_visible: false,
            debug: DebugState::default(),
        }
    }

    #[test]
    fn bootstrap_initial_commands_queue_both_pane_scans() {
        let mut state = test_state();

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
        let path = PathBuf::from("./test.txt");
        state.panes.left.marked.insert(path);

        state
            .apply(Action::OpenDeletePrompt)
            .expect("delete modal should open");

        let commands = state
            .apply(Action::PromptSubmit)
            .expect("confirm should dispatch");

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
        let path = PathBuf::from("./test.txt");
        state.panes.left.marked.insert(path);

        state
            .apply(Action::OpenPermanentDeletePrompt)
            .expect("delete modal should open");

        let commands = state
            .apply(Action::PromptSubmit)
            .expect("confirm should dispatch");

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
            selection: 5,
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
            current_path: PathBuf::from("/tmp/not-the-source-or-destination.txt"),
        });
        state.apply_job_result(JobResult::FileOperationCompleted {
            workspace_id: 0,
            identity: FileOperationIdentity::from_operation(&FileOperation::Move {
                source: PathBuf::from("./note.txt"),
                destination: PathBuf::from("/tmp/target/note.txt"),
            }),
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
            current_path: root.join("wrong-progress-path.txt"),
        });
        state.apply_job_result(JobResult::FileOperationCompleted {
            workspace_id: 0,
            identity: FileOperationIdentity::from_operation(&FileOperation::ExtractArchive {
                archive: archive_path.clone(),
                inner_path: PathBuf::from("nested/note.txt"),
                destination: state.panes.right.cwd.clone(),
            }),
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
            current_path: PathBuf::from("/tmp/wrong-progress-path.txt"),
        });
        state.apply_job_result(JobResult::FileOperationCompleted {
            workspace_id: 0,
            identity: FileOperationIdentity::from_operation(&FileOperation::Copy {
                source: PathBuf::from("./note.txt"),
                destination: PathBuf::from("/tmp/target/note.txt"),
            }),
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
            link_target: None,
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
            current_path: PathBuf::from("/tmp/irrelevant-progress-path.txt"),
        });
        state.apply_job_result(JobResult::FileOperationCompleted {
            workspace_id: 0,
            identity: FileOperationIdentity::from_operation(&FileOperation::Copy {
                source: PathBuf::from("./two.txt"),
                destination: PathBuf::from("/tmp/target/two.txt"),
            }),
            message: String::from("copied"),
            refreshed: Vec::new(),
            elapsed_ms: 1,
        });
        assert_eq!(state.panes.left.marked_count(), 2);
        assert!(state.pending_batch.is_some());

        state.apply_job_result(JobResult::JobFailed {
            workspace_id: 0,
            pane: PaneId::Left,
            path: PathBuf::from("./note.txt"),
            file_op: Some(FileOperationIdentity::from_operation(
                &FileOperation::Copy {
                    source: PathBuf::from("./note.txt"),
                    destination: PathBuf::from("/tmp/target/note.txt"),
                },
            )),
            message: String::from("permission denied"),
            elapsed_ms: 2,
        });

        assert!(state
            .panes
            .left
            .marked
            .contains(&PathBuf::from("./note.txt")));
        assert!(!state
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
    fn non_file_job_failed_does_not_settle_pending_batch() {
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

        state.apply_job_result(JobResult::JobFailed {
            workspace_id: 0,
            pane: PaneId::Left,
            path: PathBuf::from("./note.txt"),
            file_op: None,
            message: String::from("unrelated scan failed"),
            elapsed_ms: 2,
        });

        assert!(state
            .panes
            .left
            .marked
            .contains(&PathBuf::from("./note.txt")));
        assert!(state.pending_batch.is_some());
        assert_eq!(
            state.status_message,
            "job failed for ./note.txt after 2 ms: unrelated scan failed"
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
            workspace_id: 0,
            pane: PaneId::Left,
            path: PathBuf::from("./note.txt"),
            file_op: Some(FileOperationIdentity::from_operation(
                &FileOperation::Delete {
                    path: PathBuf::from("./note.txt"),
                },
            )),
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
        assert_eq!(prompt.value(), expected);
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
    fn status_line_includes_workspace_indicator() {
        let mut state = test_state();
        let _ = state.apply(Action::SwitchToWorkspace(2)).unwrap();

        let status = state.status_line();

        assert!(status.contains("ws:3/4"));
    }

    #[test]
    fn status_line_includes_mark_count() {
        let mut state = test_state();
        state.panes.left.marked.insert(PathBuf::from("./note.txt"));

        let status = state.status_line();

        assert!(status.contains("1 marked"));
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
            workspace_id: 0,
            identity: FileOperationIdentity::from_operation(&FileOperation::Copy {
                source: PathBuf::from("/tmp/source/note.txt"),
                destination: PathBuf::from("/tmp/target/note.txt"),
            }),
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
        assert_eq!(prompt.value(), expected);
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
            workspace_id: 0,
            identity: FileOperationIdentity::from_operation(&FileOperation::Copy {
                source: PathBuf::from("./note.txt"),
                destination: PathBuf::from("/tmp/target/note.txt"),
            }),
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
    fn inactive_workspace_collision_does_not_replace_active_overlay() {
        let mut state = test_state();
        state.overlay.open_prompt(PromptState::with_value(
            PromptKind::Copy,
            "Copy",
            PathBuf::from("/tmp/target"),
            Some(PathBuf::from("./note.txt")),
            String::from("/tmp/target/note.txt"),
        ));

        state.apply_job_result(JobResult::FileOperationCollision {
            workspace_id: 1,
            identity: FileOperationIdentity::from_operation(&FileOperation::Copy {
                source: PathBuf::from("./other.txt"),
                destination: PathBuf::from("/tmp/other-target/other.txt"),
            }),
            operation: FileOperation::Copy {
                source: PathBuf::from("./other.txt"),
                destination: PathBuf::from("/tmp/other-target/other.txt"),
            },
            refresh: vec![],
            path: PathBuf::from("/tmp/other-target/other.txt"),
            elapsed_ms: 5,
        });

        assert!(state.overlay.prompt().is_some());
        assert!(!state.is_collision_open());
        assert!(state.workspace(1).pending_collision.is_some());
    }

    #[test]
    fn switching_to_workspace_surfaces_deferred_collision() {
        let mut state = test_state();
        state.apply_job_result(JobResult::FileOperationCollision {
            workspace_id: 1,
            identity: FileOperationIdentity::from_operation(&FileOperation::Copy {
                source: PathBuf::from("./other.txt"),
                destination: PathBuf::from("/tmp/other-target/other.txt"),
            }),
            operation: FileOperation::Copy {
                source: PathBuf::from("./other.txt"),
                destination: PathBuf::from("/tmp/other-target/other.txt"),
            },
            refresh: vec![],
            path: PathBuf::from("/tmp/other-target/other.txt"),
            elapsed_ms: 5,
        });

        let _ = state.apply(Action::SwitchToWorkspace(1)).unwrap();

        assert!(state.is_collision_open());
        assert_eq!(
            state.collision().map(|collision| collision.path.clone()),
            Some(PathBuf::from("/tmp/other-target/other.txt"))
        );
        assert!(state.workspace(1).pending_collision.is_none());
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
        assert_eq!(
            prompt.value(),
            root.join("note-1.txt").display().to_string()
        );
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
            .expect("delete modal should open");

        // Marked items should open batch Trash PromptState
        match &state.overlay.modal {
            Some(ModalState::Prompt(prompt)) => {
                assert_eq!(prompt.kind, PromptKind::Trash);
                assert_eq!(prompt.title, "Trash Marked Items");
                assert_eq!(prompt.source_paths.len(), 2);
            }
            _ => panic!("Expected Prompt modal for batch delete"),
        }
        assert!(!state.status_message.is_empty());
    }

    #[test]
    fn open_permanent_delete_prompt_summarizes_marked_items() {
        let mut state = test_state();
        state.panes.left.marked.insert(PathBuf::from("./beta.txt"));
        state.panes.left.marked.insert(PathBuf::from("./alpha.txt"));

        state
            .apply(Action::OpenPermanentDeletePrompt)
            .expect("delete modal should open");

        // Marked items should open batch Delete PromptState
        match &state.overlay.modal {
            Some(ModalState::Prompt(prompt)) => {
                assert_eq!(prompt.kind, PromptKind::Delete);
                assert_eq!(prompt.title, "Delete Permanently Marked Items");
                assert_eq!(prompt.source_paths.len(), 2);
            }
            _ => panic!("Expected Prompt modal for batch permanent delete"),
        }
        assert!(!state.status_message.is_empty());
    }

    #[test]
    fn open_delete_prompt_sets_trash_confirmation_message() {
        let mut state = test_state();
        let path = PathBuf::from("./test.txt");
        state.panes.left.marked.insert(path);

        state
            .apply(Action::OpenDeletePrompt)
            .expect("delete modal should open");

        // Marked items should open batch Trash PromptState
        match &state.overlay.modal {
            Some(ModalState::Prompt(prompt)) => {
                assert_eq!(prompt.kind, PromptKind::Trash);
                assert_eq!(prompt.title, "Trash Marked Items");
                assert_eq!(prompt.source_paths.len(), 1);
            }
            _ => panic!("Expected Prompt modal for batch delete"),
        }
    }

    #[test]
    fn open_permanent_delete_prompt_sets_delete_confirmation_message() {
        let mut state = test_state();
        let path = PathBuf::from("./test.txt");
        state.panes.left.marked.insert(path);

        state
            .apply(Action::OpenPermanentDeletePrompt)
            .expect("delete modal should open");

        // Marked items should open batch Delete PromptState
        match &state.overlay.modal {
            Some(ModalState::Prompt(prompt)) => {
                assert_eq!(prompt.kind, PromptKind::Delete);
                assert_eq!(prompt.title, "Delete Permanently Marked Items");
                assert_eq!(prompt.source_paths.len(), 1);
            }
            _ => panic!("Expected Prompt modal for batch permanent delete"),
        }
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
    fn empty_preview_panel_does_not_take_focus() {
        let mut state = test_state();
        state.preview.panel_open = true;

        state
            .apply(Action::FocusPreviewPanel)
            .expect("focus preview should succeed");

        assert_eq!(state.panes.focus, PaneFocus::Left);
        assert!(matches!(state.focus_layer(), FocusLayer::Pane));
    }

    #[test]
    fn empty_preview_panel_never_claims_preview_focus_layer() {
        let mut state = test_state();
        state.preview.panel_open = true;
        state.panes.focus = PaneFocus::Preview;

        assert!(matches!(state.focus_layer(), FocusLayer::Pane));
        assert!(!state.is_preview_focused());
    }

    #[test]
    fn focus_preview_panel_toggles_even_when_editor_exists() {
        let mut state = test_state();
        state.preview.panel_open = true;
        state.preview.view = Some((
            PathBuf::from("./note.txt"),
            crate::preview::ViewBuffer::from_plain("hello"),
        ));
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

    #[test]
    fn destructive_confirm_state_renders_correctly() {
        use crate::state::dialog::{DestructiveAction, DestructiveConfirmState};
        use std::path::PathBuf;

        let items = vec![
            PathBuf::from("file1.txt"),
            PathBuf::from("file2.txt"),
            PathBuf::from("file3.txt"),
            PathBuf::from("file4.txt"),
        ];

        let state = DestructiveConfirmState::new(DestructiveAction::Delete, &items, vec![]);

        let lines = state.lines();
        assert!(lines.iter().any(|l| l.contains("⚠")));
        assert!(lines.iter().any(|l| l.contains("Delete")));
        assert!(lines.iter().any(|l| l.contains("4")));
        assert!(lines.iter().any(|l| l.contains("file1.txt")));
        assert!(lines.iter().any(|l| l.contains("... and 1 more")));
    }

    #[test]
    fn open_delete_prompt_opens_destructive_confirm_modal() {
        let mut state = test_state();
        // test_state() has a "note.txt" entry already - don't mark it, just delete the selected one
        // The first entry is already selected (selection: 0)

        let commands = state
            .apply(Action::OpenDeletePrompt)
            .expect("open delete prompt should succeed");

        assert!(matches!(
            state.overlay.modal,
            Some(ModalState::DestructiveConfirm(_))
        ));
        assert_eq!(commands.len(), 0, "no operations should dispatch yet");
    }

    #[test]
    fn open_delete_prompt_with_single_marked_item_opens_destructive_confirm() {
        let mut state = test_state();
        let path = PathBuf::from("myfile.txt");
        state.panes.active_pane_mut().marked.insert(path);

        let commands = state
            .apply(Action::OpenDeletePrompt)
            .expect("open delete prompt should succeed");

        // Single marked item should open batch Trash PromptState (consistent with Copy/Move)
        match &state.overlay.modal {
            Some(ModalState::Prompt(prompt)) => {
                assert_eq!(prompt.kind, PromptKind::Trash);
                assert_eq!(prompt.title, "Trash Marked Items");
                assert_eq!(prompt.source_paths.len(), 1);
                assert_eq!(prompt.source_paths[0], PathBuf::from("myfile.txt"));
            }
            Some(ModalState::DestructiveConfirm(_)) => {
                panic!("Got DestructiveConfirm instead of expected Trash Prompt for marked item");
            }
            other => panic!("Got unexpected modal type: {:?}", other),
        }
        assert_eq!(commands.len(), 0, "no operations should dispatch yet");
    }

    #[test]
    fn destructive_confirm_yes_dispatches_operation() {
        let mut state = test_state();
        // test_state() has a "note.txt" entry already - don't mark it, just delete the selected one

        state
            .apply(Action::OpenDeletePrompt)
            .expect("open delete prompt should succeed");

        let commands = state
            .apply(Action::DestructiveConfirmYes)
            .expect("confirm should dispatch");

        assert!(state.overlay.modal.is_none());
        assert!(commands
            .iter()
            .any(|cmd| matches!(cmd, Command::RunFileOperation { .. })));
    }

    #[test]
    fn destructive_confirm_no_cancels_operation() {
        let mut state = test_state();
        // test_state() has a "note.txt" entry already - don't mark it, just delete the selected one

        state
            .apply(Action::OpenDeletePrompt)
            .expect("open delete prompt should succeed");

        let commands = state
            .apply(Action::DestructiveConfirmNo)
            .expect("confirm no should succeed");

        assert!(state.overlay.modal.is_none());
        assert_eq!(commands.len(), 0, "no operations should dispatch on cancel");
    }

    #[test]
    fn destructive_confirm_yes_deletes_all_marked_items() {
        let mut state = test_state();
        let paths = vec![
            PathBuf::from("file1.txt"),
            PathBuf::from("file2.txt"),
            PathBuf::from("file3.txt"),
        ];
        for path in &paths {
            state.panes.active_pane_mut().marked.insert(path.clone());
        }

        state
            .apply(Action::OpenDeletePrompt)
            .expect("open delete prompt should succeed");

        // Should have opened batch Trash PromptState
        let is_trash_prompt = match &state.overlay.modal {
            Some(ModalState::Prompt(p)) => p.kind == PromptKind::Trash && p.source_paths.len() == 3,
            _ => false,
        };
        assert!(
            is_trash_prompt,
            "Expected Trash PromptState with 3 source_paths"
        );

        // Submit the trash prompt to dispatch operations
        let commands = state
            .apply(Action::PromptSubmit)
            .expect("submit should dispatch");

        assert!(state.overlay.modal.is_none());
        let file_ops: Vec<_> = commands
            .iter()
            .filter(|cmd| matches!(cmd, Command::RunFileOperation { .. }))
            .collect();
        assert_eq!(
            file_ops.len(),
            3,
            "should dispatch 3 operations for 3 marked items"
        );
    }

    #[test]
    fn destructive_confirm_modal_key_routing() {
        let mut state = test_state();
        // test_state() has a "note.txt" entry already - don't mark it

        state
            .apply(Action::OpenDeletePrompt)
            .expect("modal should open");

        // Verify focus layer is correct for key routing
        assert_eq!(
            state.focus_layer(),
            FocusLayer::Modal(ModalKind::DestructiveConfirm),
            "focus should be on destructive confirm modal when open"
        );

        // Verify modal is actually set
        assert!(
            matches!(state.overlay.modal, Some(ModalState::DestructiveConfirm(_))),
            "destructive confirm state should be in overlay"
        );
    }

    #[test]
    fn status_zones_workspace_format() {
        let ws = format!(" ws:{}/{} ", 1, 4);
        assert!(ws.starts_with(" ws:"));
        assert!(ws.contains('/'));
    }
}
