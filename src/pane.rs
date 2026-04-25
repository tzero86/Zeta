use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::fs::{scan_directory, EntryInfo, EntryKind, FileSystemError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneId {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaneMode {
    Real,
    Archive {
        source: PathBuf,
        inner_path: PathBuf,
    },
    Remote {
        address: String,
        base_path: PathBuf,
    },
}

/// Buffer for in-place filename editing (T3-3 inline rename).
#[derive(Clone, Debug)]
pub struct InlineRenameState {
    /// Current text in the edit buffer (starts as the entry's name).
    pub buffer: String,
    /// Absolute path of the entry being renamed.
    pub original_path: PathBuf,
}

/// Cached result of the last successful directory scan for one pane.
///
/// Held in `PaneState::scan_cache`. The cache is considered fresh when
/// the directory's modification time has not changed since the scan.
#[derive(Clone, Debug)]
pub struct ScanCache {
    /// The directory that was scanned.
    pub path: PathBuf,
    /// Modification time of `path` at scan time.
    pub dir_mtime: SystemTime,
    /// The raw entries returned by the scan (before ".." is prepended).
    pub entries: Vec<crate::fs::EntryInfo>,
}

impl ScanCache {
    /// Return `true` when the cached entries are still valid for `path`.
    ///
    /// Validity is defined as: the path matches AND the OS-reported
    /// modification time of the directory equals the recorded mtime.
    pub fn is_fresh(&self, path: &std::path::Path) -> bool {
        if self.path != path {
            return false;
        }
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map(|mtime| mtime == self.dir_mtime)
            .unwrap_or(false)
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
    /// Anchor index for Shift+arrow range selection. `None` when no range is active.
    pub mark_anchor: Option<usize>,
    /// When true, the pane renders a flat column view (icon | name | size | date).
    pub details_view: bool,
    /// Active inline rename; drives `FocusLayer::PaneInlineRename`.
    pub rename_state: Option<InlineRenameState>,
    // Navigation history
    pub history_back: Vec<PathBuf>, // dirs we came FROM (oldest first)
    pub history_forward: Vec<PathBuf>, // dirs we can go forward to
    pub(crate) filtered_indices: RefCell<Vec<usize>>,
    pub(crate) cache_dirty: Cell<bool>,
    pub(crate) cache_entry_count: Cell<usize>,
    pub(crate) cache_sort_mode: Cell<SortMode>,
    pub(crate) cache_filter_active: Cell<bool>,
    pub(crate) cache_filter_query: RefCell<String>,
    pub mode: PaneMode, // New: real fs or archive mode
    /// Cached result of the last completed directory scan.
    pub scan_cache: Option<ScanCache>,
}

impl PaneState {
    pub fn in_archive(&self) -> bool {
        matches!(self.mode, PaneMode::Archive { .. })
    }
    pub fn archive_source(&self) -> Option<&PathBuf> {
        match &self.mode {
            PaneMode::Archive { source, .. } => Some(source),
            _ => None,
        }
    }
    pub fn in_remote(&self) -> bool {
        matches!(self.mode, PaneMode::Remote { .. })
    }
    pub fn remote_address(&self) -> Option<&str> {
        match &self.mode {
            PaneMode::Remote { address, .. } => Some(address.as_str()),
            _ => None,
        }
    }
    pub fn empty(title: impl Into<String>, cwd: PathBuf) -> Self {
        Self {
            title: title.into(),
            cwd,
            entries: Vec::new(),
            mode: PaneMode::Real,
            selection: 0,
            scroll_offset: 0,
            show_hidden: false,
            sort_mode: SortMode::Name,
            marked: BTreeSet::new(),
            filter_query: String::new(),
            filter_active: false,
            mark_anchor: None,
            details_view: true,
            rename_state: None,
            history_back: Vec::new(),
            history_forward: Vec::new(),
            scan_cache: None,
            filtered_indices: RefCell::new(Vec::new()),
            cache_dirty: Cell::new(true),
            cache_entry_count: Cell::new(0),
            cache_sort_mode: Cell::new(SortMode::Name),
            cache_filter_active: Cell::new(false),
            cache_filter_query: RefCell::new(String::new()),
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
            // Always keep ".." even when hidden files are off (it starts with '.').
            .filter(|entry| entry.name == ".." || self.show_hidden || !entry.name.starts_with('.'))
            .collect();
        let entry_paths: BTreeSet<PathBuf> = self
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect();
        self.marked.retain(|path| entry_paths.contains(path));

        self.clamp_selection();
    }

    /// Apply an incremental scan diff to the current entry list.
    ///
    /// - Removed entries are dropped (the `".."` sentinel is always kept).
    /// - Modified entries are updated in place.
    /// - Added entries are appended, respecting the `show_hidden` setting.
    /// - Stale marks are purged for removed paths.
    ///
    /// Callers should pass raw entries (without `".."`) from
    /// [`crate::fs::scan_diff::compute_scan_diff`].
    pub fn apply_scan_diff(&mut self, diff: crate::fs::scan_diff::ScanDiff) {
        if diff.is_empty() {
            return;
        }

        let removed_paths: BTreeSet<&std::path::Path> =
            diff.removed.iter().map(|e| e.path.as_path()).collect();

        self.entries
            .retain(|e| e.name == ".." || !removed_paths.contains(e.path.as_path()));

        for removed in &diff.removed {
            self.marked.remove(&removed.path);
        }

        for modified in &diff.modified {
            if let Some(entry) = self.entries.iter_mut().find(|e| e.path == modified.path) {
                *entry = modified.clone();
            }
        }

        for added in diff.added {
            if self.show_hidden || !added.name.starts_with('.') {
                self.entries.push(added);
            }
        }

        self.refresh_filter();
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
        self.ensure_cache();
        let idx = *self.filtered_indices.borrow().get(self.selection)?;
        self.entries.get(idx)
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.selected_entry().map(|entry| entry.path.clone())
    }

    pub fn selected_marked_path(&self) -> Option<PathBuf> {
        self.selected_path()
    }

    pub fn toggle_mark_selected(&mut self) -> Option<bool> {
        let path = self.selected_path()?;
        // ".." is a navigation sentinel — never mark it.
        if self.selected_entry().is_some_and(|e| e.name == "..") {
            return None;
        }
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

    /// Clear the range-selection anchor without touching the mark set.
    pub fn reset_mark_anchor(&mut self) {
        self.mark_anchor = None;
    }

    /// Extend selection downward, marking every entry stepped over.
    /// Sets the anchor on first call; marks anchor + new position.
    pub fn extend_selection_down(&mut self) {
        if self.mark_anchor.is_none() {
            self.mark_anchor = Some(self.selection);
            // Mark the anchor entry.
            if let Some(path) = self.selected_path() {
                if self.selected_entry().is_some_and(|e| e.name != "..") {
                    self.marked.insert(path);
                }
            }
        }
        self.move_selection_down();
        if let Some(path) = self.selected_path() {
            if self.selected_entry().is_some_and(|e| e.name != "..") {
                self.marked.insert(path);
            }
        }
    }

    /// Extend selection upward, marking every entry stepped over.
    /// Sets the anchor on first call; marks anchor + new position.
    pub fn extend_selection_up(&mut self) {
        if self.mark_anchor.is_none() {
            self.mark_anchor = Some(self.selection);
            // Mark the anchor entry.
            if let Some(path) = self.selected_path() {
                if self.selected_entry().is_some_and(|e| e.name != "..") {
                    self.marked.insert(path);
                }
            }
        }
        self.move_selection_up();
        if let Some(path) = self.selected_path() {
            if self.selected_entry().is_some_and(|e| e.name != "..") {
                self.marked.insert(path);
            }
        }
    }

    pub fn is_marked(&self, path: &PathBuf) -> bool {
        self.marked.contains(path)
    }

    pub fn marked_count(&self) -> usize {
        self.marked.len()
    }

    pub fn can_enter_selected(&self) -> bool {
        self.selected_entry().is_some_and(|entry| {
            entry.kind == EntryKind::Directory || entry.kind == EntryKind::Archive
        })
    }

    pub fn parent_path(&self) -> Option<PathBuf> {
        self.cwd.parent().map(PathBuf::from)
    }

    #[cfg(test)]
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
        self.ensure_cache();
        self.filtered_indices
            .borrow()
            .iter()
            .filter_map(|&idx| self.entries.get(idx))
            .collect()
    }

    pub fn visible_entries(&self, height: usize) -> Vec<&EntryInfo> {
        self.ensure_cache();
        let indices = self.filtered_indices.borrow();
        if height == 0 || indices.is_empty() {
            return Vec::new();
        }

        let start = self.visible_start_for(height, indices.len());
        let end = (start + height).min(indices.len());
        indices[start..end]
            .iter()
            .filter_map(|&idx| self.entries.get(idx))
            .collect()
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
        self.ensure_cache();
        if let Some(index) = self
            .filtered_indices
            .borrow()
            .iter()
            .position(|&entry_index| self.entries[entry_index].path == path)
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
        self.invalidate_cache();
    }

    pub fn refresh_filter(&mut self) {
        self.invalidate_cache();
        self.clamp_selection();
    }

    pub fn filtered_len_pub(&self) -> usize {
        self.filtered_len()
    }

    pub fn filtered_count(&self) -> usize {
        self.filtered_len()
    }

    fn filtered_len(&self) -> usize {
        self.ensure_cache();
        self.filtered_indices.borrow().len()
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

    fn invalidate_cache(&self) {
        self.cache_dirty.set(true);
    }

    fn ensure_cache(&self) {
        if self.cache_dirty.get()
            || self.cache_entry_count.get() != self.entries.len()
            || self.cache_sort_mode.get() != self.sort_mode
            || self.cache_filter_active.get() != self.filter_active
            || *self.cache_filter_query.borrow() != self.filter_query
        {
            self.rebuild_cache();
        }
    }

    fn rebuild_cache(&self) {
        let indices: Vec<usize> = (0..self.entries.len()).collect();
        // Always pin ".." at index 0, sort/filter everything else.
        let (parent_indices, mut rest_indices): (Vec<usize>, Vec<usize>) = indices
            .into_iter()
            .partition(|&i| self.entries[i].name == "..");

        rest_indices.sort_by(|&left_idx, &right_idx| {
            let left = &self.entries[left_idx];
            let right = &self.entries[right_idx];
            match self.sort_mode {
                SortMode::Name => dir_first(left, right)
                    .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase())),
                SortMode::NameDesc => dir_first(left, right)
                    .then_with(|| right.name.to_lowercase().cmp(&left.name.to_lowercase())),
                SortMode::Size => dir_first(left, right).then_with(|| {
                    left.size_bytes
                        .unwrap_or(0)
                        .cmp(&right.size_bytes.unwrap_or(0))
                }),
                SortMode::SizeDesc => dir_first(left, right).then_with(|| {
                    right
                        .size_bytes
                        .unwrap_or(0)
                        .cmp(&left.size_bytes.unwrap_or(0))
                }),
                SortMode::Modified => {
                    dir_first(left, right).then_with(|| left.modified.cmp(&right.modified))
                }
                SortMode::ModifiedDesc => {
                    dir_first(left, right).then_with(|| right.modified.cmp(&left.modified))
                }
                SortMode::Extension => dir_first(left, right).then_with(|| {
                    let ext_a = left
                        .path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let ext_b = right
                        .path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    ext_a
                        .cmp(&ext_b)
                        .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                }),
            }
        });

        if self.filter_active && !self.filter_query.is_empty() {
            // ".." is always visible even during filtering.
            rest_indices.retain(|&idx| {
                crate::utils::glob_match::matches(&self.filter_query, &self.entries[idx].name)
            });
        }

        // Prepend ".." (if present) before all other entries.
        let mut indices = parent_indices;
        indices.extend(rest_indices);

        *self.filtered_indices.borrow_mut() = indices;
        self.cache_entry_count.set(self.entries.len());
        self.cache_sort_mode.set(self.sort_mode);
        self.cache_filter_active.set(self.filter_active);
        *self.cache_filter_query.borrow_mut() = self.filter_query.clone();
        self.cache_dirty.set(false);
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
            link_target: None,
        }
    }

    fn dir(name: &str) -> EntryInfo {
        EntryInfo {
            name: name.to_string(),
            path: PathBuf::from(format!("./{name}")),
            kind: EntryKind::Directory,
            size_bytes: None,
            modified: None,
            link_target: None,
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
                    link_target: None,
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
        let mut pane = pane_with_entries(vec![
            file("small.txt"),
            file("large.txt"),
            file("medium.txt"),
        ]);
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
        let mut pane =
            pane_with_entries(vec![file("main.rs"), file("README.md"), file("Cargo.toml")]);
        pane.filter_active = true;
        pane.filter_query = String::from("read");
        let names: Vec<_> = pane
            .visible_entries(10)
            .into_iter()
            .map(|e| e.name.clone())
            .collect();
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
        assert_eq!(
            pane.selected_entry().map(|e| e.name.as_str()),
            Some("README.md")
        );
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
