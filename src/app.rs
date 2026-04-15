use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use anyhow::{Context, Result};
use crossbeam_channel::Receiver;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::{Frame, Terminal};

use crate::action::{Action, Command};
use crate::config::{AppConfig, RuntimeKeymap};
use crate::event::AppEvent;
use crate::jobs::{
    self, DirSizeRequest, EditorLoadRequest, FileOpRequest, FindRequest, GitStatusRequest,
    JobResult, PreviewRequest, ScanRequest, WatchRequest, WorkerChannels,
};
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
    /// Absolute path to the loaded config file; watched for live reload.
    config_path: std::path::PathBuf,
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
        let config_path = loaded_config.path.clone();
        let state = AppState::bootstrap(loaded_config, started_at)
            .context("failed to bootstrap application state")?;
        let mut app = Self {
            workers,
            job_results,
            keymap,
            state,
            layout_cache: LayoutCache::default(),
            last_pane_click: None,
            config_path,
        };

        for command in app.state.initial_commands() {
            app.execute_command(command)?;
        }

        Ok(app)
    }

    pub fn run(&mut self) -> Result<()> {
        let mut terminal = TerminalSession::enter()?;

        while !self.state.should_quit() {
            // Process events first; draw only when state actually changed.
            self.process_next_event()?;

            if self.state.needs_redraw() {
                let mut cache = LayoutCache::default();
                terminal.draw(|frame| {
                    cache = ui::render(frame, &mut self.state);
                })?;
                self.layout_cache = cache;
                // Propagate terminal panel size to the PTY worker when the layout changes.
                if let Some(t_area) = cache.terminal_panel {
                    let inner_rows = t_area.height.saturating_sub(1);
                    let inner_cols = t_area.width;
                    if self.state.terminal.is_open() && inner_rows > 0 && inner_cols > 0 {
                        for cmd in self.state.terminal.resize(inner_rows, inner_cols) {
                            self.execute_command_try(cmd)?;
                        }
                    }
                }
                self.state.mark_drawn(); // clears needs_redraw
            }
        }

        let session = crate::session::SessionState {
            active_workspace: Some(self.state.active_workspace_index()),
            workspaces: (0..self.state.workspace_count())
                .map(|workspace_id| {
                    let workspace = self.state.workspace(workspace_id);
                    crate::session::WorkspaceSessionState {
                        left_cwd: Some(workspace.panes.left.cwd.clone()),
                        right_cwd: Some(workspace.panes.right.cwd.clone()),
                        left_sort: Some(workspace.panes.left.sort_mode),
                        right_sort: Some(workspace.panes.right.sort_mode),
                        left_hidden: workspace.panes.left.show_hidden,
                        right_hidden: workspace.panes.right.show_hidden,
                        layout: Some(workspace.panes.pane_layout),
                    }
                })
                .collect(),
            ..Default::default()
        };
        let session_path = crate::session::SessionState::session_path(std::path::Path::new(
            self.state.config_path(),
        ));
        let _ = session.save(&session_path); // non-fatal

        Ok(())
    }

    fn execute_command_try(&mut self, command: Command) -> Result<()> {
        match command {
            Command::ResizeTerminal { cols, rows } => {
                let _ = self
                    .workers
                    .terminal_tx
                    .try_send(crate::jobs::TerminalRequest::Resize {
                        workspace_id: self.state.active_workspace_index(),
                        cols,
                        rows,
                    });
            }
            other => self.execute_command(other)?,
        }
        Ok(())
    }

    fn process_next_event(&mut self) -> Result<()> {
        // Drain ALL pending job results. Any received result marks state dirty
        // so the draw loop knows to re-render.
        let mut had_job = false;
        while let Ok(result) = self.job_results.try_recv() {
            self.handle_event(AppEvent::Job(Box::new(result)))?;
            had_job = true;
        }
        if had_job {
            self.state.set_needs_redraw();
        }

        // Poll for at most one input / resize event per iteration.
        if !event::poll(Duration::from_millis(16)).context("failed to poll terminal events")? {
            // Idle tick: dispatch a debounced preview request if one is due.
            if let Some(command) = self.state.preview_command_due() {
                self.execute_command(command)?;
                // No immediate redraw needed — the job result will set the flag
                // when the preview worker completes.
            }
            return Ok(());
        }

        match event::read().context("failed to read terminal event")? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_event(AppEvent::Input(key_event))?;
                self.state.set_needs_redraw();
            }
            Event::Mouse(mouse_event) => {
                self.handle_event(AppEvent::Mouse(mouse_event))?;
                self.state.set_needs_redraw();
            }
            Event::Resize(width, height) => {
                self.handle_event(AppEvent::Resize { width, height })?;
                self.state.set_needs_redraw();
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Input(key_event) => {
                let focus = self.state.focus_layer();
                let is_preview_open = self.state.is_preview_panel_open();
                let is_settings_rebinding = self.state.is_settings_rebinding();
                if let Some(action) = route_key_event(
                    key_event,
                    &self.keymap,
                    focus,
                    is_preview_open,
                    is_settings_rebinding,
                ) {
                    self.dispatch(action)?;
                }
            }
            AppEvent::Mouse(mouse_event) => {
                let focus = self.state.focus_layer();
                let editor_menu_mode =
                    self.state.is_editor_fullscreen() && self.state.editor().is_some();
                if let Some(action) =
                    route_mouse_event(mouse_event, &self.layout_cache, focus, editor_menu_mode)
                {
                    // Intercept PaneClick to detect double-clicks.
                    let action = if let Action::PaneClick { left_pane, row } = action {
                        let now = std::time::Instant::now();
                        let double = self.last_pane_click.is_some_and(|(lp, r, t)| {
                            lp == left_pane && r == row && now.duration_since(t).as_millis() < 400
                        });
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
            AppEvent::Job(result) => match *result {
                JobResult::DirectoryChanged { path } => {
                    for workspace_id in 0..self.state.workspace_count() {
                        for pane in [crate::pane::PaneId::Left, crate::pane::PaneId::Right] {
                            let pane_state = self.state.workspace(workspace_id).panes.pane(pane);
                            if pane_state.cwd == path {
                                let scan_path = path.clone();
                                if let Some(address) = pane_state.remote_address() {
                                    let session_id = format!(
                                        "{}@{}",
                                        std::env::var("USER")
                                            .unwrap_or_else(|_| "user".to_string()),
                                        address
                                    );
                                    self.workers
                                        .sftp_tx
                                        .send(jobs::SftpRequest::Scan(jobs::SftpScanRequest {
                                            workspace_id,
                                            pane,
                                            path: scan_path,
                                            session_id,
                                        }))
                                        .context("failed to queue SFTP scan job")?;
                                } else {
                                    self.workers
                                        .scan_tx
                                        .send(ScanRequest {
                                            workspace_id,
                                            pane,
                                            path: scan_path.clone(),
                                        })
                                        .context("failed to queue background scan job")?;
                                    self.workers
                                        .git_tx
                                        .send(GitStatusRequest {
                                            workspace_id,
                                            pane,
                                            path: scan_path,
                                        })
                                        .context("failed to queue git status job")?;
                                }
                            }
                        }
                    }
                }
                JobResult::ConfigChanged => {
                    if let Ok(new_config) = AppConfig::load(&self.config_path) {
                        if new_config.keymap != self.state.config().keymap {
                            if let Ok(km) = new_config.compile_keymap() {
                                self.keymap = km;
                            }
                        }
                        self.state.apply_config_reload(new_config);
                    }
                }
                other => {
                    // When SSH connects, queue an SFTP home scan BEFORE delegating to state,
                    // so the pane-mode change and scan happen atomically from the UI's perspective.
                    if let jobs::JobResult::SshConnected {
                        workspace_id,
                        pane,
                        ref session_id,
                        ..
                    } = &other
                    {
                        let ws = *workspace_id;
                        let p = *pane;
                        let sid = session_id.clone();
                        self.workers
                            .sftp_tx
                            .send(jobs::SftpRequest::Scan(jobs::SftpScanRequest {
                                workspace_id: ws,
                                pane: p,
                                path: std::path::PathBuf::from("/"),
                                session_id: sid,
                            }))
                            .context("failed to queue SFTP home scan")?;
                    }
                    let scanned_target =
                        if let JobResult::DirectoryScanned {
                            workspace_id, pane, ..
                        } = &other
                        {
                            Some((*workspace_id, *pane))
                        } else {
                            None
                        };
                    let refresh_watch = matches!(&other, JobResult::DirectoryScanned { .. });
                    self.state.apply_job_result(other);
                    if refresh_watch {
                        self.sync_watched_paths()?;
                    }
                    if let Some((workspace_id, pane)) = scanned_target {
                        let pane_state = self.state.workspace(workspace_id).panes.pane(pane);
                        if pane_state.details_view
                            || matches!(
                                pane_state.sort_mode,
                                crate::pane::SortMode::Size | crate::pane::SortMode::SizeDesc
                            )
                        {
                            for entry in &pane_state.entries {
                                if entry.kind == crate::fs::EntryKind::Directory
                                    && entry.name != ".."
                                {
                                    let _ = self.workers.dir_size_tx.try_send(DirSizeRequest {
                                        workspace_id,
                                        pane,
                                        path: entry.path.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            },
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
        let mut paths = Vec::new();
        for workspace_id in 0..self.state.workspace_count() {
            let workspace = self.state.workspace(workspace_id);
            for path in [&workspace.panes.left.cwd, &workspace.panes.right.cwd] {
                if paths.iter().all(|existing| existing != path) {
                    paths.push(path.clone());
                }
            }
        }
        let config_path = if self.config_path.as_os_str().is_empty() {
            None
        } else {
            Some(self.config_path.clone())
        };
        self.workers
            .watch_tx
            .send(WatchRequest { paths, config_path })
            .context("failed to update watched directories")?;
        Ok(())
    }

    /// Determine source and destination sessions for a file operation based on
    /// which panes the paths belong to
    fn determine_backends_for_operation(
        &self,
        operation: &crate::action::FileOperation,
    ) -> (
        Option<crate::jobs::SessionId>,
        Option<crate::jobs::SessionId>,
    ) {
        use crate::action::FileOperation;

        let (src_path, dst_path): (Option<&std::path::Path>, Option<&std::path::Path>) =
            match operation {
                FileOperation::Copy {
                    source,
                    destination,
                } => (Some(source), Some(destination)),
                FileOperation::Move {
                    source,
                    destination,
                } => (Some(source), Some(destination)),
                FileOperation::Rename {
                    source,
                    destination,
                } => (Some(source), Some(destination)),
                FileOperation::Delete { path } => (Some(path), None),
                FileOperation::Trash { path } => (Some(path), None),
                FileOperation::CreateDirectory { path } => (None, Some(path)),
                FileOperation::CreateFile { path } => (None, Some(path)),
                FileOperation::ExtractArchive {
                    archive,
                    destination,
                    ..
                } => (Some(archive), Some(destination)),
            };

        let get_session = |path: Option<&std::path::Path>| {
            path.and_then(|_p| {
                // Check if path is in a remote pane's working directory
                // For now, we use a simple heuristic: paths that look like they
                // belong to a remote pane based on cwd
                let pane = self.state.panes.active_pane();
                if pane.in_remote() {
                    // Use the current active pane's remote session
                    pane.remote_address().map(|addr| addr.to_string())
                } else {
                    None
                }
            })
        };

        (get_session(src_path), get_session(dst_path))
    }

    fn execute_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::OpenEditor { path } => {
                let workspace_id = self.state.active_workspace_index();
                self.state.begin_open_editor(path.clone());
                self.workers
                    .editor_tx
                    .send(EditorLoadRequest { workspace_id, path })
                    .context("failed to queue background editor load job")?;
            }
            Command::PreviewFile { path } => {
                let workspace_id = self.state.active_workspace_index();
                let mut archive = None;
                let mut inner = None;
                if self.state.panes.active_pane().in_archive() {
                    if let crate::pane::PaneMode::Archive { source, inner_path } =
                        &self.state.panes.active_pane().mode
                    {
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
                        workspace_id,
                        path,
                        syntect_theme: self.state.theme().palette.syntect_theme.to_string(),
                        archive,
                        inner_path: inner,
                    })
                    .context("failed to queue background preview job")?;
            }
            Command::RunFileOperation {
                operation,
                refresh,
                collision,
            } => {
                let workspace_id = self.state.active_workspace_index();
                let (src_session, dst_session) = self.determine_backends_for_operation(&operation);

                if src_session.is_some() || dst_session.is_some() {
                    self.workers
                        .sftp_tx
                        .send(jobs::SftpRequest::FileOp(jobs::SftpFileOpRequest {
                            workspace_id,
                            operation: operation.clone(),
                            src_session: src_session.clone(),
                            dst_session: dst_session.clone(),
                            refresh: refresh.clone(),
                            collision,
                        }))
                        .context("failed to queue SFTP file operation")?;
                } else {
                    self.workers
                        .file_op_tx
                        .send(FileOpRequest {
                            workspace_id,
                            operation,
                            backend: crate::jobs::BackendRef::Local,
                            refresh,
                            collision,
                            src_session: None,
                            dst_session: None,
                        })
                        .context("failed to queue background file operation")?;
                }
            }
            Command::ScanPane { pane, path } => {
                let workspace_id = self.state.active_workspace_index();
                if let Some(address) = self.state.panes.pane(pane).remote_address() {
                    let session_id = format!(
                        "{}@{}",
                        std::env::var("USER").unwrap_or_else(|_| "user".to_string()),
                        address
                    );

                    self.workers
                        .sftp_tx
                        .send(jobs::SftpRequest::Scan(jobs::SftpScanRequest {
                            workspace_id,
                            pane,
                            path: path.clone(),
                            session_id,
                        }))
                        .context("failed to queue SFTP scan job")?;
                } else {
                    self.workers
                        .scan_tx
                        .send(ScanRequest {
                            workspace_id,
                            pane,
                            path: path.clone(),
                        })
                        .context("failed to queue background scan job")?;
                    self.workers
                        .git_tx
                        .send(GitStatusRequest {
                            workspace_id,
                            pane,
                            path,
                        })
                        .context("failed to queue git status job")?;
                }
            }
            Command::FindFiles {
                pane,
                root,
                max_depth,
            } => {
                let workspace_id = self.state.active_workspace_index();
                self.workers
                    .find_tx
                    .send(FindRequest {
                        workspace_id,
                        pane,
                        root,
                        max_depth,
                    })
                    .context("failed to queue background file finder job")?;
            }
            Command::OpenArchive { path, inner } => {
                let workspace_id = self.state.active_workspace_index();
                let pane = self.state.panes.focused_pane_id();
                let req = jobs::ArchiveListRequest {
                    workspace_id,
                    pane,
                    archive_path: path.clone(),
                    inner_path: inner.clone(),
                };
                self.workers
                    .archive_tx
                    .send(req)
                    .context("failed to queue archive listing job")?;
            }
            Command::OpenShell { path } => {
                use crossterm::execute;
                use crossterm::terminal::{
                    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
                };
                use std::io::{self};
                use std::process::Command as StdCommand;

                disable_raw_mode().ok();
                let mut stdout = io::stdout();
                execute!(stdout, LeaveAlternateScreen).ok();

                let shell = std::env::var("SHELL").unwrap_or_else(|_| {
                    if cfg!(windows) {
                        std::env::var("COMSPEC").unwrap_or_else(|_| String::from("cmd.exe"))
                    } else {
                        String::from("/bin/sh")
                    }
                });

                let _ = StdCommand::new(shell).current_dir(path).status();

                execute!(stdout, EnterAlternateScreen).ok();
                enable_raw_mode().ok();
            }

            Command::ConnectSSH {
                address,
                auth_method,
                credential,
                pane,
                trust_unknown_host,
            } => {
                let workspace_id = self.state.active_workspace_index();
                self.workers
                    .sftp_tx
                    .send(jobs::SftpRequest::Connect {
                        workspace_id,
                        pane,
                        address,
                        auth_method,
                        credential,
                        trust_unknown_host,
                    })
                    .context("failed to queue SSH connect job")?;
            }
            Command::DisconnectSSH { pane } => {
                self.state.panes.pane_mut(pane).mode = crate::pane::PaneMode::Real;
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                self.execute_command(Command::ScanPane {
                    pane,
                    path: std::path::PathBuf::from(home),
                })?;
            }
            Command::SpawnTerminal { cwd, spawn_id } => {
                self.workers
                    .terminal_tx
                    .send(crate::jobs::TerminalRequest::Spawn {
                        workspace_id: self.state.active_workspace_index(),
                        cwd,
                        cols: self.state.terminal.cols,
                        rows: self.state.terminal.rows,
                        spawn_id,
                    })
                    .context("failed to queue terminal spawn job")?;
            }
            Command::WriteTerminal(bytes) => {
                self.workers
                    .terminal_tx
                    .send(crate::jobs::TerminalRequest::Write {
                        workspace_id: self.state.active_workspace_index(),
                        bytes,
                    })
                    .context("failed to queue terminal write job")?;
            }
            Command::ResizeTerminal { cols, rows } => {
                self.workers
                    .terminal_tx
                    .send(crate::jobs::TerminalRequest::Resize {
                        workspace_id: self.state.active_workspace_index(),
                        cols,
                        rows,
                    })
                    .context("failed to queue terminal resize job")?;
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
            Command::UpdateKeymap(new_keymap) => {
                self.keymap = new_keymap;
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
    is_settings_rebinding: bool,
) -> Option<Action> {
    use crossterm::event::{KeyCode, KeyModifiers};

    let alt_f3 = key_event.code == KeyCode::F(3) && key_event.modifiers == KeyModifiers::ALT;
    match focus {
        FocusLayer::Modal(ModalKind::Palette) => Action::from_palette_key_event(key_event),
        FocusLayer::Modal(ModalKind::Collision) => Action::from_collision_key_event(key_event),
        FocusLayer::Modal(ModalKind::Prompt) => Action::from_prompt_key_event(key_event),
        FocusLayer::Modal(ModalKind::Dialog) => Action::from_dialog_key_event(key_event),
        FocusLayer::Modal(ModalKind::Menu) => Action::from_menu_key_event(key_event, keymap),
        FocusLayer::Modal(ModalKind::Settings) => {
            Action::from_settings_key_event(key_event, is_settings_rebinding)
        }
        FocusLayer::Modal(ModalKind::Bookmarks) => Action::from_bookmarks_key_event(key_event),
        FocusLayer::Modal(ModalKind::FileFinder) => Action::from_file_finder_key_event(key_event),
        FocusLayer::Modal(ModalKind::SshConnect) => Action::from_ssh_connect_key_event(key_event),
        FocusLayer::Modal(ModalKind::SshTrustPrompt) => Action::from_ssh_trust_key_event(key_event),
        FocusLayer::PaneFilter => Action::from_pane_filter_key_event(key_event),
        FocusLayer::PaneInlineRename => Action::from_inline_rename_key_event(key_event),
        FocusLayer::Preview => Action::from_preview_key_event(key_event),
        FocusLayer::Terminal => Action::from_terminal_key_event(key_event),
        FocusLayer::MarkdownPreview => {
            if is_preview_open && alt_f3 {
                return Some(Action::FocusPreviewPanel);
            }
            Action::from_markdown_preview_key_event(key_event)
                .or_else(|| Action::from_editor_key_event(key_event, keymap))
                .or_else(|| Action::from_pane_key_event(key_event, keymap))
        }
        FocusLayer::Editor => {
            if is_preview_open && alt_f3 {
                return Some(Action::FocusPreviewPanel);
            }
            Action::from_editor_key_event(key_event, keymap)
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

    // Inline rename is keyboard-only; absorb mouse input so the displayed row
    // cannot diverge from the file that will actually be renamed.
    if matches!(focus, FocusLayer::PaneInlineRename) {
        return None;
    }

    match event.kind {
        // -------------------------------------------------------------------
        // Scroll wheel
        // -------------------------------------------------------------------
        MouseEventKind::ScrollUp => {
            // Dialog scroll takes priority — route anywhere on screen when dialog is open.
            if matches!(focus, FocusLayer::Modal(ModalKind::Dialog)) {
                return Some(Action::ScrollDialogUp);
            }
            // All other open modals absorb scroll — don't leak through to panes.
            if matches!(focus, FocusLayer::Modal(_)) {
                return None;
            }
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
                || cache
                    .editor_panel
                    .is_some_and(|r| rect_contains(r, col, row))
            {
                return Some(Action::EditorMoveUp);
            }
            if rect_contains(cache.left_pane, col, row) || rect_contains(cache.right_pane, col, row)
            {
                return Some(Action::MoveSelectionUp);
            }
            None
        }
        MouseEventKind::ScrollDown => {
            // Dialog scroll takes priority — route anywhere on screen when dialog is open.
            if matches!(focus, FocusLayer::Modal(ModalKind::Dialog)) {
                return Some(Action::ScrollDialogDown);
            }
            // All other open modals absorb scroll — don't leak through to panes.
            if matches!(focus, FocusLayer::Modal(_)) {
                return None;
            }
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
                || cache
                    .editor_panel
                    .is_some_and(|r| rect_contains(r, col, row))
            {
                return Some(Action::EditorMoveDown);
            }
            if rect_contains(cache.left_pane, col, row) || rect_contains(cache.right_pane, col, row)
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

            if let Some(terminal_rect) = cache.terminal_panel {
                if rect_contains(terminal_rect, col, row) {
                    if focus != FocusLayer::Terminal {
                        return Some(Action::ToggleTerminal); // Toggle will focus if not open, but here it's open
                                                             // Actually, ToggleTerminal on open terminal might close it?
                                                             // Let's use a dedicated FocusTerminal action or just logic.
                    }
                    return None;
                }
            }

            // Click on left or right pane.
            if rect_contains(cache.left_pane, col, row) || rect_contains(cache.right_pane, col, row)
            {
                let clicked_left = rect_contains(cache.left_pane, col, row);

                // If focus is on a tool (editor/preview), return to pane layer first.
                if focus == FocusLayer::Editor
                    || focus == FocusLayer::Preview
                    || focus == FocusLayer::MarkdownPreview
                    || focus == FocusLayer::Terminal
                {
                    return Some(Action::CycleFocus);
                }

                // Calculate which entry row was clicked (subtract 1 for top border).
                let pane_rect = if clicked_left {
                    cache.left_pane
                } else {
                    cache.right_pane
                };
                let entry_row = (row as usize).saturating_sub((pane_rect.y + 1) as usize);

                return Some(Action::PaneClick {
                    left_pane: clicked_left,
                    row: entry_row,
                });
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

/// Map an x-coordinate in the menu bar to either an `OpenMenu` action or a
/// workspace switch action.
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

    cursor += 1; // spacer before workspace pills
    for workspace_idx in 0..4usize {
        let start = cursor;
        let end = cursor + 2; // `[N]`
        if col >= start && col <= end {
            return Some(Action::SwitchToWorkspace(workspace_idx));
        }
        cursor += 4; // `[N]` plus trailing spacer
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
            menu_bar: Rect {
                x: 0,
                y: 0,
                width: 80,
                height: 1,
            },
            left_pane: Rect {
                x: 0,
                y: 1,
                width: 40,
                height: 20,
            },
            right_pane: Rect {
                x: 40,
                y: 1,
                width: 40,
                height: 20,
            },
            tools_panel: None,
            editor_panel: None,
            file_preview_panel: None,
            markdown_preview_panel: None,
            status_bar: Rect {
                x: 0,
                y: 21,
                width: 80,
                height: 1,
            },
            menu_popup: None,
            hint_bar: Rect::default(),
            terminal_panel: None,
        }
    }

    #[test]
    fn mouse_event_variant_exists_in_app_event() {
        let ev = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 3,
            modifiers: KeyModifiers::NONE,
        };
        let app_event = crate::event::AppEvent::Mouse(ev);
        assert!(matches!(app_event, crate::event::AppEvent::Mouse(_)));
    }

    #[test]
    fn route_mouse_scroll_up_in_pane_produces_move_selection_up() {
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 10,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Pane,
            false,
        );
        assert_eq!(action, Some(Action::MoveSelectionUp));
    }

    #[test]
    fn route_mouse_scroll_down_in_pane_produces_move_selection_down() {
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 10,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Pane,
            false,
        );
        assert_eq!(action, Some(Action::MoveSelectionDown));
    }

    #[test]
    fn route_mouse_left_click_on_pane_produces_action() {
        // col=10, row=5 → left pane (x:0..40, y:1..21); entry_row = 5 - (1+1) = 3
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 10,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Pane,
            false,
        );
        assert_eq!(
            action,
            Some(Action::PaneClick {
                left_pane: true,
                row: 3
            })
        );
    }

    #[test]
    fn route_mouse_left_click_on_right_pane_produces_right_pane_click() {
        // col=50, row=3 → right pane (x:40..80, y:1..21); entry_row = 3 - (1+1) = 1
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 50,
                row: 3,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Pane,
            false,
        );
        assert_eq!(
            action,
            Some(Action::PaneClick {
                left_pane: false,
                row: 1
            })
        );
    }

    #[test]
    fn route_mouse_left_click_on_file_menu_opens_file_menu() {
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 10,
                row: 0,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Pane,
            false,
        );
        assert_eq!(action, Some(Action::OpenMenu(crate::action::MenuId::File)));
    }

    #[test]
    fn route_mouse_left_click_on_workspace_pill_2_switches_workspace() {
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 49,
                row: 0,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Pane,
            false,
        );
        assert_eq!(action, Some(Action::SwitchToWorkspace(1)));
    }

    #[test]
    fn route_mouse_left_click_on_workspace_pill_4_switches_workspace() {
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 57,
                row: 0,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Pane,
            false,
        );
        assert_eq!(action, Some(Action::SwitchToWorkspace(3)));
    }

    #[test]
    fn route_mouse_left_click_on_dialog_closes_it() {
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 10,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Modal(ModalKind::Dialog),
            false,
        );
        assert_eq!(action, Some(Action::CloseDialog));
    }

    #[test]
    fn route_mouse_scroll_in_preview_layer_scrolls_preview() {
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 10,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Preview,
            false,
        );
        assert_eq!(action, Some(Action::ScrollPreviewDown));
    }

    #[test]
    fn route_mouse_scroll_in_editor_layer_moves_cursor() {
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 10,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Editor,
            false,
        );
        assert_eq!(action, Some(Action::EditorMoveUp));
    }
    #[test]
    fn route_mouse_scroll_on_dialog_scrolls_dialog() {
        // Scroll anywhere (including over a pane rect) must route to the dialog,
        // not fall through to MoveSelectionUp/Down.
        let up = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 10,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Modal(ModalKind::Dialog),
            false,
        );
        assert_eq!(up, Some(Action::ScrollDialogUp));

        let down = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 10,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Modal(ModalKind::Dialog),
            false,
        );
        assert_eq!(down, Some(Action::ScrollDialogDown));
    }

    #[test]
    fn route_mouse_scroll_on_other_modal_is_absorbed() {
        // Scroll while a non-dialog modal is open must not reach the pane.
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 10,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Modal(ModalKind::Prompt),
            false,
        );
        assert_eq!(action, None);
    }

    #[test]
    fn command_palette_remains_available_while_editor_is_open() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(
                KeyCode::Char('P'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            &keymap,
            FocusLayer::Editor,
            false,
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
            false,
        );
        assert_eq!(action, None);
    }

    #[test]
    fn prompt_layer_absorbs_workspace_shortcuts() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Char('2'), KeyModifiers::ALT),
            &keymap,
            FocusLayer::Modal(ModalKind::Prompt),
            false,
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
            false,
        );
        assert_eq!(action, Some(Action::EditorOpenSearch));
    }
}
