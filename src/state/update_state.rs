use std::path::PathBuf;
use std::time::SystemTime;

use crate::update::{Release, UpdateStatus};

#[derive(Debug, Clone)]
pub struct UpdateState {
    pub status: UpdateStatus,
    pub available_release: Option<Release>,
    pub last_check_time: Option<SystemTime>,
    pub download_in_progress: bool,
    pub downloaded_binary_path: Option<PathBuf>,
    pub restart_pending: bool,
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
        }
    }
}

impl UpdateState {
    pub fn is_update_available(&self) -> bool {
        matches!(self.status, UpdateStatus::Available(_))
    }

    pub fn set_checking(&mut self) {
        self.status = UpdateStatus::Checking;
    }

    pub fn set_available(&mut self, release: Release) {
        self.available_release = Some(release.clone());
        self.status = UpdateStatus::Available(release);
        self.last_check_time = Some(SystemTime::now());
    }

    pub fn set_current(&mut self) {
        self.status = UpdateStatus::Current;
        self.available_release = None;
        self.last_check_time = Some(SystemTime::now());
    }

    pub fn set_error(&mut self, error: String) {
        self.status = UpdateStatus::Error(error);
        self.last_check_time = Some(SystemTime::now());
    }

    pub fn start_download(&mut self) {
        self.download_in_progress = true;
    }

    pub fn complete_download(&mut self, path: PathBuf) {
        self.download_in_progress = false;
        self.downloaded_binary_path = Some(path);
        self.restart_pending = true;
    }
}
