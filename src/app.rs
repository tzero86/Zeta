use std::io::{self, Stdout};
use std::time::Duration;
use std::time::Instant;

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::{Frame, Terminal};

use crate::action::{Action, Command};
use crate::config::{AppConfig, RuntimeKeymap};
use crate::editor::EditorBuffer;
use crate::event::AppEvent;
use crate::jobs::{self, JobRequest, JobResult};
use crate::state::AppState;
use crate::ui;

type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

pub struct App {
    job_requests: Sender<JobRequest>,
    job_results: Receiver<JobResult>,
    keymap: RuntimeKeymap,
    state: AppState,
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
        let (job_requests, job_results) = jobs::spawn_scan_worker();
        let state = AppState::bootstrap(loaded_config, started_at)
            .context("failed to bootstrap application state")?;
        Ok(Self {
            job_requests,
            job_results,
            keymap,
            state,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let mut terminal = TerminalSession::enter()?;

        while !self.state.should_quit() {
            if self.state.needs_redraw() {
                terminal.draw(|frame| ui::render(frame, &self.state))?;
                self.state.mark_drawn();
            }

            self.process_next_event()?;
        }

        Ok(())
    }

    fn process_next_event(&mut self) -> Result<()> {
        let Some(app_event) = self.next_event()? else {
            return Ok(());
        };

        self.handle_event(app_event)?;

        Ok(())
    }

    fn next_event(&mut self) -> Result<Option<AppEvent>> {
        match self.job_results.try_recv() {
            Ok(result) => return Ok(Some(AppEvent::Job(result))),
            Err(TryRecvError::Disconnected) => {
                anyhow::bail!("background scan worker disconnected")
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
            Event::Resize(width, height) => Ok(Some(AppEvent::Resize { width, height })),
            _ => Ok(None),
        }
    }

    fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Input(key_event) => {
                let action = if self.state.is_prompt_open() {
                    Action::from_prompt_key_event(key_event)
                } else if self.state.is_dialog_open() {
                    Action::from_dialog_key_event(key_event)
                } else if self.state.is_menu_open() {
                    Action::from_menu_key_event(key_event)
                } else if self.state.is_editor_focused() {
                    Action::from_editor_key_event(key_event)
                } else {
                    Action::from_key_event(key_event, &self.keymap)
                };

                if let Some(action) = action {
                    self.dispatch(action)?;
                }
            }
            AppEvent::Resize { width, height } => {
                self.dispatch(Action::Resize { width, height })?;
            }
            AppEvent::Job(result) => {
                self.state.apply_job_result(result);
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

    fn execute_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::OpenEditor { path } => match EditorBuffer::open(&path) {
                Ok(editor) => self.state.open_editor(editor),
                Err(error) => self
                    .state
                    .set_error_status(format!("failed to open editor buffer: {error}")),
            },
            Command::ScanPane { pane, path } => self
                .job_requests
                .send(JobRequest::ScanDirectory { pane, path })
                .context("failed to queue background scan job")?,
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

struct TerminalSession {
    terminal: TuiTerminal,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("failed to enable raw mode")?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;

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
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
