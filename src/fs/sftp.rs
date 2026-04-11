use std::path::{Path, PathBuf};
use std::sync::Arc;

use ssh2::{Session, Sftp};

use crate::action::CollisionPolicy;
use crate::fs::{EntryInfo, FileSystemError, EntryKind};
use crate::fs::backend::{FsBackend, CopyProgress};

/// SFTP filesystem backend that operates over SSH
pub struct SftpBackend {
    /// Established SFTP session
    sftp: Arc<Sftp>,
    /// Base path on remote server (for relative operations)
    base_path: PathBuf,
}

impl SftpBackend {
    /// Create a new SftpBackend from an established SSH session
    pub fn new(session: Session, base_path: PathBuf) -> Result<Self, FileSystemError> {
        let sftp = session
            .sftp()
            .map_err(|source| FileSystemError::Other {
                message: format!("Failed to initialize SFTP session: {}", source),
            })?;

        Ok(Self {
            sftp: Arc::new(sftp),
            base_path,
        })
    }

    /// Resolve a relative path to absolute path on remote server
    fn resolve_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_path.join(path)
        }
    }

    /// Convert SFTP stat to EntryInfo
    fn stat_to_entry_info(&self, path: &Path, stat: ssh2::FileStat) -> EntryInfo {
        let kind = if stat.is_dir() {
            EntryKind::Directory
        } else if stat.is_file() {
            EntryKind::File
        } else {
            EntryKind::Other
        };

        EntryInfo {
            name: path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string(),
            path: path.to_path_buf(),
            kind,
            size_bytes: Some(stat.size.unwrap_or(0)),
            modified: stat.mtime.map(|mtime| {
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(mtime as u64)
            }),
        }
    }
}

impl FsBackend for SftpBackend {
    fn scan_directory(&self, path: &Path) -> Result<Vec<EntryInfo>, FileSystemError> {
        let remote_path = self.resolve_path(path);
        let path_str = remote_path.to_string_lossy();

        // Read directory entries
        let entries = self.sftp.readdir(&remote_path)
            .map_err(|source| FileSystemError::ReadEntryType {
                path: path_str.to_string(),
                source,
            })?;

        let mut result = Vec::new();

        // Add parent directory entry ("..")
        if let Some(parent) = remote_path.parent() {
            result.push(EntryInfo {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                kind: EntryKind::Directory,
                size_bytes: None,
                modified: None,
            });
        }

        // Convert SFTP entries to EntryInfo
        for (entry_path, stat) in entries {
            let relative_path = entry_path.strip_prefix(&remote_path)
                .unwrap_or(&entry_path);

            // Skip current directory "." entry
            if relative_path == Path::new(".") {
                continue;
            }

            result.push(self.stat_to_entry_info(&entry_path, stat));
        }

        Ok(result)
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>, FileSystemError> {
        let remote_path = self.resolve_path(path);
        let path_str = remote_path.to_string_lossy();

        let mut file = self.sftp.open(&remote_path)
            .map_err(|source| FileSystemError::ReadEntryType {
                path: path_str.to_string(),
                source,
            })?;

        let mut contents = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut contents)
            .map_err(|source| FileSystemError::ReadEntryType {
                path: path_str.to_string(),
                source,
            })?;

        Ok(contents)
    }

    fn write_file(&self, path: &Path, contents: &[u8]) -> Result<(), FileSystemError> {
        let remote_path = self.resolve_path(path);
        let path_str = remote_path.to_string_lossy();

        let mut file = self.sftp.create(&remote_path)
            .map_err(|source| FileSystemError::CreateFile {
                path: path_str.to_string(),
                source,
            })?;

        std::io::Write::write_all(&mut file, contents)
            .map_err(|source| FileSystemError::CreateFile {
                path: path_str.to_string(),
                source,
            })?;

        Ok(())
    }

