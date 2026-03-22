use std::path::Path;
use std::time::Instant;

use anyhow::Result;

use crate::action::{Action, Command};
use crate::config::LoadedConfig;
use crate::fs;
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
            Action::Refresh => {
                let pane = self.focused_pane_id();
                let path = self.active_pane().cwd.clone();
                self.status_message = format!("refreshing {}", path.display());
                self.needs_redraw = true;
                commands.push(Command::ScanPane { pane, path });
            }
            Action::Quit => {
                self.should_quit = true;
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

    pub fn focus(&self) -> PaneId {
        self.focused_pane_id()
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
