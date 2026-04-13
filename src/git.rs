use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
            Self::Added => 'A',
            Self::Modified => 'M',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
            Self::Untracked => '?',
        }
    }

    /// Ratatui colour for the indicator.
    pub fn colour(self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Self::Conflicted => Color::Red,
            Self::Added => Color::Green,
            Self::Modified => Color::Yellow,
            Self::Deleted => Color::Red,
            Self::Renamed => Color::Cyan,
            Self::Untracked => Color::DarkGray,
        }
    }
}

/// Snapshot of `git status` for one repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoStatus {
    /// Absolute path to the repository root.
    pub root: PathBuf,
    /// Current branch name (e.g. `"main"`, short hash when detached).
    pub branch: String,
    /// Status of every dirty file, keyed by path relative to `root`.
    pub file_statuses: HashMap<PathBuf, FileStatus>,
    /// Pre-normalized lookup map for O(1) status checks during pane rendering.
    normalized_statuses: HashMap<String, FileStatus>,
}

impl RepoStatus {
    pub fn new(root: PathBuf, branch: String, file_statuses: HashMap<PathBuf, FileStatus>) -> Self {
        let normalized_statuses = file_statuses
            .iter()
            .map(|(path, status)| (normalize_lookup_string(path), *status))
            .collect();
        Self {
            root,
            branch,
            file_statuses,
            normalized_statuses,
        }
    }

    /// Look up the status for an absolute path.
    /// Returns `None` for clean (unmodified) files.
    pub fn status_for(&self, absolute_path: &Path) -> Option<FileStatus> {
        let relative = relative_lookup_path(&self.root, absolute_path)?;
        let wanted = normalize_lookup_string(&relative);
        self.normalized_statuses.get(&wanted).copied()
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
    let output = run_git(path, &["rev-parse", "--show-toplevel"])?;
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

    let output = [
        ["status", "--porcelain=v1", "-u", "--no-optional-locks"].as_slice(),
        ["status", "--porcelain=v1", "-u"].as_slice(),
        ["status", "--porcelain"].as_slice(),
    ]
    .into_iter()
    .filter_map(|args| run_git(&root, args))
    .find(|output| output.status.success());

    let file_statuses = output
        .map(|output| parse_porcelain(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or_default();

    Some(RepoStatus::new(root, branch, file_statuses))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn run_git(cwd: &Path, args: &[&str]) -> Option<Output> {
    if let Ok(output) = Command::new("git").args(args).current_dir(cwd).output() {
        return Some(output);
    }

    #[cfg(windows)]
    {
        for candidate in windows_git_candidates() {
            if let Ok(output) = Command::new(&candidate)
                .args(args)
                .current_dir(cwd)
                .output()
            {
                return Some(output);
            }
        }
    }

    None
}

#[cfg(windows)]
fn windows_git_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(where_out) = Command::new("where.exe").args(["git"]).output() {
        if where_out.status.success() {
            for line in String::from_utf8_lossy(&where_out.stdout).lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    candidates.push(PathBuf::from(trimmed));
                }
            }
        }
    }

    for base in [
        std::env::var_os("ProgramFiles"),
        std::env::var_os("ProgramW6432"),
        std::env::var_os("LocalAppData"),
    ]
    .into_iter()
    .flatten()
    {
        let base = PathBuf::from(base);
        for rel in [
            PathBuf::from("Git\\cmd\\git.exe"),
            PathBuf::from("Git\\bin\\git.exe"),
            PathBuf::from("Programs\\Git\\cmd\\git.exe"),
            PathBuf::from("Programs\\Git\\bin\\git.exe"),
        ] {
            let candidate = base.join(rel);
            if candidate.exists() {
                candidates.push(candidate);
            }
        }
    }

    candidates
}

fn current_branch(repo_root: &Path) -> Option<String> {
    let output = run_git(repo_root, &["symbolic-ref", "--short", "HEAD"])?;
    if output.status.success() {
        return Some(String::from_utf8(output.stdout).ok()?.trim().to_string());
    }
    // Detached HEAD — fall back to short commit hash.
    let hash = run_git(repo_root, &["rev-parse", "--short", "HEAD"])?;
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
        map.insert(normalize_relative_path(Path::new(path)), status);
    }
    map
}

fn normalize_relative_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        normalized.push(component.as_os_str());
    }
    normalized
}

fn relative_lookup_path(root: &Path, absolute_path: &Path) -> Option<PathBuf> {
    if let Ok(relative) = absolute_path.strip_prefix(root) {
        return Some(normalize_relative_path(relative));
    }

    let root_norm = normalize_lookup_string(root);
    let abs_norm = normalize_lookup_string(absolute_path);
    abs_norm.strip_prefix(&(root_norm + "/")).map(PathBuf::from)
}

fn normalize_lookup_string(path: &Path) -> String {
    let value = path.display().to_string().replace('\\', "/");
    #[cfg(windows)]
    {
        value.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        value
    }
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
        let status = RepoStatus::new(PathBuf::from("/repo"), String::from("main"), HashMap::new());
        assert_eq!(status.status_for(Path::new("/repo/clean.rs")), None);
    }

    #[test]
    fn repo_status_for_resolves_absolute_to_relative() {
        let mut file_statuses = HashMap::new();
        file_statuses.insert(PathBuf::from("src/lib.rs"), FileStatus::Modified);
        let status = RepoStatus::new(PathBuf::from("/repo"), String::from("main"), file_statuses);
        assert_eq!(
            status.status_for(Path::new("/repo/src/lib.rs")),
            Some(FileStatus::Modified)
        );
    }
}
