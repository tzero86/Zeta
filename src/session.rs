//! Lightweight session state persisted between runs.
//!
//! Writes to `session.toml` alongside the active config file. Failures are
//! non-fatal in both directions — a missing or malformed session file is
//! treated as a first run.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::pane::SortMode;
use crate::state::PaneLayout;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SessionState {
    pub left_cwd: Option<PathBuf>,
    pub right_cwd: Option<PathBuf>,
    pub left_sort: Option<SortMode>,
    pub right_sort: Option<SortMode>,
    pub left_hidden: bool,
    pub right_hidden: bool,
    pub layout: Option<PaneLayout>,
}

impl SessionState {
    /// Derive the session file path from the config file path.
    pub fn session_path(config_path: &Path) -> PathBuf {
        config_path.with_file_name("session.toml")
    }

    /// Load from disk, returning a default value on any error.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Persist to disk. Non-fatal — caller should log or ignore errors.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let content =
            toml::to_string_pretty(self).map_err(|e| std::io::Error::other(e.to_string()))?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, content)
    }
}
