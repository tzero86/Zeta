use std::collections::HashMap;

use crate::fs::EntryInfo;

/// Incremental result of comparing two directory entry snapshots.
///
/// Computed by [`compute_scan_diff`] and applied to a [`PaneState`] via
/// `apply_scan_diff` to update the entry list without a wholesale replace.
///
/// [`PaneState`]: crate::pane::PaneState
#[derive(Debug, Default)]
pub struct ScanDiff {
    pub added: Vec<EntryInfo>,
    pub removed: Vec<EntryInfo>,
    pub modified: Vec<EntryInfo>,
}

impl ScanDiff {
    /// Returns `true` when the diff contains no changes.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }
}

/// Compare `old` and `new` entry lists and return the minimal delta.
///
/// Entries are matched by `path`. An entry is:
/// - **added**    — present in `new` but not in `old`
/// - **removed**  — present in `old` but not in `new`
/// - **modified** — present in both but differs in `modified` time, `size_bytes`, or `kind`
///
/// The `".."` parent sentinel is never included in the inputs and must be
/// stripped by the caller before passing in either slice.
pub fn compute_scan_diff(old: &[EntryInfo], new: &[EntryInfo]) -> ScanDiff {
    let old_map: HashMap<&std::path::Path, &EntryInfo> =
        old.iter().map(|e| (e.path.as_path(), e)).collect();
    let new_map: HashMap<&std::path::Path, &EntryInfo> =
        new.iter().map(|e| (e.path.as_path(), e)).collect();

    let mut diff = ScanDiff::default();

    for (&path, &new_entry) in &new_map {
        match old_map.get(path) {
            None => diff.added.push(new_entry.clone()),
            Some(&old_entry) => {
                if old_entry.modified != new_entry.modified
                    || old_entry.size_bytes != new_entry.size_bytes
                    || old_entry.kind != new_entry.kind
                {
                    diff.modified.push(new_entry.clone());
                }
            }
        }
    }

    for (&path, &old_entry) in &old_map {
        if !new_map.contains_key(path) {
            diff.removed.push(old_entry.clone());
        }
    }

    diff
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::EntryKind;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    fn entry(name: &str, kind: EntryKind, size: u64, mtime_offset_secs: u64) -> EntryInfo {
        EntryInfo {
            name: name.to_string(),
            path: PathBuf::from(name),
            kind,
            size_bytes: Some(size),
            modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(mtime_offset_secs)),
            link_target: None,
        }
    }

    #[test]
    fn empty_diff_when_no_changes() {
        let entries = vec![
            entry("a.txt", EntryKind::File, 100, 1000),
            entry("b.txt", EntryKind::File, 200, 2000),
        ];
        let diff = compute_scan_diff(&entries, &entries);
        assert!(diff.is_empty());
    }

    #[test]
    fn detects_added_entry() {
        let old = vec![entry("a.txt", EntryKind::File, 100, 1000)];
        let new = vec![
            entry("a.txt", EntryKind::File, 100, 1000),
            entry("b.txt", EntryKind::File, 200, 2000),
        ];
        let diff = compute_scan_diff(&old, &new);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].name, "b.txt");
        assert!(diff.removed.is_empty());
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn detects_removed_entry() {
        let old = vec![
            entry("a.txt", EntryKind::File, 100, 1000),
            entry("b.txt", EntryKind::File, 200, 2000),
        ];
        let new = vec![entry("a.txt", EntryKind::File, 100, 1000)];
        let diff = compute_scan_diff(&old, &new);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].name, "b.txt");
        assert!(diff.added.is_empty());
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn detects_modified_entry_by_mtime() {
        let old = vec![entry("a.txt", EntryKind::File, 100, 1000)];
        let new = vec![entry("a.txt", EntryKind::File, 100, 9999)];
        let diff = compute_scan_diff(&old, &new);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0].name, "a.txt");
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn detects_modified_entry_by_size() {
        let old = vec![entry("a.txt", EntryKind::File, 100, 1000)];
        let new = vec![entry("a.txt", EntryKind::File, 999, 1000)];
        let diff = compute_scan_diff(&old, &new);
        assert_eq!(diff.modified.len(), 1);
    }
}
