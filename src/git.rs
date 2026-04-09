use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Per-file git status, collapsed to the most significant flag.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileStatus {
    /// Merge conflict (both sides modified).
    Conflicted,
    /// Staged new file.
    Added,
    /// Modified in working tree or index.
    Modified,
    /// Deleted.
    Deleted,
    /// Renamed or copied.
    Renamed,
    /// Not tracked by git.
    Untracked,
}

impl FileStatus {
    /// Single-character indicator shown in the pane gutter.
    pub fn symbol(self) -> char {
        match self {
            Self::Conflicted => 'U',
            Self::Added      => 'A',
            Self::Modified   => 'M',
            Self::Deleted    => 'D',
            Self::Renamed    => 'R',
            Self::Untracked  => '?',
        }
    }

    /// Ratatui colour for the indicator.
    pub fn colour(self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Self::Conflicted => Color::Red,
            Self::Added      => Color::Green,
            Self::Modified   => Color::Yellow,
            Self::Deleted    => Color::Red,
            Self::Renamed    => Color::Cyan,
            Self::Untracked  => Color::DarkGray,
        }
    }
}

/// Snapshot of `git status` for one repository.
#[derive(Clone, Debug)]
pub struct RepoStatus {
    /// Absolute path to the repository root.
    pub root: PathBuf,
    /// Current branch name (e.g. `"main"`, short hash when detached).
    pub branch: String,
    /// Status of every dirty file, keyed by path relative to `root`.
    pub file_statuses: HashMap<PathBuf, FileStatus>,
}

impl RepoStatus {
    /// Look up the status for an absolute path.
    /// Returns `None` for clean (unmodified) files.
    pub fn status_for(&self, absolute_path: &Path) -> Option<FileStatus> {
        let relative = absolute_path.strip_prefix(&self.root).ok()?;
        self.file_statuses.get(relative).copied()
    }
}

// ---------------------------------------------------------------------------
// Detection and fetching
// ---------------------------------------------------------------------------

/// Returns the repository root if `path` is inside a git repository.
///
/// Shells out to `git rev-parse --show-toplevel`. Returns `None` if `git` is
/// not on `PATH`, the path is not inside a repo, or any error occurs.
pub fn detect_repo(path: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root = String::from_utf8(output.stdout).ok()?;
    Some(PathBuf::from(root.trim()))
}

