use std::collections::HashMap;

use ratatui::style::Color;

use crate::fs::EntryInfo;

// ---------------------------------------------------------------------------
// Diff status
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffStatus {
    LeftOnly,
    RightOnly,
    Same,
    Different,
}

impl DiffStatus {
    /// Colour to apply to the entry name in the given pane.
    pub fn colour(self, is_left_pane: bool) -> Color {
        match self {
            Self::Same => Color::Reset,
            Self::Different => Color::Yellow,
            Self::LeftOnly => {
                if is_left_pane {
                    Color::Green
                } else {
                    Color::Red
                }
            }
            Self::RightOnly => {
                if is_left_pane {
                    Color::Red
                } else {
                    Color::Green
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Diff computation
// ---------------------------------------------------------------------------

/// Compare two directory listings by filename.
/// Files are considered the same when both size and modified timestamp match.
/// Directories are compared by name only.
pub fn compute_diff(left: &[EntryInfo], right: &[EntryInfo]) -> HashMap<String, DiffStatus> {
    let left_map: HashMap<&str, &EntryInfo> = left.iter().map(|e| (e.name.as_str(), e)).collect();
    let right_map: HashMap<&str, &EntryInfo> = right.iter().map(|e| (e.name.as_str(), e)).collect();

    let mut result = HashMap::new();

    for (name, l_entry) in &left_map {
        let status = match right_map.get(name) {
            None => DiffStatus::LeftOnly,
            Some(r_entry) => {
                if entries_match(l_entry, r_entry) {
                    DiffStatus::Same
                } else {
                    DiffStatus::Different
                }
            }
        };
        result.insert((*name).to_string(), status);
    }

    for name in right_map.keys() {
        if !left_map.contains_key(name) {
            result.insert((*name).to_string(), DiffStatus::RightOnly);
        }
    }

    result
}

/// Returns a brief human-readable summary for the pane title.
pub fn diff_summary(map: &HashMap<String, DiffStatus>) -> String {
    let same = map.values().filter(|&&s| s == DiffStatus::Same).count();
    let diff = map
        .values()
        .filter(|&&s| s == DiffStatus::Different)
        .count();
    let left = map.values().filter(|&&s| s == DiffStatus::LeftOnly).count();
    let right = map
        .values()
        .filter(|&&s| s == DiffStatus::RightOnly)
        .count();
    format!("diff: {same} same · {diff} diff · {left}← · {right}→")
}

fn entries_match(a: &EntryInfo, b: &EntryInfo) -> bool {
    use crate::fs::EntryKind;
    // Directories: match by name only (already guaranteed by the outer loop key).
    if a.kind == EntryKind::Directory {
        return b.kind == EntryKind::Directory;
    }
    // Files: compare size and modified timestamp.
    a.size_bytes == b.size_bytes && a.modified == b.modified
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::fs::{EntryInfo, EntryKind};

    use super::*;

    fn file(name: &str, size: u64) -> EntryInfo {
        EntryInfo {
            name: name.to_string(),
            path: PathBuf::from(name),
            kind: EntryKind::File,
            size_bytes: Some(size),
            modified: None,
        }
    }

    fn dir(name: &str) -> EntryInfo {
        EntryInfo {
            name: name.to_string(),
            path: PathBuf::from(name),
            kind: EntryKind::Directory,
            size_bytes: None,
            modified: None,
        }
    }

    #[test]
    fn compute_diff_left_only() {
        let diff = compute_diff(&[file("a.txt", 100)], &[]);
        assert_eq!(diff["a.txt"], DiffStatus::LeftOnly);
    }

    #[test]
    fn compute_diff_right_only() {
        let diff = compute_diff(&[], &[file("b.txt", 200)]);
        assert_eq!(diff["b.txt"], DiffStatus::RightOnly);
    }

    #[test]
    fn compute_diff_same_entry() {
        let entry = file("c.txt", 300);
        let diff = compute_diff(std::slice::from_ref(&entry), std::slice::from_ref(&entry));
        assert_eq!(diff["c.txt"], DiffStatus::Same);
    }

    #[test]
    fn compute_diff_different_size() {
        let diff = compute_diff(&[file("d.txt", 100)], &[file("d.txt", 200)]);
        assert_eq!(diff["d.txt"], DiffStatus::Different);
    }

    #[test]
    fn compute_diff_symmetric_count() {
        let left = vec![file("a.txt", 0), file("b.txt", 0)];
        let right = vec![file("b.txt", 0), file("c.txt", 0)];
        let diff = compute_diff(&left, &right);
        assert_eq!(diff.len(), 3);
    }

    #[test]
    fn compute_diff_directories_match_by_name() {
        let diff = compute_diff(&[dir("src")], &[dir("src")]);
        assert_eq!(diff["src"], DiffStatus::Same);
    }

    #[test]
    fn diff_colour_left_only_in_left_pane_is_green() {
        assert_eq!(
            DiffStatus::LeftOnly.colour(true),
            ratatui::style::Color::Green
        );
    }

    #[test]
    fn diff_colour_left_only_in_right_pane_is_red() {
        assert_eq!(
            DiffStatus::LeftOnly.colour(false),
            ratatui::style::Color::Red
        );
    }

    #[test]
    fn diff_colour_different_is_yellow() {
        assert_eq!(
            DiffStatus::Different.colour(true),
            ratatui::style::Color::Yellow
        );
        assert_eq!(
            DiffStatus::Different.colour(false),
            ratatui::style::Color::Yellow
        );
    }

    #[test]
    fn diff_colour_same_is_reset() {
        assert_eq!(DiffStatus::Same.colour(true), ratatui::style::Color::Reset);
    }
}
