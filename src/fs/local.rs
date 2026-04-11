use std::path::Path;

use crate::action::CollisionPolicy;
use crate::fs::{EntryInfo, FileSystemError};
use crate::fs::backend::{FsBackend, CopyProgress};

/// Local filesystem backend that delegates to existing fs functions
pub struct LocalBackend;

impl FsBackend for LocalBackend {
    fn scan_directory(&self, path: &Path) -> Result<Vec<EntryInfo>, FileSystemError> {
        crate::fs::scan_directory(path)
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>, FileSystemError> {
        std::fs::read(path).map_err(|source| FileSystemError::ReadEntryType {
            path: path.display().to_string(),
            source,
        })
    }

    fn write_file(&self, path: &Path, contents: &[u8]) -> Result<(), FileSystemError> {
        std::fs::write(path, contents).map_err(|source| FileSystemError::CreateFile {
            path: path.display().to_string(),
            source,
        })
    }

    fn create_directory(&self, path: &Path, collision: CollisionPolicy) -> Result<(), FileSystemError> {
        crate::fs::create_directory(path, collision)
    }

    fn delete_path(&self, path: &Path) -> Result<(), FileSystemError> {
        crate::fs::delete_path(path)
    }

    fn rename_path(&self, src: &Path, dst: &Path, collision: CollisionPolicy) -> Result<(), FileSystemError> {
        crate::fs::rename_path(src, dst, collision)
    }

    fn copy_path(&self, src: &Path, dst: &Path, collision: CollisionPolicy,
                 progress: &mut dyn CopyProgress) -> Result<(), FileSystemError> {
        // We need to create a closure that calls the progress callback
        crate::fs::copy_path_with_progress(src, dst, collision, &mut |op_progress| {
            progress(op_progress);
        })
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn metadata(&self, path: &Path) -> Result<EntryInfo, FileSystemError> {
        let metadata = path.metadata().map_err(|source| FileSystemError::ReadEntryType {
            path: path.display().to_string(),
            source,
        })?;
        
        let name = path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        
        let kind = if metadata.is_dir() {
            crate::fs::EntryKind::Directory
        } else if metadata.is_file() {
            crate::fs::EntryKind::File
        } else if metadata.is_symlink() {
            crate::fs::EntryKind::Symlink
        } else {
            crate::fs::EntryKind::Other
        };
        
        Ok(EntryInfo {
            name,
            path: path.to_path_buf(),
            kind,
            size_bytes: if metadata.is_file() { Some(metadata.len()) } else { None },
            modified: metadata.modified().ok(),
        })
    }
}