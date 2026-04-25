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
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Conflicted => "U",
            Self::Added => "A",
            Self::Modified => "M",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Untracked => "?",
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

/// A file that has pending git changes, shown in the diff viewer file list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitDiffFile {
    pub path: PathBuf,
    pub status: FileStatus,
    pub added: usize,
    pub removed: usize,
}

/// The semantic kind of a single line in a unified diff.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffLineKind {
    Added,
    Removed,
    Context,
    HunkHeader,
    FileHeader,
}

/// A single parsed line from a unified diff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
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

/// Fetch all files with pending changes (staged + unstaged + untracked).
///
/// Combines `git diff HEAD --numstat` (tracked changes) with
/// `git status --porcelain=v1 -u` (untracked files).
/// Returns `None` when `path` is not inside a repo or `git` is unavailable.
pub fn fetch_diff_files(path: &Path) -> Option<Vec<GitDiffFile>> {
    let root = detect_repo(path)?;

    // Get +/- counts for tracked changes
    let counts = run_git(&root, &["diff", "HEAD", "--numstat"])
        .filter(|o| o.status.success())
        .map(|o| parse_numstat(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default();

    // Get status + untracked from porcelain
    let status_out = [
        &["status", "--porcelain=v1", "-u", "--no-optional-locks"][..],
        &["status", "--porcelain=v1", "-u"][..],
    ]
    .iter()
    .filter_map(|args| run_git(&root, args))
    .find(|o| o.status.success())?;
    let statuses = parse_porcelain(&String::from_utf8_lossy(&status_out.stdout));

    let mut files: Vec<GitDiffFile> = statuses
        .into_iter()
        .map(|(rel_path, status)| {
            let key = rel_path.display().to_string().replace('\\', "/");
            let (added, removed) = counts.get(&key).copied().unwrap_or((0, 0));
            GitDiffFile { path: rel_path, status, added, removed }
        })
        .collect();

    // Sort: conflicted → modified → added → renamed → deleted → untracked; alpha within groups
    files.sort_by(|a, b| {
        status_sort_key(a.status).cmp(&status_sort_key(b.status))
            .then(a.path.cmp(&b.path))
    });

    Some(files)
}

fn status_sort_key(s: FileStatus) -> u8 {
    match s {
        FileStatus::Conflicted => 0,
        FileStatus::Modified   => 1,
        FileStatus::Added      => 2,
        FileStatus::Renamed    => 3,
        FileStatus::Deleted    => 4,
        FileStatus::Untracked  => 5,
    }
}

/// Fetch the unified diff for a single file.
///
/// For tracked files: runs `git diff HEAD -- <path>`.
/// For untracked files: reads the file directly and marks all lines as Added.
///
/// `is_untracked` must be `true` only for `FileStatus::Untracked` files.
/// Staged-but-new files (`FileStatus::Added`) belong in the tracked branch.
pub fn fetch_file_diff(path: &Path, rel_path: &Path, is_untracked: bool) -> Vec<DiffLine> {
    let root = detect_repo(path);

    if is_untracked {
        let full_path = root.map_or_else(|| rel_path.to_path_buf(), |r| r.join(rel_path));
        return match std::fs::read_to_string(&full_path) {
            Ok(content) => content
                .lines()
                .map(|l| DiffLine { kind: DiffLineKind::Added, content: format!("+{l}") })
                .collect(),
            Err(_) => vec![DiffLine {
                kind: DiffLineKind::Context,
                content: String::from("(binary or unreadable file)"),
            }],
        };
    }

    let root = match root {
        Some(r) => r,
        None => return Vec::new(),
    };

    let output = run_git(
        &root,
        &["diff", "HEAD", "--", &rel_path.display().to_string()],
    );

    match output {
        Some(o) if o.status.success() => {
            parse_unified_diff(&String::from_utf8_lossy(&o.stdout))
        }
        _ => Vec::new(),
    }
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

/// Parse `git diff HEAD --numstat` output into a map of path → (added, removed).
/// Pure function — no subprocess calls.
pub(crate) fn parse_numstat(output: &str) -> HashMap<String, (usize, usize)> {
    let mut map = HashMap::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let added = parts[0].parse::<usize>().unwrap_or(0);
        let removed = parts[1].parse::<usize>().unwrap_or(0);
        let path = parts[2].trim().to_string();
        if !path.is_empty() {
            map.insert(path, (added, removed));
        }
    }
    map
}

/// Parse the stdout of `git diff` (unified format) into a vec of typed lines.
/// Pure function — no subprocess calls.
pub(crate) fn parse_unified_diff(output: &str) -> Vec<DiffLine> {
    output
        .lines()
        .map(|line| {
            let kind = if line.starts_with("diff ")
                || line.starts_with("index ")
                || line.starts_with("--- ")
                || line.starts_with("+++ ")
                || line.starts_with("new file mode")
                || line.starts_with("deleted file mode")
                || line.starts_with("old mode")
                || line.starts_with("new mode")
                || line.starts_with("similarity index")
                || line.starts_with("rename from")
                || line.starts_with("rename to")
                || line.starts_with("Binary files")
            {
                DiffLineKind::FileHeader
            } else if line.starts_with("@@") {
                DiffLineKind::HunkHeader
            } else if line.starts_with('+') {
                DiffLineKind::Added
            } else if line.starts_with('-') {
                DiffLineKind::Removed
            } else {
                DiffLineKind::Context
            };
            DiffLine {
                kind,
                content: line.to_string(),
            }
        })
        .collect()
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
        assert_eq!(FileStatus::Modified.symbol(), "M");
        assert_eq!(FileStatus::Untracked.symbol(), "?");
        assert_eq!(FileStatus::Added.symbol(), "A");
        assert_eq!(FileStatus::Deleted.symbol(), "D");
        assert_eq!(FileStatus::Renamed.symbol(), "R");
        assert_eq!(FileStatus::Conflicted.symbol(), "U");
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

    #[test]
    fn diff_line_kind_covers_all_variants() {
        for kind in [
            DiffLineKind::Added,
            DiffLineKind::Removed,
            DiffLineKind::Context,
            DiffLineKind::HunkHeader,
            DiffLineKind::FileHeader,
        ] {
            let _ = match kind {
                DiffLineKind::Added => "+",
                DiffLineKind::Removed => "-",
                DiffLineKind::Context => " ",
                DiffLineKind::HunkHeader => "@",
                DiffLineKind::FileHeader => "~",
            };
        }
    }

    #[test]
    fn git_diff_file_fields_are_accessible() {
        let f = GitDiffFile {
            path: PathBuf::from("src/main.rs"),
            status: FileStatus::Modified,
            added: 5,
            removed: 2,
        };
        assert_eq!(f.added, 5);
        assert_eq!(f.removed, 2);
        assert_eq!(f.status, FileStatus::Modified);
        assert_eq!(f.path, PathBuf::from("src/main.rs"));
    }

    #[test]
    fn parse_numstat_parses_modified_file() {
        let out = "12\t3\tsrc/git.rs\n";
        let map = parse_numstat(out);
        assert_eq!(map.get("src/git.rs"), Some(&(12usize, 3usize)));
    }

    #[test]
    fn parse_numstat_binary_file_gives_zeros() {
        // git outputs "-\t-\tfile.bin" for binary files
        let out = "-\t-\tassets/logo.png\n";
        let map = parse_numstat(out);
        assert_eq!(map.get("assets/logo.png"), Some(&(0usize, 0usize)));
    }

    #[test]
    fn parse_numstat_empty_gives_empty_map() {
        assert!(parse_numstat("").is_empty());
    }

    #[test]
    fn parse_unified_diff_added_line() {
        let out = "+    let x = 1;\n";
        let lines = parse_unified_diff(out);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].kind, DiffLineKind::Added);
    }

    #[test]
    fn parse_unified_diff_removed_line() {
        let out = "-    let x = 0;\n";
        let lines = parse_unified_diff(out);
        assert_eq!(lines[0].kind, DiffLineKind::Removed);
    }

    #[test]
    fn parse_unified_diff_hunk_header() {
        let out = "@@ -1,4 +1,5 @@ fn main() {\n";
        let lines = parse_unified_diff(out);
        assert_eq!(lines[0].kind, DiffLineKind::HunkHeader);
    }

    #[test]
    fn parse_unified_diff_file_header() {
        let out = "diff --git a/src/main.rs b/src/main.rs\n";
        let lines = parse_unified_diff(out);
        assert_eq!(lines[0].kind, DiffLineKind::FileHeader);
    }

    #[test]
    fn parse_unified_diff_context_line() {
        let out = " fn existing_function() {\n";
        let lines = parse_unified_diff(out);
        assert_eq!(lines[0].kind, DiffLineKind::Context);
    }

    #[test]
    fn parse_unified_diff_triple_plus_is_file_header() {
        let out = "+++ b/src/main.rs\n";
        let lines = parse_unified_diff(out);
        assert_eq!(lines[0].kind, DiffLineKind::FileHeader);
    }

    #[test]
    fn parse_unified_diff_triple_minus_is_file_header() {
        let out = "--- a/src/main.rs\n";
        let lines = parse_unified_diff(out);
        assert_eq!(lines[0].kind, DiffLineKind::FileHeader);
    }

    #[test]
    fn parse_unified_diff_new_file_mode_is_file_header() {
        let out = "new file mode 100644\n";
        let lines = parse_unified_diff(out);
        assert_eq!(lines[0].kind, DiffLineKind::FileHeader);
    }

    #[test]
    fn parse_unified_diff_binary_files_is_file_header() {
        let out = "Binary files a/assets/logo.png and b/assets/logo.png differ\n";
        let lines = parse_unified_diff(out);
        assert_eq!(lines[0].kind, DiffLineKind::FileHeader);
    }
}
