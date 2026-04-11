use std::io::{self, Stdout};
use std::time::Duration;
use std::time::Instant;
use std::path::PathBuf;

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, TryRecvError};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::
    {disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::{Frame, Terminal};

use crate::action::{Action, Command};
use crate::config::{AppConfig, RuntimeKeymap};
use crate::event::AppEvent;
use crate::jobs::{self, EditorLoadRequest, FileOpRequest, FindRequest, GitStatusRequest, JobResult, PreviewRequest, ScanRequest, WatchRequest, WorkerChannels};
use crate::state::{AppState, FocusLayer, ModalKind};
use crate::ui;
use crate::ui::layout_cache::{rect_contains, LayoutCache};

type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

pub struct App {
    workers: WorkerChannels,
    job_results: Receiver<JobResult>,
    keymap: RuntimeKeymap,
    state: AppState,
    pub layout_cache: LayoutCache,
    last_pane_click: Option<(bool, usize, std::time::Instant)>, // (left_pane, row, time)
}

impl App {
    pub fn bootstrap() -> Result<Self> {
        let started_at = Instant::now();
        let loaded_config =
            AppConfig::load_default_location().context("failed to resolve application config")?;
        let keymap = loaded_config
            .config
            .compile_keymap()
            .context("failed to compile configured key bindings")?;
        let (workers, job_results) = jobs::spawn_workers();
        let state = AppState::bootstrap(loaded_config, started_at)
            .context("failed to bootstrap application state")?;
        let mut app = Self {
            workers,
            job_results,
            keymap,
            state,
            layout_cache: LayoutCache::default(),
            last_pane_click: None,
        };

        for command in app.state.initial_commands() {
            app.execute_command(command)?;
        }

        Ok(app)
    }

    pub fn run(&mut self) -> Result<()> {
        let mut terminal = TerminalSession::enter()?;

        while !self.state.should_quit() {
            let mut cache = LayoutCache::default();
            terminal.draw(|frame| {
                cache = ui::render(frame, &mut self.state);
            })?;
            self.layout_cache = cache;
            self.state.mark_drawn();
            self.process_next_event()?;
        }

        Ok(())
    }

    fn process_next_event(&mut self) -> Result<()> {
        let Some(app_event) = self.next_event()? else {
            if let Some(command) = self.state.preview_command_due() {
                self.execute_command(command)?;
            }
            return Ok(());
        };

        self.handle_event(app_event)?;

        Ok(())
    }

    fn next_event(&mut self) -> Result<Option<AppEvent>> {
        match self.job_results.try_recv() {
            Ok(result) => return Ok(Some(AppEvent::Job(result))),
            Err(TryRecvError::Disconnected) => {
                anyhow::bail!("background worker disconnected")
            }
            Err(TryRecvError::Empty) => {}
        }

        if !event::poll(Duration::from_millis(250)).context("failed to poll terminal events")? {
            return Ok(None);
        }

        match event::read().context("failed to read terminal event")? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                Ok(Some(AppEvent::Input(key_event)))
            }
            Event::Mouse(mouse_event) => Ok(Some(AppEvent::Mouse(mouse_event))),
        Event::Resize(width, height) => Ok(Some(AppEvent::Resize { width, height })),
            _ => Ok(None),
        }
    }

    fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Input(key_event) => {
                let focus = self.state.focus_layer();
                let is_preview_open = self.state.is_preview_panel_open();
                if let Some(action) =
                    route_key_event(key_event, &self.keymap, focus, is_preview_open)
                {
                    self.dispatch(action)?;
                }
            }
            AppEvent::Mouse(mouse_event) => {
                let focus = self.state.focus_layer();
                let editor_menu_mode = self.state.is_editor_fullscreen() && self.state.editor().is_some();
                if let Some(action) =
                    route_mouse_event(mouse_event, &self.layout_cache, focus, editor_menu_mode)
                {
                    // Intercept PaneClick to detect double-clicks.
                    let action = if let Action::PaneClick { left_pane, row } = action {
                        let now = std::time::Instant::now();
                        let double = self.last_pane_click
                            .is_some_and(|(lp, r, t)| lp == left_pane && r == row && now.duration_since(t).as_millis() < 400);
                        if double {
                            self.last_pane_click = None;
                            Action::PaneDoubleClick { left_pane, row }
                        } else {
                            self.last_pane_click = Some((left_pane, row, now));
                            Action::PaneClick { left_pane, row }
                        }
                    } else {
                        action
                    };
                    self.dispatch(action)?;
                }
            }
            AppEvent::Resize { width, height } => {
                self.dispatch(Action::Resize { width, height })?;
            }
            AppEvent::Job(result) => match result {
                JobResult::DirectoryChanged { path } => {
                    if self.state.left_pane().cwd == path {
                        self.execute_command(Command::ScanPane {
                            pane: crate::pane::PaneId::Left,
                            path: path.clone(),
                        })?;
                    }
                    if self.state.right_pane().cwd == path {
                        self.execute_command(Command::ScanPane {
                            pane: crate::pane::PaneId::Right,
                            path,
                        })?;
                    }
                }
                other => {
                    let refresh_watch = matches!(&other, JobResult::DirectoryScanned { .. });
                    self.state.apply_job_result(other);
                    if refresh_watch {
                        self.sync_watched_paths()?;
                    }
                }
            }
        }
        Ok(())
    }

    fn dispatch(&mut self, action: Action) -> Result<()> {
        for command in self.state.apply(action)? {
            self.execute_command(command)?;
        }

        Ok(())
    }

    fn sync_watched_paths(&mut self) -> Result<()> {
        let mut paths = vec![self.state.left_pane().cwd.clone()];
        let right = self.state.right_pane().cwd.clone();
        if right != paths[0] {
            paths.push(right);
        }
        self.workers
            .watch_tx
            .send(WatchRequest { paths })
            .context("failed to update watched directories")?;
        Ok(())
    }

    fn execute_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::OpenEditor { path } => {
                self.state.begin_open_editor(path.clone());
                self.workers
                    .editor_tx
                    .send(EditorLoadRequest { path })
                    .context("failed to queue background editor load job")?;
            }
            Command::PreviewFile { path } => {
                // If the active pane is in archive mode, include archive metadata so the
                // preview worker can extract the archived file contents.
                let mut archive = None;
                let mut inner = None;
                if self.state.panes.active_pane().in_archive() {
                    if let crate::pane::PaneMode::Archive { source, inner_path } = &self.state.panes.active_pane().mode {
                        archive = Some(source.clone());
                        if let Some(name) = path.file_name() {
                            if inner_path.as_os_str().is_empty() {
                                inner = Some(PathBuf::from(name));
                            } else {
                                inner = Some(inner_path.join(name));
                            }
                        }
                    }
                }
                self.workers
                    .preview_tx
                    .send(PreviewRequest {
                        path,
                        syntect_theme: self.state.theme().palette.syntect_theme.to_string(),
                        archive,
                        inner_path: inner,
                    })
                    .context("failed to queue background preview job")?;
            },

            Command::RunFileOperation {
                operation,
                refresh,
                collision,
            } => self
                .workers
                .file_op_tx
                .send(FileOpRequest { operation, backend: crate::jobs::BackendRef::Local, refresh, collision })
                .context("failed to queue background file operation")?,
            Command::ScanPane { pane, path } => {
                self.workers
                    .scan_tx
                    .send(ScanRequest { pane, path: path.clone() })
                    .context("failed to queue background scan job")?;
                // Fire a git status refresh alongside every directory scan.
                self.workers
                    .git_tx
                    .send(GitStatusRequest { pane, path })
                    .context("failed to queue git status job")?;
            }
            Command::FindFiles { pane, root, max_depth } => {
                self.workers
                    .find_tx
                    .send(FindRequest { pane, root, max_depth })
                    .context("failed to queue background file finder job")?;
            }
            Command::OpenArchive { path, inner } => {
                // Request archive listing for the active pane with provided inner path.
                let pane = self.state.panes.focused_pane_id();
                let req = jobs::ArchiveListRequest { pane, archive_path: path.clone(), inner_path: inner.clone() };
                self.workers
                    .archive_tx
                    .send(req)
                    .context("failed to queue archive listing job")?;
            }
            Command::OpenShell { path } => {
                // Drop out of TUI, spawn shell process, then re-enter.
                use std::process::Command as StdCommand;
                use crossterm::terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen, EnterAlternateScreen};
                use crossterm::execute;
                use std::io::{self};

                // Leave alternate screen, restore terminal, spawn shell
                disable_raw_mode().ok();
                let mut stdout = io::stdout();
                execute!(stdout, LeaveAlternateScreen).ok();

                // Pick a shell (Windows/cmd, others/sh)
                let shell = std::env::var("SHELL").unwrap_or_else(|_| {
                    if cfg!(windows) {
                        std::env::var("COMSPEC").unwrap_or_else(|_| String::from("cmd.exe"))
                    } else {
                        String::from("/bin/sh")
                    }
                });

                let _ = StdCommand::new(shell)
                    .current_dir(path)
                    .status();

                // Wait for shell to exit, then re-enter alternate screen and raw mode
                execute!(stdout, EnterAlternateScreen).ok();
                enable_raw_mode().ok();
            }
            Command::ConnectSSH { address, auth_method: _, credential: _, pane: _ } => {
                // TODO: Implement SSH connection logic
                self.state.set_error_status(format!("SSH connect to {} - not yet implemented", address));
            }
            Command::DisconnectSSH { pane } => {
                // Switch back to local mode and scan home directory
                self.state.panes.pane_mut(pane).mode = crate::pane::PaneMode::Real;
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                self.execute_command(Command::ScanPane {
                    pane,
                    path: std::path::PathBuf::from(home),
                })?;

            }
            Command::DispatchAction(action) => {
                self.dispatch(action)?;
            }
            Command::SaveEditor => {
                if let Some(editor) = self.state.editor_mut() {
                    match editor.save() {
                        Ok(()) => self.state.mark_editor_saved(),
                        Err(error) => self
                            .state
                            .set_error_status(format!("failed to save editor buffer: {error}")),
                    }
                } else {
                    self.state.set_error_status("no editor buffer is open");
                }
            }
        }

        Ok(())
    }
}

