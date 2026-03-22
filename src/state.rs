use std::path::Path;
use std::time::Instant;

use anyhow::Result;

use crate::action::{Action, Command};
use crate::config::LoadedConfig;
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
    editor: Option<EditorBuffer>,
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

        let left = PaneState::load("Left", cwd.clone())?;
        let right = PaneState::load("Right", secondary)?;

        Ok(Self {
            left,
            right,
            focus: PaneFocus::Left,
            app_label: loaded_config.config.theme.status_bar_label,
            config_path: loaded_config.path.display().to_string(),
            editor: None,
            status_message: format!(
                "ready | config {} ({})",
                loaded_config.path.display(),
                loaded_config.source.label()
            ),
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
            Action::CloseEditor => {
                if let Some(editor) = &self.editor {
                    if editor.is_dirty {
                        self.status_message =
                            String::from("editor has unsaved changes; press Ctrl+S before closing");
                    } else {
                        self.editor = None;
                        self.status_message = String::from("closed editor");
                    }
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
            Action::FocusNextPane => {
                self.focus = match self.focus {
                    PaneFocus::Left => PaneFocus::Right,
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
            Action::Quit => {
                if self.editor.as_ref().is_some_and(|editor| editor.is_dirty) {
                    self.status_message = String::from(
                        "editor has unsaved changes; save or close it before quitting",
                    );
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

    pub fn right_pane(&self) -> &PaneState {
        &self.right
    }

    pub fn editor(&self) -> Option<&EditorBuffer> {
        self.editor.as_ref()
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

    pub fn status_line(&self) -> String {
        let scan_segment = match self.last_scan_time_ms {
            Some(scan_time) => format!("scan:{scan_time}ms"),
            None => String::from("scan:n/a"),
        };

        format!(
            "{} | {} | startup:{}ms | {} | draws:{} | Ctrl+Q quit | Enter open dir | Backspace parent | F4 editor | Ctrl+S save | Esc close | cfg:{}",
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
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::action::{Action, Command};
    use crate::editor::EditorBuffer;
    use crate::fs::{EntryInfo, EntryKind};
    use crate::pane::{PaneId, PaneState, SortMode};

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
            editor: None,
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
}
