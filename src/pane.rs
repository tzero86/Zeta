use std::path::PathBuf;

use crate::fs::{scan_directory, EntryInfo, EntryKind, FileSystemError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneId {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortMode {
    Name,         // dirs first, then files, alphabetical
    NameDesc,     // reverse
    Size,         // smallest first (dirs show as 0)
    SizeDesc,     // largest first
    Modified,     // oldest first
    ModifiedDesc, // newest first
    Extension,    // alphabetical by extension, then name
}

impl SortMode {
    /// Cycle to the next mode (wraps around).
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::NameDesc,
            Self::NameDesc => Self::Size,
            Self::Size => Self::SizeDesc,
            Self::SizeDesc => Self::Modified,
            Self::Modified => Self::ModifiedDesc,
            Self::ModifiedDesc => Self::Extension,
            Self::Extension => Self::Name,
        }
    }

    /// Short label shown in the pane title bar.
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "name \u{2191}",
            Self::NameDesc => "name \u{2193}",
            Self::Size => "size \u{2191}",
            Self::SizeDesc => "size \u{2193}",
            Self::Modified => "date \u{2191}",
            Self::ModifiedDesc => "date \u{2193}",
            Self::Extension => "ext  \u{2191}",
        }
    }
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
    pub fn empty(title: impl Into<String>, cwd: PathBuf) -> Self {
        Self {
            title: title.into(),
            cwd,
            entries: Vec::new(),
            selection: 0,
            scroll_offset: 0,
            show_hidden: false,
            sort_mode: SortMode::Name,
        }
    }

    pub fn load(title: impl Into<String>, cwd: PathBuf) -> Result<Self, FileSystemError> {
        let mut pane = Self::empty(title, cwd);
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

    pub fn set_show_hidden(&mut self, show_hidden: bool) -> Result<(), FileSystemError> {
        self.show_hidden = show_hidden;
        let entries = scan_directory(&self.cwd)?;
        self.set_entries(entries);
        Ok(())
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
        // Selection index follows the sorted view order.
        self.sorted_entries().into_iter().nth(self.selection)
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

    pub fn sorted_entries(&self) -> Vec<&EntryInfo> {
        let mut entries: Vec<&EntryInfo> = self.entries.iter().collect();
        match self.sort_mode {
            SortMode::Name => {
                entries.sort_by(|a, b| {
                    dir_first(a, b).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
            }
            SortMode::NameDesc => {
                entries.sort_by(|a, b| {
                    dir_first(a, b).then_with(|| b.name.to_lowercase().cmp(&a.name.to_lowercase()))
                });
            }
            SortMode::Size => {
                entries.sort_by(|a, b| {
                    dir_first(a, b)
                        .then_with(|| a.size_bytes.unwrap_or(0).cmp(&b.size_bytes.unwrap_or(0)))
                });
            }
            SortMode::SizeDesc => {
                entries.sort_by(|a, b| {
                    dir_first(a, b)
                        .then_with(|| b.size_bytes.unwrap_or(0).cmp(&a.size_bytes.unwrap_or(0)))
                });
            }
            SortMode::Modified => {
                entries.sort_by(|a, b| dir_first(a, b).then_with(|| a.modified.cmp(&b.modified)));
            }
            SortMode::ModifiedDesc => {
                entries.sort_by(|a, b| dir_first(a, b).then_with(|| b.modified.cmp(&a.modified)));
            }
            SortMode::Extension => {
                entries.sort_by(|a, b| {
                    dir_first(a, b).then_with(|| {
                        let ext_a = a
                            .path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let ext_b = b
                            .path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        ext_a
                            .cmp(&ext_b)
                            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                    })
                });
            }
        }
        entries
    }

    pub fn visible_entries(&self, height: usize) -> Vec<EntryInfo> {
        let sorted = self.sorted_entries();
        if height == 0 || sorted.is_empty() {
            return Vec::new();
        }

        let start = self.visible_start_for(height, sorted.len());
        let end = (start + height).min(sorted.len());
        sorted[start..end].iter().map(|e| (*e).clone()).collect()
    }

    pub fn visible_selection(&self, height: usize) -> Option<usize> {
        let count = self.entries.len();
        if count == 0 || height == 0 {
            return None;
        }

        Some(
            self.selection
                .saturating_sub(self.visible_start_for(height, count)),
        )
    }

    fn visible_start_for(&self, height: usize, count: usize) -> usize {
        if self.selection < self.scroll_offset {
            self.selection
        } else if self.selection >= self.scroll_offset + height {
            self.selection + 1 - height
        } else {
            self.scroll_offset.min(count.saturating_sub(height))
        }
    }
}

fn dir_first(a: &EntryInfo, b: &EntryInfo) -> std::cmp::Ordering {
    match (a.kind, b.kind) {
        (EntryKind::Directory, EntryKind::Directory) => std::cmp::Ordering::Equal,
        (EntryKind::Directory, _) => std::cmp::Ordering::Less,
        (_, EntryKind::Directory) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
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
                    size_bytes: Some(index as u64 * 16),
                    modified: None,
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

    #[test]
    fn sort_by_name_puts_dirs_first() {
        let pane = PaneState {
            title: String::from("left"),
            cwd: PathBuf::from("."),
            entries: vec![
                EntryInfo {
                    name: String::from("zzz.txt"),
                    path: PathBuf::from("./zzz.txt"),
                    kind: EntryKind::File,
                    size_bytes: Some(10),
                    modified: None,
                },
                EntryInfo {
                    name: String::from("aaa"),
                    path: PathBuf::from("./aaa"),
                    kind: EntryKind::Directory,
                    size_bytes: None,
                    modified: None,
                },
                EntryInfo {
                    name: String::from("aaa.txt"),
                    path: PathBuf::from("./aaa.txt"),
                    kind: EntryKind::File,
                    size_bytes: Some(5),
                    modified: None,
                },
            ],
            selection: 0,
            scroll_offset: 0,
            show_hidden: false,
            sort_mode: SortMode::Name,
        };

        let sorted = pane.sorted_entries();
        assert_eq!(sorted[0].kind, EntryKind::Directory);
        assert_eq!(sorted[0].name, "aaa");
        assert_eq!(sorted[1].name, "aaa.txt");
        assert_eq!(sorted[2].name, "zzz.txt");
    }

    #[test]
    fn sort_by_size_desc_orders_largest_first() {
        let pane = PaneState {
            title: String::from("left"),
            cwd: PathBuf::from("."),
            entries: vec![
                EntryInfo {
                    name: String::from("small.txt"),
                    path: PathBuf::from("./small.txt"),
                    kind: EntryKind::File,
                    size_bytes: Some(10),
                    modified: None,
                },
                EntryInfo {
                    name: String::from("large.txt"),
                    path: PathBuf::from("./large.txt"),
                    kind: EntryKind::File,
                    size_bytes: Some(9999),
                    modified: None,
                },
                EntryInfo {
                    name: String::from("medium.txt"),
                    path: PathBuf::from("./medium.txt"),
                    kind: EntryKind::File,
                    size_bytes: Some(500),
                    modified: None,
                },
            ],
            selection: 0,
            scroll_offset: 0,
            show_hidden: false,
            sort_mode: SortMode::SizeDesc,
        };

        let sorted = pane.sorted_entries();
        assert_eq!(sorted[0].name, "large.txt");
        assert_eq!(sorted[1].name, "medium.txt");
        assert_eq!(sorted[2].name, "small.txt");
    }

    #[test]
    fn sort_by_extension_groups_by_ext() {
        let pane = PaneState {
            title: String::from("left"),
            cwd: PathBuf::from("."),
            entries: vec![
                EntryInfo {
                    name: String::from("b.rs"),
                    path: PathBuf::from("./b.rs"),
                    kind: EntryKind::File,
                    size_bytes: Some(1),
                    modified: None,
                },
                EntryInfo {
                    name: String::from("a.md"),
                    path: PathBuf::from("./a.md"),
                    kind: EntryKind::File,
                    size_bytes: Some(1),
                    modified: None,
                },
                EntryInfo {
                    name: String::from("a.rs"),
                    path: PathBuf::from("./a.rs"),
                    kind: EntryKind::File,
                    size_bytes: Some(1),
                    modified: None,
                },
            ],
            selection: 0,
            scroll_offset: 0,
            show_hidden: false,
            sort_mode: SortMode::Extension,
        };

        let sorted = pane.sorted_entries();
        // md < rs alphabetically; within same ext, names are sorted
        assert_eq!(sorted[0].name, "a.md");
        assert_eq!(sorted[1].name, "a.rs");
        assert_eq!(sorted[2].name, "b.rs");
    }

    #[test]
    fn cycle_sort_mode_wraps_around() {
        let mut mode = SortMode::Name;
        mode = mode.next(); // NameDesc
        mode = mode.next(); // Size
        mode = mode.next(); // SizeDesc
        mode = mode.next(); // Modified
        mode = mode.next(); // ModifiedDesc
        mode = mode.next(); // Extension
        mode = mode.next(); // back to Name
        assert_eq!(mode, SortMode::Name);
    }
}
