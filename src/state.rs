use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;

use crate::action::{Action, Command, MenuId};
use crate::config::{LoadedConfig, ResolvedTheme, ThemePalette, ThemePreset};
use crate::editor::EditorBuffer;
use crate::fs;
use crate::fs::EntryKind;
use crate::jobs::JobResult;
use crate::pane::{PaneId, PaneState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneFocus {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub struct AppState {
    left: PaneState,
    right: PaneState,
    focus: PaneFocus,
    app_label: String,
    config_path: String,
    theme: ResolvedTheme,
    active_menu: Option<MenuId>,
    editor: Option<EditorBuffer>,
    menu_selection: usize,
    prompt: Option<PromptState>,
    dialog: Option<DialogState>,
    status_message: String,
    last_size: Option<(u16, u16)>,
    redraw_count: u64,
    startup_time_ms: u128,
    last_scan_time_ms: Option<u128>,
    needs_redraw: bool,
    should_quit: bool,
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

        let left = PaneState::load("Left", cwd.clone())?;
        let right = PaneState::load("Right", secondary)?;

        Ok(Self {
            left,
            right,
            focus: PaneFocus::Left,
            app_label: status_bar_label,
            config_path: loaded_config.path.display().to_string(),
            theme: resolved_theme.clone(),
            active_menu: None,
            editor: None,
            menu_selection: 0,
            prompt: None,
            dialog: None,
            status_message: resolved_theme.warning.unwrap_or_else(|| {
                format!(
                    "ready | config {} ({})",
                    loaded_config.path.display(),
                    loaded_config.source.label()
                )
            }),
            last_size: None,
            redraw_count: 0,
            startup_time_ms: started_at.elapsed().as_millis(),
            last_scan_time_ms: None,
            needs_redraw: true,
            should_quit: false,
        })
    }

    pub fn apply(&mut self, action: Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();

        match action {
            Action::CloseDialog => {
                self.dialog = None;
                self.status_message = String::from("closed dialog");
                self.needs_redraw = true;
            }
            Action::CloseMenu => {
                self.active_menu = None;
                self.menu_selection = 0;
                self.needs_redraw = true;
            }
            Action::CloseEditor => {
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
                    editor.backspace();
                    self.status_message = String::from("edited buffer");
                    self.needs_redraw = true;
                }
            }
            Action::EditorInsert(ch) => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.insert_char(ch);
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
                    editor.insert_newline();
                    self.status_message = String::from("edited buffer");
                    self.needs_redraw = true;
                }
            }
            Action::EnterSelection => {
                if self.active_pane().can_enter_selected() {
                    if let Some(path) = self.active_pane().selected_path() {
                        let pane = self.focused_pane_id();
                        self.status_message = format!("opening directory {}", path.display());
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
                    self.needs_redraw = true;
                    commands.push(Command::ScanPane { pane, path });
                } else {
                    self.status_message = String::from("already at filesystem root");
                    self.needs_redraw = true;
                }
            }
            Action::OpenMenu(menu) => {
                self.dialog = None;
                self.active_menu = Some(menu);
                self.menu_selection = 0;
                self.needs_redraw = true;
            }
            Action::OpenAboutDialog => {
                self.active_menu = None;
                self.menu_selection = 0;
                self.dialog = Some(DialogState::about(
                    self.theme.preset.clone(),
                    self.config_path.clone(),
                ));
                self.status_message = String::from("opened about");
                self.needs_redraw = true;
            }
            Action::OpenDeletePrompt => {
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
            Action::FocusNextPane => {
                self.focus = match self.focus {
                    PaneFocus::Left => PaneFocus::Right,
                    PaneFocus::Right => PaneFocus::Left,
                };
                self.needs_redraw = true;
            }
            Action::MenuActivate => {
                if let Some(menu) = self.active_menu {
                    if let Some(item) = self.menu_items_for(menu).get(self.menu_selection).copied()
                    {
                        self.active_menu = None;
                        self.menu_selection = 0;
                        commands.extend(self.apply(item.action)?);
                    }
                }
            }
            Action::MenuMnemonic(ch) => {
                if let Some(menu) = self.active_menu {
                    if let Some(item) = self
                        .menu_items_for(menu)
                        .into_iter()
                        .find(|item| item.mnemonic.eq_ignore_ascii_case(&ch))
                    {
                        self.active_menu = None;
                        self.menu_selection = 0;
                        commands.extend(self.apply(item.action)?);
                    }
                }
            }
            Action::MenuMoveDown => {
                if let Some(menu) = self.active_menu {
                    let len = self.menu_items_for(menu).len();
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
            Action::MoveSelectionDown => {
                self.active_pane_mut().move_selection_down();
                self.needs_redraw = true;
            }
            Action::MoveSelectionUp => {
                self.active_pane_mut().move_selection_up();
                self.needs_redraw = true;
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
            Action::OpenHelpDialog => {
                self.active_menu = None;
                self.menu_selection = 0;
                self.dialog = Some(DialogState::help());
                self.status_message = String::from("opened help");
                self.needs_redraw = true;
            }
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
                        prompt.value.push(ch);
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
                        let base_path = prompt.base_path.clone();
                        let value = prompt.value.trim().to_string();
                        let path = base_path.join(&value);
                        match kind {
                            PromptKind::NewDirectory => fs::create_directory(&path)?,
                            PromptKind::NewFile => fs::create_file(&path)?,
                            PromptKind::Rename => {
                                if let Some(source_path) = prompt.source_path.as_ref() {
                                    fs::rename_path(source_path, &path)?;
                                }
                            }
                            PromptKind::Delete => {
                                if let Some(source_path) = prompt.source_path.as_ref() {
                                    fs::delete_path(source_path)?;
                                }
                            }
                        }
                        let entries = fs::scan_directory(&base_path)?;
                        self.active_pane_mut().set_entries(entries);
                        self.status_message = match kind {
                            PromptKind::Rename => format!("renamed to {}", path.display()),
                            PromptKind::Delete => String::from("deleted item"),
                            _ => format!("created {}", path.display()),
                        };
                        self.prompt = None;
                    }
                    self.needs_redraw = true;
                }
            }
            Action::Refresh => {
                let pane = self.focused_pane_id();
                let path = self.active_pane().cwd.clone();
                self.status_message = format!("refreshing {}", path.display());
                self.needs_redraw = true;
                commands.push(Command::ScanPane { pane, path });
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
            Action::SetTheme(preset) => {
                self.theme = ThemePalette::from_preset(preset);
                self.status_message = format!("theme set to {}", preset.as_str());
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
                self.last_size = Some((width, height));
                self.status_message = format!("resized to {width}x{height}");
                self.needs_redraw = true;
            }
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
                let target = self.pane_mut(pane);
                target.cwd = path.clone();
                target.set_entries(entries);
                self.status_message = format!("refreshed {} in {elapsed_ms} ms", path.display());
                self.last_scan_time_ms = Some(elapsed_ms);
                self.needs_redraw = true;
            }
            JobResult::JobFailed {
                pane: _,
                path,
                message,
                elapsed_ms,
            } => {
                self.status_message = format!(
                    "refresh failed for {} after {elapsed_ms} ms: {message}",
                    path.display()
                );
                self.last_scan_time_ms = Some(elapsed_ms);
                self.needs_redraw = true;
            }
        }
    }

    pub fn left_pane(&self) -> &PaneState {
        &self.left
    }

    pub fn active_menu(&self) -> Option<MenuId> {
        self.active_menu
    }

    pub fn theme(&self) -> &ResolvedTheme {
        &self.theme
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

    pub fn editor_mut(&mut self) -> Option<&mut EditorBuffer> {
        self.editor.as_mut()
    }

    pub fn focus(&self) -> PaneId {
        self.focused_pane_id()
    }

    pub fn is_editor_focused(&self) -> bool {
        self.editor.is_some() && self.focused_pane_id() == PaneId::Right
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

    pub fn menu_items(&self) -> Vec<MenuItem> {
        self.active_menu
            .map(|menu| self.menu_items_for(menu))
            .unwrap_or_default()
    }

    pub fn menu_selection(&self) -> usize {
        self.menu_selection
    }

    pub fn status_line(&self) -> String {
        let scan_segment = match self.last_scan_time_ms {
            Some(scan_time) => format!("scan:{scan_time}ms"),
            None => String::from("scan:n/a"),
        };

        format!(
            "{} | {} | startup:{}ms | {} | draws:{} | cfg:{}",
            self.app_label,
            self.status_message,
            self.startup_time_ms,
            scan_segment,
            self.redraw_count,
            self.config_path,
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
        self.editor = Some(editor);
        self.focus = PaneFocus::Right;
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

    fn active_pane(&self) -> &PaneState {
        match self.focus {
            PaneFocus::Left => &self.left,
            PaneFocus::Right => &self.right,
        }
    }

    fn active_pane_mut(&mut self) -> &mut PaneState {
        match self.focus {
            PaneFocus::Left => &mut self.left,
            PaneFocus::Right => &mut self.right,
        }
    }

    fn pane_mut(&mut self, pane: PaneId) -> &mut PaneState {
        match pane {
            PaneId::Left => &mut self.left,
            PaneId::Right => &mut self.right,
        }
    }

    fn focused_pane_id(&self) -> PaneId {
        match self.focus {
            PaneFocus::Left => PaneId::Left,
            PaneFocus::Right => PaneId::Right,
        }
    }

    fn menu_items_for(&self, menu: MenuId) -> Vec<MenuItem> {
        match menu {
            MenuId::File => vec![
                MenuItem {
                    label: "Open in Editor",
                    shortcut: "F4",
                    mnemonic: 'o',
                    action: Action::OpenSelectedInEditor,
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MenuItem {
    pub label: &'static str,
    pub shortcut: &'static str,
    pub mnemonic: char,
    pub action: Action,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromptKind {
    Delete,
    NewDirectory,
    NewFile,
    Rename,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DialogState {
    pub title: &'static str,
    pub lines: Vec<String>,
}

impl DialogState {
    fn about(theme_name: String, config_path: String) -> Self {
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

    fn help() -> Self {
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
pub struct PromptState {
    pub kind: PromptKind,
    pub title: &'static str,
    pub base_path: PathBuf,
    pub source_path: Option<PathBuf>,
    pub value: String,
}

impl PromptState {
    fn new(kind: PromptKind, title: &'static str, base_path: PathBuf) -> Self {
        Self::with_value(kind, title, base_path, None, String::new())
    }

    fn with_value(
        kind: PromptKind,
        title: &'static str,
        base_path: PathBuf,
        source_path: Option<PathBuf>,
        value: String,
    ) -> Self {
        Self {
            kind,
            title,
            base_path,
            source_path,
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::action::{Action, Command, MenuId};
    use crate::editor::EditorBuffer;
    use crate::fs::{EntryInfo, EntryKind};
    use crate::pane::{PaneId, PaneState, SortMode};

    use crate::config::{ResolvedTheme, ThemePalette, ThemePreset};

    use super::{AppState, PaneFocus};

    fn pane_with_file(path: &str) -> PaneState {
        PaneState {
            title: String::from("left"),
            cwd: PathBuf::from("."),
            entries: vec![EntryInfo {
                name: String::from("note.txt"),
                path: PathBuf::from(path),
                kind: EntryKind::File,
            }],
            selection: 0,
            scroll_offset: 0,
            show_hidden: false,
            sort_mode: SortMode::Name,
        }
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
            },
            focus: PaneFocus::Left,
            app_label: String::from("Zeta"),
            config_path: String::from("/tmp/zeta/config.toml"),
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
            status_message: String::from("ready"),
            last_size: None,
            redraw_count: 0,
            startup_time_ms: 0,
            last_scan_time_ms: None,
            needs_redraw: false,
            should_quit: false,
        }
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
            Some("Help")
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
