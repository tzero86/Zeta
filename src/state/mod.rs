mod dialog;
mod menu;
mod prompt;
mod settings;
mod types;

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;

use crate::action::{Action, CollisionPolicy, Command, FileOperation, MenuId, RefreshTarget};
use crate::config::{AppConfig, IconMode, LoadedConfig, ResolvedTheme, ThemePalette, ThemePreset};
use crate::editor::EditorBuffer;
use crate::fs;
use crate::fs::EntryKind;
use crate::jobs::{FileOperationStatus, JobResult, PreviewContent};
use crate::pane::{PaneId, PaneState};

use crate::palette::PaletteState;
pub use dialog::{CollisionState, DialogState};
use menu::menu_items_for;
pub use prompt::{resolve_prompt_target, PromptKind, PromptState};
pub use settings::{SettingsEntry, SettingsField, SettingsState};
pub use types::{MenuItem, PaneFocus, PaneLayout};

#[derive(Clone, Debug)]
pub struct AppState {
    left: PaneState,
    right: PaneState,
    focus: PaneFocus,
    pane_layout: PaneLayout,
    app_label: String,
    config_path: String,
    config: AppConfig,
    icon_mode: IconMode,
    theme: ResolvedTheme,
    active_menu: Option<MenuId>,
    editor: Option<EditorBuffer>,
    menu_selection: usize,
    prompt: Option<PromptState>,
    dialog: Option<DialogState>,
    collision: Option<CollisionState>,
    pub preview: Option<(PathBuf, PreviewContent)>,
    pub preview_panel_open: bool,
    preview_on_selection: bool,
    preview_scroll: usize,
    settings: Option<SettingsState>,
    status_message: String,
    last_size: Option<(u16, u16)>,
    redraw_count: u64,
    startup_time_ms: u128,
    last_scan_time_ms: Option<u128>,
    file_operation_status: Option<FileOperationStatus>,
    needs_redraw: bool,
    should_quit: bool,
    command_palette: Option<PaletteState>,
}

impl AppState {
    pub fn bootstrap(loaded_config: LoadedConfig, started_at: Instant) -> Result<Self> {
        let cwd = fs::current_dir()?;
        let secondary = cwd
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| cwd.clone());
        let resolved_theme = loaded_config.config.resolve_theme();
        let status_bar_label = loaded_config.config.theme.status_bar_label.clone();

        let left = PaneState::empty("Left", cwd.clone());
        let right = PaneState::empty("Right", secondary.clone());