fn route_key_event(
    key_event: crossterm::event::KeyEvent,
    keymap: &RuntimeKeymap,
    focus: FocusLayer,
    is_preview_open: bool,
) -> Option<Action> {
    use crossterm::event::{KeyCode, KeyModifiers};

    let alt_f3 = key_event.code == KeyCode::F(3)
        && key_event.modifiers == KeyModifiers::ALT;

    match focus {
        FocusLayer::Modal(ModalKind::Palette) => Action::from_palette_key_event(key_event),
        FocusLayer::Modal(ModalKind::Collision) => Action::from_collision_key_event(key_event),
        FocusLayer::Modal(ModalKind::Prompt) => Action::from_prompt_key_event(key_event),
        FocusLayer::Modal(ModalKind::Dialog) => Action::from_dialog_key_event(key_event),
        FocusLayer::Modal(ModalKind::Menu) => Action::from_menu_key_event(key_event),
        FocusLayer::Modal(ModalKind::Settings) => Action::from_settings_key_event(key_event),
            FocusLayer::Modal(ModalKind::Bookmarks) => Action::from_bookmarks_key_event(key_event),
            FocusLayer::Modal(ModalKind::FileFinder) => Action::from_file_finder_key_event(key_event),
            FocusLayer::Modal(ModalKind::SshConnect) => Action::from_ssh_connect_key_event(key_event),
            FocusLayer::PaneFilter => Action::from_pane_filter_key_event(key_event),
        FocusLayer::Preview => Action::from_preview_key_event(key_event),
        FocusLayer::MarkdownPreview => {
            if is_preview_open && alt_f3 {
                return Some(Action::FocusPreviewPanel);
            }
            Action::from_markdown_preview_key_event(key_event)
                .or_else(|| Action::from_editor_key_event(key_event))
                .or_else(|| Action::from_pane_key_event(key_event, keymap))
        }
        FocusLayer::Editor => {
            if is_preview_open && alt_f3 {
                return Some(Action::FocusPreviewPanel);
            }
            Action::from_editor_key_event(key_event)
                .or_else(|| Action::from_pane_key_event(key_event, keymap))
        }
        FocusLayer::Pane => {
            if is_preview_open && alt_f3 {
                return Some(Action::FocusPreviewPanel);
            }
            Action::from_pane_key_event(key_event, keymap)
        }
    }
}

