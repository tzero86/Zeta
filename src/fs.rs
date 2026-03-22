use std::fs as std_fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

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

pub fn create_directory(path: &Path) -> Result<(), FileSystemError> {
    std_fs::create_dir(path).map_err(|source| FileSystemError::CreateDir {
        path: path.display().to_string(),
        source,
    })
}

pub fn create_file(path: &Path) -> Result<(), FileSystemError> {
    std_fs::File::create(path).map_err(|source| FileSystemError::CreateFile {
        path: path.display().to_string(),
        source,
    })?;
    Ok(())
}

pub fn rename_path(from: &Path, to: &Path) -> Result<(), FileSystemError> {
    std_fs::rename(from, to).map_err(|source| FileSystemError::RenamePath {
        from: from.display().to_string(),
        to: to.display().to_string(),
        source,
    })
}

pub fn copy_path(from: &Path, to: &Path) -> Result<(), FileSystemError> {
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
            copy_path(&child_from, &child_to)?;
        }

        Ok(())
    } else {
        std_fs::copy(from, to).map_err(|source| FileSystemError::CopyPath {
            from: from.display().to_string(),
            to: to.display().to_string(),
            source,
        })?;
        Ok(())
    }
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

    use super::{scan_directory, EntryKind};

    #[test]
    fn sorts_directories_before_files() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("zeta-fs-test-{unique}"));

        fs::create_dir_all(root.join("aaa-dir")).expect("dir should be created");
        fs::write(root.join("zzz-file.txt"), "demo").expect("file should be created");

        let entries = scan_directory(&root).expect("scan should succeed");

        assert_eq!(entries[0].kind, EntryKind::Directory);
        assert_eq!(entries[1].kind, EntryKind::File);

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }
}
