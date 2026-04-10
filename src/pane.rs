use std::collections::BTreeSet;
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
    pub marked: BTreeSet<PathBuf>,
    pub filter_query: String,
    pub filter_active: bool,
    // Navigation history
    pub history_back: Vec<PathBuf>, // dirs we came FROM (oldest first)
    pub history_forward: Vec<PathBuf>, // dirs we can go forward to
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
            marked: BTreeSet::new(),
            filter_query: String::new(),
            filter_active: false,
            history_back: Vec::new(),
            history_forward: Vec::new(),
        }
    }

    /// Push current cwd to back-stack and clear forward-stack before navigating.
    pub fn push_history(&mut self) {
        let current = self.cwd.clone();
        self.history_back.push(current);
        if self.history_back.len() > 50 {
            self.history_back.remove(0);
        }
        self.history_forward.clear();
    }

    /// Returns the path to navigate back to, if any.
    pub fn pop_back(&mut self) -> Option<PathBuf> {
        let prev = self.history_back.pop()?;
        self.history_forward.push(self.cwd.clone());
        Some(prev)
    }

    /// Returns the path to navigate forward to, if any.
    pub fn pop_forward(&mut self) -> Option<PathBuf> {
        let next = self.history_forward.pop()?;
        self.history_back.push(self.cwd.clone());
        Some(next)
    }

    pub fn can_go_back(&self) -> bool {
        !self.history_back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.history_forward.is_empty()
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
        let entry_paths: BTreeSet<PathBuf> = self
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect();
        self.marked.retain(|path| entry_paths.contains(path));

        self.clamp_selection();
    }

    pub fn set_show_hidden(&mut self, show_hidden: bool) -> Result<(), FileSystemError> {
        self.show_hidden = show_hidden;
        let entries = scan_directory(&self.cwd)?;
        self.set_entries(entries);
        Ok(())
    }

    pub fn move_selection_down(&mut self) {
        if self.selection + 1 < self.filtered_len() {
            self.selection += 1;
        }
    }

    pub fn move_selection_up(&mut self) {
        self.selection = self.selection.saturating_sub(1);
    }

    pub fn selected_entry(&self) -> Option<&EntryInfo> {
        self.filtered_entries().into_iter().nth(self.selection)
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.selected_entry().map(|entry| entry.path.clone())
    }

    pub fn selected_marked_path(&self) -> Option<PathBuf> {
        self.selected_path()
    }

    pub fn toggle_mark_selected(&mut self) -> Option<bool> {
        let path = self.selected_path()?;
        if self.marked.contains(&path) {
            self.marked.remove(&path);
            Some(false)
        } else {
            self.marked.insert(path);
            Some(true)
        }
    }

    pub fn clear_marks(&mut self) {
        self.marked.clear();
    }

    pub fn is_marked(&self, path: &PathBuf) -> bool {
        self.marked.contains(path)
    }

    pub fn marked_count(&self) -> usize {
        self.marked.len()
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

    pub fn filtered_entries(&self) -> Vec<&EntryInfo> {
        let sorted = self.sorted_entries();
        if self.filter_active && !self.filter_query.is_empty() {
            let query = self.filter_query.to_lowercase();
            sorted
                .into_iter()
                .filter(|entry| entry.name.to_lowercase().contains(&query))
                .collect()
        } else {
            sorted
        }
    }

    pub fn visible_entries(&self, height: usize) -> Vec<EntryInfo> {
        let filtered = self.filtered_entries();
        if height == 0 || filtered.is_empty() {
            return Vec::new();
        }

        let start = self.visible_start_for(height, filtered.len());
        let end = (start + height).min(filtered.len());
        filtered[start..end].iter().map(|e| (*e).clone()).collect()
    }

    pub fn visible_selection(&self, height: usize) -> Option<usize> {
        let count = self.filtered_len();
        if count == 0 || height == 0 {
            return None;
        }

        Some(
            self.selection
                .saturating_sub(self.visible_start_for(height, count)),
        )
    }

    pub fn select_path(&mut self, path: &std::path::Path) -> bool {
        if let Some(index) = self
            .filtered_entries()
            .iter()
            .position(|entry| entry.path == path)
        {
            self.selection = index;
            true
        } else {
            false
        }
    }

    pub fn clear_filter(&mut self) {
        self.filter_active = false;
        self.filter_query.clear();
        self.selection = 0;
        self.scroll_offset = 0;
    }

    pub fn refresh_filter(&mut self) {
        self.clamp_selection();
    }

    fn filtered_len(&self) -> usize {
        self.filtered_entries().len()
    }

    fn clamp_selection(&mut self) {
        let count = self.filtered_len();
        if self.selection >= count {
            self.selection = count.saturating_sub(1);
        }

        if self.scroll_offset > self.selection {
            self.scroll_offset = self.selection;
        }
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
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use crate::fs::{EntryInfo, EntryKind};

    use super::{PaneState, SortMode};

    fn file(name: &str) -> EntryInfo {
        EntryInfo {
            name: name.to_string(),
            path: PathBuf::from(format!("./{name}")),
            kind: EntryKind::File,
            size_bytes: Some(1),
            modified: None,
        }
    }

    fn dir(name: &str) -> EntryInfo {
        EntryInfo {
            name: name.to_string(),
            path: PathBuf::from(format!("./{name}")),
            kind: EntryKind::Directory,
            size_bytes: None,
            modified: None,
        }
    }

    fn pane_with_entries(entries: Vec<EntryInfo>) -> PaneState {
        let mut pane = PaneState::empty("left", PathBuf::from("."));
        pane.entries = entries;
        pane
    }

    fn empty_pane(cwd: &str) -> PaneState {
        PaneState::empty("test", PathBuf::from(cwd))
    }

    #[test]
    fn clamps_selection_at_zero() {
        let mut pane = empty_pane(".");
        pane.move_selection_up();
        assert_eq!(pane.selection, 0);
    }

    #[test]
    fn visible_selection_tracks_scrolled_window() {
        let mut pane = pane_with_entries(
            (0..10)
                .map(|index| EntryInfo {
                    name: format!("item-{index}"),
                    path: PathBuf::from(format!("./item-{index}")),
                    kind: EntryKind::File,
                    size_bytes: Some(index as u64 * 16),
                    modified: None,
                })
                .collect(),
        );
        pane.selection = 7;

        assert_eq!(pane.visible_selection(4), Some(3));
        assert_eq!(pane.visible_entries(4).len(), 4);

        pane.move_selection_up();
        assert_eq!(pane.visible_selection(4), Some(3));
    }

    #[test]
    fn sort_by_name_puts_dirs_first() {
        let pane = pane_with_entries(vec![file("zzz.txt"), dir("aaa"), file("aaa.txt")]);
        let sorted = pane.sorted_entries();
        assert_eq!(sorted[0].kind, EntryKind::Directory);
        assert_eq!(sorted[0].name, "aaa");
        assert_eq!(sorted[1].name, "aaa.txt");
        assert_eq!(sorted[2].name, "zzz.txt");
    }

    #[test]
    fn sort_by_size_desc_orders_largest_first() {
        let mut pane = pane_with_entries(vec![file("small.txt"), file("large.txt"), file("medium.txt")]);
        pane.entries[0].size_bytes = Some(10);
        pane.entries[1].size_bytes = Some(9999);
        pane.entries[2].size_bytes = Some(500);
        pane.sort_mode = SortMode::SizeDesc;

        let sorted = pane.sorted_entries();
        assert_eq!(sorted[0].name, "large.txt");
        assert_eq!(sorted[1].name, "medium.txt");
        assert_eq!(sorted[2].name, "small.txt");
    }

    #[test]
    fn sort_by_extension_groups_by_ext() {
        let mut pane = pane_with_entries(vec![file("b.rs"), file("a.md"), file("a.rs")]);
        pane.sort_mode = SortMode::Extension;
        let sorted = pane.sorted_entries();
        assert_eq!(sorted[0].name, "a.md");
        assert_eq!(sorted[1].name, "a.rs");
        assert_eq!(sorted[2].name, "b.rs");
    }

    #[test]
    fn cycle_sort_mode_wraps_around() {
        let mut mode = SortMode::Name;
        mode = mode.next();
        mode = mode.next();
        mode = mode.next();
        mode = mode.next();
        mode = mode.next();
        mode = mode.next();
        mode = mode.next();
        assert_eq!(mode, SortMode::Name);
    }

    #[test]
    fn toggle_mark_selected_adds_and_removes_mark() {
        let mut pane = pane_with_entries(vec![file("file.txt")]);
        assert_eq!(pane.toggle_mark_selected(), Some(true));
        assert_eq!(pane.marked_count(), 1);
        assert_eq!(pane.toggle_mark_selected(), Some(false));
        assert_eq!(pane.marked_count(), 0);
    }

    #[test]
    fn clear_marks_removes_all() {
        let mut pane = pane_with_entries(vec![file("file.txt")]);
        pane.marked = BTreeSet::from([PathBuf::from("./file.txt")]);
        pane.clear_marks();
        assert_eq!(pane.marked_count(), 0);
    }

    #[test]
    fn set_entries_purges_stale_marks() {
        let mut pane = pane_with_entries(vec![file("file.txt")]);
        pane.marked = BTreeSet::from([PathBuf::from("./file.txt"), PathBuf::from("./missing.txt")]);
        pane.set_entries(vec![file("file.txt")]);
        assert_eq!(pane.marked_count(), 1);
        assert!(pane.is_marked(&PathBuf::from("./file.txt")));
    }

    #[test]
    fn marked_count_tracks_number_of_marks() {
        let mut pane = pane_with_entries(vec![file("a.txt"), file("b.txt")]);
        assert_eq!(pane.marked_count(), 0);
        let _ = pane.toggle_mark_selected();
        pane.move_selection_down();
        let _ = pane.toggle_mark_selected();
        assert_eq!(pane.marked_count(), 2);
    }

    #[test]
    fn filter_active_hides_non_matching_entries() {
        let mut pane = pane_with_entries(vec![file("main.rs"), file("README.md"), file("Cargo.toml")]);
        pane.filter_active = true;
        pane.filter_query = String::from("read");
        let names: Vec<_> = pane.visible_entries(10).into_iter().map(|e| e.name).collect();
        assert_eq!(names, vec![String::from("README.md")]);
    }

    #[test]
    fn filter_empty_query_shows_all_entries() {
        let mut pane = pane_with_entries(vec![file("a.rs"), file("b.rs")]);
        pane.filter_active = true;
        assert_eq!(pane.visible_entries(10).len(), 2);
    }

    #[test]
    fn filter_is_case_insensitive() {
        let mut pane = pane_with_entries(vec![file("README.md"), file("main.rs")]);
        pane.filter_active = true;
        pane.filter_query = String::from("read");
        assert_eq!(pane.selected_entry().map(|e| e.name.as_str()), Some("README.md"));
    }

    #[test]
    fn push_history_records_previous_dir() {
        let mut pane = empty_pane("/home");
        pane.push_history();
        assert_eq!(pane.history_back, vec![PathBuf::from("/home")]);
    }

    #[test]
    fn pop_back_returns_previous_and_moves_current_to_forward() {
        let mut pane = empty_pane("/home");
        pane.push_history();
        pane.cwd = PathBuf::from("/home/user");

        let back = pane.pop_back();
        assert_eq!(back, Some(PathBuf::from("/home")));
        assert_eq!(pane.history_forward, vec![PathBuf::from("/home/user")]);
        assert!(pane.history_back.is_empty());
    }

    #[test]
    fn pop_forward_returns_next_and_moves_current_to_back() {
        let mut pane = empty_pane("/home");
        pane.push_history();
        pane.cwd = PathBuf::from("/home/user");
        let _back = pane.pop_back();
        pane.cwd = PathBuf::from("/home");

        let fwd = pane.pop_forward();
        assert_eq!(fwd, Some(PathBuf::from("/home/user")));
        assert_eq!(pane.history_back, vec![PathBuf::from("/home")]);
        assert!(pane.history_forward.is_empty());
    }

    #[test]
    fn push_history_clears_forward_stack() {
        let mut pane = empty_pane("/home");
        pane.push_history();
        pane.cwd = PathBuf::from("/home/user");
        let _back = pane.pop_back();
        pane.cwd = PathBuf::from("/home");
        pane.push_history();
        assert!(pane.history_forward.is_empty());
    }

    #[test]
    fn history_capped_at_50_entries() {
        let mut pane = empty_pane("/start");
        for i in 0..60 {
            pane.cwd = PathBuf::from(format!("/dir/{i}"));
            pane.push_history();
        }
        assert_eq!(pane.history_back.len(), 50);
    }
}