/// Translate a raw mouse event into an `Action` using the last-rendered
/// `LayoutCache` for hit-testing. Returns `None` for unhandled events.
fn route_mouse_event(
    event: crossterm::event::MouseEvent,
    cache: &LayoutCache,
    focus: FocusLayer,
    editor_menu_mode: bool,
) -> Option<Action> {
    use crossterm::event::{MouseButton, MouseEventKind};

    let col = event.column;
    let row = event.row;

    match event.kind {
        // -------------------------------------------------------------------
        // Scroll wheel
        // -------------------------------------------------------------------
        MouseEventKind::ScrollUp => {
            if focus == FocusLayer::Preview
                || cache
                    .file_preview_panel
                    .is_some_and(|r| rect_contains(r, col, row))
            {
                return Some(Action::ScrollPreviewUp);
            }
            if focus == FocusLayer::MarkdownPreview
                || cache
                    .markdown_preview_panel
                    .is_some_and(|r| rect_contains(r, col, row))
            {
                return Some(Action::ScrollMarkdownPreviewUp);
            }
            if focus == FocusLayer::Editor
                || cache.editor_panel.is_some_and(|r| rect_contains(r, col, row))
            {
                return Some(Action::EditorMoveUp);
            }
            if rect_contains(cache.left_pane, col, row)
                || rect_contains(cache.right_pane, col, row)
            {
                return Some(Action::MoveSelectionUp);
            }
            None
        }
        MouseEventKind::ScrollDown => {
            if focus == FocusLayer::Preview
                || cache
                    .file_preview_panel
                    .is_some_and(|r| rect_contains(r, col, row))
            {
                return Some(Action::ScrollPreviewDown);
            }
            if focus == FocusLayer::MarkdownPreview
                || cache
                    .markdown_preview_panel
                    .is_some_and(|r| rect_contains(r, col, row))
            {
                return Some(Action::ScrollMarkdownPreviewDown);
            }
            if focus == FocusLayer::Editor
                || cache.editor_panel.is_some_and(|r| rect_contains(r, col, row))
            {
                return Some(Action::EditorMoveDown);
            }
            if rect_contains(cache.left_pane, col, row)
                || rect_contains(cache.right_pane, col, row)
            {
                return Some(Action::MoveSelectionDown);
            }
            None
        }

        // -------------------------------------------------------------------
        // Left click
        // -------------------------------------------------------------------
        MouseEventKind::Down(MouseButton::Left) => {
            // Menu open: allow menu bar clicks (switch menus) and popup item clicks.
            if matches!(focus, FocusLayer::Modal(ModalKind::Menu)) {
                if rect_contains(cache.menu_bar, col, row) {
                    return route_menu_bar_click(col, cache.menu_bar.x, editor_menu_mode);
                }
                if let Some(popup) = cache.menu_popup {
                    if rect_contains(popup, col, row) {
                        // Use same menu_bar anchor as hover for consistency.
                        let popup_top = cache.menu_bar.y + cache.menu_bar.height;
                        let item_row = row.saturating_sub(popup_top + 1) as usize;
                        return Some(Action::MenuClickItem(item_row));
                    }
                }
                // Click outside menu — close it.
                return Some(Action::CloseMenu);
            }

            if matches!(focus, FocusLayer::Modal(ModalKind::Dialog)) {
                return Some(Action::CloseDialog);
            }

            // All other modal states absorb left clicks.
            if matches!(focus, FocusLayer::Modal(_)) {
                return None;
            }

            // Click on menu bar item.
            if rect_contains(cache.menu_bar, col, row) {
                return route_menu_bar_click(col, cache.menu_bar.x, editor_menu_mode);
            }

            if let Some(md_rect) = cache.markdown_preview_panel {
                if rect_contains(md_rect, col, row) {
                    if focus != FocusLayer::MarkdownPreview {
                        return Some(Action::FocusMarkdownPreview);
                    }
                    return None;
                }
            }

            if let Some(editor_rect) = cache.editor_panel {
                if rect_contains(editor_rect, col, row) {
                    if focus == FocusLayer::MarkdownPreview {
                        return Some(Action::FocusMarkdownPreview);
                    }
                    return None;
                }
            }

            if let Some(preview_rect) = cache.file_preview_panel {
                if rect_contains(preview_rect, col, row) {
                    if focus != FocusLayer::Preview {
                        return Some(Action::FocusPreviewPanel);
                    }
                    return None;
                }
            }

            // Click on left or right pane.
            if rect_contains(cache.left_pane, col, row)
                || rect_contains(cache.right_pane, col, row)
            {
                let clicked_left = rect_contains(cache.left_pane, col, row);

                // If focus is on a tool (editor/preview), return to pane layer first.
                if focus == FocusLayer::Editor
                    || focus == FocusLayer::Preview
                    || focus == FocusLayer::MarkdownPreview
                {
                    return Some(Action::CycleFocus);
                }

                // Calculate which entry row was clicked (subtract 1 for top border).
                let pane_rect = if clicked_left { cache.left_pane } else { cache.right_pane };
                let entry_row = (row as usize).saturating_sub((pane_rect.y + 1) as usize);

                return Some(Action::PaneClick { left_pane: clicked_left, row: entry_row });
            }

            None
        }

        // Mouse move / drag — update menu selection highlight on hover.
        // We use the menu bar y-position to anchor the calculation rather than
        // the cached popup rect so coordinate drift can't cause silent misses.
        MouseEventKind::Moved | MouseEventKind::Drag(_) => {
            if matches!(focus, FocusLayer::Modal(ModalKind::Menu)) {
                // Popup top border sits one row below the menu bar.
                let popup_top = cache.menu_bar.y + cache.menu_bar.height;
                if row > popup_top {
                    // row - popup_top gives 1-based item row (1 = first item).
                    let item_row = (row - popup_top).saturating_sub(1) as usize;
                    return Some(Action::MenuSetSelection(item_row));
                }
            }
            None
        }

        _ => None,
    }
}

