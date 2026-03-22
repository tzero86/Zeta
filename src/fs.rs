use std::fs as std_fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::action::CollisionPolicy;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryInfo {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
    pub size_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EntryKind {
    Directory,
    File,
    Symlink,
    Other,
}

impl EntryKind {
    pub fn symbol(self) -> char {
        match self {
            Self::Directory => 'd',
            Self::File => 'f',
            Self::Symlink => 'l',
            Self::Other => '?',
        }
    }

    pub fn ascii_label(self) -> &'static str {
        match self {
            Self::Directory => "[D]",
            Self::File => "[F]",
            Self::Symlink => "[L]",
            Self::Other => "[?]",
        }
    }
}

#[derive(Debug, Error)]
pub enum FileSystemError {
    #[error("failed to determine current working directory: {source}")]
    CurrentDir { source: io::Error },
    #[error("failed to read directory {path}: {source}")]
    ReadDir { path: String, source: io::Error },
    #[error("failed to inspect entry in {path}: {source}")]
    ReadEntryType { path: String, source: io::Error },
    #[error("destination already exists: {path}")]
    PathExists { path: String },
    #[error("failed to create directory {path}: {source}")]
    CreateDir { path: String, source: io::Error },
    #[error("failed to create file {path}: {source}")]
    CreateFile { path: String, source: io::Error },
    #[error("failed to copy {from} to {to}: {source}")]
    CopyPath {
        from: String,
        to: String,
        source: io::Error,
    },
    #[error("failed to rename {from} to {to}: {source}")]
    RenamePath {
        from: String,
        to: String,
        source: io::Error,
    },
    #[error("failed to delete {path}: {source}")]
    DeletePath { path: String, source: io::Error },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationProgress {
    pub completed: u64,
    pub total: u64,
    pub current_path: PathBuf,
}

pub fn current_dir() -> Result<PathBuf, FileSystemError> {
    std::env::current_dir().map_err(|source| FileSystemError::CurrentDir { source })
}

pub fn scan_directory(path: &Path) -> Result<Vec<EntryInfo>, FileSystemError> {
    let entries = std_fs::read_dir(path).map_err(|source| FileSystemError::ReadDir {
        path: path.display().to_string(),
        source,
    })?;

    let mut results = Vec::new();

    for entry_result in entries {
        let entry = entry_result.map_err(|source| FileSystemError::ReadDir {
            path: path.display().to_string(),
            source,
        })?;

        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| FileSystemError::ReadEntryType {
                path: entry_path.display().to_string(),
                source,
            })?;

        let kind = if file_type.is_dir() {
            EntryKind::Directory
        } else if file_type.is_file() {
            EntryKind::File
        } else if file_type.is_symlink() {
            EntryKind::Symlink
        } else {
            EntryKind::Other
        };
        let size_bytes = if file_type.is_file() {
            entry.metadata().ok().map(|metadata| metadata.len())
        } else {
            None
        };

        results.push(EntryInfo {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry_path,
            kind,
            size_bytes,
        });
    }

    results.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    Ok(results)
}

pub fn create_directory(path: &Path, collision: CollisionPolicy) -> Result<(), FileSystemError> {
    prepare_destination(path, collision)?;
    std_fs::create_dir(path).map_err(|source| FileSystemError::CreateDir {
        path: path.display().to_string(),
        source,
    })
}

pub fn create_file(path: &Path, collision: CollisionPolicy) -> Result<(), FileSystemError> {
    prepare_destination(path, collision)?;
    std_fs::File::create(path).map_err(|source| FileSystemError::CreateFile {
        path: path.display().to_string(),
        source,
    })?;
    Ok(())
}

pub fn rename_path(
    from: &Path,
    to: &Path,
    collision: CollisionPolicy,
) -> Result<(), FileSystemError> {
    if from != to {
        prepare_destination(to, collision)?;
    }
    std_fs::rename(from, to).map_err(|source| FileSystemError::RenamePath {
        from: from.display().to_string(),
        to: to.display().to_string(),
        source,
    })
}

pub fn copy_path(
    from: &Path,
    to: &Path,
    collision: CollisionPolicy,
) -> Result<(), FileSystemError> {
    copy_path_with_progress(from, to, collision, &mut |_| {})
}

