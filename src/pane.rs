use std::path::PathBuf;

use crate::fs::{scan_directory, EntryInfo, EntryKind, FileSystemError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneId {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortMode {
    Name,
}

#[derive(Clone, Debug)]
pub struct PaneState {
    pub title: String,
    pub cwd: PathBuf,
    pub entries: Vec<EntryInfo>,
    pub selection: usize,
    pub show_hidden: bool,
    pub sort_mode: SortMode,
}

impl PaneState {
    pub fn load(title: impl Into<String>, cwd: PathBuf) -> Result<Self, FileSystemError> {
        let mut pane = Self {
            title: title.into(),
            cwd,
            entries: Vec::new(),
            selection: 0,
            show_hidden: false,
            sort_mode: SortMode::Name,
        };
        let entries = scan_directory(&pane.cwd)?;
        pane.set_entries(entries);
        Ok(pane)
    }

    pub fn set_entries(&mut self, entries: Vec<EntryInfo>) {
        self.entries = entries
            .into_iter()
            .filter(|entry| self.show_hidden || !entry.name.starts_with('.'))
            .collect();

        if self.selection >= self.entries.len() {
            self.selection = self.entries.len().saturating_sub(1);
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.selection + 1 < self.entries.len() {
            self.selection += 1;
        }
    }

    pub fn move_selection_up(&mut self) {
        self.selection = self.selection.saturating_sub(1);
    }

    pub fn selected_entry(&self) -> Option<&EntryInfo> {
        self.entries.get(self.selection)
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.selected_entry().map(|entry| entry.path.clone())
    }

    pub fn can_enter_selected(&self) -> bool {
        self.selected_entry()
            .is_some_and(|entry| entry.kind == EntryKind::Directory)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{PaneState, SortMode};

    #[test]
    fn clamps_selection_at_zero() {
        let mut pane = PaneState {
            title: String::from("left"),
            cwd: PathBuf::from("."),
            entries: Vec::new(),
            selection: 0,
            show_hidden: false,
            sort_mode: SortMode::Name,
        };

        pane.move_selection_up();

        assert_eq!(pane.selection, 0);
    }
}