        Ok(Self {
            left,
            right,
            focus: PaneFocus::Left,
            pane_layout: PaneLayout::SideBySide,
            app_label: status_bar_label,
            config_path: loaded_config.path.display().to_string(),
            config: loaded_config.config.clone(),
            icon_mode: loaded_config.config.icon_mode,
            theme: resolved_theme.clone(),
            active_menu: None,
            editor: None,
            menu_selection: 0,
            prompt: None,
            dialog: None,
            collision: None,
            preview: None,
            preview_panel_open: loaded_config.config.preview_panel_open,
            preview_on_selection: loaded_config.config.preview_on_selection,
            preview_scroll: 0,
            settings: None,
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
            needs_redraw: true,
            should_quit: false,
            command_palette: None,
        })
    }

    pub fn initial_commands(&self) -> Vec<Command> {
        vec![
            Command::ScanPane {
                pane: PaneId::Left,
                path: self.left.cwd.clone(),
            },
            Command::ScanPane {
                pane: PaneId::Right,
                path: self.right.cwd.clone(),
            },
        ]
    }

    pub fn apply(&mut self, action: Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();

        // When the palette is open only palette actions are processed — no bleed.
        if self.command_palette.is_some() {
            return self.reduce_palette(&action);
        }

        // Delegate to focused reducers
        commands.extend(self.reduce_palette(&action)?);
        commands.extend(self.reduce_collision(&action)?);
        commands.extend(self.reduce_dialog(&action)?);
        commands.extend(self.reduce_menu(&action)?);
        commands.extend(self.reduce_pane(&action)?);
        commands.extend(self.reduce_editor(&action)?);
        commands.extend(self.reduce_file_op_prompts(&action)?);
        commands.extend(self.reduce_prompt_input(&action)?);
        commands.extend(self.reduce_view(&action)?);
        commands.extend(self.reduce_preview(&action)?);
        commands.extend(self.reduce_settings(&action)?);

        Ok(commands)
    }

    // =========================================================================
    // Palette Reducer
    // =========================================================================

    fn reduce_palette(&mut self, action: &Action) -> Result<Vec<Command>> {
        match action {
            Action::OpenCommandPalette => {
                // Don't open if another modal is active.
                if self.active_menu.is_none()
                    && self.prompt.is_none()
                    && self.dialog.is_none()
                    && self.collision.is_none()
                {
                    self.command_palette = Some(PaletteState::new());
                    self.needs_redraw = true;
                }
            }
            Action::CloseCommandPalette => {
                self.command_palette = None;
                self.needs_redraw = true;
            }
            Action::PaletteInput(c) => {
                if let Some(p) = self.command_palette.as_mut() {
                    p.query.push(*c);
                    p.selection = 0;
                    self.needs_redraw = true;
                }
            }
            Action::PaletteBackspace => {
                if let Some(p) = self.command_palette.as_mut() {
                    p.query.pop();
                    p.selection = 0;
                    self.needs_redraw = true;
                }
            }
            Action::PaletteMoveDown => {
                if let Some(p) = self.command_palette.as_mut() {
                    let entries = crate::palette::all_entries();
                    let matches = crate::palette::filter_entries(&entries, &p.query);
                    if !matches.is_empty() {
                        p.selection = (p.selection + 1).min(matches.len() - 1);
                    }
                    self.needs_redraw = true;
                }
            }
            Action::PaletteMoveUp => {
                if let Some(p) = self.command_palette.as_mut() {
                    p.selection = p.selection.saturating_sub(1);
                    self.needs_redraw = true;
                }
            }
            Action::PaletteConfirm => {
                if let Some(p) = self.command_palette.take() {
                    // take() removes the palette before the recursive apply call.
                    self.needs_redraw = true;
                    let entries = crate::palette::all_entries();
                    let matches = crate::palette::filter_entries(&entries, &p.query);
                    if let Some(entry) = matches.get(p.selection) {
                        return self.apply(entry.action.clone());
                    }
                }
            }
            _ => {}
        }
        Ok(vec![])
    }

    // =========================================================================
    // Collision Reducer
    // =========================================================================

    fn reduce_collision(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();

        match action {
            Action::CollisionCancel => {
                self.collision = None;
                self.status_message = String::from("cancelled collision resolution");
                self.needs_redraw = true;
            }
            Action::CollisionOverwrite => {
                if let Some(collision) = self.collision.take() {
                    commands.push(Command::RunFileOperation {
                        operation: collision.operation,
                        refresh: collision.refresh,
                        collision: CollisionPolicy::Overwrite,
                    });
                    self.status_message = String::from("overwriting existing destination");
                } else {
                    self.status_message = String::from("no collision to resolve");
                }
                self.needs_redraw = true;
            }
            Action::CollisionRename => {
                if let Some(collision) = self.collision.take() {
                    self.prompt = Some(collision.rename_prompt());
                    self.status_message = String::from("edit the new destination");
                } else {
                    self.status_message = String::from("no collision to resolve");
                }
                self.needs_redraw = true;
            }
            Action::CollisionSkip => {
                self.collision = None;
                self.status_message = String::from("skipped collided destination");
                self.needs_redraw = true;
            }
            _ => {}
        }

        Ok(commands)
    }

    // =========================================================================
    // Dialog Reducer
    // =========================================================================

    fn reduce_dialog(&mut self, action: &Action) -> Result<Vec<Command>> {
        match action {
            Action::CloseDialog => {
                self.dialog = None;
                self.status_message = String::from("closed dialog");
                self.needs_redraw = true;
            }
            Action::OpenAboutDialog => {
                self.collision = None;
                self.active_menu = None;
                self.menu_selection = 0;
                self.dialog = Some(DialogState::about(
                    self.theme.preset.clone(),
                    self.config_path.clone(),
                ));
                self.status_message = String::from("opened about");
                self.needs_redraw = true;
            }
            Action::OpenHelpDialog => {
                self.collision = None;
                self.active_menu = None;
                self.menu_selection = 0;
                self.dialog = Some(DialogState::help());
                self.status_message = String::from("opened help");
                self.needs_redraw = true;
            }
            _ => {}
        }

        Ok(Vec::new())
    }

    // =========================================================================
    // Menu Reducer
    // =========================================================================

    fn reduce_menu(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();

        match action {
            Action::CloseMenu => {
                self.active_menu = None;
                self.menu_selection = 0;
                self.needs_redraw = true;
            }
            Action::OpenMenu(menu) => {
                self.collision = None;
                self.dialog = None;
                self.active_menu = Some(*menu);
                self.menu_selection = 0;
                self.needs_redraw = true;
            }
            Action::MenuActivate => {
                if let Some(menu) = self.active_menu {
                    if let Some(item) = menu_items_for(menu).get(self.menu_selection).cloned() {
                        self.active_menu = None;
                        self.menu_selection = 0;
                        commands.extend(self.apply(item.action)?);
                    }
                }
            }
            Action::MenuMnemonic(ch) => {
                if let Some(menu) = self.active_menu {
                    if let Some(item) = menu_items_for(menu)
                        .into_iter()
                        .find(|item| item.mnemonic.eq_ignore_ascii_case(ch))
                    {
                        self.active_menu = None;
                        self.menu_selection = 0;
                        commands.extend(self.apply(item.action)?);
                    }
                }
            }
            Action::MenuMoveDown => {
                if let Some(menu) = self.active_menu {
                    let len = menu_items_for(menu).len();
                    if len > 0 {
                        self.menu_selection = (self.menu_selection + 1).min(len.saturating_sub(1));
                        self.needs_redraw = true;
                    }
                }
            }
            Action::MenuMoveUp => {
                self.menu_selection = self.menu_selection.saturating_sub(1);
                self.needs_redraw = true;
            }
            Action::MenuNext => {
                if let Some(menu) = self.active_menu {
                    self.active_menu = Some(match menu {
                        MenuId::File => MenuId::Navigate,
                        MenuId::Navigate => MenuId::View,
                        MenuId::View => MenuId::Help,
                        MenuId::Help => MenuId::File,
                    });
                    self.menu_selection = 0;
                    self.needs_redraw = true;
                }
            }
            Action::MenuPrevious => {
                if let Some(menu) = self.active_menu {
                    self.active_menu = Some(match menu {
                        MenuId::File => MenuId::Help,
                        MenuId::Navigate => MenuId::File,
                        MenuId::View => MenuId::Navigate,
                        MenuId::Help => MenuId::View,
                    });
                    self.menu_selection = 0;
                    self.needs_redraw = true;
                }
            }
            _ => {}
        }

        Ok(commands)
    }

    // =========================================================================
    // Pane Reducer
    // =========================================================================

    fn reduce_pane(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();

        match action {
            Action::EnterSelection => {
                if self.active_pane().can_enter_selected() {
                    if let Some(path) = self.active_pane().selected_path() {
                        let pane = self.focused_pane_id();
                        self.status_message = format!("opening directory {}", path.display());
                        let active = self.active_pane_mut();
                        active.clear_marks();
                        active.push_history();
                        self.needs_redraw = true;
                        commands.push(Command::ScanPane { pane, path });
                    }
                } else {
                    self.status_message = String::from("selected item is not a directory");
                    self.needs_redraw = true;
                }
            }
            Action::NavigateToParent => {
                if let Some(path) = self.active_pane().parent_path() {
                    let pane = self.focused_pane_id();
                    self.status_message = format!("opening parent {}", path.display());
                    let active = self.active_pane_mut();
                    active.clear_marks();
                    active.push_history();
                    self.needs_redraw = true;
                    commands.push(Command::ScanPane { pane, path });
                } else {
                    self.status_message = String::from("already at filesystem root");
                    self.needs_redraw = true;
                }
            }
            Action::NavigateBack => {
                if let Some(path) = self.active_pane_mut().pop_back() {
                    let pane_id = self.focused_pane_id();
                    self.active_pane_mut().clear_marks();
                    self.needs_redraw = true;
                    return Ok(vec![Command::ScanPane {
                        pane: pane_id,
                        path,
                    }]);
                }
            }
            Action::NavigateForward => {
                if let Some(path) = self.active_pane_mut().pop_forward() {
                    let pane_id = self.focused_pane_id();
                    self.active_pane_mut().clear_marks();
                    self.needs_redraw = true;
                    return Ok(vec![Command::ScanPane {
                        pane: pane_id,
                        path,
                    }]);
                }
            }
            Action::FocusNextPane => {
                self.focus = match self.focus {
                    PaneFocus::Left | PaneFocus::Preview => PaneFocus::Right,
                    PaneFocus::Right => PaneFocus::Left,
                };
                self.needs_redraw = true;
            }
            Action::MoveSelectionDown => {
                self.active_pane_mut().move_selection_down();
                self.needs_redraw = true;
            }
            Action::MoveSelectionUp => {
                self.active_pane_mut().move_selection_up();
                self.needs_redraw = true;
            }
            Action::ToggleMark => {
                if self.active_pane_mut().toggle_mark_selected().is_some() {
                    self.needs_redraw = true;
                }
            }
            Action::ClearMarks => {
                self.active_pane_mut().clear_marks();
                self.needs_redraw = true;
            }
            Action::Refresh => {
                let pane = self.focused_pane_id();
                let path = self.active_pane().cwd.clone();
                self.status_message = format!("refreshing {}", path.display());
                self.needs_redraw = true;
                commands.push(Command::ScanPane { pane, path });
            }
            Action::CycleSortMode => {
                let pane = self.active_pane_mut();
                pane.sort_mode = pane.sort_mode.next();
                pane.selection = 0;
                pane.scroll_offset = 0;
                self.needs_redraw = true;
            }
            _ => {}
        }

        Ok(commands)
    }

    // =========================================================================
    // Editor Reducer
    // =========================================================================

    fn reduce_editor(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();

        match action {
            Action::CloseEditor => {
                // If search is active, Esc closes the search bar first.
                if let Some(editor) = self.editor.as_mut() {
                    if editor.search_active {
                        editor.search_active = false;
                        editor.search_query.clear();
                        self.status_message = String::from("search closed");
                        self.needs_redraw = true;
                        return Ok(vec![]);
                    }
                }
                if let Some(editor) = &self.editor {
                    if editor.is_dirty {
                        self.status_message = String::from(
                            "unsaved changes: Ctrl+S save, Ctrl+D discard, Esc cancel",
                        );
                    } else {
                        self.editor = None;
                        self.status_message = String::from("closed editor");
                    }
                } else {
                    self.status_message = String::from("no editor buffer is open");
                }
                self.needs_redraw = true;
            }
            Action::EditorOpenSearch => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.search_active = true;
                    editor.search_query.clear();
                    editor.search_match_idx = 0;
                    self.status_message = String::from("search: type to find");
                    self.needs_redraw = true;
                }
            }
            Action::EditorCloseSearch => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.search_active = false;
                    editor.search_query.clear();
                    self.status_message = String::from("search closed");
                    self.needs_redraw = true;
                }
            }
            Action::EditorSearchBackspace => {
                if let Some(editor) = self.editor.as_mut() {
                    if editor.search_active {
                        editor.search_query.pop();
                        if !editor.search_query.is_empty() {
                            editor.search_next();
                        }
                        self.needs_redraw = true;
                    }
                }
            }
            Action::EditorSearchNext => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.search_next();
                    self.needs_redraw = true;
                }
            }
            Action::EditorSearchPrev => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.search_prev();
                    self.needs_redraw = true;
                }
            }
            Action::DiscardEditorChanges => {
                if let Some(editor) = &self.editor {
                    let path = editor
                        .path
                        .as_ref()
                        .map(|value| value.display().to_string())
                        .unwrap_or_else(|| String::from("<unnamed>"));
                    self.editor = None;
                    self.status_message = format!("discarded unsaved changes for {path}");
                } else {
                    self.status_message = String::from("no editor buffer is open");
                }
                self.needs_redraw = true;
            }
            Action::EditorBackspace => {
                if let Some(editor) = self.editor.as_mut() {
                    // Route backspace through search when search is active.
                    if editor.search_active {
                        editor.search_query.pop();
                        if !editor.search_query.is_empty() {
                            editor.search_next();
                        }
                        self.needs_redraw = true;
                        return Ok(vec![]);
                    }
                    editor.backspace();
                    self.status_message = String::from("edited buffer");
                    self.needs_redraw = true;
                }
            }
            Action::EditorInsert(ch) => {
                if let Some(editor) = self.editor.as_mut() {
                    // Route character input through search when search is active.
                    if editor.search_active {
                        editor.search_query.push(*ch);
                        editor.search_next();
                        self.needs_redraw = true;
                        return Ok(vec![]);
                    }
                    editor.insert_char(*ch);
                    self.status_message = String::from("edited buffer");
                    self.needs_redraw = true;
                }
            }
            Action::EditorMoveDown => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.move_down();
                    self.needs_redraw = true;
                }
            }
            Action::EditorMoveLeft => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.move_left();
                    self.needs_redraw = true;
                }
            }
            Action::EditorMoveRight => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.move_right();
                    self.needs_redraw = true;
                }
            }
            Action::EditorMoveUp => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.move_up();
                    self.needs_redraw = true;
                }
            }
            Action::EditorNewline => {
                if let Some(editor) = self.editor.as_mut() {
                    // Enter jumps to next match when search is active.
                    if editor.search_active {
                        editor.search_next();
                        self.needs_redraw = true;
                        return Ok(vec![]);
                    }
                    editor.insert_newline();
                    self.status_message = String::from("edited buffer");
                    self.needs_redraw = true;
                }
            }
            Action::OpenSelectedInEditor => {
                if let Some(entry) = self.active_pane().selected_entry() {
                    if entry.kind == EntryKind::File {
                        commands.push(Command::OpenEditor {
                            path: entry.path.clone(),
                        });
                        self.status_message = format!("opening {}", entry.path.display());
                    } else {
                        self.status_message =
                            String::from("only files can be opened in the editor");
                    }
                } else {
                    self.status_message = String::from("no file selected for editor");
                }
                self.needs_redraw = true;
            }
            Action::SaveEditor => {
                if let Some(editor) = &self.editor {
                    if editor.is_dirty {
                        commands.push(Command::SaveEditor);
                        self.status_message = String::from("saving editor buffer");
                    } else {
                        self.status_message = String::from("editor buffer is already saved");
                    }
                } else {
                    self.status_message = String::from("no editor buffer is open");
                }
                self.needs_redraw = true;
            }
            _ => {}
        }

        Ok(commands)
    }

    // =========================================================================
    // File Operation Prompts Reducer
    // =========================================================================

    fn reduce_file_op_prompts(&mut self, action: &Action) -> Result<Vec<Command>> {
        match action {
            Action::OpenCopyPrompt => {
                self.collision = None;
                self.dialog = None;
                self.active_menu = None;
                self.menu_selection = 0;
                if let Some(entry) = self.active_pane().selected_entry() {
                    let target_dir = self.inactive_pane().cwd.clone();
                    let suggested = target_dir.join(&entry.name);
                    self.prompt = Some(PromptState::with_value(
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
                self.needs_redraw = true;
            }
            Action::OpenDeletePrompt => {
                self.collision = None;
                self.dialog = None;
                self.active_menu = None;
                self.menu_selection = 0;
                if let Some((entry_path, entry_name)) = self
                    .active_pane()
                    .selected_entry()
                    .map(|entry| (entry.path.clone(), entry.name.clone()))
                {
                    self.prompt = Some(PromptState::with_value(
                        PromptKind::Delete,
                        "Delete",
                        self.active_pane().cwd.clone(),
                        Some(entry_path),
                        String::new(),
                    ));
                    self.status_message = format!("confirm delete for {entry_name}");
                } else {
                    self.status_message = String::from("no item selected to delete");
                }
                self.needs_redraw = true;
            }
            Action::OpenNewDirectoryPrompt => {
                self.collision = None;
                self.dialog = None;
                self.active_menu = None;
                self.menu_selection = 0;
                self.prompt = Some(PromptState::new(
                    PromptKind::NewDirectory,
                    "New Directory",
                    self.active_pane().cwd.clone(),
                ));
                self.status_message = String::from("enter directory name");
                self.needs_redraw = true;
            }
            Action::OpenNewFilePrompt => {
                self.collision = None;
                self.dialog = None;
                self.active_menu = None;
                self.menu_selection = 0;
                self.prompt = Some(PromptState::new(
                    PromptKind::NewFile,
                    "New File",
                    self.active_pane().cwd.clone(),
                ));
                self.status_message = String::from("enter file name");
                self.needs_redraw = true;
            }
            Action::OpenRenamePrompt => {
                self.collision = None;
                self.dialog = None;
                self.active_menu = None;
                self.menu_selection = 0;
                if let Some(entry) = self.active_pane().selected_entry() {
                    self.prompt = Some(PromptState::with_value(
                        PromptKind::Rename,
                        "Rename",
                        self.active_pane().cwd.clone(),
                        Some(entry.path.clone()),
                        entry.name.clone(),
                    ));
                    self.status_message = String::from("edit the new name");
                } else {
                    self.status_message = String::from("no item selected to rename");
                }
                self.needs_redraw = true;
            }
            Action::OpenMovePrompt => {
                self.collision = None;
                self.dialog = None;
                self.active_menu = None;
                self.menu_selection = 0;
                if let Some(entry) = self.active_pane().selected_entry() {
                    let target_dir = self.inactive_pane().cwd.clone();
                    let suggested = target_dir.join(&entry.name);
                    self.prompt = Some(PromptState::with_value(
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
                self.needs_redraw = true;
            }
            _ => {}
        }

        Ok(Vec::new())
    }

    // =========================================================================
    // Prompt Input Reducer
    // =========================================================================

    fn reduce_prompt_input(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();

        match action {
            Action::PromptBackspace => {
                if let Some(prompt) = self.prompt.as_mut() {
                    if prompt.kind != PromptKind::Delete {
                        prompt.value.pop();
                    }
                    self.needs_redraw = true;
                }
            }
            Action::PromptCancel => {
                self.prompt = None;
                self.status_message = String::from("cancelled prompt");
                self.needs_redraw = true;
            }
            Action::PromptInput(ch) => {
                if let Some(prompt) = self.prompt.as_mut() {
                    if prompt.kind != PromptKind::Delete {
                        prompt.value.push(*ch);
                    }
                    self.needs_redraw = true;
                }
            }
            Action::PromptSubmit => {
                if let Some(prompt) = self.prompt.as_ref() {
                    if prompt.kind != PromptKind::Delete && prompt.value.trim().is_empty() {
                        self.status_message = String::from("name cannot be empty");
                    } else {
                        let kind = prompt.kind;
                        let value = prompt.value.trim().to_string();
                        let target_path = resolve_prompt_target(prompt, &value);
                        let refresh = self.refresh_targets_for_prompt(kind, &target_path);
                        let operation =
                            match kind {
                                PromptKind::Copy => {
                                    prompt
                                        .source_path
                                        .as_ref()
                                        .map(|source| FileOperation::Copy {
                                            source: source.clone(),
                                            destination: target_path.clone(),
                                        })
                                }
                                PromptKind::Delete => prompt
                                    .source_path
                                    .as_ref()
                                    .map(|path| FileOperation::Delete { path: path.clone() }),
                                PromptKind::Move => {
                                    prompt
                                        .source_path
                                        .as_ref()
                                        .map(|source| FileOperation::Move {
                                            source: source.clone(),
                                            destination: target_path.clone(),
                                        })
                                }
                                PromptKind::NewDirectory => Some(FileOperation::CreateDirectory {
                                    path: target_path.clone(),
                                }),
                                PromptKind::NewFile => Some(FileOperation::CreateFile {
                                    path: target_path.clone(),
                                }),
                                PromptKind::Rename => prompt.source_path.as_ref().map(|source| {
                                    FileOperation::Rename {
                                        source: source.clone(),
                                        destination: target_path.clone(),
                                    }
                                }),
                            };

                        if let Some(operation) = operation {
                            commands.push(Command::RunFileOperation {
                                operation,
                                refresh,
                                collision: CollisionPolicy::Fail,
                            });
                            self.status_message = match kind {
                                PromptKind::Copy => String::from("copying item"),
                                PromptKind::Delete => String::from("deleting item"),
                                PromptKind::Move => String::from("moving item"),
                                PromptKind::NewDirectory => String::from("creating directory"),
                                PromptKind::NewFile => String::from("creating file"),
                                PromptKind::Rename => String::from("renaming item"),
                            };
                        } else {
                            self.status_message = String::from("missing source for operation");
                        }
                        self.prompt = None;
                    }
                    self.needs_redraw = true;
                }
            }
            _ => {}
        }

        Ok(commands)
    }

    // =========================================================================
    // View/Settings Reducer
    // =========================================================================

    fn reduce_view(&mut self, action: &Action) -> Result<Vec<Command>> {
        match action {
            Action::SetPaneLayout(layout) => {
                self.pane_layout = *layout;
                self.config.pane_layout = *layout;
                self.status_message = match layout {
                    PaneLayout::SideBySide => String::from("layout set to side-by-side"),
                    PaneLayout::Stacked => String::from("layout set to stacked"),
                };
                let _ = self.config.save(Path::new(&self.config_path));
                self.needs_redraw = true;
            }
            Action::SetTheme(preset) => {
                self.theme = ThemePalette::from_preset(*preset);
                self.config.theme.preset = preset.as_str().to_string();
                self.status_message = format!("theme set to {}", preset.as_str());
                let _ = self.config.save(Path::new(&self.config_path));
                self.needs_redraw = true;
            }
            Action::ToggleHiddenFiles => {
                let new_value = !self.active_pane().show_hidden;
                self.active_pane_mut().set_show_hidden(new_value)?;
                self.status_message = if new_value {
                    String::from("showing hidden files")
                } else {
                    String::from("hiding hidden files")
                };
                self.needs_redraw = true;
            }
            Action::Quit => {
                if self.editor.as_ref().is_some_and(|editor| editor.is_dirty) {
                    self.status_message =
                        String::from("unsaved changes: Ctrl+S save, Ctrl+D discard, Esc cancel");
                    self.needs_redraw = true;
                } else {
                    self.should_quit = true;
                }
            }
            Action::Resize { width, height } => {
                self.last_size = Some((*width, *height));
                self.status_message = format!("resized to {width}x{height}");
                self.needs_redraw = true;
            }
            _ => {}
        }

        Ok(Vec::new())
    }

    // =========================================================================
    // Preview Reducer
    // =========================================================================

    fn reduce_preview(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();

        match action {
            Action::ClearPreview => {
                self.preview = None;
                self.preview_scroll = 0;
                self.needs_redraw = true;
            }
            Action::TogglePreviewPanel => {
                self.preview_panel_open = !self.preview_panel_open;
                self.config.preview_panel_open = self.preview_panel_open;
                let _ = self.config.save(Path::new(&self.config_path));
                self.needs_redraw = true;
            }
            Action::PreviewFile { path } => {
                self.preview_scroll = 0;
                commands.push(Command::PreviewFile { path: path.clone() });
            }
            Action::FocusPreviewPanel => {
                if self.preview_panel_open && self.editor.is_none() {
                    if self.focus == PaneFocus::Preview {
                        self.focus = PaneFocus::Left;
                    } else {
                        self.focus = PaneFocus::Preview;
                    }
                    self.needs_redraw = true;
                }
            }
            Action::ScrollPreviewDown => {
                self.preview_scroll = self.preview_scroll.saturating_add(1);
                self.needs_redraw = true;
            }
            Action::ScrollPreviewUp => {
                self.preview_scroll = self.preview_scroll.saturating_sub(1);
                self.needs_redraw = true;
            }
            Action::ScrollPreviewPageDown => {
                self.preview_scroll = self.preview_scroll.saturating_add(20);
                self.needs_redraw = true;
            }
            Action::ScrollPreviewPageUp => {
                self.preview_scroll = self.preview_scroll.saturating_sub(20);
                self.needs_redraw = true;
            }
            // After navigation actions, request a preview for the newly selected file.
            Action::MoveSelectionDown
            | Action::MoveSelectionUp
            | Action::FocusNextPane
            | Action::EnterSelection => {
                if let Some(entry) = self.active_pane().selected_entry() {
                    if entry.kind == EntryKind::File {
                        commands.push(Command::PreviewFile {
                            path: entry.path.clone(),
                        });
                    } else {
                        self.preview = None;
                        self.needs_redraw = true;
                    }
                } else {
                    self.preview = None;
                    self.needs_redraw = true;
                }
            }
            _ => {}
        }

        Ok(commands)
    }

    // =========================================================================
    // Job Result Handler
    // =========================================================================

    pub fn apply_job_result(&mut self, result: JobResult) {
        self.reduce_job_result(result);
    }

    fn reduce_job_result(&mut self, result: JobResult) {
        match result {
            JobResult::DirectoryScanned {
                pane,
                path,
                entries,
                elapsed_ms,
            } => {
                let target = self.pane_mut(pane);
                target.cwd = path.clone();
                target.set_entries(entries);
                self.status_message = format!("refreshed {} in {elapsed_ms} ms", path.display());
                self.last_scan_time_ms = Some(elapsed_ms);
                self.needs_redraw = true;
            }
            JobResult::FileOperationCompleted {
                message,
                refreshed,
                elapsed_ms,
            } => {
                self.collision = None;
                self.file_operation_status = None;
                for pane in refreshed {
                    let target = self.pane_mut(pane.pane);
                    target.cwd = pane.path;
                    target.set_entries(pane.entries);
                }
                self.status_message = format!("{message} in {elapsed_ms} ms");
                self.last_scan_time_ms = Some(elapsed_ms);
                self.needs_redraw = true;
            }
            JobResult::FileOperationCollision {
                operation,
                refresh,
                path,
                elapsed_ms,
            } => {
                self.file_operation_status = None;
                self.collision = Some(CollisionState {
                    operation,
                    refresh,
                    path: path.clone(),
                });
                self.status_message = format!(
                    "destination exists after {elapsed_ms} ms: {}",
                    path.display()
                );
                self.last_scan_time_ms = Some(elapsed_ms);
                self.needs_redraw = true;
            }
            JobResult::FileOperationProgress { status } => {
                self.file_operation_status = Some(status);
                self.needs_redraw = true;
            }
            JobResult::JobFailed {
                pane: _,
                path,
                message,
                elapsed_ms,
            } => {
                self.file_operation_status = None;
                self.status_message = format!(
                    "job failed for {} after {elapsed_ms} ms: {message}",
                    path.display()
                );
                self.last_scan_time_ms = Some(elapsed_ms);
                self.needs_redraw = true;
            }
            JobResult::PreviewLoaded { path, content } => {
                self.preview = Some((path, content));
                self.preview_scroll = 0;
                self.needs_redraw = true;
            }
        }
    }

    // =========================================================================
    // Public Accessors
    // =========================================================================

    pub fn left_pane(&self) -> &PaneState {
        &self.left
    }

    pub fn preview(&self) -> Option<&(PathBuf, PreviewContent)> {
        self.preview.as_ref()
    }

    pub fn is_preview_panel_open(&self) -> bool {
        self.preview_panel_open
    }

    pub fn active_pane_title(&self) -> &str {
        self.active_pane()
            .selected_entry()
            .map(|e| e.name.as_str())
            .unwrap_or("")
    }

    pub fn active_menu(&self) -> Option<MenuId> {
        self.active_menu
    }

    pub fn theme(&self) -> &ResolvedTheme {
        &self.theme
    }

    pub fn icon_mode(&self) -> IconMode {
        self.icon_mode
    }

    pub fn pane_layout(&self) -> PaneLayout {
        self.pane_layout
    }

    pub fn right_pane(&self) -> &PaneState {
        &self.right
    }

    pub fn editor(&self) -> Option<&EditorBuffer> {
        self.editor.as_ref()
    }

    pub fn prompt(&self) -> Option<&PromptState> {
        self.prompt.as_ref()
    }

    pub fn dialog(&self) -> Option<&DialogState> {
        self.dialog.as_ref()
    }

    pub fn settings(&self) -> Option<&SettingsState> {
        self.settings.as_ref()
    }

    pub fn settings_entries(&self) -> Vec<SettingsEntry> {
        vec![
            SettingsEntry {
                label: "Theme",
                value: self.theme.preset.clone(),
                hint: "Enter",
                field: SettingsField::Theme(match self.theme.preset.as_str() {
                    "sandbar" => ThemePreset::Sandbar,
                    "oxide" => ThemePreset::Oxide,
                    _ => ThemePreset::Fjord,
                }),
            },
            SettingsEntry {
                label: "Icon mode",
                value: match self.icon_mode {
                    IconMode::Unicode => String::from("unicode"),
                    IconMode::Ascii => String::from("ascii"),
                },
                hint: "Space",
                field: SettingsField::IconMode(self.icon_mode),
            },
            SettingsEntry {
                label: "Pane layout",
                value: match self.pane_layout {
                    PaneLayout::SideBySide => String::from("side by side"),
                    PaneLayout::Stacked => String::from("stacked"),
                },
                hint: "Enter",
                field: SettingsField::PaneLayout(self.pane_layout),
            },
            SettingsEntry {
                label: "Preview panel",
                value: if self.preview_panel_open {
                    String::from("enabled")
                } else {
                    String::from("disabled")
                },
                hint: "Space",
                field: SettingsField::PreviewPanel,
            },
            SettingsEntry {
                label: "Preview on selection",
                value: if self.preview_on_selection {
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

    pub fn is_settings_open(&self) -> bool {
        self.settings.is_some()
    }

    #[allow(dead_code)]
    fn reduce_settings(&mut self, action: &Action) -> Result<Vec<Command>> {
        match action {
            Action::OpenSettingsPanel => {
                self.command_palette = None;
                self.active_menu = None;
                self.dialog = None;
                self.collision = None;
                self.settings = Some(SettingsState::new());
                self.status_message = String::from("opened settings");
                self.needs_redraw = true;
            }
            Action::CloseSettingsPanel => {
                self.settings = None;
                self.status_message = String::from("closed settings");
                self.needs_redraw = true;
            }
            Action::SettingsMoveDown => {
                let max_index = self.settings_entries().len().saturating_sub(1);
                if let Some(settings) = self.settings.as_mut() {
                    settings.selection = (settings.selection + 1).min(max_index);
                    self.needs_redraw = true;
                }
            }
            Action::SettingsMoveUp => {
                if let Some(settings) = self.settings.as_mut() {
                    settings.selection = settings.selection.saturating_sub(1);
                    self.needs_redraw = true;
                }
            }
            Action::SettingsToggleCurrent => {
                if let Some(settings) = self.settings.as_ref() {
                    if let Some(entry) = self.settings_entries().get(settings.selection).cloned() {
                        self.apply_settings_entry(entry);
                    }
                }
            }
            _ => {}
        }

        Ok(Vec::new())
    }

    #[allow(dead_code)]
    fn apply_settings_entry(&mut self, entry: SettingsEntry) {
        match entry.field {
            SettingsField::Theme(current) => {
                let next = match current {
                    ThemePreset::Fjord => ThemePreset::Sandbar,
                    ThemePreset::Sandbar => ThemePreset::Oxide,
                    ThemePreset::Oxide => ThemePreset::Fjord,
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
                };
                self.icon_mode = next;
                self.config.icon_mode = next;
                self.status_message = match next {
                    IconMode::Unicode => String::from("icons set to unicode"),
                    IconMode::Ascii => String::from("icons set to ASCII"),
                };
                let _ = self.config.save(Path::new(&self.config_path));
            }
            SettingsField::PaneLayout(current) => {
                let next = match current {
                    PaneLayout::SideBySide => PaneLayout::Stacked,
                    PaneLayout::Stacked => PaneLayout::SideBySide,
                };
                self.pane_layout = next;
                self.config.pane_layout = next;
                self.status_message = match next {
                    PaneLayout::SideBySide => String::from("layout set to side-by-side"),
                    PaneLayout::Stacked => String::from("layout set to stacked"),
                };
                let _ = self.config.save(Path::new(&self.config_path));
            }
            SettingsField::PreviewPanel => {
                self.preview_panel_open = !self.preview_panel_open;
                self.config.preview_panel_open = self.preview_panel_open;
                self.status_message = if self.preview_panel_open {
                    String::from("preview panel enabled")
                } else {
                    String::from("preview panel disabled")
                };
                let _ = self.config.save(Path::new(&self.config_path));
            }
            SettingsField::PreviewOnSelection => {
                self.preview_on_selection = !self.preview_on_selection;
                self.config.preview_on_selection = self.preview_on_selection;
                self.status_message = if self.preview_on_selection {
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

        self.needs_redraw = true;
    }

    pub fn collision(&self) -> Option<&CollisionState> {
        self.collision.as_ref()
    }

    pub fn editor_mut(&mut self) -> Option<&mut EditorBuffer> {
        self.editor.as_mut()
    }

    pub fn focus(&self) -> PaneId {
        self.focused_pane_id()
    }

    pub fn is_editor_focused(&self) -> bool {
        self.editor.is_some()
    }

    pub fn preview_scroll(&self) -> usize {
        self.preview_scroll
    }

    pub fn is_preview_focused(&self) -> bool {
        self.focus == PaneFocus::Preview
    }

    pub fn is_menu_open(&self) -> bool {
        self.active_menu.is_some()
    }

    pub fn is_prompt_open(&self) -> bool {
        self.prompt.is_some()
    }

    pub fn is_dialog_open(&self) -> bool {
        self.dialog.is_some()
    }

    pub fn is_collision_open(&self) -> bool {
        self.collision.is_some()
    }

    pub fn is_palette_open(&self) -> bool {
        self.command_palette.is_some()
    }

    pub fn palette(&self) -> Option<&PaletteState> {
        self.command_palette.as_ref()
    }

    pub fn menu_items(&self) -> Vec<MenuItem> {
        self.active_menu.map(menu_items_for).unwrap_or_default()
    }

    pub fn menu_selection(&self) -> usize {
        self.menu_selection
    }

    pub fn status_line(&self) -> String {
        let mark_count = self.active_pane().marked_count();
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
                    .and_then(|value| value.to_str())
                    .unwrap_or(".");
                format!(
                    " | {}:{}/{} {}",
                    status.operation, status.completed, status.total, current
                )
            })
            .unwrap_or_default();
        format!(
            "{} | {} | {} | up:{}ms {}{}{} | d:{}",
            self.app_label,
            self.status_message,
            self.theme.preset,
            self.startup_time_ms,
            scan,
            marks,
            progress,
            self.redraw_count
        )
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    pub fn mark_drawn(&mut self) {
        self.redraw_count += 1;
        self.needs_redraw = false;
    }

    pub fn open_editor(&mut self, editor: EditorBuffer) {
        let path = editor
            .path
            .as_ref()
            .map(|value| value.display().to_string())
            .unwrap_or_else(|| String::from("<unnamed>"));
        if self.focus == PaneFocus::Preview {
            self.focus = PaneFocus::Left;
        }
        self.editor = Some(editor);
        self.status_message = format!("opened editor for {path}");
        self.needs_redraw = true;
    }

    pub fn mark_editor_saved(&mut self) {
        let message = self
            .editor
            .as_ref()
            .and_then(|editor| editor.path.as_ref())
            .map(|path| format!("saved editor buffer {}", path.display()))
            .unwrap_or_else(|| String::from("saved editor buffer"));
        self.status_message = message;
        self.needs_redraw = true;
    }

    pub fn set_error_status(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
        self.needs_redraw = true;
    }

    // =========================================================================
    // Private Helpers
    // =========================================================================

    fn active_pane(&self) -> &PaneState {
        match self.focus {
            PaneFocus::Left | PaneFocus::Preview => &self.left,
            PaneFocus::Right => &self.right,
        }
    }

    fn active_pane_mut(&mut self) -> &mut PaneState {
        match self.focus {
            PaneFocus::Left | PaneFocus::Preview => &mut self.left,
            PaneFocus::Right => &mut self.right,
        }
    }

    fn inactive_pane(&self) -> &PaneState {
        match self.focus {
            PaneFocus::Left | PaneFocus::Preview => &self.right,
            PaneFocus::Right => &self.left,
        }
    }

    fn refresh_targets_for_prompt(
        &self,
        kind: PromptKind,
        target_path: &Path,
    ) -> Vec<RefreshTarget> {
        let mut refresh = vec![RefreshTarget {
            pane: self.focused_pane_id(),
            path: self.active_pane().cwd.clone(),
        }];

        let target_dir = match kind {
            PromptKind::Copy | PromptKind::Move => target_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| self.inactive_pane().cwd.clone()),
            _ => self.active_pane().cwd.clone(),
        };

        if target_dir != self.active_pane().cwd && target_dir == self.inactive_pane().cwd {
            refresh.push(RefreshTarget {
                pane: match self.focus {
                    PaneFocus::Left | PaneFocus::Preview => PaneId::Right,
                    PaneFocus::Right => PaneId::Left,
                },
                path: target_dir,
            });
        }

        refresh
    }

    fn pane_mut(&mut self, pane: PaneId) -> &mut PaneState {
        match pane {
            PaneId::Left => &mut self.left,
            PaneId::Right => &mut self.right,
        }
    }

    fn focused_pane_id(&self) -> PaneId {
        match self.focus {
            PaneFocus::Left | PaneFocus::Preview => PaneId::Left,
            PaneFocus::Right => PaneId::Right,
        }
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
    use crate::pane::{PaneId, PaneState, SortMode};

    use super::{
        resolve_prompt_target, AppState, CollisionState, PaneFocus, PaneLayout, PromptKind,
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
            history_back: Vec::new(),
            history_forward: Vec::new(),
        }
    }

    fn temp_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("zeta-state-test-{unique}"))
    }

    fn test_state() -> AppState {
        AppState {
            left: pane_with_file("./note.txt"),
            right: PaneState {
                title: String::from("right"),
                cwd: PathBuf::from("."),
                entries: Vec::new(),
                selection: 0,
                scroll_offset: 0,
                show_hidden: false,
                sort_mode: SortMode::Name,
                marked: std::collections::BTreeSet::new(),
                history_back: Vec::new(),
                history_forward: Vec::new(),
            },
            focus: PaneFocus::Left,
            pane_layout: PaneLayout::SideBySide,
            app_label: String::from("Zeta"),
            config_path: String::from("/tmp/zeta/config.toml"),
            icon_mode: crate::config::IconMode::Unicode,
            theme: ResolvedTheme {
                palette: ThemePalette::resolve(&crate::config::ThemeConfig::default()).palette,
                preset: String::from("fjord"),
                warning: None,
            },
            active_menu: None,
            editor: None,
            menu_selection: 0,
            prompt: None,
            dialog: None,
            collision: None,
            preview: None,
            preview_panel_open: false,
            preview_on_selection: true,
            preview_scroll: 0,
            settings: None,
            status_message: String::from("ready"),
            last_size: None,
            redraw_count: 0,
            startup_time_ms: 0,
            last_scan_time_ms: None,
            file_operation_status: None,
            needs_redraw: false,
            should_quit: false,
            command_palette: None,
            config: crate::config::AppConfig::default(),
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
        let mut editor = EditorBuffer {
            path: Some(PathBuf::from("./note.txt")),
            ..EditorBuffer::default()
        };
        editor.insert(0, "hello");
        state.editor = Some(editor);

        let commands = state
            .apply(Action::SaveEditor)
            .expect("action should succeed");

        assert_eq!(commands, vec![Command::SaveEditor]);
    }

    #[test]
    fn close_editor_is_guarded_when_dirty() {
        let mut state = test_state();
        let mut editor = EditorBuffer {
            path: Some(PathBuf::from("./note.txt")),
            ..EditorBuffer::default()
        };
        editor.insert_char('x');
        state.editor = Some(editor);

        let commands = state
            .apply(Action::CloseEditor)
            .expect("action should succeed");

        assert!(commands.is_empty());
        assert!(state.editor.is_some());
    }

    #[test]
    fn discard_editor_changes_closes_dirty_buffer() {
        let mut state = test_state();
        let mut editor = EditorBuffer {
            path: Some(PathBuf::from("./note.txt")),
            ..EditorBuffer::default()
        };
        editor.insert_char('x');
        state.editor = Some(editor);

        let commands = state
            .apply(Action::DiscardEditorChanges)
            .expect("action should succeed");

        assert!(commands.is_empty());
        assert!(state.editor.is_none());
    }

    #[test]
    fn enter_selection_enqueues_directory_scan() {
        let mut state = test_state();
        state.left.entries[0].kind = EntryKind::Directory;

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
        state.left.cwd = PathBuf::from("/tmp/example");

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
        state.active_menu = Some(MenuId::Navigate);
        state.menu_selection = 1;

        let commands = state
            .apply(Action::MenuActivate)
            .expect("action should succeed");

        assert_eq!(
            commands,
            vec![Command::ScanPane {
                pane: PaneId::Left,
                path: PathBuf::new(),
            }]
        );
    }

    #[test]
    fn toggle_hidden_files_flips_active_pane_flag() {
        let mut state = test_state();

        state
            .apply(Action::ToggleHiddenFiles)
            .expect("toggle hidden should succeed");

        assert!(state.left.show_hidden);
    }

    #[test]
    fn open_new_file_prompt_sets_prompt_state() {
        let mut state = test_state();

        state
            .apply(Action::OpenNewFilePrompt)
            .expect("prompt should open");

        assert!(state.prompt.is_some());
    }

    #[test]
    fn open_copy_prompt_prefills_inactive_pane_destination() {
        let mut state = test_state();
        state.right.cwd = PathBuf::from("/tmp/target");

        state
            .apply(Action::OpenCopyPrompt)
            .expect("copy prompt should open");

        let prompt = state.prompt.as_ref().expect("prompt should exist");
        assert_eq!(prompt.title, "Copy");
        assert_eq!(prompt.value, "/tmp/target/note.txt");
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
        state.left.marked.insert(PathBuf::from("./note.txt"));

        let status = state.status_line();

        assert!(status.contains("marks:1"));
    }

    #[test]
    fn toggle_mark_action_updates_active_pane_marks() {
        let mut state = test_state();

        state.apply(Action::ToggleMark).expect("toggle should work");

        assert_eq!(state.left.marked_count(), 1);
    }

    #[test]
    fn clear_marks_action_clears_active_pane_marks() {
        let mut state = test_state();
        state.left.marked.insert(PathBuf::from("./note.txt"));

        state.apply(Action::ClearMarks).expect("clear should work");

        assert_eq!(state.left.marked_count(), 0);
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
        state.right.cwd = PathBuf::from("/tmp/target");

        state
            .apply(Action::OpenMovePrompt)
            .expect("move prompt should open");

        let prompt = state.prompt.as_ref().expect("prompt should exist");
        assert_eq!(prompt.title, "Move");
        assert_eq!(prompt.value, "/tmp/target/note.txt");
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
        state.right.cwd = PathBuf::from("/tmp/target");
        state.prompt = Some(PromptState::with_value(
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
        assert!(state.prompt.is_none());
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
        state.collision = Some(CollisionState {
            operation: FileOperation::Copy {
                source: PathBuf::from("./note.txt"),
                destination: PathBuf::from("/tmp/target/note.txt"),
            },
            refresh: vec![RefreshTarget {
                pane: PaneId::Right,
                path: PathBuf::from("/tmp/target"),
            }],
            path: PathBuf::from("/tmp/target/note.txt"),
        });

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
        state.collision = Some(CollisionState {
            operation: FileOperation::Rename {
                source: root.join("source.txt"),
                destination: root.join("note.txt"),
            },
            refresh: vec![],
            path: root.join("note.txt"),
        });

        state
            .apply(Action::CollisionRename)
            .expect("rename should reopen prompt");

        let prompt = state.prompt().expect("prompt should reopen");
        assert_eq!(prompt.kind, PromptKind::Rename);
        assert_eq!(prompt.value, root.join("note-1.txt").display().to_string());
        assert!(!state.is_collision_open());

        fs::remove_dir_all(root).expect("temp dir should be removed");
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
    fn open_delete_prompt_sets_confirmation_message() {
        let mut state = test_state();

        state
            .apply(Action::OpenDeletePrompt)
            .expect("delete prompt should open");

        assert!(state.prompt.is_some());
        assert_eq!(
            state.prompt.as_ref().map(|prompt| prompt.title),
            Some("Delete")
        );
    }

    #[test]
    fn open_help_dialog_sets_dialog_state() {
        let mut state = test_state();

        state
            .apply(Action::OpenHelpDialog)
            .expect("help dialog should open");

        assert_eq!(
            state.dialog.as_ref().map(|dialog| dialog.title),
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

        let dialog = state.dialog.as_ref().expect("dialog should exist");
        assert_eq!(dialog.title, "About Zeta");
        assert!(dialog.lines.iter().any(|line| line.contains("sandbar")));
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