/// Map an x-coordinate in the menu bar to an `OpenMenu` action.
/// Layout (0-indexed from bar_x):
///   0-7   " [Z]eta "  (logo — ignored)
///   8-13  " File "
///   14-23 " Navigate "
///   24-29 " View "
///   30-35 " Help "
fn route_menu_bar_click(col: u16, bar_x: u16, editor_menu_mode: bool) -> Option<Action> {
    let mut cursor = bar_x + 8;
    for tab in crate::state::menu_tabs(editor_menu_mode) {
        let start = cursor;
        let end = cursor + tab.label.len() as u16 - 1;
        if col >= start && col <= end {
            return Some(Action::OpenMenu(tab.id));
        }
        cursor += tab.label.len() as u16;
    }
    None
}

struct TerminalSession {
    terminal: TuiTerminal,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("failed to enable raw mode")?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
                .context("failed to enter alternate screen and enable mouse")?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).context("failed to create terminal backend")?;
        terminal.clear().context("failed to clear terminal")?;

        Ok(Self { terminal })
    }

    fn draw<F>(&mut self, render: F) -> Result<()>
    where
        F: FnOnce(&mut Frame<'_>),
    {
        self.terminal
            .draw(render)
            .context("failed to render terminal frame")?;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use ratatui::layout::Rect;

    use crate::action::Action;
    use crate::config::RuntimeKeymap;
    use crate::state::{FocusLayer, ModalKind};
    use crate::ui::layout_cache::LayoutCache;

    use super::{route_key_event, route_mouse_event};

    fn test_cache() -> LayoutCache {
        LayoutCache {
            menu_bar:   Rect { x: 0,  y: 0,  width: 80, height: 1  },
            left_pane:  Rect { x: 0,  y: 1,  width: 40, height: 20 },
            right_pane: Rect { x: 40, y: 1,  width: 40, height: 20 },
            tools_panel: None,
            editor_panel: None,
            file_preview_panel: None,
            markdown_preview_panel: None,
            status_bar: Rect { x: 0,  y: 21, width: 80, height: 1  },
            menu_popup: None,
        }
    }

    #[test]
    fn mouse_event_variant_exists_in_app_event() {
        let ev = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5, row: 3,
            modifiers: KeyModifiers::NONE,
        };
        let app_event = crate::event::AppEvent::Mouse(ev);
        assert!(matches!(app_event, crate::event::AppEvent::Mouse(_)));
    }

    #[test]
    fn route_mouse_scroll_up_in_pane_produces_move_selection_up() {
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::ScrollUp, column: 10, row: 5, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Pane, false,
        );
        assert_eq!(action, Some(Action::MoveSelectionUp));
    }

    #[test]
    fn route_mouse_scroll_down_in_pane_produces_move_selection_down() {
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::ScrollDown, column: 10, row: 5, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Pane, false,
        );
        assert_eq!(action, Some(Action::MoveSelectionDown));
    }

    #[test]
    fn route_mouse_left_click_on_pane_produces_action() {
        // col=10, row=5 → left pane (x:0..40, y:1..21); entry_row = 5 - (1+1) = 3
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 10, row: 5, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Pane, false,
        );
        assert_eq!(action, Some(Action::PaneClick { left_pane: true, row: 3 }));
    }

    #[test]
    fn route_mouse_left_click_on_right_pane_produces_right_pane_click() {
        // col=50, row=3 → right pane (x:40..80, y:1..21); entry_row = 3 - (1+1) = 1
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 50, row: 3, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Pane, false,
        );
        assert_eq!(action, Some(Action::PaneClick { left_pane: false, row: 1 }));
    }

    #[test]
    fn route_mouse_left_click_on_file_menu_opens_file_menu() {
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 10, row: 0, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Pane, false,
        );
        assert_eq!(action, Some(Action::OpenMenu(crate::action::MenuId::File)));
    }

    #[test]
    fn route_mouse_left_click_on_dialog_closes_it() {
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 10, row: 5, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Modal(ModalKind::Dialog), false,
        );
        assert_eq!(action, Some(Action::CloseDialog));
    }

    #[test]
    fn route_mouse_scroll_in_preview_layer_scrolls_preview() {
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::ScrollDown, column: 10, row: 5, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Preview, false,
        );
        assert_eq!(action, Some(Action::ScrollPreviewDown));
    }

    #[test]
    fn route_mouse_scroll_in_editor_layer_moves_cursor() {
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::ScrollUp, column: 10, row: 5, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Editor, false,
        );
        assert_eq!(action, Some(Action::EditorMoveUp));
    }

    #[test]
    fn command_palette_remains_available_while_editor_is_open() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Char('P'), KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            &keymap,
            FocusLayer::Editor,
            false,
        );
        assert_eq!(action, Some(Action::OpenCommandPalette));
    }

    #[test]
    fn editor_shortcuts_still_take_priority_over_global_fallbacks() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            &keymap,
            FocusLayer::Editor,
            false,
        );
        assert_eq!(action, Some(Action::EditorOpenSearch));
    }

    #[test]
    fn palette_open_state_blocks_lower_priority_input_paths() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            &keymap,
            FocusLayer::Modal(ModalKind::Palette),
            false,
        );
        assert_eq!(action, None);
    }

    #[test]
    fn palette_layer_routes_esc_to_close_palette() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            &keymap,
            FocusLayer::Modal(ModalKind::Palette),
            false,
        );
        assert_eq!(action, Some(Action::CloseCommandPalette));
    }

    #[test]
    fn bookmarks_layer_routes_enter_to_confirm_selection() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &keymap,
            FocusLayer::Modal(ModalKind::Bookmarks),
            false,
        );
        assert_eq!(action, Some(Action::BookmarkConfirm));
    }

    #[test]
    fn pane_layer_ctrl_q_quits() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
            &keymap,
            FocusLayer::Pane,
            false,
        );
        assert_eq!(action, Some(Action::Quit));
    }

    #[test]
    fn editor_layer_ctrl_f_opens_search() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            &keymap,
            FocusLayer::Editor,
            false,
        );
        assert_eq!(action, Some(Action::EditorOpenSearch));
    }
}
