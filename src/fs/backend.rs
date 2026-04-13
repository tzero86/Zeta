use crate::action::CollisionPolicy;
use crate::fs::{EntryInfo, FileSystemError};
use std::path::Path;

/// Copy progress callback trait
pub trait CopyProgress: FnMut(crate::fs::OperationProgress) {}

impl<F> CopyProgress for F where F: FnMut(crate::fs::OperationProgress) {}

/// Filesystem backend trait that abstracts local and remote filesystem operations
pub trait FsBackend: Send + Sync {
    fn scan_directory(&self, path: &Path) -> Result<Vec<EntryInfo>, FileSystemError>;
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, FileSystemError>;
    fn write_file(&self, path: &Path, contents: &[u8]) -> Result<(), FileSystemError>;
    fn create_directory(
        &self,
        path: &Path,
        collision: CollisionPolicy,
    ) -> Result<(), FileSystemError>;
    fn delete_path(&self, path: &Path) -> Result<(), FileSystemError>;
    fn rename_path(
        &self,
        src: &Path,
        dst: &Path,
        collision: CollisionPolicy,
    ) -> Result<(), FileSystemError>;
    fn copy_path(
        &self,
        src: &Path,
        dst: &Path,
        collision: CollisionPolicy,
        progress: &mut dyn CopyProgress,
    ) -> Result<(), FileSystemError>;
    fn exists(&self, path: &Path) -> bool;
    fn metadata(&self, path: &Path) -> Result<EntryInfo, FileSystemError>;
}