pub fn copy_path_with_progress<F>(
    from: &Path,
    to: &Path,
    collision: CollisionPolicy,
    on_progress: &mut F,
) -> Result<(), FileSystemError>
where
    F: FnMut(OperationProgress),
{
    prepare_destination(to, collision)?;
    let total = count_path_entries(from)?;
    let mut completed = 0;

    on_progress(OperationProgress {
        completed,
        total,
        current_path: from.to_path_buf(),
    });

    copy_path_recursive(from, to, total, &mut completed, on_progress)
}

fn copy_path_recursive<F>(
    from: &Path,
    to: &Path,
    total: u64,
    completed: &mut u64,
    on_progress: &mut F,
) -> Result<(), FileSystemError>
where
    F: FnMut(OperationProgress),
{
    let metadata = std_fs::symlink_metadata(from).map_err(|source| FileSystemError::CopyPath {
        from: from.display().to_string(),
        to: to.display().to_string(),
        source,
    })?;

    if metadata.is_dir() {
        std_fs::create_dir_all(to).map_err(|source| FileSystemError::CopyPath {
            from: from.display().to_string(),
            to: to.display().to_string(),
            source,
        })?;

        *completed += 1;
        on_progress(OperationProgress {
            completed: *completed,
            total,
            current_path: from.to_path_buf(),
        });

        for entry in std_fs::read_dir(from).map_err(|source| FileSystemError::CopyPath {
            from: from.display().to_string(),
            to: to.display().to_string(),
            source,
        })? {
            let entry = entry.map_err(|source| FileSystemError::CopyPath {
                from: from.display().to_string(),
                to: to.display().to_string(),
                source,
            })?;
            let child_from = entry.path();
            let child_to = to.join(entry.file_name());
            copy_path_recursive(&child_from, &child_to, total, completed, on_progress)?;
        }

        Ok(())
    } else {
        std_fs::copy(from, to).map_err(|source| FileSystemError::CopyPath {
            from: from.display().to_string(),
            to: to.display().to_string(),
            source,
        })?;

        *completed += 1;
        on_progress(OperationProgress {
            completed: *completed,
            total,
            current_path: from.to_path_buf(),
        });

        Ok(())
    }
}

pub fn count_path_entries(path: &Path) -> Result<u64, FileSystemError> {
    let metadata = std_fs::symlink_metadata(path).map_err(|source| FileSystemError::CopyPath {
        from: path.display().to_string(),
        to: path.display().to_string(),
        source,
    })?;

    if !metadata.is_dir() {
        return Ok(1);
    }

    let mut total = 1;
    for entry in std_fs::read_dir(path).map_err(|source| FileSystemError::CopyPath {
        from: path.display().to_string(),
        to: path.display().to_string(),
        source,
    })? {
        let entry = entry.map_err(|source| FileSystemError::CopyPath {
            from: path.display().to_string(),
            to: path.display().to_string(),
            source,
        })?;
        total += count_path_entries(&entry.path())?;
    }

    Ok(total)
}

fn prepare_destination(path: &Path, collision: CollisionPolicy) -> Result<(), FileSystemError> {
    if !path.exists() {
        return Ok(());
    }

    match collision {
        CollisionPolicy::Fail => Err(FileSystemError::PathExists {
            path: path.display().to_string(),
        }),
        CollisionPolicy::Overwrite => delete_path(path),
    }
}

pub fn suggest_non_conflicting_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .or_else(|| path.file_name().and_then(|value| value.to_str()))
        .unwrap_or("untitled");
    let ext = path.extension().and_then(|value| value.to_str());

    for index in 1.. {
        let candidate = match ext {
            Some(ext) if !ext.is_empty() => parent.join(format!("{stem}-{index}.{ext}")),
            _ => parent.join(format!("{stem}-{index}")),
        };

        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("rename suggestion loop should always terminate")
}

