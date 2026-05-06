use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use anyhow::{Context, Result};
use crossbeam_channel::Receiver;
use crossterm::event::{
    self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture, Event,
    KeyEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::{Frame, Terminal};
use ratatui_image::picker::Picker;

use crate::action::{Action, Command};
use crate::config::{AppConfig, RuntimeKeymap};
use crate::event::AppEvent;
use crate::jobs::{
    self, DirSizeRequest, EditorLoadRequest, FileOpRequest, FindRequest, GitStatusRequest,
    JobResult, PreviewRequest, ScanRequest, UpdateCheckRequest, WatchRequest, WorkerChannels,
};
use crate::state::{AppState, FocusLayer, ModalKind};
use crate::ui;
use crate::ui::layout_cache::{rect_contains, LayoutCache};

type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

pub struct App {
    workers: WorkerChannels,
    job_results: Receiver<JobResult>,
    /// Dedicated receiver for PTY output, drained with a per-frame cap to prevent
    /// a verbose process from starving keyboard/mouse handling.
    terminal_output: Receiver<JobResult>,
    keymap: RuntimeKeymap,
    state: AppState,
    pub layout_cache: LayoutCache,
    last_pane_click: Option<(bool, usize, std::time::Instant)>, // (left_pane, row, time)
    /// Absolute path to the loaded config file; watched for live reload.
    config_path: std::path::PathBuf,
    /// Last second value displayed in the clock; used to trigger a redraw each second.
    last_clock_second: u8,
    /// Tracks a pending Ctrl+Q press for double-press confirmation.
    pending_quit: Option<std::time::Instant>,
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
        let (workers, job_results, terminal_output) = jobs::spawn_workers();
        let config_path = loaded_config.path.clone();
        let state = AppState::bootstrap(loaded_config, started_at)
            .context("failed to bootstrap application state")?;
        let mut app = Self {
            workers,
            job_results,
            terminal_output,
            keymap,
            state,
            layout_cache: LayoutCache::default(),
            last_pane_click: None,
            config_path,
            last_clock_second: 255, // force redraw on first tick
            pending_quit: None,
        };

        for command in app.state.initial_commands() {
            app.execute_command(command)?;
        }

        // Spawn background update check on startup
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        if app.state.config().check_updates_on_startup {
            let _ = app
                .workers
                .update_check_tx
                .send(UpdateCheckRequest::CheckLatestRelease { current_version });
        }

        Ok(app)
    }

    pub fn run(&mut self) -> Result<()> {
        // Panic hook: write to file so we can diagnose crashes when terminal is in raw mode.
        let orig_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let msg = format!("Panic: {}\n", info);
            let backtrace = std::backtrace::Backtrace::force_capture();
            std::fs::write("zeta_panic.log", format!("{}\n{}", msg, backtrace)).ok();
            orig_hook(info);
        }));

        // Run TUI in inner scope so terminal is fully restored before post-exit logic.
        {
            let mut terminal = TerminalSession::enter()?;

            // Use halfblocks by default to avoid potential hangs with from_query_stdio().
            // The query can block indefinitely in some terminal environments (e.g., WSL).
            self.state.set_image_picker(Picker::halfblocks());

            while !self.state.should_quit() {
                // Increment pulse counter for update indicator animation (wraps 0-255).
                self.state.update_pulse_frame = self.state.update_pulse_frame.wrapping_add(1);

                // Process events first; draw only when state actually changed.
                self.process_next_event()?;

                if self.state.needs_redraw() {
                    let size = terminal.terminal.size()?;
                    // Skip rendering entirely when the terminal reports zero dimensions.
                    // This happens on some platforms when the terminal window is minimized
                    // (SIGWINCH with 0×0).  Writing frames into a zero-size terminal
                    // corrupts the output buffer and can panic on dimension arithmetic.
                    if size.width == 0 || size.height == 0 {
                        self.state.mark_drawn();
                    } else {
                        // On focus-gain, clear ratatui's internal buffer so the next draw
                        // is a full repaint rather than a diff.  This fixes visual corruption
                        // (black squares, stale cells) that can appear after switching away
                        // from the terminal window and back.
                        if self.state.take_full_redraw() {
                            terminal.terminal.clear()?;
                        }
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
                        self.state.mark_drawn();
                    }
                }
            }

            // Fire on_exit hooks (fire-and-forget, best effort — may outlive Zeta).
            {
                let hook_env = crate::hooks::HookEnv {
                    path: self
                        .state
                        .active_workspace()
                        .panes
                        .active_pane()
                        .cwd
                        .display()
                        .to_string(),
                    ..crate::hooks::HookEnv::default()
                };
                let hook_cmds = crate::hooks::commands_for_event(
                    &self.state.config().hooks,
                    crate::config::HookEvent::OnExit,
                    &hook_env,
                    self.state.active_workspace_index(),
                );
                for cmd in hook_cmds {
                    let _ = self.execute_command_try(cmd);
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
                            left_history: workspace.panes.left.history_back.clone(),
                            right_history: workspace.panes.right.history_back.clone(),
                        }
                    })
                    .collect(),
                ..Default::default()
            };
            let session_path = crate::session::SessionState::session_path(std::path::Path::new(
                self.state.config_path(),
            ));
            let _ = session.save(&session_path); // non-fatal
        } // terminal dropped here, raw mode restored

        // Post-exit logic: if update scheduled, run cargo install and relaunch.
        if self.state.update_state.install_on_exit {
            let target_tag = self
                .state
                .update_state
                .available_release
                .as_ref()
                .map(|r| r.tag_name.clone());
            run_update_and_restart(target_tag.as_deref())?;
        }

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
        // Drain non-terminal job results completely. These workers are rate-limited
        // by bounded queues and finite work units, so the loop terminates promptly.
        let mut had_job = false;
        while let Ok(result) = self.job_results.try_recv() {
            self.handle_event(AppEvent::Job(Box::new(result)))?;
            had_job = true;
        }
        if had_job {
            self.state.set_needs_redraw();
        }

        // Drain PTY output with a per-frame cap. The channel is unbounded so no
        // bytes are ever dropped, but we stop after this many chunks per frame so a
        // verbose process (e.g. `yes`) cannot keep the loop running indefinitely.
        const MAX_TERMINAL_CHUNKS_PER_FRAME: usize = 64;
        let mut had_terminal = false;
        for _ in 0..MAX_TERMINAL_CHUNKS_PER_FRAME {
            match self.terminal_output.try_recv() {
                Ok(result) => {
                    self.handle_event(AppEvent::Job(Box::new(result)))?;
                    had_terminal = true;
                }
                Err(_) => break,
            }
        }
        if had_terminal {
            self.state.set_needs_redraw();
        }

        // Handle update check results
        if let Ok(result) = self.workers.update_check_rx.try_recv() {
            match result.release {
                Ok(Some(release)) => {
                    self.state.update_state.set_available(release);
                }
                Ok(None) => {
                    self.state.update_state.set_current();
                }
                Err(e) => {
                    self.state.update_state.set_error(e.to_string());
                }
            }
            self.state.set_needs_redraw();
        }

        // Poll for at most one input / resize event per iteration.
        if !event::poll(Duration::from_millis(16)).context("failed to poll terminal events")? {
            // Idle tick: dispatch a debounced preview request if one is due.
            if let Some(command) = self.state.preview_command_due() {
                self.execute_command(command)?;
            }
            // Trigger a redraw whenever the wall-clock second advances so the status
            // bar clock stays live even when the user isn't pressing keys.
            let current_second = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                % 60) as u8;
            if current_second != self.last_clock_second {
                self.last_clock_second = current_second;
                self.state.set_needs_redraw();
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
                // Always force a full repaint on resize.  Restoring a minimised window
                // reliably sends a Resize event on virtually every terminal/OS, making
                // this more dependable than FocusGained (which requires terminal support
                // for the EnableFocusChange escape sequence).
                self.state.set_full_redraw();
                self.state.set_needs_redraw();
            }
            // When the terminal regains focus, force a full repaint so ratatui's
            // differential renderer doesn't apply a diff to a potentially corrupted
            // display buffer (the cause of black-square corruption after alt-tab).
            Event::FocusGained => {
                self.state.set_full_redraw();
                self.state.set_needs_redraw();
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Input(key_event) => {
                // Record every key press into the debug state (visible via F12 panel).
                let key_desc = format!("{:?} + {:?}", key_event.code, key_event.modifiers);
                self.state.debug.record_key(key_desc);

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
                    // Require a second Ctrl+Q within 1.5 s to actually quit.
                    if action == Action::Quit {
                        let now = std::time::Instant::now();
                        if let Some(last) = self.pending_quit {
                            if now.duration_since(last).as_millis() < 1500 {
                                self.dispatch(Action::Quit)?;
                                return Ok(());
                            }
                        }
                        self.pending_quit = Some(now);
                        self.state.set_status("Press Ctrl+Q again to quit");
                        return Ok(());
                    }
                    self.pending_quit = None;
                    self.dispatch(action)?;
                }
            }
            AppEvent::Mouse(mouse_event) => {
                let focus = self.state.focus_layer();
                let menu_ctx = self.state.menu_context();
                if let Some(action) =
                    route_mouse_event(mouse_event, &self.layout_cache, focus, menu_ctx)
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

                    // Intercept editor mouse-click/drag actions so we can resolve
                    // screen coordinates using the layout cache before handing
                    // to the state.
                    match action {
                        Action::EditorClickAt { col, row } => {
                            self.handle_editor_click(col, row, false)?;
                            return Ok(());
                        }
                        Action::EditorDragTo { col, row } => {
                            self.handle_editor_click(col, row, true)?;
                            return Ok(());
                        }
                        _ => {}
                    }

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
                                    // Use try_send so a full channel never blocks the event loop.
                                    let _ = self.workers.sftp_tx.try_send(jobs::SftpRequest::Scan(
                                        jobs::SftpScanRequest {
                                            workspace_id,
                                            pane,
                                            path: scan_path,
                                            session_id,
                                        },
                                    ));
                                } else {
                                    // Background refresh jobs: drop silently when workers are
                                    // backlogged rather than blocking the main thread.
                                    let _ = self.workers.scan_tx.try_send(ScanRequest {
                                        workspace_id,
                                        pane,
                                        path: scan_path.clone(),
                                    });
                                    let _ = self.workers.git_tx.try_send(GitStatusRequest {
                                        workspace_id,
                                        pane,
                                        path: scan_path,
                                    });
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
                        // Use try_send: if sftp_tx is full the scan will be re-triggered
                        // after the existing jobs drain.  A blocking send here would stall
                        // the event loop and kill keyboard responsiveness.
                        let _ = self.workers.sftp_tx.try_send(jobs::SftpRequest::Scan(
                            jobs::SftpScanRequest {
                                workspace_id: ws,
                                pane: p,
                                path: std::path::PathBuf::from("/"),
                                session_id: sid,
                            },
                        ));
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
                    let hook_cmds = self.state.apply_job_result_commands(other);
                    for cmd in hook_cmds {
                        self.execute_command_try(cmd)?;
                    }
                    if let Some((workspace_id, pane)) = scanned_target {
                        self.post_scan_completed(workspace_id, pane)?;
                    }
                }
            },
        }
        Ok(())
    }

    /// Resolve an editor mouse click or drag (screen coords) to buffer coordinates
    /// and call the appropriate `TextAreaAdapter` method.
    /// `is_drag` = false for a new click (sets anchor + cursor), true for drag (moves cursor only).
    fn handle_editor_click(&mut self, col: u16, row: u16, is_drag: bool) -> Result<()> {
        let Some(editor_rect) = self.layout_cache.editor_panel else {
            return Ok(());
        };
        let tab_width = self.state.config().editor.tab_width;
        let Some(editor) = self.state.editor_mut() else {
            return Ok(());
        };

        // Match tui-textarea-2's gutter: num_digits(line_count) + 2 margin chars
        let line_count = editor.line_count();
        let digits = if line_count == 0 {
            1
        } else {
            (line_count.ilog10() + 1) as u16
        };
        let gutter_width = digits + 2;

        // Convert screen col/row to viewport-relative coords.
        // Editor content starts at left edge + gutter (no border).
        let content_start_col = editor_rect.x + gutter_width;
        let content_start_row = editor_rect.y;

        // Clamp to the content area to avoid underflow.
        let viewport_col = col.saturating_sub(content_start_col) as usize;
        let viewport_row = row.saturating_sub(content_start_row) as usize;

        let logical_line = self.layout_cache.editor_visible_start + viewport_row;
        let display_col = viewport_col + self.layout_cache.editor_scroll_col;

        if is_drag {
            editor.extend_selection_to_line_display_col(logical_line, display_col, tab_width);
        } else {
            editor.start_selection_at_line_display_col(logical_line, display_col, tab_width);
        }
        Ok(())
    }

    fn dispatch(&mut self, action: Action) -> Result<()> {
        if action == Action::CheckForUpdates {
            let current_version = env!("CARGO_PKG_VERSION").to_string();
            self.state.update_state.set_checking();
            // Use try_send to avoid blocking the UI thread if a check is already in progress.
            // If the bounded queue is full, silently ignore the request; the user can retry.
            let _ = self
                .workers
                .update_check_tx
                .try_send(UpdateCheckRequest::CheckLatestRelease { current_version });
            return Ok(());
        }

        let action_name = format!("{:?}", action);
        for command in self.state.apply(action)? {
            self.execute_command(command)?;
        }
        self.state.debug.record_action(action_name);
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
        let _ = self
            .workers
            .watch_tx
            .try_send(WatchRequest { paths, config_path });
        Ok(())
    }

    /// Shared post-scan completion logic: update file-system watchers and
    /// enqueue directory-size requests when in details view or size sort.
    fn post_scan_completed(
        &mut self,
        workspace_id: usize,
        pane: crate::pane::PaneId,
    ) -> Result<()> {
        self.sync_watched_paths()?;
        let pane_state = self.state.workspace(workspace_id).panes.pane(pane);
        if pane_state.details_view
            || matches!(
                pane_state.sort_mode,
                crate::pane::SortMode::Size | crate::pane::SortMode::SizeDesc
            )
        {
            let entries_snapshot: Vec<_> = pane_state
                .entries
                .iter()
                .filter(|e| e.kind == crate::fs::EntryKind::Directory && e.name != "..")
                .map(|e| e.path.clone())
                .collect();
            for path in entries_snapshot {
                let _ = self.workers.dir_size_tx.try_send(DirSizeRequest {
                    workspace_id,
                    pane,
                    path,
                });
            }
        }
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
                // Preview requests are triggered by cursor movement and can arrive
                // faster than the worker can consume them.  Drop silently when full.
                let _ = self.workers.preview_tx.try_send(PreviewRequest {
                    workspace_id,
                    path,
                    syntect_theme: self.state.theme().palette.syntect_theme.to_string(),
                    archive,
                    inner_path: inner,
                    picker: self.state.image_picker().clone(),
                });
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
                    // For local panes, serve from the scan cache when it is still
                    // fresh (directory mtime unchanged).  Cloning the entries
                    // inside the scoped block ends the immutable borrow before
                    // any mutable state mutation happens below.
                    let cached_entries = {
                        let pane_state = self.state.panes.pane(pane);
                        if !pane_state.in_archive() {
                            pane_state
                                .scan_cache
                                .as_ref()
                                .filter(|cache| cache.is_fresh(&path))
                                .map(|cache| cache.entries.clone())
                        } else {
                            None
                        }
                    };

                    if let Some(entries) = cached_entries {
                        let result = JobResult::DirectoryScanned {
                            workspace_id,
                            pane,
                            path,
                            entries,
                            elapsed_ms: 0,
                        };
                        let hook_cmds = self.state.apply_job_result_commands(result);
                        for cmd in hook_cmds {
                            self.execute_command_try(cmd)?;
                        }
                        self.post_scan_completed(workspace_id, pane)?;
                    } else {
                        let _ = self.workers.scan_tx.try_send(ScanRequest {
                            workspace_id,
                            pane,
                            path: path.clone(),
                        });
                        let _ = self.workers.git_tx.try_send(GitStatusRequest {
                            workspace_id,
                            pane,
                            path,
                        });
                    }
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
                // PTY writes can arrive faster than the worker drains them.  Use
                // try_send so a backlogged terminal worker never stalls the event loop.
                let _ = self
                    .workers
                    .terminal_tx
                    .try_send(crate::jobs::TerminalRequest::Write {
                        workspace_id: self.state.active_workspace_index(),
                        bytes,
                    });
            }
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
            Command::RunHook {
                command,
                env,
                workspace_id,
            } => {
                let result_tx = self.workers.result_tx.clone();
                std::thread::spawn(move || {
                    let status = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&command)
                        .envs(env)
                        .status();
                    if let Err(e) = status {
                        let _ = result_tx.send(crate::jobs::JobResult::JobFailed {
                            workspace_id,
                            pane: crate::pane::PaneId::Left,
                            path: std::path::PathBuf::new(),
                            file_op: None,
                            message: format!("hook failed: {e}"),
                            elapsed_ms: 0,
                        });
                    }
                });
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
    let alt_p = matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'))
        && key_event.modifiers == KeyModifiers::ALT;
    match focus {
        FocusLayer::Modal(ModalKind::Palette) => Action::from_palette_key_event(key_event),
        FocusLayer::Modal(ModalKind::Collision) => Action::from_collision_key_event(key_event),
        FocusLayer::Modal(ModalKind::DestructiveConfirm) => {
            Action::from_destructive_confirm_key_event(key_event)
        }
        FocusLayer::Modal(ModalKind::Prompt) => Action::from_prompt_key_event(key_event),
        FocusLayer::Modal(ModalKind::Dialog) => Action::from_dialog_key_event(key_event),
        FocusLayer::Modal(ModalKind::Menu) => Action::from_menu_key_event(key_event, keymap),
        FocusLayer::Modal(ModalKind::Settings) => {
            Action::from_settings_key_event(key_event, is_settings_rebinding)
        }
        FocusLayer::Modal(ModalKind::Bookmarks) => Action::from_bookmarks_key_event(key_event),
        FocusLayer::Modal(ModalKind::OpenWith) => Action::from_open_with_key_event(key_event),
        FocusLayer::Modal(ModalKind::ContextMenu) => {
            use crossterm::event::{KeyCode, KeyModifiers};
            match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => Some(Action::ContextMenuMoveUp),
                KeyCode::Down | KeyCode::Char('j') => Some(Action::ContextMenuMoveDown),
                KeyCode::Enter => Some(Action::ContextMenuConfirm),
                KeyCode::Esc => Some(Action::CloseContextMenu),
                KeyCode::Char('q') if key_event.modifiers == KeyModifiers::CONTROL => {
                    Some(Action::Quit)
                }
                _ => Some(Action::CloseContextMenu), // any other key closes menu
            }
        }
        FocusLayer::Modal(ModalKind::FileFinder) => Action::from_file_finder_key_event(key_event),
        FocusLayer::Modal(ModalKind::SshConnect) => Action::from_ssh_connect_key_event(key_event),
        FocusLayer::Modal(ModalKind::SshTrustPrompt) => Action::from_ssh_trust_key_event(key_event),
        FocusLayer::Modal(ModalKind::FirstRunWizard) => Action::from_wizard_key_event(key_event),
        FocusLayer::Modal(ModalKind::UpdatePrompt) => match key_event.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                Some(Action::UpdatePromptYes)
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Some(Action::UpdatePromptNo),
            _ => None,
        },
        FocusLayer::PaneFilter => Action::from_pane_filter_key_event(key_event),
        FocusLayer::PaneInlineRename => Action::from_inline_rename_key_event(key_event),
        FocusLayer::Preview => Action::from_preview_key_event(key_event),
        FocusLayer::Terminal => Action::from_terminal_key_event(key_event),
        FocusLayer::MarkdownPreview => {
            if is_preview_open && (alt_f3 || alt_p) {
                return Some(Action::FocusPreviewPanel);
            }
            Action::from_markdown_preview_key_event(key_event)
                .or_else(|| Action::from_editor_key_event(key_event, keymap))
                .or_else(|| Action::from_pane_key_event(key_event, keymap))
        }
        FocusLayer::Editor => {
            if is_preview_open && (alt_f3 || alt_p) {
                return Some(Action::FocusPreviewPanel);
            }
            Action::from_editor_key_event(key_event, keymap)
                .or_else(|| Action::from_pane_key_event(key_event, keymap))
        }
        FocusLayer::GitDiffFileList => Action::from_git_diff_file_list_key_event(&key_event)
            .or_else(|| Action::from_pane_key_event(key_event, keymap)),
        FocusLayer::GitDiffContent => Action::from_git_diff_content_key_event(&key_event)
            .or_else(|| Action::from_pane_key_event(key_event, keymap)),
        FocusLayer::Pane => {
            if is_preview_open && (alt_f3 || alt_p) {
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
    menu_ctx: crate::state::MenuContext,
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
            if matches!(focus, FocusLayer::GitDiffContent) {
                return Some(Action::GitDiffScrollUp);
            }
            if matches!(focus, FocusLayer::GitDiffFileList) {
                return Some(Action::GitDiffSelectPrev);
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
            if matches!(focus, FocusLayer::GitDiffContent) {
                return Some(Action::GitDiffScrollDown);
            }
            if matches!(focus, FocusLayer::GitDiffFileList) {
                return Some(Action::GitDiffSelectNext);
            }
            if rect_contains(cache.left_pane, col, row) || rect_contains(cache.right_pane, col, row)
            {
                return Some(Action::MoveSelectionDown);
            }
            None
        }

        // -------------------------------------------------------------------
        // Right click
        // -------------------------------------------------------------------
        MouseEventKind::Down(MouseButton::Right) => {
            // Only open context menu when pane has focus (not in modals/editor/terminal)
            if matches!(focus, FocusLayer::Pane) {
                return Some(Action::OpenContextMenu { x: col, y: row });
            }
            None
        }

        // -------------------------------------------------------------------
        // Left click
        // -------------------------------------------------------------------
        MouseEventKind::Down(MouseButton::Left) => {
            // Context menu open: any left click closes it
            if matches!(focus, FocusLayer::Modal(ModalKind::ContextMenu)) {
                return Some(Action::CloseContextMenu);
            }

            // Menu open: allow menu bar clicks (switch menus) and popup item clicks.
            if matches!(focus, FocusLayer::Modal(ModalKind::Menu)) {
                if rect_contains(cache.menu_bar, col, row) {
                    return route_menu_bar_click(
                        col,
                        cache.menu_bar.x,
                        menu_ctx,
                        &cache.workspace_pill_rects,
                    );
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
                return route_menu_bar_click(
                    col,
                    cache.menu_bar.x,
                    menu_ctx,
                    &cache.workspace_pill_rects,
                );
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
                    // Click inside the editor viewport: position cursor.
                    return Some(Action::EditorClickAt { col, row });
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
            // Drag inside editor: extend selection.
            if let MouseEventKind::Drag(MouseButton::Left) = event.kind {
                if focus == FocusLayer::Editor {
                    if let Some(editor_rect) = cache.editor_panel {
                        if rect_contains(editor_rect, col, row) {
                            return Some(Action::EditorDragTo { col, row });
                        }
                    }
                }
            }
            None
        }

        _ => None,
    }
}

/// Map an x-coordinate in the menu bar to either an `OpenMenu` action or a
/// workspace switch action using pill rects from the layout cache.
fn route_menu_bar_click(
    col: u16,
    bar_x: u16,
    menu_ctx: crate::state::MenuContext,
    workspace_pill_rects: &[Option<ratatui::layout::Rect>; 4],
) -> Option<Action> {
    // Walk menu tabs on the left side of the bar.
    let mut cursor = bar_x + 8;
    for tab in crate::state::menu_tabs(menu_ctx) {
        let start = cursor;
        let end = cursor + tab.label.len() as u16 - 1;
        if col >= start && col <= end {
            return Some(Action::OpenMenu(tab.id));
        }
        cursor += tab.label.len() as u16;
    }

    // Workspace pills on the right side — use rects computed at render time
    // so positions are exact regardless of terminal width.
    for (idx, rect_opt) in workspace_pill_rects.iter().enumerate() {
        if let Some(rect) = rect_opt {
            if col >= rect.x && col < rect.x + rect.width {
                return Some(Action::SwitchToWorkspace(idx));
            }
        }
    }

    None
}

fn run_update_and_restart(target_tag: Option<&str>) -> Result<()> {
    println!();
    println!("🔄 Installing update from https://github.com/tzero86/Zeta ...");
    println!();

    let mut cargo_args = vec!["install", "--git", "https://github.com/tzero86/Zeta"];
    // Pin to the exact release tag when available so we install a reproducible build.
    if let Some(tag) = target_tag {
        cargo_args.extend_from_slice(&["--tag", tag]);
    }
    cargo_args.push("--locked");

    // On Windows, running cargo install synchronously in the same console session
    // keeps this process alive for the entire build (potentially several minutes).
    // When the process finally exits, Windows Terminal may restart the profile if
    // the session is configured to run zeta directly, or the terminal host may
    // redraw stale content from cargo's progress output — either way the new TUI
    // appears as a ghost frame before the shell prompt is restored.
    //
    // Fix: rename the current exe to free the install target path (the same rename
    // trick as before), then immediately hand off to a new console window and exit.
    // The build runs completely independently; the original terminal session is
    // already idle before cargo even starts, eliminating the race.
    //
    // On Unix, exec() replaces the process image in-place so the shell waits on
    // the same PID and the new binary takes over the terminal cleanly.
    #[cfg(windows)]
    {
        // Renaming is allowed even for a running exe (the OS holds an inode handle,
        // not a path handle), so this frees the destination slot for cargo install.
        let exe = std::env::current_exe().context("could not resolve current executable path")?;
        let bak = exe.with_extension("exe.bak");
        if let Err(e) = std::fs::rename(&exe, &bak) {
            eprintln!(
                "⚠️  Could not rename current exe before update ({e}). \
                 Proceeding anyway — install may fail."
            );
        }

        // Build the command string that the new window will execute.
        // On success it prints a confirmation; on failure it prints instructions.
        // `pause` keeps the window open so the user can read the result.
        let cargo_cmd = format!("cargo {}", cargo_args.join(" "));
        let script = format!(
            "{cargo_cmd} \
            && (echo. & echo [OK] Update installed. Run 'zeta' to start the new version. & pause) \
            || (echo. & echo [FAIL] cargo install failed. & echo. & echo Run: {cargo_cmd} & pause)"
        );

        // Spawn a completely independent console window.  `cmd /c start` launches
        // a new window and exits immediately, so this process is free to exit
        // before cargo build even begins.
        match std::process::Command::new("cmd")
            .args(["/c", "start", "Zeta Update", "cmd.exe", "/k", &script])
            .spawn()
        {
            Ok(_) => {
                println!("🔄 Update started in a new window (\"Zeta Update\").");
                println!("   That window shows build progress and waits for a keypress when done.");
                println!("   Then run 'zeta' to start the updated version.");
                println!();
                // Any leftover .bak that could not be deleted here will be cleaned up
                // by cleanup_update_backup() on the next zeta launch.
                return Ok(());
            }
            Err(e) => {
                // Could not open the update window — restore the backup so the user
                // still has a working binary.
                let _ = std::fs::rename(&bak, &exe);
                return Err(anyhow::anyhow!("failed to open update window: {e}"));
            }
        }
    }

    // Non-Windows path: run cargo install synchronously then exec() into the new binary.
    let status = std::process::Command::new("cargo")
        .args(&cargo_args)
        .status()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "`cargo` was not found in PATH.\n\
                     Install Rust from https://rustup.rs then run:\n\
                     cargo install --git https://github.com/tzero86/Zeta"
                )
            } else {
                anyhow::anyhow!("failed to run cargo install: {}", e)
            }
        })?;

    if status.success() {
        println!();
        println!("✅ Update installed successfully!");
        println!();

        #[cfg(not(windows))]
        {
            println!("   Relaunching Zeta...");
            println!();
            relaunch_self()?;
        }
    } else {
        eprintln!();
        eprintln!(
            "❌ Update failed (cargo install exited with {:?})",
            status.code()
        );
        eprintln!();
        eprintln!("   To install manually, run:");
        eprintln!("   cargo install --git https://github.com/tzero86/Zeta --locked");
        eprintln!();
        return Err(anyhow::anyhow!(
            "cargo install failed with exit code {:?}",
            status.code()
        ));
    }

    Ok(())
}