/// Fetch a full `RepoStatus` for the repository that contains `path`.
///
/// Returns `None` when `path` is not inside a repo or `git` is unavailable.
pub fn fetch_status(path: &Path) -> Option<RepoStatus> {
    let root = detect_repo(path)?;
    let branch = current_branch(&root).unwrap_or_else(|| String::from("HEAD"));
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "-u", "--no-optional-locks"])
        .current_dir(&root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let file_statuses = parse_porcelain(&text);
    Some(RepoStatus { root, branch, file_statuses })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn current_branch(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if output.status.success() {
        return Some(String::from_utf8(output.stdout).ok()?.trim().to_string());
    }
    // Detached HEAD — fall back to short commit hash.
    let hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if hash.status.success() {
        return Some(String::from_utf8(hash.stdout).ok()?.trim().to_string());
    }
    None
}

/// Parse `git status --porcelain=v1` output into a status map.
///
/// Pure function — no subprocess calls. Exported so it can be unit-tested.
pub fn parse_porcelain(output: &str) -> HashMap<PathBuf, FileStatus> {
    let mut map = HashMap::new();
    for line in output.lines() {
        if line.len() < 4 {
            continue;
        }
        let x = line.chars().next().unwrap_or(' ');
        let y = line.chars().nth(1).unwrap_or(' ');
        let path_part = &line[3..]; // skip "XY " prefix

        // Renames: "R  old -> new" — track the destination name.
        let path = if (x == 'R' || x == 'C') && path_part.contains(" -> ") {
            path_part
                .split(" -> ")
                .nth(1)
                .map(str::trim)
                .unwrap_or(path_part)
        } else {
            path_part.trim()
        };

        let status = classify(x, y);
        map.insert(PathBuf::from(path), status);
    }
    map
}

/// Collapse XY flag pair into a single `FileStatus`.
///
/// Priority: Conflicted > Added > Modified > Deleted > Renamed > Untracked.
fn classify(x: char, y: char) -> FileStatus {
    if x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D') {
        return FileStatus::Conflicted;
    }
    if x == '?' || y == '?' {
        return FileStatus::Untracked;
    }
    if x == 'A' {
        return FileStatus::Added;
    }
    if x == 'R' || y == 'R' || x == 'C' || y == 'C' {
        return FileStatus::Renamed;
    }
    if x == 'D' || y == 'D' {
        return FileStatus::Deleted;
    }
    FileStatus::Modified
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_porcelain_modified_working_tree() {
        let output = " M src/main.rs\n";
        let statuses = parse_porcelain(output);
        assert_eq!(
            statuses.get(Path::new("src/main.rs")),
            Some(&FileStatus::Modified)
        );
    }

    #[test]
    fn parse_porcelain_untracked() {
        let output = "?? new_file.txt\n";
        let statuses = parse_porcelain(output);
        assert_eq!(
            statuses.get(Path::new("new_file.txt")),
            Some(&FileStatus::Untracked)
        );
    }

    #[test]
    fn parse_porcelain_staged_new_file() {
        let output = "A  staged.rs\n";
        let statuses = parse_porcelain(output);
        assert_eq!(
            statuses.get(Path::new("staged.rs")),
            Some(&FileStatus::Added)
        );
    }

    #[test]
    fn parse_porcelain_rename() {
        let output = "R  old.rs -> new.rs\n";
        let statuses = parse_porcelain(output);
        assert_eq!(
            statuses.get(Path::new("new.rs")),
            Some(&FileStatus::Renamed)
        );
    }

    #[test]
    fn parse_porcelain_conflict() {
        let output = "UU conflict.rs\n";
        let statuses = parse_porcelain(output);
        assert_eq!(
            statuses.get(Path::new("conflict.rs")),
            Some(&FileStatus::Conflicted)
        );
    }

    #[test]
    fn parse_porcelain_deletion() {
        let output = " D gone.rs\n";
        let statuses = parse_porcelain(output);
        assert_eq!(
            statuses.get(Path::new("gone.rs")),
            Some(&FileStatus::Deleted)
        );
    }

    #[test]
    fn parse_porcelain_multiple_entries() {
        let output = " M a.rs\n?? b.rs\nA  c.rs\n";
        let statuses = parse_porcelain(output);
        assert_eq!(statuses.len(), 3);
    }

    #[test]
    fn parse_porcelain_empty_gives_no_entries() {
        assert!(parse_porcelain("").is_empty());
    }

    #[test]
    fn file_status_symbols() {
        assert_eq!(FileStatus::Modified.symbol(), 'M');
        assert_eq!(FileStatus::Untracked.symbol(), '?');
        assert_eq!(FileStatus::Added.symbol(), 'A');
        assert_eq!(FileStatus::Deleted.symbol(), 'D');
        assert_eq!(FileStatus::Renamed.symbol(), 'R');
        assert_eq!(FileStatus::Conflicted.symbol(), 'U');
    }

    #[test]
    fn repo_status_for_returns_none_for_clean_file() {
        let status = RepoStatus {
            root: PathBuf::from("/repo"),
            branch: String::from("main"),
            file_statuses: HashMap::new(),
        };
        assert_eq!(status.status_for(Path::new("/repo/clean.rs")), None);
    }

    #[test]
    fn repo_status_for_resolves_absolute_to_relative() {
        let mut file_statuses = HashMap::new();
        file_statuses.insert(PathBuf::from("src/lib.rs"), FileStatus::Modified);
        let status = RepoStatus {
            root: PathBuf::from("/repo"),
            branch: String::from("main"),
            file_statuses,
        };
        assert_eq!(
            status.status_for(Path::new("/repo/src/lib.rs")),
            Some(FileStatus::Modified)
        );
    }
}