    fn create_directory(&self, path: &Path, collision: CollisionPolicy) -> Result<(), FileSystemError> {
        let remote_path = self.resolve_path(path);
        let path_str = remote_path.to_string_lossy();

        // Check if directory already exists
        if self.exists(&remote_path) {
            match collision {
                CollisionPolicy::Fail => {
                    return Err(FileSystemError::CreateDirectory {
                        path: path_str.to_string(),
                        source: std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Directory exists"),
                    });
                }
                CollisionPolicy::Overwrite => {
                    // For directories, we don't overwrite, just return success
                    return Ok(());
                }
            }
        }

        self.sftp.mkdir(&remote_path, 0o755)
            .map_err(|source| FileSystemError::CreateDirectory {
                path: path_str.to_string(),
                source,
            })?;

        Ok(())
    }

    fn delete_path(&self, path: &Path) -> Result<(), FileSystemError> {
        let remote_path = self.resolve_path(path);
        let path_str = remote_path.to_string_lossy();

        let stat = self.sftp.stat(&remote_path)
            .map_err(|source| FileSystemError::DeleteEntry {
                path: path_str.to_string(),
                source,
            })?;

        if stat.is_dir() {
            self.sftp.rmdir(&remote_path)
        } else {
            self.sftp.unlink(&remote_path)
        }.map_err(|source| FileSystemError::DeleteEntry {
            path: path_str.to_string(),
            source,
        })?;

        Ok(())
    }

    fn rename_path(&self, src: &Path, dst: &Path, collision: CollisionPolicy) -> Result<(), FileSystemError> {
        let src_path = self.resolve_path(src);
        let dst_path = self.resolve_path(dst);
        let dst_str = dst_path.to_string_lossy();

        // Check if destination exists
        if self.exists(&dst_path) {
            match collision {
                CollisionPolicy::Fail => {
                    return Err(FileSystemError::MoveEntry {
                        src: src_path.to_string_lossy().to_string(),
                        dst: dst_str.to_string(),
                        source: std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Destination exists"),
                    });
                }
                CollisionPolicy::Overwrite => {
                    // Remove destination first
                    self.delete_path(dst)?;
                }
            }
        }

        self.sftp.rename(&src_path, &dst_path, None)
            .map_err(|source| FileSystemError::MoveEntry {
                src: src_path.to_string_lossy().to_string(),
                dst: dst_str.to_string(),
                source,
            })?;

        Ok(())
    }

    fn copy_path(&self, src: &Path, dst: &Path, collision: CollisionPolicy,
                 progress: &mut dyn CopyProgress) -> Result<(), FileSystemError> {
        let src_path = self.resolve_path(src);
        let dst_path = self.resolve_path(dst);
        let dst_str = dst_path.to_string_lossy();

        // Check if destination exists
        if self.exists(&dst_path) {
            match collision {
                CollisionPolicy::Fail => {
                    return Err(FileSystemError::CopyEntry {
                        src: src_path.to_string_lossy().to_string(),
                        dst: dst_str.to_string(),
                        source: std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Destination exists"),
                    });
                }
                CollisionPolicy::Overwrite => {
                    // Remove destination first
                    self.delete_path(dst)?;
                }
            }
        }

        // For SFTP, we need to read the source file and write to destination
        let contents = self.read_file(src)?;

        // Report progress
        progress(crate::fs::OperationProgress {
            bytes_copied: 0,
            total_bytes: contents.len() as u64,
        });

        self.write_file(dst, &contents)?;

        // Report completion
        progress(crate::fs::OperationProgress {
            bytes_copied: contents.len() as u64,
            total_bytes: contents.len() as u64,
        });

        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        let remote_path = self.resolve_path(path);
        self.sftp.stat(&remote_path).is_ok()
    }

    fn metadata(&self, path: &Path) -> Result<EntryInfo, FileSystemError> {
        let remote_path = self.resolve_path(path);
        let path_str = remote_path.to_string_lossy();

        let stat = self.sftp.stat(&remote_path)
            .map_err(|source| FileSystemError::ReadEntryType {
                path: path_str.to_string(),
                source,
            })?;

        Ok(self.stat_to_entry_info(&remote_path, stat))
    }
}