#[cfg(not(windows))]
fn relaunch_self() -> Result<()> {
    let current_exe = std::env::current_exe()?;

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(&current_exe)
            .args(std::env::args().skip(1))
            .exec(); // replaces the process image in-place (no-return on success)
        Err(anyhow::anyhow!("exec failed: {}", err))
    }

    #[cfg(not(unix))]
    {
        std::process::Command::new(&current_exe)
            .args(std::env::args().skip(1))
            .spawn()?;
        Ok(())
    }
}

/// Remove any leftover `*.exe.bak` files left by a previous interrupted update.
/// Called at startup; failures are silently ignored so a stale backup never
/// prevents the app from launching.
#[cfg(windows)]
pub fn cleanup_update_backup() {
    if let Ok(exe) = std::env::current_exe() {
        let bak = exe.with_extension("exe.bak");
        if bak.exists() {
            let _ = std::fs::remove_file(&bak);
        }
    }
}

struct TerminalSession {
    terminal: TuiTerminal,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("failed to enable raw mode")?;

        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableFocusChange
        )
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
            DisableFocusChange,
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
        // Workspace pills occupy the right side of an 80-column menu bar.
        // With 4 pills each " N " (3 cols wide) plus 1-col gap between = 3+1+3+1+3+1+3 = 15 cols
        // plus 1 leading space. Right-justified in an 80-col bar: pills start at col ~64.
        // For tests, place them at fixed columns:
        //   pill 0: x=65 w=3, pill 1: x=69 w=3, pill 2: x=73 w=3, pill 3: x=77 w=3
        let pill_rects: [Option<ratatui::layout::Rect>; 4] = [
            Some(Rect {
                x: 65,
                y: 0,
                width: 3,
                height: 1,
            }),
            Some(Rect {
                x: 69,
                y: 0,
                width: 3,
                height: 1,
            }),
            Some(Rect {
                x: 73,
                y: 0,
                width: 3,
                height: 1,
            }),
            Some(Rect {
                x: 77,
                y: 0,
                width: 3,
                height: 1,
            }),
        ];
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
            workspace_pill_rects: pill_rects,
            editor_visible_start: 0,
            editor_scroll_col: 0,
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
            crate::state::MenuContext::Pane,
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
            crate::state::MenuContext::Pane,
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
            crate::state::MenuContext::Pane,
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
            crate::state::MenuContext::Pane,
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
            crate::state::MenuContext::Pane,
        );
        assert_eq!(action, Some(Action::OpenMenu(crate::action::MenuId::File)));
    }

    #[test]
    fn route_mouse_left_click_on_workspace_pill_2_switches_workspace() {
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                // pill ws_idx 1 rect: x=69, width=3 → click at col=70
                column: 70,
                row: 0,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Pane,
            crate::state::MenuContext::Pane,
        );
        assert_eq!(action, Some(Action::SwitchToWorkspace(1)));
    }

    #[test]
    fn route_mouse_left_click_on_workspace_pill_4_switches_workspace() {
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                // pill ws_idx 3 rect: x=77, width=3 → click at col=78
                column: 78,
                row: 0,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Pane,
            crate::state::MenuContext::Pane,
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
            crate::state::MenuContext::Pane,
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
            crate::state::MenuContext::Pane,
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
            crate::state::MenuContext::Pane,
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
            crate::state::MenuContext::Pane,
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
            crate::state::MenuContext::Pane,
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
            crate::state::MenuContext::Pane,
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

    #[test]
    fn route_mouse_right_click_in_pane_opens_context_menu() {
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Right),
                column: 10,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Pane,
            crate::state::MenuContext::Pane,
        );
        assert_eq!(action, Some(Action::OpenContextMenu { x: 10, y: 5 }));
    }

    #[test]
    fn route_mouse_right_click_in_modal_does_nothing() {
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Right),
                column: 10,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Editor,
            crate::state::MenuContext::Pane,
        );
        assert_eq!(action, None);
    }

    // --- workspace pill rect-based routing ---------------------------------

    #[test]
    fn route_menu_bar_click_workspace_pill_1_via_rect() {
        // Pill 0 rect: x=65, width=3. Click at col 65 (leftmost cell).
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 65,
                row: 0,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Pane,
            crate::state::MenuContext::Pane,
        );
        assert_eq!(action, Some(Action::SwitchToWorkspace(0)));
    }

    #[test]
    fn route_menu_bar_click_workspace_pill_3_via_rect() {
        // Pill 2 rect: x=73, width=3. Click at col 74 (middle cell).
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 74,
                row: 0,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Pane,
            crate::state::MenuContext::Pane,
        );
        assert_eq!(action, Some(Action::SwitchToWorkspace(2)));
    }

    #[test]
    fn route_menu_bar_click_between_pills_opens_no_workspace() {
        // Col 68 falls in the 1-col gap between pill 0 (x=65..68) and pill 1 (x=69..72).
        // It should NOT trigger SwitchToWorkspace.
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 68,
                row: 0,
                modifiers: KeyModifiers::NONE,
            },
            &test_cache(),
            FocusLayer::Pane,
            crate::state::MenuContext::Pane,
        );
        // Should fall through to None (no menu tab or pill at col 68).
        assert!(
            !matches!(action, Some(Action::SwitchToWorkspace(_))),
            "gap between pills should not route to a workspace switch"
        );
    }

    // --- editor mouse click routing ----------------------------------------

    #[test]
    fn left_click_inside_editor_rect_produces_editor_click_at() {
        use crate::state::FocusLayer;
        let mut cache = test_cache();
        cache.editor_panel = Some(Rect {
            x: 0,
            y: 1,
            width: 80,
            height: 20,
        });
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 20,
                row: 5,
                modifiers: KeyModifiers::NONE,
            },
            &cache,
            FocusLayer::Editor,
            crate::state::MenuContext::Pane,
        );
        assert_eq!(
            action,
            Some(Action::EditorClickAt { col: 20, row: 5 }),
            "click inside editor rect should produce EditorClickAt"
        );
    }

    #[test]
    fn drag_inside_editor_rect_while_editor_focused_produces_drag_to() {
        use crate::state::FocusLayer;
        let mut cache = test_cache();
        cache.editor_panel = Some(Rect {
            x: 0,
            y: 1,
            width: 80,
            height: 20,
        });
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Left),
                column: 25,
                row: 8,
                modifiers: KeyModifiers::NONE,
            },
            &cache,
            FocusLayer::Editor,
            crate::state::MenuContext::Pane,
        );
        assert_eq!(
            action,
            Some(Action::EditorDragTo { col: 25, row: 8 }),
            "left drag inside editor rect should produce EditorDragTo"
        );
    }

    #[test]
    fn drag_outside_editor_rect_does_not_produce_drag_action() {
        use crate::state::FocusLayer;
        let mut cache = test_cache();
        cache.editor_panel = Some(Rect {
            x: 0,
            y: 5,
            width: 80,
            height: 15,
        });
        // row=3 is above the editor rect (y=5).
        let action = route_mouse_event(
            MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Left),
                column: 10,
                row: 3,
                modifiers: KeyModifiers::NONE,
            },
            &cache,
            FocusLayer::Editor,
            crate::state::MenuContext::Pane,
        );
        assert!(
            !matches!(action, Some(Action::EditorDragTo { .. })),
            "drag outside editor rect should not route to EditorDragTo"
        );
    }

    // -----------------------------------------------------------------------
    // FocusGained / full-redraw regression tests
    //
    // The FocusGained event must:
    //   1. Set needs_redraw so the event loop draws on the next tick.
    //   2. Set the full_redraw flag so the draw loop calls terminal.clear()
    //      before rendering, recovering from display corruption that occurs
    //      when the user switches away from and back to the terminal window.
    //
    // The handler in process_next_event does exactly:
    //   self.state.set_full_redraw();
    //   self.state.set_needs_redraw();
    // These unit tests verify that state API contract is solid.
    // -----------------------------------------------------------------------

    #[test]
    fn focus_gained_state_sets_both_flags() {
        use crate::config::AppConfig;
        use crate::state::AppState;
        use std::time::Instant;

        // Build a minimal AppState the same way bootstrap() does.
        let loaded = AppConfig::load_default_location().expect("config must load for test");
        let mut state =
            AppState::bootstrap(loaded, Instant::now()).expect("AppState bootstrap must succeed");

        // Simulate what process_next_event does on Event::FocusGained.
        state.mark_drawn(); // reset from initial needs_redraw=true
        assert!(
            !state.needs_redraw(),
            "baseline: needs_redraw should be false"
        );
        assert!(
            !state.take_full_redraw(),
            "baseline: full_redraw should be false"
        );

        // FocusGained handler
        state.set_full_redraw();
        state.set_needs_redraw();

        assert!(state.needs_redraw(), "FocusGained must set needs_redraw");
        assert!(
            state.take_full_redraw(),
            "FocusGained must set full_redraw so terminal.clear() is called"
        );
    }

    #[test]
    fn no_spurious_full_redraw_on_normal_events() {
        use crate::config::AppConfig;
        use crate::state::AppState;
        use std::time::Instant;

        let loaded = AppConfig::load_default_location().expect("config must load for test");
        let mut state =
            AppState::bootstrap(loaded, Instant::now()).expect("AppState bootstrap must succeed");

        // Key / mouse events must NOT set the full_redraw flag.
        state.mark_drawn();
        state.set_needs_redraw(); // key event
        assert!(
            !state.take_full_redraw(),
            "key/mouse events must not trigger a full terminal clear"
        );
    }

    /// Resize events must trigger a full repaint.  When a window is restored from
    /// minimised the OS emits a Resize event on virtually every terminal/platform.
    /// This is more reliable than FocusGained which requires terminal support for
    /// the EnableFocusChange escape sequence.
    #[test]
    fn resize_event_triggers_full_redraw() {
        use crate::config::AppConfig;
        use crate::state::AppState;
        use std::time::Instant;

        let loaded = AppConfig::load_default_location().expect("config must load for test");
        let mut state =
            AppState::bootstrap(loaded, Instant::now()).expect("AppState bootstrap must succeed");

        state.mark_drawn();
        // Simulate what process_next_event does on Event::Resize.
        state.set_full_redraw();
        state.set_needs_redraw();

        assert!(state.needs_redraw(), "Resize must set needs_redraw");
        assert!(
            state.take_full_redraw(),
            "Resize must set full_redraw so terminal.clear() is called on restore"
        );
    }
}
