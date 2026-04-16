//! Lightweight session state persisted between runs.
//!
//! Writes to `session.toml` alongside the active config file. Failures are
//! non-fatal in both directions — a missing or malformed session file is
//! treated as a first run.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::pane::SortMode;
use crate::state::PaneLayout;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WorkspaceSessionState {
    pub left_cwd: Option<PathBuf>,
    pub right_cwd: Option<PathBuf>,
    pub left_sort: Option<SortMode>,
    pub right_sort: Option<SortMode>,
    pub left_hidden: bool,
    pub right_hidden: bool,
    pub layout: Option<PaneLayout>,
    /// Navigation history for the left pane (oldest first, capped at 50).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left_history: Vec<PathBuf>,
    /// Navigation history for the right pane (oldest first, capped at 50).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub right_history: Vec<PathBuf>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SessionState {
    #[serde(default)]
    pub active_workspace: Option<usize>,
    #[serde(default)]
    pub workspaces: Vec<WorkspaceSessionState>,
    #[serde(default, skip_serializing)]
    pub left_cwd: Option<PathBuf>,
    #[serde(default, skip_serializing)]
    pub right_cwd: Option<PathBuf>,
    #[serde(default, skip_serializing)]
    pub left_sort: Option<SortMode>,
    #[serde(default, skip_serializing)]
    pub right_sort: Option<SortMode>,
    #[serde(default, skip_serializing)]
    pub left_hidden: bool,
    #[serde(default, skip_serializing)]
    pub right_hidden: bool,
    #[serde(default, skip_serializing)]
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

    pub fn workspace(&self, idx: usize) -> Option<WorkspaceSessionState> {
        if let Some(workspace) = self.workspaces.get(idx) {
            return Some(workspace.clone());
        }

        if idx != 0
            && self.left_cwd.is_none()
            && self.right_cwd.is_none()
            && self.left_sort.is_none()
            && self.right_sort.is_none()
            && self.layout.is_none()
            && !self.left_hidden
            && !self.right_hidden
        {
            return None;
        }

        (idx == 0).then(|| WorkspaceSessionState {
            left_cwd: self.left_cwd.clone(),
            right_cwd: self.right_cwd.clone(),
            left_sort: self.left_sort,
            right_sort: self.right_sort,
            left_hidden: self.left_hidden,
            right_hidden: self.right_hidden,
            layout: self.layout,
            left_history: Vec::new(),
            right_history: Vec::new(),
        })
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

#[cfg(test)]
mod tests {
    use super::{SessionState, WorkspaceSessionState};
    use crate::state::PaneLayout;

    #[test]
    fn session_round_trips_multiple_workspaces_and_active_index() {
        let session = SessionState {
            active_workspace: Some(2),
            workspaces: vec![
                WorkspaceSessionState {
                    left_cwd: Some(std::path::PathBuf::from("/repo-a")),
                    right_cwd: Some(std::path::PathBuf::from("/repo-b")),
                    left_sort: None,
                    right_sort: None,
                    left_hidden: false,
                    right_hidden: true,
                    layout: Some(PaneLayout::SideBySide),
                    left_history: Vec::new(),
                    right_history: Vec::new(),
                },
                WorkspaceSessionState::default(),
                WorkspaceSessionState::default(),
                WorkspaceSessionState::default(),
            ],
            ..Default::default()
        };

        let text = toml::to_string(&session).expect("session should serialize");
        let round_trip: SessionState = toml::from_str(&text).expect("session should deserialize");

        assert_eq!(round_trip.active_workspace, Some(2));
        assert_eq!(
            round_trip.workspaces[0].left_cwd,
            Some(std::path::PathBuf::from("/repo-a"))
        );
        assert_eq!(
            round_trip.workspaces[0].right_cwd,
            Some(std::path::PathBuf::from("/repo-b"))
        );
        assert_eq!(
            round_trip.workspaces[0].layout,
            Some(PaneLayout::SideBySide)
        );
    }

    #[test]
    fn legacy_session_fields_migrate_into_first_workspace() {
        let legacy = r#"
left_cwd = "/repo-a"
right_cwd = "/repo-b"
left_hidden = true
right_hidden = false
layout = "SideBySide"
"#;

        let session: SessionState =
            toml::from_str(legacy).expect("legacy session should deserialize");
        let workspace = session
            .workspace(0)
            .expect("legacy workspace should map to index 0");

        assert_eq!(
            workspace.left_cwd,
            Some(std::path::PathBuf::from("/repo-a"))
        );
        assert_eq!(
            workspace.right_cwd,
            Some(std::path::PathBuf::from("/repo-b"))
        );
        assert!(workspace.left_hidden);
        assert!(!workspace.right_hidden);
        assert_eq!(workspace.layout, Some(PaneLayout::SideBySide));
        assert!(session.workspace(1).is_none());
    }

    #[test]
    fn session_round_trips_history() {
        let history = vec![
            std::path::PathBuf::from("/home/user"),
            std::path::PathBuf::from("/tmp"),
        ];
        let session = SessionState {
            active_workspace: Some(0),
            workspaces: vec![WorkspaceSessionState {
                left_cwd: Some(std::path::PathBuf::from("/home/user/docs")),
                left_history: history.clone(),
                right_history: vec![std::path::PathBuf::from("/var")],
                ..Default::default()
            }],
            ..Default::default()
        };

        let text = toml::to_string(&session).expect("should serialize");
        let rt: SessionState = toml::from_str(&text).expect("should deserialize");
        assert_eq!(rt.workspaces[0].left_history, history);
        assert_eq!(
            rt.workspaces[0].right_history,
            vec![std::path::PathBuf::from("/var")]
        );
    }

    #[test]
    fn empty_history_is_omitted_from_serialized_output() {
        let session = SessionState {
            workspaces: vec![WorkspaceSessionState::default()],
            ..Default::default()
        };
        let text = toml::to_string(&session).expect("should serialize");
        assert!(
            !text.contains("left_history"),
            "empty history should be omitted"
        );
    }
}
