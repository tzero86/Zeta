use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use crate::update::{Release, UpdateStatus};

#[derive(Debug, Clone)]
pub struct UpdateState {
    pub status: UpdateStatus,
    pub available_release: Option<Release>,
    pub last_check_time: Option<SystemTime>,
    pub download_in_progress: bool,
    pub downloaded_binary_path: Option<PathBuf>,
    pub restart_pending: bool,
    /// When a check completes, show a notification in the status bar for ~3 seconds
    pub notification_shown_at: Option<Instant>,
    /// Whether to run `cargo install` on app exit.
    pub install_on_exit: bool,
    /// Whether the on-exit update confirmation prompt is currently open.
    pub prompt_open: bool,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            status: UpdateStatus::Current,
            available_release: None,
            last_check_time: None,
            download_in_progress: false,
            downloaded_binary_path: None,
            restart_pending: false,
            notification_shown_at: None,
            install_on_exit: false,
            prompt_open: false,
        }
    }
}

impl UpdateState {
    pub fn is_update_available(&self) -> bool {
        matches!(self.status, UpdateStatus::Available(_))
    }

    pub fn set_checking(&mut self) {
        self.status = UpdateStatus::Checking;
        self.notification_shown_at = Some(Instant::now());
    }

    pub fn set_available(&mut self, release: Release) {
        self.available_release = Some(release.clone());
        self.status = UpdateStatus::Available(release);
        self.last_check_time = Some(SystemTime::now());
        self.notification_shown_at = Some(Instant::now());
    }

    pub fn set_current(&mut self) {
        self.status = UpdateStatus::Current;
        self.available_release = None;
        self.last_check_time = Some(SystemTime::now());
        self.notification_shown_at = Some(Instant::now());
    }

    pub fn set_error(&mut self, error: String) {
        self.status = UpdateStatus::Error(error);
        self.last_check_time = Some(SystemTime::now());
        self.notification_shown_at = Some(Instant::now());
    }

    pub fn start_download(&mut self) {
        self.download_in_progress = true;
    }

    pub fn complete_download(&mut self, path: PathBuf) {
        self.download_in_progress = false;
        self.downloaded_binary_path = Some(path);
        self.restart_pending = true;
    }

    /// Check if notification should be shown (expires after ~3 seconds)
    pub fn should_show_notification(&self) -> bool {
        if let Some(shown_at) = self.notification_shown_at {
            shown_at.elapsed().as_secs() < 3
        } else {
            false
        }
    }

    /// Returns true if an update is available and not yet scheduled.
    pub fn can_schedule_install(&self) -> bool {
        matches!(self.status, UpdateStatus::Available(_)) && !self.install_on_exit
    }

    /// Schedule install on exit and mark prompt closed.
    pub fn schedule_install(&mut self) {
        self.install_on_exit = true;
        self.prompt_open = false;
    }

    /// Open the update prompt (called when user triggers ApplyUpdate or on Quit with update pending).
    pub fn open_prompt(&mut self) {
        self.prompt_open = true;
    }

    /// Close the prompt without scheduling.
    pub fn dismiss_prompt(&mut self) {
        self.prompt_open = false;
    }
}