pub fn delete_path(path: &Path) -> Result<(), FileSystemError> {
    let metadata =
        std_fs::symlink_metadata(path).map_err(|source| FileSystemError::DeletePath {
            path: path.display().to_string(),
            source,
        })?;

    let result = if metadata.is_dir() {
        std_fs::remove_dir_all(path)
    } else {
        std_fs::remove_file(path)
    };

    result.map_err(|source| FileSystemError::DeletePath {
        path: path.display().to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::action::CollisionPolicy;

    use super::{
        copy_path, copy_path_with_progress, count_path_entries, create_file, rename_path,
        scan_directory, suggest_non_conflicting_path, EntryKind, FileSystemError,
    };

    fn temp_root() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("zeta-fs-test-{unique}"))
    }

    #[test]
    fn sorts_directories_before_files() {
        let root = temp_root();

        fs::create_dir_all(root.join("aaa-dir")).expect("dir should be created");
        fs::write(root.join("zzz-file.txt"), "demo").expect("file should be created");

        let entries = scan_directory(&root).expect("scan should succeed");

        assert_eq!(entries[0].kind, EntryKind::Directory);
        assert_eq!(entries[1].kind, EntryKind::File);

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn create_file_rejects_existing_destination() {
        let root = temp_root();
        fs::create_dir_all(&root).expect("temp dir should be created");
        let path = root.join("note.txt");
        fs::write(&path, "demo").expect("file should be created");

        let error =
            create_file(&path, CollisionPolicy::Fail).expect_err("existing path should fail");
        assert!(matches!(error, FileSystemError::PathExists { .. }));

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn copy_path_rejects_existing_destination() {
        let root = temp_root();
        fs::create_dir_all(&root).expect("temp dir should be created");
        let source = root.join("source.txt");
        let destination = root.join("destination.txt");
        fs::write(&source, "demo").expect("source should be created");
        fs::write(&destination, "existing").expect("destination should be created");

        let error = copy_path(&source, &destination, CollisionPolicy::Fail)
            .expect_err("existing dest should fail");
        assert!(matches!(error, FileSystemError::PathExists { .. }));

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn rename_path_rejects_existing_destination() {
        let root = temp_root();
        fs::create_dir_all(&root).expect("temp dir should be created");
        let source = root.join("source.txt");
        let destination = root.join("destination.txt");
        fs::write(&source, "demo").expect("source should be created");
        fs::write(&destination, "existing").expect("destination should be created");

        let error = rename_path(&source, &destination, CollisionPolicy::Fail)
            .expect_err("existing dest should fail");
        assert!(matches!(error, FileSystemError::PathExists { .. }));

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn count_path_entries_counts_directories_and_files() {
        let root = temp_root();
        let source = root.join("source");
        fs::create_dir_all(source.join("nested")).expect("source tree should be created");
        fs::write(source.join("alpha.txt"), "alpha").expect("file should be created");
        fs::write(source.join("nested").join("beta.txt"), "beta")
            .expect("nested file should be created");

        let total = count_path_entries(&source).expect("count should succeed");

        assert_eq!(total, 4);

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn copy_path_with_progress_reports_completed_entries() {
        let root = temp_root();
        let source = root.join("source");
        let destination = root.join("destination");
        fs::create_dir_all(source.join("nested")).expect("source tree should be created");
        fs::write(source.join("alpha.txt"), "alpha").expect("file should be created");
        fs::write(source.join("nested").join("beta.txt"), "beta")
            .expect("nested file should be created");

        let mut updates = Vec::new();
        copy_path_with_progress(
            &source,
            &destination,
            CollisionPolicy::Fail,
            &mut |progress| {
                updates.push((
                    progress.completed,
                    progress.total,
                    progress
                        .current_path
                        .file_name()
                        .map(|value| value.to_os_string()),
                ));
            },
        )
        .expect("copy should succeed");

        assert_eq!(
            updates.first().map(|value| (value.0, value.1)),
            Some((0, 4))
        );
        assert_eq!(updates.last().map(|value| (value.0, value.1)), Some((4, 4)));
        assert!(destination.join("alpha.txt").exists());
        assert!(destination.join("nested").join("beta.txt").exists());

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn overwrite_replaces_existing_destination() {
        let root = temp_root();
        fs::create_dir_all(&root).expect("temp dir should be created");
        let source = root.join("source.txt");
        let destination = root.join("destination.txt");
        fs::write(&source, "new").expect("source should be created");
        fs::write(&destination, "old").expect("destination should be created");

        copy_path(&source, &destination, CollisionPolicy::Overwrite)
            .expect("overwrite copy should succeed");

        assert_eq!(
            fs::read_to_string(&destination).expect("destination should exist"),
            "new"
        );

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn suggest_non_conflicting_path_increments_suffix() {
        let root = temp_root();
        fs::create_dir_all(&root).expect("temp dir should be created");
        let original = root.join("note.txt");
        fs::write(&original, "demo").expect("original should be created");
        fs::write(root.join("note-1.txt"), "demo").expect("first collision should be created");

        assert_eq!(
            suggest_non_conflicting_path(&original),
            root.join("note-2.txt")
        );

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }
}
