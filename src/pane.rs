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
    pub scroll_offset: usize,
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
            scroll_offset: 0,
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

        if self.scroll_offset > self.selection {
            self.scroll_offset = self.selection;
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

    pub fn parent_path(&self) -> Option<PathBuf> {
        self.cwd.parent().map(PathBuf::from)
    }

    pub fn visible_entries(&self, height: usize) -> &[EntryInfo] {
        if height == 0 || self.entries.is_empty() {
            return &self.entries[0..0];
        }

        let start = self.visible_start(height);
        let end = (start + height).min(self.entries.len());
        &self.entries[start..end]
    }

    pub fn visible_selection(&self, height: usize) -> Option<usize> {
        if self.entries.is_empty() || height == 0 {
            return None;
        }

        Some(self.selection.saturating_sub(self.visible_start(height)))
    }

    fn visible_start(&self, height: usize) -> usize {
        if self.selection < self.scroll_offset {
            self.selection
        } else if self.selection >= self.scroll_offset + height {
            self.selection + 1 - height
        } else {
            self.scroll_offset
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::fs::{EntryInfo, EntryKind};

    use super::{PaneState, SortMode};

    #[test]
    fn clamps_selection_at_zero() {
        let mut pane = PaneState {
            title: String::from("left"),
            cwd: PathBuf::from("."),
            entries: Vec::new(),
            selection: 0,
            scroll_offset: 0,
            show_hidden: false,
            sort_mode: SortMode::Name,
        };

        pane.move_selection_up();

        assert_eq!(pane.selection, 0);
    }

    #[test]
    fn visible_selection_tracks_scrolled_window() {
        let mut pane = PaneState {
            title: String::from("left"),
            cwd: PathBuf::from("."),
            entries: (0..10)
                .map(|index| EntryInfo {
                    name: format!("item-{index}"),
                    path: PathBuf::from(format!("./item-{index}")),
                    kind: EntryKind::File,
                })
                .collect(),
            selection: 7,
            scroll_offset: 0,
            show_hidden: false,
            sort_mode: SortMode::Name,
        };

        assert_eq!(pane.visible_selection(4), Some(3));
        assert_eq!(pane.visible_entries(4).len(), 4);

        pane.move_selection_up();
        assert_eq!(pane.visible_selection(4), Some(3));
    }
}
