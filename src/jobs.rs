use std::io::Read;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

use crossbeam_channel::{bounded, unbounded, Receiver, Sender};

use crate::action::{CollisionPolicy, FileOperation, RefreshTarget};
use crate::fs::{
    backend::FsBackend, copy_path_with_progress, count_path_entries, create_directory, create_file,
    delete_path, local::LocalBackend, looks_like_binary, rename_path, trash_path, EntryInfo,
    FileSystemError,
};
use crate::pane::PaneId;

// ---------------------------------------------------------------------------
// Public request types — one per worker
// ---------------------------------------------------------------------------

pub type SessionId = String;

#[derive(Clone, Debug)]
pub struct SftpScanRequest {
    pub workspace_id: usize,
    pub pane: PaneId,
    pub path: PathBuf,
    pub session_id: SessionId,
}

#[derive(Clone, Debug)]
pub struct SftpFileOpRequest {
    pub workspace_id: usize,
    pub operation: FileOperation,
    pub src_session: Option<SessionId>,
    pub dst_session: Option<SessionId>,
    pub refresh: Vec<RefreshTarget>,
    pub collision: CollisionPolicy,
}

pub enum SftpRequest {
    Connect {
        workspace_id: usize,
        pane: PaneId,
        address: String,
        auth_method: crate::state::ssh::SshAuthMethod,
        credential: String,
        /// When true the connection proceeds even when the host is not in known_hosts.
        /// This flag is set after the user accepts the trust prompt.
        trust_unknown_host: bool,
    },
    Disconnect {
        session_id: SessionId,
    },
    Scan(SftpScanRequest),
    FileOp(SftpFileOpRequest),
}

/// Result of a host-key verification check.
#[derive(Debug)]
enum HostCheckResult {
    Match,
    UnknownHost { fingerprint: String },
    Mismatch,
    Failure(String),
}

/// Internal outcome type for `connect_sftp`.
enum SftpConnectOutcome {
    Connected(SessionId, crate::fs::sftp::SftpBackend),
    UnknownHost { fingerprint: String },
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct ArchiveListRequest {
    pub workspace_id: usize,
    pub pane: PaneId,
    pub archive_path: PathBuf,
    pub inner_path: PathBuf, // for navigating inside nested directories in the archive
}

#[derive(Clone, Debug)]
pub enum BackendRef {
    Local,
    Remote { address: String },
}

#[derive(Clone, Debug)]
pub struct ScanRequest {
    pub workspace_id: usize,
    pub pane: PaneId,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct FileOpRequest {
    pub workspace_id: usize,
    pub backend: BackendRef,
    pub operation: FileOperation,
    pub refresh: Vec<RefreshTarget>,
    pub collision: CollisionPolicy,
    /// Source session for cross-backend operations
    pub src_session: Option<SessionId>,
    /// Destination session for cross-backend operations
    pub dst_session: Option<SessionId>,
}

#[derive(Clone, Debug)]
pub struct PreviewRequest {
    pub workspace_id: usize,
    pub path: PathBuf,
    pub syntect_theme: String,
    pub archive: Option<PathBuf>,
    pub inner_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct EditorLoadRequest {
    pub workspace_id: usize,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct GitStatusRequest {
    pub workspace_id: usize,
    pub pane: PaneId,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct FindRequest {
    pub workspace_id: usize,
    pub pane: PaneId,
    pub root: PathBuf,
    pub max_depth: usize,
}

#[derive(Clone, Debug)]
pub struct WatchRequest {
    pub paths: Vec<PathBuf>,
    /// When set, this file is also watched; changes emit `JobResult::ConfigChanged`.
    pub config_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct DirSizeRequest {
    pub workspace_id: usize,
    pub pane: PaneId,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub enum TerminalRequest {
    Spawn {
        workspace_id: usize,
        cwd: PathBuf,
        cols: u16,
        rows: u16,
        spawn_id: u64,
    },
    Write {
        workspace_id: usize,
        bytes: Vec<u8>,
    },
    Resize {
        workspace_id: usize,
        cols: u16,
        rows: u16,
    },
}

// ---------------------------------------------------------------------------
// Result types (unchanged public surface)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobResult {
    DirectoryScanned {
        workspace_id: usize,
        pane: PaneId,
        path: PathBuf,
        entries: Vec<EntryInfo>,
        elapsed_ms: u128,
    },
    ArchiveListed {
        workspace_id: usize,
        pane: PaneId,
        archive_path: PathBuf,
        inner_path: PathBuf,
        entries: Vec<EntryInfo>,
        elapsed_ms: u128,
    },
    FileOperationCompleted {
        workspace_id: usize,
        identity: FileOperationIdentity,
        message: String,
        refreshed: Vec<RefreshedPane>,
        elapsed_ms: u128,
    },
    FileOperationCollision {
        workspace_id: usize,
        identity: FileOperationIdentity,
        operation: FileOperation,
        refresh: Vec<RefreshTarget>,
        path: PathBuf,
        elapsed_ms: u128,
    },
    FileOperationProgress {
        workspace_id: usize,
        status: FileOperationStatus,
    },
    JobFailed {
        workspace_id: usize,
        pane: PaneId,
        path: PathBuf,
        file_op: Option<FileOperationIdentity>,
        message: String,
        elapsed_ms: u128,
    },
    PreviewLoaded {
        workspace_id: usize,
        path: PathBuf,
        view: crate::preview::ViewBuffer,
    },
    EditorLoaded {
        workspace_id: usize,
        path: PathBuf,
        contents: String,
    },
    EditorLoadFailed {
        workspace_id: usize,
        path: PathBuf,
        message: String,
    },
    /// Git status fetched successfully for a pane's working directory.
    GitStatusLoaded {
        workspace_id: usize,
        pane: PaneId,
        status: crate::git::RepoStatus,
    },
    /// The path is not inside a git repository (or git is not available).
    GitStatusAbsent {
        workspace_id: usize,
        pane: PaneId,
    },
    FindResults {
        workspace_id: usize,
        pane: PaneId,
        root: PathBuf,
        entries: Vec<PathBuf>,
    },
    DirectoryChanged {
        path: PathBuf,
    },
    TerminalOutput {
        workspace_id: usize,
        bytes: Vec<u8>,
    },
    TerminalDiagnostic {
        workspace_id: usize,
        message: String,
    },
    TerminalExited {
        workspace_id: usize,
        spawn_id: u64,
    },
    /// Directory size calculated by recursively summing file sizes.
    DirSizeCalculated {
        workspace_id: usize,
        pane: PaneId,
        path: PathBuf,
        bytes: u64,
    },
    /// The user's config file changed on disk; the app should re-read it.
    ConfigChanged,
    /// SSH connection succeeded; the pane should switch to remote mode.
    SshConnected {
        workspace_id: usize,
        pane: PaneId,
        session_id: SessionId,
        address: String,
    },
    /// SSH connection reached an unknown host; the UI should show a trust prompt.
    SshHostUnknown {
        workspace_id: usize,
        pane: PaneId,
        address: String,
        auth_method: crate::state::ssh::SshAuthMethod,
        credential: String,
        fingerprint: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefreshedPane {
    pub pane: PaneId,
    pub path: PathBuf,
    pub entries: Vec<EntryInfo>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FileOperationKind {
    Copy,
    CreateDirectory,
    CreateFile,
    Delete,
    Trash,
    Move,
    Rename,
    ExtractArchive,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FileOperationIdentity {
    pub kind: FileOperationKind,
    pub source: PathBuf,
    pub destination: Option<PathBuf>,
}

impl FileOperationIdentity {
    pub fn from_operation(operation: &FileOperation) -> Self {
        match operation {
            FileOperation::Copy {
                source,
                destination,
            } => Self {
                kind: FileOperationKind::Copy,
                source: source.clone(),
                destination: Some(destination.clone()),
            },
            FileOperation::CreateDirectory { path } => Self {
                kind: FileOperationKind::CreateDirectory,
                source: path.clone(),
                destination: Some(path.clone()),
            },
            FileOperation::CreateFile { path } => Self {
                kind: FileOperationKind::CreateFile,
                source: path.clone(),
                destination: Some(path.clone()),
            },
            FileOperation::Delete { path } => Self {
                kind: FileOperationKind::Delete,
                source: path.clone(),
                destination: None,
            },
            FileOperation::Trash { path } => Self {
                kind: FileOperationKind::Trash,
                source: path.clone(),
                destination: None,
            },
            FileOperation::Move {
                source,
                destination,
            } => Self {
                kind: FileOperationKind::Move,
                source: source.clone(),
                destination: Some(destination.clone()),
            },
            FileOperation::Rename {
                source,
                destination,
            } => Self {
                kind: FileOperationKind::Rename,
                source: source.clone(),
                destination: Some(destination.clone()),
            },
            FileOperation::ExtractArchive {
                archive,
                inner_path,
                destination,
            } => Self {
                kind: FileOperationKind::ExtractArchive,
                source: archive.join(inner_path),
                destination: Some(destination.clone()),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileOperationStatus {
    pub operation: &'static str,
    pub completed: u64,
    pub total: u64,
    pub current_path: PathBuf,
}

// ---------------------------------------------------------------------------
// Worker channels
// ---------------------------------------------------------------------------

/// Three typed senders — one per dedicated worker thread.
pub struct WorkerChannels {
    pub scan_tx: Sender<ScanRequest>,
    pub file_op_tx: Sender<FileOpRequest>,
    pub preview_tx: Sender<PreviewRequest>,
    pub editor_tx: Sender<EditorLoadRequest>,
    pub git_tx: Sender<GitStatusRequest>,
    pub find_tx: Sender<FindRequest>,
    pub watch_tx: Sender<WatchRequest>,
    pub archive_tx: Sender<ArchiveListRequest>,
    pub sftp_tx: Sender<SftpRequest>,
    pub terminal_tx: Sender<TerminalRequest>,
    pub dir_size_tx: Sender<DirSizeRequest>,
}

/// Spawn three dedicated background workers that all fan results into a single
/// `Receiver<JobResult>`. Each worker processes its queue sequentially; because
/// the queues are independent, a slow file operation never delays a scan.
pub fn spawn_workers() -> (WorkerChannels, Receiver<JobResult>, Receiver<JobResult>) {
    let (result_tx, result_rx) = bounded::<JobResult>(512);
    // Dedicated unbounded channel for PTY output. Isolated from the shared queue so
    // the reader thread never blocks and a verbose process cannot starve input handling.
    let (term_out_tx, term_out_rx) = unbounded::<JobResult>();

    // --- Scan worker ---
    let (scan_tx, scan_rx) = bounded::<ScanRequest>(32);
    {
        let result_tx = result_tx.clone();
        thread::Builder::new()
            .name("zeta-scan".into())
            .spawn(move || {
                for req in scan_rx {
                    let started_at = Instant::now();
                    let backend = LocalBackend;
                    let job_result = match backend.scan_directory(&req.path) {
                        Ok(entries) => JobResult::DirectoryScanned {
                            workspace_id: req.workspace_id,
                            pane: req.pane,
                            path: req.path,
                            entries,
                            elapsed_ms: started_at.elapsed().as_millis(),
                        },
                        Err(err) => JobResult::JobFailed {
                            workspace_id: req.workspace_id,
                            pane: req.pane,
                            path: req.path,
                            file_op: None,
                            message: err.to_string(),
                            elapsed_ms: started_at.elapsed().as_millis(),
                        },
                    };
                    if result_tx.send(job_result).is_err() {
                        break;
                    }
                }
            })
            .expect("failed to spawn scan worker");
    }

    // --- File operation worker ---
    let (file_op_tx, file_op_rx) = bounded::<FileOpRequest>(16);
    {
        let result_tx = result_tx.clone();
        thread::Builder::new()
            .name("zeta-file-op".into())
            .spawn(move || {
                for req in file_op_rx {
                    let workspace_id = req.workspace_id;
                    let outcome = run_file_operation(
                        workspace_id,
                        req.operation,
                        req.refresh,
                        req.collision,
                        &result_tx,
                    );
                    let job_result = match outcome {
                        Ok(r) | Err(r) => r,
                    };
                    if result_tx.send(job_result).is_err() {
                        break;
                    }
                }
            })
            .expect("failed to spawn file-op worker");
    }

    // --- Preview worker ---
    let (preview_tx, preview_rx) = bounded::<PreviewRequest>(16);
    {
        let result_tx_preview = result_tx.clone();
        thread::Builder::new()
            .name("zeta-preview".into())
            .spawn(move || {
                for req in preview_rx {
                    let view = if req.archive.is_none() {
                        load_preview_content(&req.path, &req.syntect_theme)
                    } else if let (Some(archive_path), Some(inner_path)) =
                        (req.archive.clone(), req.inner_path.clone())
                    {
                        // Attempt to extract single file from archive into memory
                        match std::fs::File::open(&archive_path) {
                            Ok(f) => {
                                let name = archive_path
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("")
                                    .to_lowercase();
                                if name.ends_with(".zip") {
                                    match zip::ZipArchive::new(f) {
                                        Ok(mut za) => {
                                            let inner_name = inner_path.to_string_lossy();
                                            match za.by_name(&inner_name) {
                                                Ok(mut entry) => {
                                                    let mut buf = Vec::new();
                                                    use std::io::Read;
                                                    let _ = entry.read_to_end(&mut buf);
                                                    load_preview_from_bytes(
                                                        &buf,
                                                        &inner_path,
                                                        &req.syntect_theme,
                                                    )
                                                }
                                                Err(_) => crate::preview::ViewBuffer::from_plain(
                                                    "[empty file]",
                                                ),
                                            }
                                        }
                                        Err(_) => {
                                            crate::preview::ViewBuffer::from_plain("[empty file]")
                                        }
                                    }
                                } else {
                                    // Try tar variants with decompression based on extension
                                    let archive_reader: Box<dyn std::io::Read> = if name
                                        .ends_with(".tar.gz")
                                        || name.ends_with(".tgz")
                                    {
                                        Box::new(flate2::read::GzDecoder::new(f))
                                    } else if name.ends_with(".tar.bz2") || name.ends_with(".tbz2")
                                    {
                                        Box::new(bzip2::read::BzDecoder::new(f))
                                    } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
                                        Box::new(xz2::read::XzDecoder::new(f))
                                    } else {
                                        Box::new(f)
                                    };
                                    let mut ar = tar::Archive::new(archive_reader);
                                    let mut found = None;
                                    if let Ok(entries) = ar.entries() {
                                        for mut e in entries.flatten() {
                                            if let Ok(path) = e.path() {
                                                if path == inner_path {
                                                    let mut buf = Vec::new();
                                                    let _ = e.read_to_end(&mut buf);
                                                    found = Some(buf);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    if let Some(buf) = found {
                                        load_preview_from_bytes(
                                            &buf,
                                            &inner_path,
                                            &req.syntect_theme,
                                        )
                                    } else {
                                        crate::preview::ViewBuffer::from_plain("[empty file]")
                                    }
                                }
                            }
                            Err(_) => crate::preview::ViewBuffer::from_plain("[empty file]"),
                        }
                    } else {
                        load_preview_content(&req.path, &req.syntect_theme)
                    };
                    if result_tx_preview
                        .send(JobResult::PreviewLoaded {
                            workspace_id: req.workspace_id,
                            path: req.path,
                            view,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("failed to spawn preview worker");
    }

    // --- Editor load worker ---
    let (editor_tx, editor_rx) = bounded::<EditorLoadRequest>(4);
    {
        let result_tx_editor = result_tx.clone();
        thread::Builder::new()
            .name("zeta-editor-load".into())
            .spawn(move || {
                for req in editor_rx {
                    let result = match std::fs::read(&req.path) {
                        Ok(bytes) => JobResult::EditorLoaded {
                            workspace_id: req.workspace_id,
                            path: req.path,
                            contents: String::from_utf8_lossy(&bytes).into_owned(),
                        },
                        Err(error) => JobResult::EditorLoadFailed {
                            workspace_id: req.workspace_id,
                            path: req.path,
                            message: error.to_string(),
                        },
                    };
                    if result_tx_editor.send(result).is_err() {
                        break;
                    }
                }
            })
            .expect("failed to spawn editor load worker");
    }

    // --- Git status worker ---
    let (git_tx, git_rx) = bounded::<GitStatusRequest>(16);
    {
        let result_tx = result_tx.clone();
        thread::Builder::new()
            .name("zeta-git".into())
            .spawn(move || {
                for req in git_rx {
                    let result = match crate::git::fetch_status(&req.path) {
                        Some(status) => JobResult::GitStatusLoaded {
                            workspace_id: req.workspace_id,
                            pane: req.pane,
                            status,
                        },
                        None => JobResult::GitStatusAbsent {
                            workspace_id: req.workspace_id,
                            pane: req.pane,
                        },
                    };
                    if result_tx.send(result).is_err() {
                        break;
                    }
                }
            })
            .expect("failed to spawn git worker");
    }

    // --- Finder worker ---
    let (find_tx, find_rx) = bounded::<FindRequest>(8);
    {
        let result_tx = result_tx.clone();
        thread::Builder::new()
            .name("zeta-find".into())
            .spawn(move || {
                for req in find_rx {
                    let entries = walk_for_files(&req.root, req.max_depth);
                    if result_tx
                        .send(JobResult::FindResults {
                            workspace_id: req.workspace_id,
                            pane: req.pane,
                            root: req.root,
                            entries,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("failed to spawn finder worker");
    }

    // --- Watcher worker ---
    let (watch_tx, watch_rx) = bounded::<WatchRequest>(8);
    {
        let result_tx = result_tx.clone();
        thread::Builder::new()
            .name("zeta-watch".into())
            .spawn(move || run_watcher_worker(watch_rx, result_tx))
            .expect("failed to spawn watcher worker");
    }

    // --- Archive worker ---
    let (archive_tx, archive_rx) = bounded::<ArchiveListRequest>(4);
    {
        let result_tx = result_tx.clone();
        thread::Builder::new()
            .name("zeta-archive".into())
            .spawn(move || {
                for req in archive_rx {
                    let started_at = std::time::Instant::now();
                    let archive_path = req.archive_path.clone();

                    // Try ZIP first (open a fresh file handle by path).
                    if let Ok(zip_file) = std::fs::File::open(&archive_path) {
                        if let Ok(mut archive) = zip::ZipArchive::new(zip_file) {
                            let mut seen = std::collections::BTreeSet::new();
                            let mut entries: Vec<EntryInfo> = Vec::new();
                            let inner = req.inner_path.to_string_lossy().replace("\\", "/");
                            let prefix = if inner.is_empty() {
                                String::new()
                            } else if inner.ends_with('/') {
                                inner.clone()
                            } else {
                                format!("{}/", inner)
                            };

                            for i in 0..archive.len() {
                                if let Ok(file) = archive.by_index(i) {
                                    let name = file.name().to_string();
                                    let rest = if prefix.is_empty() {
                                        name.as_str()
                                    } else if name.starts_with(&prefix) {
                                        &name[prefix.len()..]
                                    } else {
                                        continue;
                                    };
                                    if rest.is_empty() {
                                        continue;
                                    }
                                    let first = rest.split('/').next().unwrap().to_string();
                                    if !seen.insert(first.clone()) {
                                        continue;
                                    }
                                    let is_dir = rest.contains('/') || name.ends_with('/');
                                    let kind = if is_dir {
                                        crate::fs::EntryKind::Directory
                                    } else {
                                        crate::fs::EntryKind::File
                                    };
                                    let size_bytes = if kind == crate::fs::EntryKind::File {
                                        Some(file.size())
                                    } else {
                                        None
                                    };
                                    entries.push(EntryInfo {
                                        name: first.clone(),
                                        path: archive_path.join(first),
                                        kind,
                                        size_bytes,
                                        modified: None,
                                        link_target: None,
                                    });
                                }
                            }

                            entries.sort_by(|l, r| {
                                l.kind
                                    .cmp(&r.kind)
                                    .then_with(|| l.name.to_lowercase().cmp(&r.name.to_lowercase()))
                            });
                            let _ = result_tx.send(JobResult::ArchiveListed {
                                workspace_id: req.workspace_id,
                                pane: req.pane,
                                archive_path: archive_path.clone(),
                                inner_path: req.inner_path,
                                entries,
                                elapsed_ms: started_at.elapsed().as_millis(),
                            });
                            continue; // done for this request
                        }
                    }

                    // Fall back to tar variants (plain tar, tar.gz, tar.bz2, tar.xz).
                    let ext_name = archive_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let tar_file = match std::fs::File::open(&archive_path) {
                        Ok(f) => f,
                        Err(err) => {
                            let _ = result_tx.send(JobResult::JobFailed {
                                workspace_id: req.workspace_id,
                                pane: req.pane,
                                path: archive_path,
                                file_op: None,
                                message: format!("failed to open archive: {err}"),
                                elapsed_ms: started_at.elapsed().as_millis(),
                            });
                            continue;
                        }
                    };

                    let reader: Box<dyn std::io::Read> =
                        if ext_name.ends_with(".tar.gz") || ext_name.ends_with(".tgz") {
                            Box::new(flate2::read::GzDecoder::new(tar_file))
                        } else if ext_name.ends_with(".tar.bz2") || ext_name.ends_with(".tbz2") {
                            Box::new(bzip2::read::BzDecoder::new(tar_file))
                        } else if ext_name.ends_with(".tar.xz") || ext_name.ends_with(".txz") {
                            Box::new(xz2::read::XzDecoder::new(tar_file))
                        } else {
                            Box::new(tar_file)
                        };

                    let mut ar = tar::Archive::new(reader);
                    let mut seen = std::collections::BTreeSet::new();
                    let mut entries: Vec<EntryInfo> = Vec::new();
                    let inner = req.inner_path.to_string_lossy().replace("\\", "/");
                    let prefix = if inner.is_empty() {
                        String::new()
                    } else if inner.ends_with('/') {
                        inner.clone()
                    } else {
                        format!("{}/", inner)
                    };

                    if let Ok(entries_iter) = ar.entries() {
                        for entry in entries_iter.flatten() {
                            if let Ok(path) = entry.path() {
                                let name = path.to_string_lossy().to_string();
                                let rest = if prefix.is_empty() {
                                    name.as_str()
                                } else if name.starts_with(&prefix) {
                                    &name[prefix.len()..]
                                } else {
                                    continue;
                                };
                                if rest.is_empty() {
                                    continue;
                                }
                                let first = rest.split('/').next().unwrap().to_string();
                                if !seen.insert(first.clone()) {
                                    continue;
                                }
                                let is_dir = rest.contains('/');
                                let kind = if is_dir {
                                    crate::fs::EntryKind::Directory
                                } else {
                                    crate::fs::EntryKind::File
                                };
                                let size_bytes = if kind == crate::fs::EntryKind::File {
                                    Some(entry.size())
                                } else {
                                    None
                                };
                                entries.push(EntryInfo {
                                    name: first.clone(),
                                    path: archive_path.join(first),
                                    kind,
                                    size_bytes,
                                    modified: None,
                                    link_target: None,
                                });
                            }
                        }
                    }

                    entries.sort_by(|l, r| {
                        l.kind
                            .cmp(&r.kind)
                            .then_with(|| l.name.to_lowercase().cmp(&r.name.to_lowercase()))
                    });
                    let _ = result_tx.send(JobResult::ArchiveListed {
                        workspace_id: req.workspace_id,
                        pane: req.pane,
                        archive_path,
                        inner_path: req.inner_path,
                        entries,
                        elapsed_ms: started_at.elapsed().as_millis(),
                    });
                }
            })
            .expect("failed to spawn archive worker");
    }

    // --- SFTP worker ---
    let (sftp_tx, sftp_rx) = bounded::<SftpRequest>(8);
    {
        let result_tx = result_tx.clone();
        thread::Builder::new()
            .name("zeta-sftp".into())
            .spawn(move || {
                let mut sessions: std::collections::HashMap<
                    SessionId,
                    crate::fs::sftp::SftpBackend,
                > = std::collections::HashMap::new();

                for req in sftp_rx {
                    match req {
                        SftpRequest::Connect {
                            workspace_id,
                            pane,
                            address,
                            auth_method,
                            credential,
                            trust_unknown_host,
                        } => {
                            let address_display = address.clone();
                            match connect_sftp(
                                &address,
                                auth_method,
                                &credential,
                                None,
                                trust_unknown_host,
                            ) {
                                SftpConnectOutcome::Connected(session_id, backend) => {
                                    sessions.insert(session_id.clone(), backend);
                                    let _ = result_tx.send(JobResult::SshConnected {
                                        workspace_id,
                                        pane,
                                        session_id,
                                        address: address_display,
                                    });
                                }
                                SftpConnectOutcome::UnknownHost { fingerprint } => {
                                    let _ = result_tx.send(JobResult::SshHostUnknown {
                                        workspace_id,
                                        pane,
                                        address,
                                        auth_method,
                                        credential,
                                        fingerprint,
                                    });
                                }
                                SftpConnectOutcome::Failed(msg) => {
                                    let _ = result_tx.send(JobResult::JobFailed {
                                        workspace_id,
                                        pane,
                                        path: std::path::PathBuf::new(),
                                        file_op: None,
                                        message: msg,
                                        elapsed_ms: 0,
                                    });
                                }
                            }
                        }
                        SftpRequest::Disconnect { session_id } => {
                            sessions.remove(&session_id);
                        }
                        SftpRequest::Scan(req) => {
                            if let Some(backend) = sessions.get(&req.session_id) {
                                let started_at = Instant::now();
                                let job_result = match backend.scan_directory(&req.path) {
                                    Ok(entries) => JobResult::DirectoryScanned {
                                        workspace_id: req.workspace_id,
                                        pane: req.pane,
                                        path: req.path.clone(),
                                        entries,
                                        elapsed_ms: started_at.elapsed().as_millis(),
                                    },
                                    Err(e) => JobResult::JobFailed {
                                        workspace_id: req.workspace_id,
                                        pane: req.pane,
                                        path: req.path,
                                        file_op: None,
                                        message: format!("SFTP scan failed: {}", e),
                                        elapsed_ms: started_at.elapsed().as_millis(),
                                    },
                                };
                                let _ = result_tx.send(job_result);
                            }
                        }
                        SftpRequest::FileOp(req) => {
                            // Handle cross-backend file operations
                            let started_at = Instant::now();
                            let pane = req.refresh.first().map(|r| r.pane).unwrap_or(PaneId::Left);

                            // Get source and destination backends
                            let src_backend =
                                req.src_session.as_ref().and_then(|id| sessions.get(id));
                            let dst_backend =
                                req.dst_session.as_ref().and_then(|id| sessions.get(id));
                            // Use src_backend as fallback for remote→remote in same session
                            let dst_backend = dst_backend.or(src_backend);

                            let result = execute_sftp_file_op(
                                &req.operation,
                                src_backend,
                                dst_backend,
                                req.collision,
                                &result_tx,
                                pane,
                            );

                            let identity = FileOperationIdentity::from_operation(&req.operation);

                            let job_result = match result {
                                Ok(msg) => JobResult::FileOperationCompleted {
                                    workspace_id: req.workspace_id,
                                    identity: identity.clone(),
                                    message: msg,
                                    refreshed: vec![],
                                    elapsed_ms: started_at.elapsed().as_millis(),
                                },
                                Err((path, msg)) => JobResult::JobFailed {
                                    workspace_id: req.workspace_id,
                                    pane,
                                    path,
                                    file_op: Some(identity),
                                    message: msg,
                                    elapsed_ms: started_at.elapsed().as_millis(),
                                },
                            };
                            let _ = result_tx.send(job_result);
                        }
                    }
                }
            })
            .expect("failed to spawn sftp worker");
    }

    // --- Terminal worker ---
    let (terminal_tx, terminal_rx) = bounded::<TerminalRequest>(16);
    {
        let result_tx = result_tx.clone();
        let term_out_tx = term_out_tx.clone();
        thread::Builder::new()
            .name("zeta-terminal".into())
            .spawn(move || {
                run_terminal_worker(terminal_rx, result_tx, term_out_tx);
            })
            .expect("failed to spawn terminal worker");
    }

    // --- Directory size worker ---
    let (dir_size_tx, dir_size_rx) = bounded::<DirSizeRequest>(64);
    {
        let result_tx = result_tx.clone();
        thread::Builder::new()
            .name("zeta-dir-size".into())
            .spawn(move || {
                for req in dir_size_rx {
                    let bytes = sum_dir_size(&req.path);
                    if result_tx
                        .send(JobResult::DirSizeCalculated {
                            workspace_id: req.workspace_id,
                            pane: req.pane,
                            path: req.path,
                            bytes,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("failed to spawn dir-size worker");
    }
    (
        WorkerChannels {
            scan_tx,
            file_op_tx,
            preview_tx,
            editor_tx,
            git_tx,
            find_tx,
            watch_tx,
            archive_tx,
            sftp_tx,
            terminal_tx,
            dir_size_tx,
        },
        result_rx,
        term_out_rx,
    )
}

// ---------------------------------------------------------------------------
// Internal worker logic
// ---------------------------------------------------------------------------

fn run_watcher_worker(watch_rx: Receiver<WatchRequest>, result_tx: Sender<JobResult>) {
    use std::sync::mpsc;

    let (notify_tx, notify_rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(notify_tx, Config::default())
        .expect("failed to create filesystem watcher");
    let mut watched_paths: Vec<PathBuf> = Vec::new();
    // The config file path, if any; changes emit `ConfigChanged` instead of `DirectoryChanged`.
    let mut watched_config: Option<PathBuf> = None;

    loop {
        while let Ok(req) = watch_rx.try_recv() {
            for path in &watched_paths {
                let _ = watcher.unwatch(path);
            }
            watched_paths.clear();
            // Register config file's parent dir (if not already covered by a pane path).
            if let Some(ref cfg) = req.config_path {
                if let Some(parent) = cfg.parent() {
                    let parent = parent.to_path_buf();
                    if watched_paths.iter().all(|p| p != &parent)
                        && watcher.watch(&parent, RecursiveMode::NonRecursive).is_ok()
                    {
                        watched_paths.push(parent);
                    }
                }
            }
            watched_config = req.config_path;
            for path in req.paths {
                if watched_paths.iter().all(|p| p != &path)
                    && watcher.watch(&path, RecursiveMode::NonRecursive).is_ok()
                {
                    watched_paths.push(path);
                }
            }
        }

        while let Ok(event_result) = notify_rx.try_recv() {
            let Ok(event) = event_result else {
                continue;
            };
            for path in event.paths {
                // Exact match on the config file → emit ConfigChanged and skip dir change.
                if watched_config.as_deref() == Some(path.as_path()) {
                    if result_tx.send(JobResult::ConfigChanged).is_err() {
                        return;
                    }
                    continue;
                }
                let changed_dir = if path.is_dir() {
                    path
                } else {
                    path.parent().map(Path::to_path_buf).unwrap_or(path)
                };
                if result_tx
                    .send(JobResult::DirectoryChanged { path: changed_dir })
                    .is_err()
                {
                    return;
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

/// Recursively sum the size in bytes of all regular files under `path`.
/// Directories without read permission are silently skipped.
fn sum_dir_size(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    let mut total = 0u64;
    for entry in entries.flatten() {
        match entry.file_type() {
            Ok(ft) if ft.is_file() => {
                if let Ok(meta) = entry.metadata() {
                    total += meta.len();
                }
            }
            Ok(ft) if ft.is_dir() => {
                total += sum_dir_size(&entry.path());
            }
            _ => {}
        }
    }
    total
}

fn walk_for_files(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut results = Vec::new();
    walk_recursive(root, max_depth, &mut results);
    results
}

fn walk_recursive(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth == 0 {
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.')
            || matches!(name, "target" | "node_modules" | "__pycache__" | ".git")
        {
            continue;
        }
        if path.is_dir() {
            walk_recursive(&path, depth - 1, out);
        } else {
            out.push(path);
        }
    }
}

fn load_preview_content(path: &Path, syntect_theme: &str) -> crate::preview::ViewBuffer {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return crate::preview::ViewBuffer::from_plain("[empty file]"),
    };
    load_preview_from_bytes(&bytes, path, syntect_theme)
}

fn load_preview_from_bytes(
    bytes: &[u8],
    path: &Path,
    syntect_theme: &str,
) -> crate::preview::ViewBuffer {
    if bytes.is_empty() {
        return crate::preview::ViewBuffer::from_plain("[empty file]");
    }

    if looks_like_binary(bytes) {
        let size_bytes = std::fs::metadata(path)
            .map(|m| m.len())
            .unwrap_or(bytes.len() as u64);
        let label = format!("[binary file — {size_bytes} bytes]");
        return crate::preview::ViewBuffer::from_plain(&label);
    }

    let text = String::from_utf8_lossy(bytes);
    let extension = path.extension().and_then(|e| e.to_str());

    // Markdown: store raw text for AST rendering at display time.
    if extension == Some("md") {
        return crate::preview::ViewBuffer::from_markdown(text.into_owned());
    }

    if let Some(lines) = crate::highlight::highlight_text(&text, extension, syntect_theme) {
        return crate::preview::ViewBuffer::from_highlighted(lines);
    }

    // Fallback: truncate to 200 lines, capped at 200 × 80 chars.
    let truncated: String = text
        .lines()
        .take(200)
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(200 * 80)
        .collect();

    crate::preview::ViewBuffer::from_plain(&truncated)
}

/// Format raw host key bytes as a colon-separated MD5 fingerprint for display.
fn format_fingerprint(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(":")
}

/// Encode bytes as standard base64 (RFC 4648, with padding).
/// Avoids an external dependency for this single use-case.
fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let cap = bytes.len().div_ceil(3) * 4;
    let mut out = String::with_capacity(cap);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let combined = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((combined >> 18) & 63) as usize] as char);
        out.push(TABLE[((combined >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((combined >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(combined & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Append the server's host key to `~/.ssh/known_hosts` in OpenSSH format.
///
/// Creates `~/.ssh/` (mode 0700 on Unix) and the file if they don't exist.
/// Errors are silently swallowed — the connection already succeeded, so a
/// persistence failure must not abort the session.
fn persist_host_key(host: &str, port: u16, session: &ssh2::Session) {
    let (key_bytes, key_type) = match session.host_key() {
        Some(kv) => kv,
        None => return,
    };
    let key_type_str = match key_type {
        ssh2::HostKeyType::Rsa => "ssh-rsa",
        ssh2::HostKeyType::Dss => "ssh-dss",
        ssh2::HostKeyType::Ecdsa256 => "ecdsa-sha2-nistp256",
        ssh2::HostKeyType::Ecdsa384 => "ecdsa-sha2-nistp384",
        ssh2::HostKeyType::Ecdsa521 => "ecdsa-sha2-nistp521",
        ssh2::HostKeyType::Ed25519 => "ssh-ed25519",
        // Unknown or future types: do not write a malformed entry.
        _ => return,
    };
    let key_b64 = base64_encode(key_bytes);
    // OpenSSH format: plain hostname for port 22, [hostname]:port otherwise.
    let host_field = if port == 22 {
        host.to_string()
    } else {
        format!("[{}]:{}", host, port)
    };
    let entry = format!(
        "{} {} {}
",
        host_field, key_type_str, key_b64
    );

    let ssh_dir = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("~"))
        .join(".ssh");

    if !ssh_dir.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            let _ = std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&ssh_dir);
        }
        #[cfg(not(unix))]
        {
            let _ = std::fs::create_dir_all(&ssh_dir);
        }
    }

    use std::io::Write;
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ssh_dir.join("known_hosts"))
        .and_then(|mut f| f.write_all(entry.as_bytes()));
}

/// Verify SSH host key against known_hosts.
///
/// Returns a structured `HostCheckResult` instead of `Result<(), String>` so the
/// caller can decide whether to prompt the user or fail immediately.
fn verify_host_key(
    host: &str,
    port: u16,
    session: &ssh2::Session,
    known_hosts_file: Option<&std::path::Path>,
) -> HostCheckResult {
    let mut known_hosts = match session.known_hosts() {
        Ok(kh) => kh,
        Err(e) => {
            return HostCheckResult::Failure(format!("Failed to initialize known_hosts: {}", e))
        }
    };

    let default_path = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("~"))
        .join(".ssh")
        .join("known_hosts");
    let known_hosts_path = known_hosts_file.unwrap_or(&default_path);

    if known_hosts_path.exists() {
        if let Err(e) = known_hosts.read_file(known_hosts_path, ssh2::KnownHostFileKind::OpenSSH) {
            return HostCheckResult::Failure(format!("Failed to read known_hosts: {}", e));
        }
    }
    // When known_hosts doesn't exist we still run the check; it will return NotFound.

    let (key, _key_type) = match session.host_key() {
        Some(kv) => kv,
        None => return HostCheckResult::Failure("Server provided no host key".to_string()),
    };

    match known_hosts.check_port(host, port, key) {
        ssh2::CheckResult::Match => HostCheckResult::Match,
        ssh2::CheckResult::NotFound => {
            // Build a human-readable fingerprint from the MD5 hash of the key.
            let fingerprint = session
                .host_key_hash(ssh2::HashType::Md5)
                .map(format_fingerprint)
                .unwrap_or_else(|| String::from("(fingerprint unavailable)"));
            HostCheckResult::UnknownHost { fingerprint }
        }
        ssh2::CheckResult::Mismatch => HostCheckResult::Mismatch,
        ssh2::CheckResult::Failure => {
            HostCheckResult::Failure("Host key verification failed".to_string())
        }
    }
}

/// Parse SSH address in format user@host:port or user@host
fn parse_ssh_address(address: &str) -> Result<(String, String, u16), String> {
    let (user, rest) = address
        .split_once('@')
        .ok_or("Address must be in format user@host:port")?;

    let (host, port) = match rest.rsplit_once(':') {
        Some((h, p)) => {
            let port = p.parse().map_err(|_| "Invalid port number")?;
            (h, port)
        }
        None => (rest, 22u16),
    };

    Ok((user.to_string(), host.to_string(), port))
}

/// Connect to SSH host and create SftpBackend.
///
/// When `trust_unknown_host` is true the connection proceeds even when the host
/// is not present in `~/.ssh/known_hosts`. The host key is NOT written to
/// known_hosts in this release; a future enhancement should persist the key.
fn connect_sftp(
    address: &str,
    auth_method: crate::state::ssh::SshAuthMethod,
    credential: &str,
    known_hosts_file: Option<&std::path::Path>,
    trust_unknown_host: bool,
) -> SftpConnectOutcome {
    use std::net::TcpStream;

    let (user, host, port) = match parse_ssh_address(address) {
        Ok(v) => v,
        Err(e) => return SftpConnectOutcome::Failed(e),
    };

    // Create session ID for tracking
    let session_id = format!("{}@{}:{}", user, host, port);

    // Connect to SSH server
    let tcp = match TcpStream::connect((host.as_str(), port)) {
        Ok(t) => t,
        Err(e) => return SftpConnectOutcome::Failed(format!("Connection failed: {}", e)),
    };

    let mut session = match ssh2::Session::new() {
        Ok(s) => s,
        Err(e) => return SftpConnectOutcome::Failed(format!("Failed to create session: {}", e)),
    };

    session.set_tcp_stream(tcp);
    if let Err(e) = session.handshake() {
        return SftpConnectOutcome::Failed(format!("Handshake failed: {}", e));
    }

    // Verify host key; allow bypass when user has explicitly trusted this session.
    let mut should_persist = false;
    match verify_host_key(&host, port, &session, known_hosts_file) {
        HostCheckResult::Match => {}
        HostCheckResult::UnknownHost { fingerprint: _ } if trust_unknown_host => {
            // User accepted the trust prompt — proceed and persist the host key.
            // `persist_host_key` is called after authentication succeeds (below)
            // so we have a live session with the verified key still available.
            should_persist = true;
        }
        HostCheckResult::UnknownHost { fingerprint } => {
            return SftpConnectOutcome::UnknownHost { fingerprint };
        }
        HostCheckResult::Mismatch => {
            return SftpConnectOutcome::Failed(
                "WARNING: Host key changed! Possible MITM attack. Investigate manually."
                    .to_string(),
            );
        }
        HostCheckResult::Failure(msg) => {
            return SftpConnectOutcome::Failed(format!("Host verification failed: {}", msg));
        }
    }

    // Attempt SSH Agent authentication first
    let mut authenticated = false;

    if let Ok(mut agent) = session.agent() {
        if agent.connect().is_ok() && agent.list_identities().is_ok() {
            if let Ok(identities) = agent.identities() {
                for identity in identities {
                    if agent.userauth(&user, &identity).is_ok() {
                        authenticated = true;
                        break;
                    }
                }
            }
        }
        let _ = agent.disconnect();
    }

    // Authenticate with fallback if agent failed
    if !authenticated {
        match auth_method {
            crate::state::ssh::SshAuthMethod::Password => {
                if let Err(e) = session.userauth_password(&user, credential) {
                    return SftpConnectOutcome::Failed(format!("Authentication failed: {}", e));
                }
            }
            crate::state::ssh::SshAuthMethod::KeyFile => {
                if let Err(e) = session.userauth_pubkey_file(
                    &user,
                    None,
                    std::path::Path::new(credential),
                    None,
                ) {
                    return SftpConnectOutcome::Failed(format!("Key authentication failed: {}", e));
                }
            }
            crate::state::ssh::SshAuthMethod::Agent => {
                return SftpConnectOutcome::Failed(
                    "Agent authentication failed and no other credential provided".to_string(),
                );
            }
        }
    }

    // Persist the host key now that authentication has proved the session is valid.
    // This runs only when the user accepted the trust prompt for an unknown host.
    if should_persist {
        persist_host_key(&host, port, &session);
    }

    // Create SFTP backend
    let backend = match crate::fs::sftp::SftpBackend::new(session, std::path::PathBuf::from("/")) {
        Ok(b) => b,
        Err(e) => return SftpConnectOutcome::Failed(format!("Failed to initialize SFTP: {}", e)),
    };

    SftpConnectOutcome::Connected(session_id, backend)
}

/// Execute a file operation with SFTP backends (cross-backend support)
fn execute_sftp_file_op(
    operation: &FileOperation,
    src_backend: Option<&crate::fs::sftp::SftpBackend>,
    dst_backend: Option<&crate::fs::sftp::SftpBackend>,
    _collision: CollisionPolicy,
    _result_tx: &Sender<JobResult>,
    _pane: crate::pane::PaneId,
) -> Result<String, (PathBuf, String)> {
    use crate::fs::backend::FsBackend;

    match operation {
        FileOperation::Copy {
            source,
            destination,
        } => {
            // Cross-backend copy: read from source, write to destination
            let contents = if let Some(backend) = src_backend {
                backend
                    .read_file(source)
                    .map_err(|e| (source.clone(), e.to_string()))?
            } else {
                // Local source
                std::fs::read(source).map_err(|e| (source.clone(), e.to_string()))?
            };

            if let Some(backend) = dst_backend {
                backend
                    .write_file(destination, &contents)
                    .map_err(|e| (destination.clone(), e.to_string()))?;
            } else {
                // Local destination
                std::fs::write(destination, &contents)
                    .map_err(|e| (destination.clone(), e.to_string()))?;
            }

            Ok(format!(
                "Copied {} to {}",
                source.display(),
                destination.display()
            ))
        }
        FileOperation::Delete { path } => {
            if let Some(backend) = src_backend {
                backend
                    .delete_path(path)
                    .map_err(|e| (path.clone(), e.to_string()))?;
            } else {
                std::fs::remove_file(path).map_err(|e| (path.clone(), e.to_string()))?;
            }
            Ok(format!("Deleted {}", path.display()))
        }
        _ => Err((
            PathBuf::new(),
            "Operation not yet implemented for SFTP".to_string(),
        )),
    }
}

#[allow(clippy::result_large_err)]
fn run_file_operation(
    workspace_id: usize,
    operation: FileOperation,
    refresh: Vec<RefreshTarget>,
    collision: CollisionPolicy,
    result_tx: &Sender<JobResult>,
) -> Result<JobResult, JobResult> {
    let started_at = Instant::now();
    let identity = FileOperationIdentity::from_operation(&operation);
    let operation_label = describe_operation(&operation);
    let failure_pane = refresh
        .first()
        .map(|target| target.pane)
        .unwrap_or(PaneId::Left);

    let op_result = match &operation {
        FileOperation::Copy {
            source,
            destination,
        } => run_copy_with_progress(workspace_id, source, destination, collision, result_tx),
        FileOperation::CreateDirectory { path } => create_directory(path, collision),
        FileOperation::CreateFile { path } => create_file(path, collision),
        FileOperation::Delete { path } => delete_path(path),
        FileOperation::Trash { path } => trash_path(path),
        FileOperation::Move {
            source,
            destination,
        } => run_move_with_progress(workspace_id, source, destination, collision, result_tx),
        FileOperation::Rename {
            source,
            destination,
        } => rename_path(source, destination, collision),
        FileOperation::ExtractArchive {
            archive,
            inner_path,
            destination,
        } => run_extract_archive(workspace_id, archive, inner_path, destination, result_tx),
    };

    if let Err(error) = op_result {
        return match error {
            FileSystemError::PathExists { path } => Err(JobResult::FileOperationCollision {
                workspace_id,
                identity,
                operation,
                refresh,
                path: PathBuf::from(path),
                elapsed_ms: started_at.elapsed().as_millis(),
            }),
            other => Err(JobResult::JobFailed {
                workspace_id,
                pane: failure_pane,
                path: identity.source.clone(),
                file_op: Some(identity),
                message: other.to_string(),
                elapsed_ms: started_at.elapsed().as_millis(),
            }),
        };
    }

    let mut refreshed = Vec::with_capacity(refresh.len());
    let backend = LocalBackend;
    for target in refresh {
        match backend.scan_directory(&target.path) {
            Ok(entries) => refreshed.push(RefreshedPane {
                pane: target.pane,
                path: target.path,
                entries,
            }),
            Err(error) => {
                return Err(JobResult::JobFailed {
                    workspace_id,
                    pane: target.pane,
                    path: target.path,
                    file_op: Some(identity),
                    message: error.to_string(),
                    elapsed_ms: started_at.elapsed().as_millis(),
                });
            }
        }
    }

    Ok(JobResult::FileOperationCompleted {
        workspace_id,
        identity,
        message: operation_label,
        refreshed,
        elapsed_ms: started_at.elapsed().as_millis(),
    })
}

fn run_copy_with_progress(
    workspace_id: usize,
    source: &Path,
    destination: &Path,
    collision: CollisionPolicy,
    result_tx: &Sender<JobResult>,
) -> Result<(), FileSystemError> {
    copy_path_with_progress(source, destination, collision, &mut |progress| {
        let _ = send_progress_update(
            result_tx,
            workspace_id,
            "copy",
            progress.completed,
            progress.total,
            progress.current_path,
        );
    })
}

fn run_move_with_progress(
    workspace_id: usize,
    source: &Path,
    destination: &Path,
    collision: CollisionPolicy,
    result_tx: &Sender<JobResult>,
) -> Result<(), FileSystemError> {
    match rename_path(source, destination, collision) {
        Ok(()) => {
            let _ = send_progress_update(
                result_tx,
                workspace_id,
                "move",
                1,
                1,
                destination.to_path_buf(),
            );
            Ok(())
        }
        Err(error) if is_cross_device_error(&error) => {
            let total = count_path_entries(source)?.saturating_add(1);
            let _ = send_progress_update(
                result_tx,
                workspace_id,
                "move",
                0,
                total,
                source.to_path_buf(),
            );

            copy_path_with_progress(source, destination, collision, &mut |progress| {
                let _ = send_progress_update(
                    result_tx,
                    workspace_id,
                    "move",
                    progress.completed,
                    total,
                    progress.current_path,
                );
            })?;

            delete_path(source)?;
            let _ = send_progress_update(
                result_tx,
                workspace_id,
                "move",
                total,
                total,
                destination.to_path_buf(),
            );
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn send_progress_update(
    result_tx: &Sender<JobResult>,
    workspace_id: usize,
    operation: &'static str,
    completed: u64,
    total: u64,
    current_path: PathBuf,
) -> Result<(), ()> {
    result_tx
        .send(JobResult::FileOperationProgress {
            workspace_id,
            status: FileOperationStatus {
                operation,
                completed,
                total,
                current_path,
            },
        })
        .map_err(|_| ())
}

fn is_cross_device_error(error: &FileSystemError) -> bool {
    matches!(
        error,
        FileSystemError::RenamePath { source, .. } if source.raw_os_error() == Some(EXDEV_ERROR)
    )
}

#[cfg(unix)]
const EXDEV_ERROR: i32 = 18;

#[cfg(not(unix))]
const EXDEV_ERROR: i32 = -1;

fn describe_operation(operation: &FileOperation) -> String {
    match operation {
        FileOperation::Copy { destination, .. } => format!("copied to {}", destination.display()),
        FileOperation::CreateDirectory { path } => format!("created {}", path.display()),
        FileOperation::CreateFile { path } => format!("created {}", path.display()),
        FileOperation::Delete { path } => format!("deleted {}", path.display()),
        FileOperation::Trash { path } => format!("trashed {}", path.display()),
        FileOperation::Move { destination, .. } => format!("moved to {}", destination.display()),
        FileOperation::Rename { destination, .. } => {
            format!("renamed to {}", destination.display())
        }
        FileOperation::ExtractArchive { destination, .. } => {
            format!("extracted to {}", destination.display())
        }
    }
}

/// Resolve `rel` relative to `base`, rejecting any component that would escape `base`.
/// Strips leading separators, silently skips `.` components, and returns `None` if
/// any `..` or absolute-root component is encountered.
fn safe_archive_join(base: &Path, rel: &str) -> Option<PathBuf> {
    let rel = rel.trim_start_matches(['/', '\\']);
    let mut out = base.to_path_buf();
    for component in Path::new(rel).components() {
        match component {
            std::path::Component::Normal(c) => out.push(c),
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    Some(out)
}

fn run_extract_archive(
    workspace_id: usize,
    archive: &Path,
    inner_path: &Path,
    destination: &Path,
    result_tx: &Sender<JobResult>,
) -> Result<(), FileSystemError> {
    use std::io::Read;

    // Open archive file
    let file = std::fs::File::open(archive).map_err(|source| FileSystemError::CopyPath {
        from: archive.display().to_string(),
        to: destination.display().to_string(),
        source,
    })?;

    // Normalize inner path prefix with forward slashes
    let inner = inner_path.to_string_lossy().replace("\\", "/");
    let prefix = if inner.is_empty() {
        String::new()
    } else if inner.ends_with('/') {
        inner.clone()
    } else {
        format!("{}/", inner)
    };

    let name = archive.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let lower = name.to_lowercase();

    if lower.ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(file).map_err(|e| FileSystemError::CopyPath {
            from: archive.display().to_string(),
            to: destination.display().to_string(),
            source: std::io::Error::other(e.to_string()),
        })?;

        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| FileSystemError::CopyPath {
                from: archive.display().to_string(),
                to: destination.display().to_string(),
                source: std::io::Error::other(e.to_string()),
            })?;
            let entry_name = entry.name().to_string();
            if !prefix.is_empty() && !entry_name.starts_with(&prefix) {
                continue;
            }
            let rel = if prefix.is_empty() {
                entry_name.as_str()
            } else {
                &entry_name[prefix.len()..]
            };
            if rel.is_empty() {
                continue;
            }

            let out_path = match safe_archive_join(destination, rel) {
                Some(p) => p,
                None => {
                    return Err(FileSystemError::CopyPath {
                        from: archive.display().to_string(),
                        to: destination.display().to_string(),
                        source: std::io::Error::other(format!(
                            "archive entry escapes destination: {:?}",
                            rel
                        )),
                    })
                }
            };
            if entry.name().ends_with('/') {
                std::fs::create_dir_all(&out_path).map_err(|source| FileSystemError::CopyPath {
                    from: archive.display().to_string(),
                    to: out_path.display().to_string(),
                    source,
                })?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|source| {
                        FileSystemError::CopyPath {
                            from: archive.display().to_string(),
                            to: parent.display().to_string(),
                            source,
                        }
                    })?;
                }
                let mut outfile = std::fs::File::create(&out_path).map_err(|source| {
                    FileSystemError::CopyPath {
                        from: archive.display().to_string(),
                        to: out_path.display().to_string(),
                        source,
                    }
                })?;
                std::io::copy(&mut entry, &mut outfile).map_err(|source| {
                    FileSystemError::CopyPath {
                        from: archive.display().to_string(),
                        to: out_path.display().to_string(),
                        source,
                    }
                })?;
            }
            let _ = send_progress_update(result_tx, workspace_id, "extract", 1, 1, out_path);
        }

        Ok(())
    } else if lower.ends_with(".tar") || lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        // support gzipped tars
        let archive_reader: Box<dyn Read> = if lower.ends_with(".tar.gz") || lower.ends_with(".tgz")
        {
            Box::new(flate2::read::GzDecoder::new(file))
        } else if lower.ends_with(".tar.bz2") || lower.ends_with(".tbz2") {
            Box::new(bzip2::read::BzDecoder::new(file))
        } else if lower.ends_with(".tar.xz") || lower.ends_with(".txz") {
            Box::new(xz2::read::XzDecoder::new(file))
        } else {
            Box::new(file)
        };
        let mut ar = tar::Archive::new(archive_reader);
        for entry in ar.entries().map_err(|e| FileSystemError::CopyPath {
            from: archive.display().to_string(),
            to: destination.display().to_string(),
            source: std::io::Error::other(e.to_string()),
        })? {
            let mut entry = entry.map_err(|e| FileSystemError::CopyPath {
                from: archive.display().to_string(),
                to: destination.display().to_string(),
                source: std::io::Error::other(e.to_string()),
            })?;
            let path = entry.path().map_err(|e| FileSystemError::CopyPath {
                from: archive.display().to_string(),
                to: destination.display().to_string(),
                source: std::io::Error::other(e.to_string()),
            })?;
            let path_str = path.to_string_lossy();
            if !prefix.is_empty() && !path_str.starts_with(&prefix) {
                continue;
            }
            let rel = if prefix.is_empty() {
                path_str.as_ref()
            } else {
                &path_str[prefix.len()..]
            };
            if rel.is_empty() {
                continue;
            }
            let out_path = match safe_archive_join(destination, rel) {
                Some(p) => p,
                None => {
                    return Err(FileSystemError::CopyPath {
                        from: archive.display().to_string(),
                        to: destination.display().to_string(),
                        source: std::io::Error::other(format!(
                            "archive entry escapes destination: {:?}",
                            rel
                        )),
                    })
                }
            };
            if entry.header().entry_type().is_dir() {
                std::fs::create_dir_all(&out_path).map_err(|source| FileSystemError::CopyPath {
                    from: archive.display().to_string(),
                    to: out_path.display().to_string(),
                    source,
                })?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|source| {
                        FileSystemError::CopyPath {
                            from: archive.display().to_string(),
                            to: parent.display().to_string(),
                            source,
                        }
                    })?;
                }
                let mut outfile = std::fs::File::create(&out_path).map_err(|source| {
                    FileSystemError::CopyPath {
                        from: archive.display().to_string(),
                        to: out_path.display().to_string(),
                        source,
                    }
                })?;
                std::io::copy(&mut entry, &mut outfile).map_err(|source| {
                    FileSystemError::CopyPath {
                        from: archive.display().to_string(),
                        to: out_path.display().to_string(),
                        source,
                    }
                })?;
            }
            let _ = send_progress_update(result_tx, workspace_id, "extract", 1, 1, out_path);
        }
        Ok(())
    } else {
        Err(FileSystemError::CopyPath {
            from: archive.display().to_string(),
            to: destination.display().to_string(),
            source: std::io::Error::other("unsupported archive format"),
        })
    }
}

/// Terminal worker: handles PTY spawn and raw I/O.
///
/// Uses `conpty` on Windows and `portable-pty` on Unix via [`crate::pty::PtySession`].
pub fn run_terminal_worker(
    terminal_rx: Receiver<TerminalRequest>,
    result_tx: Sender<JobResult>,
    term_out_tx: Sender<JobResult>,
) {
    use std::collections::HashMap;
    use std::io::{Read, Write};

    let mut sessions: HashMap<usize, crate::pty::PtySession> = HashMap::new();
    let mut writers: HashMap<usize, Box<dyn Write + Send>> = HashMap::new();

    for req in terminal_rx {
        match req {
            TerminalRequest::Spawn {
                workspace_id,
                cwd,
                cols,
                rows,
                spawn_id,
            } => {
                let safe_cols = if cols == 0 { 80 } else { cols };
                let safe_rows = if rows == 0 { 24 } else { rows };

                match crate::pty::PtySession::spawn(&cwd, safe_cols, safe_rows) {
                    Ok(mut pty) => match (pty.take_reader(), pty.take_writer()) {
                        (Ok(mut r), Ok(w)) => {
                            let term_out_tx_inner = term_out_tx.clone();
                            thread::Builder::new()
                                .name(format!("zeta-terminal-reader-{workspace_id}"))
                                .spawn(move || {
                                    let mut buffer = [0u8; 8192];
                                    while let Ok(n) = r.read(&mut buffer) {
                                        if n == 0 {
                                            break;
                                        }
                                        if term_out_tx_inner
                                            .send(JobResult::TerminalOutput {
                                                workspace_id,
                                                bytes: buffer[..n].to_vec(),
                                            })
                                            .is_err()
                                        {
                                            break;
                                        }
                                    }
                                })
                                .expect("failed to spawn terminal reader thread");

                            if let Ok(waiter) = pty.exit_waiter() {
                                let result_tx_exit = result_tx.clone();
                                thread::Builder::new()
                                    .name(format!("zeta-terminal-watcher-{workspace_id}"))
                                    .spawn(move || {
                                        waiter();
                                        let _ = result_tx_exit.send(JobResult::TerminalExited {
                                            workspace_id,
                                            spawn_id,
                                        });
                                    })
                                    .expect("failed to spawn terminal watcher thread");
                            }

                            let _ = result_tx.send(JobResult::TerminalDiagnostic {
                                workspace_id,
                                message: String::from("Terminal ready"),
                            });

                            writers.insert(workspace_id, w);
                            sessions.insert(workspace_id, pty);
                        }
                        (r_res, w_res) => {
                            let msg = format!(
                                "PTY I/O setup failed: reader={:?}, writer={:?}",
                                r_res.err(),
                                w_res.err(),
                            );
                            let _ = result_tx.send(JobResult::JobFailed {
                                workspace_id,
                                pane: PaneId::Left,
                                path: PathBuf::new(),
                                file_op: None,
                                message: msg,
                                elapsed_ms: 0,
                            });
                        }
                    },
                    Err(e) => {
                        let _ = result_tx.send(JobResult::JobFailed {
                            workspace_id,
                            pane: PaneId::Left,
                            path: PathBuf::new(),
                            file_op: None,
                            message: format!("Failed to spawn terminal: {e}"),
                            elapsed_ms: 0,
                        });
                    }
                }
            }
            TerminalRequest::Write {
                workspace_id,
                bytes,
            } => {
                if let Some(writer) = writers.get_mut(&workspace_id) {
                    let _ = writer.write_all(&bytes);
                    let _ = writer.flush();
                }
            }
            TerminalRequest::Resize {
                workspace_id,
                cols,
                rows,
            } => {
                if let Some(session) = sessions.get_mut(&workspace_id) {
                    let safe_cols = if cols == 0 { 80 } else { cols };
                    let safe_rows = if rows == 0 { 24 } else { rows };
                    let _ = session.resize(safe_cols, safe_rows);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_operation_identity_maps_each_variant() {
        let shared_path = PathBuf::from("/tmp/note.txt");
        let cases = [
            (
                FileOperation::Copy {
                    source: PathBuf::from("/tmp/source.txt"),
                    destination: PathBuf::from("/tmp/dest.txt"),
                },
                FileOperationIdentity {
                    kind: FileOperationKind::Copy,
                    source: PathBuf::from("/tmp/source.txt"),
                    destination: Some(PathBuf::from("/tmp/dest.txt")),
                },
            ),
            (
                FileOperation::CreateDirectory {
                    path: shared_path.clone(),
                },
                FileOperationIdentity {
                    kind: FileOperationKind::CreateDirectory,
                    source: shared_path.clone(),
                    destination: Some(shared_path.clone()),
                },
            ),
            (
                FileOperation::CreateFile {
                    path: shared_path.clone(),
                },
                FileOperationIdentity {
                    kind: FileOperationKind::CreateFile,
                    source: shared_path.clone(),
                    destination: Some(shared_path.clone()),
                },
            ),
            (
                FileOperation::Delete {
                    path: shared_path.clone(),
                },
                FileOperationIdentity {
                    kind: FileOperationKind::Delete,
                    source: shared_path.clone(),
                    destination: None,
                },
            ),
            (
                FileOperation::Trash {
                    path: shared_path.clone(),
                },
                FileOperationIdentity {
                    kind: FileOperationKind::Trash,
                    source: shared_path.clone(),
                    destination: None,
                },
            ),
            (
                FileOperation::Move {
                    source: PathBuf::from("/tmp/old.txt"),
                    destination: PathBuf::from("/tmp/new.txt"),
                },
                FileOperationIdentity {
                    kind: FileOperationKind::Move,
                    source: PathBuf::from("/tmp/old.txt"),
                    destination: Some(PathBuf::from("/tmp/new.txt")),
                },
            ),
            (
                FileOperation::Rename {
                    source: PathBuf::from("/tmp/before.txt"),
                    destination: PathBuf::from("/tmp/after.txt"),
                },
                FileOperationIdentity {
                    kind: FileOperationKind::Rename,
                    source: PathBuf::from("/tmp/before.txt"),
                    destination: Some(PathBuf::from("/tmp/after.txt")),
                },
            ),
            (
                FileOperation::ExtractArchive {
                    archive: PathBuf::from("/tmp/bundle.zip"),
                    inner_path: PathBuf::from("nested/note.txt"),
                    destination: PathBuf::from("/tmp/out"),
                },
                FileOperationIdentity {
                    kind: FileOperationKind::ExtractArchive,
                    source: PathBuf::from("/tmp/bundle.zip").join("nested/note.txt"),
                    destination: Some(PathBuf::from("/tmp/out")),
                },
            ),
        ];

        for (operation, expected) in cases {
            assert_eq!(FileOperationIdentity::from_operation(&operation), expected);
        }
    }

    #[test]
    fn run_file_operation_completed_result_carries_identity() {
        let tmpdir = tempfile::tempdir().expect("tempdir should exist");
        let path = tmpdir.path().join("created.txt");
        let (result_tx, _result_rx) = bounded(1);

        let result = run_file_operation(
            0,
            FileOperation::CreateFile { path: path.clone() },
            vec![],
            CollisionPolicy::Fail,
            &result_tx,
        )
        .expect("create file should succeed");

        assert_eq!(
            result,
            JobResult::FileOperationCompleted {
                workspace_id: 0,
                identity: FileOperationIdentity {
                    kind: FileOperationKind::CreateFile,
                    source: path.clone(),
                    destination: Some(path.clone()),
                },
                message: format!("created {}", path.display()),
                refreshed: Vec::new(),
                elapsed_ms: result_elapsed_ms(&result),
            }
        );
    }

    #[test]
    fn run_file_operation_failure_results_carry_identity() {
        let tmpdir = tempfile::tempdir().expect("tempdir should exist");
        let collision_path = tmpdir.path().join("existing.txt");
        std::fs::write(&collision_path, b"existing").expect("existing file should be created");
        let missing_path = tmpdir.path().join("missing.txt");
        let (result_tx, _result_rx) = bounded(1);

        let collision = run_file_operation(
            0,
            FileOperation::CreateFile {
                path: collision_path.clone(),
            },
            vec![],
            CollisionPolicy::Fail,
            &result_tx,
        )
        .expect_err("existing destination should collide");
        let failure = run_file_operation(
            0,
            FileOperation::Delete {
                path: missing_path.clone(),
            },
            vec![],
            CollisionPolicy::Fail,
            &result_tx,
        )
        .expect_err("missing path should fail");

        assert!(matches!(
            collision,
            JobResult::FileOperationCollision {
                identity: FileOperationIdentity {
                    kind: FileOperationKind::CreateFile,
                    source,
                    destination: Some(destination),
                },
                ..
            } if source == collision_path && destination == collision_path
        ));
        assert!(matches!(
            failure,
            JobResult::JobFailed {
                path,
                file_op: Some(FileOperationIdentity {
                    kind: FileOperationKind::Delete,
                    source,
                    destination: None,
                }),
                ..
            } if path == missing_path && source == missing_path
        ));
    }

    fn result_elapsed_ms(result: &JobResult) -> u128 {
        match result {
            JobResult::FileOperationCompleted { elapsed_ms, .. } => *elapsed_ms,
            _ => panic!("expected file operation completion result"),
        }
    }

    #[test]
    fn git_worker_responds_to_request() {
        let (workers, results, _term_out) = spawn_workers();
        let tmp = std::env::temp_dir();
        workers
            .git_tx
            .send(GitStatusRequest {
                workspace_id: 0,
                pane: PaneId::Left,
                path: tmp,
            })
            .unwrap();
        let result = results
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert!(
            matches!(
                result,
                JobResult::GitStatusLoaded {
                    workspace_id: 0,
                    pane: PaneId::Left,
                    ..
                } | JobResult::GitStatusAbsent {
                    workspace_id: 0,
                    pane: PaneId::Left,
                }
            ),
            "unexpected result: {result:?}"
        );
    }

    #[test]
    fn worker_channels_can_send_and_receive_scan_request() {
        let (workers, results, _term_out) = spawn_workers();
        let tmp = std::env::temp_dir();
        workers
            .scan_tx
            .send(ScanRequest {
                workspace_id: 0,
                pane: PaneId::Left,
                path: tmp,
            })
            .unwrap();
        let result = results
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert!(
            matches!(
                result,
                JobResult::DirectoryScanned {
                    pane: PaneId::Left,
                    ..
                } | JobResult::JobFailed {
                    pane: PaneId::Left,
                    ..
                }
            ),
            "unexpected result: {result:?}"
        );
    }

    #[test]
    fn finder_worker_returns_find_results() {
        let (workers, results, _term_out) = spawn_workers();
        let tmp = std::env::temp_dir();
        workers
            .find_tx
            .send(FindRequest {
                workspace_id: 0,
                pane: PaneId::Left,
                root: tmp,
                max_depth: 2,
            })
            .unwrap();

        loop {
            let result = results
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            if matches!(
                result,
                JobResult::FindResults {
                    pane: PaneId::Left,
                    ..
                }
            ) {
                break;
            }
        }
    }

    #[test]
    fn archive_worker_lists_zip_and_tar() {
        use std::io::Write;
        use zip::write::FileOptions;

        let tmpdir = tempfile::tempdir().unwrap();
        let archive_path = tmpdir.path().join("test.zip");

        {
            let file = std::fs::File::create(&archive_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file("a.txt", FileOptions::default()).unwrap();
            zip.write_all(b"hello").unwrap();
            zip.add_directory("dir/", FileOptions::default()).unwrap();
            zip.start_file("dir/b.txt", FileOptions::default()).unwrap();
            zip.write_all(b"world").unwrap();
            zip.finish().unwrap();
        }

        let (workers, results, _term_out) = spawn_workers();
        workers
            .archive_tx
            .send(ArchiveListRequest {
                workspace_id: 0,
                pane: PaneId::Left,
                archive_path: archive_path.clone(),
                inner_path: std::path::PathBuf::new(),
            })
            .unwrap();

        loop {
            let res = results
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            if let JobResult::ArchiveListed {
                pane,
                archive_path: ap,
                entries,
                ..
            } = res
            {
                assert_eq!(pane, PaneId::Left);
                assert_eq!(ap, archive_path);
                let names: Vec<_> = entries.iter().map(|e| e.name.clone()).collect();
                assert!(names.contains(&"a.txt".to_string()));
                assert!(names.contains(&"dir".to_string()));
                break;
            }
        }
    }

    #[test]
    fn file_op_extracts_zip_contents() {
        use std::io::Write;
        use zip::write::FileOptions;

        let tmpdir = tempfile::tempdir().unwrap();
        let archive_path = tmpdir.path().join("test2.zip");
        let outdir = tmpdir.path().join("out");

        {
            let file = std::fs::File::create(&archive_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file("a.txt", FileOptions::default()).unwrap();
            zip.write_all(b"hello").unwrap();
            zip.finish().unwrap();
        }

        let (workers, results, _term_out) = spawn_workers();
        workers
            .file_op_tx
            .send(FileOpRequest {
                workspace_id: 0,
                backend: BackendRef::Local,
                operation: FileOperation::ExtractArchive {
                    archive: archive_path.clone(),
                    inner_path: std::path::PathBuf::new(),
                    destination: outdir.clone(),
                },
                refresh: vec![],
                collision: CollisionPolicy::Fail,
                src_session: None,
                dst_session: None,
            })
            .unwrap();

        loop {
            let res = results
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            match res {
                JobResult::FileOperationCompleted { .. } => break,
                JobResult::JobFailed { .. } => panic!("extract failed"),
                _ => {}
            }
        }

        assert!(outdir.join("a.txt").exists());
        let contents = std::fs::read_to_string(outdir.join("a.txt")).unwrap();
        assert_eq!(contents, "hello");
    }

    #[test]
    fn watcher_worker_emits_directory_changed() {
        let (workers, results, _term_out) = spawn_workers();
        let root = std::env::temp_dir().join(format!(
            "zeta-watch-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be after unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        workers
            .watch_tx
            .send(WatchRequest {
                paths: vec![root.clone()],
                config_path: None,
            })
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        std::fs::write(root.join("created.txt"), "hello").unwrap();

        let mut saw_change = false;
        for _ in 0..10 {
            let result = results
                .recv_timeout(std::time::Duration::from_secs(1))
                .unwrap();
            if let JobResult::DirectoryChanged { path } = result {
                if path == root {
                    saw_change = true;
                    break;
                }
            }
        }

        let _ = std::fs::remove_file(root.join("created.txt"));
        let _ = std::fs::remove_dir(&root);
        assert!(saw_change, "expected watcher to emit DirectoryChanged");
    }

    #[test]
    fn workers_process_requests_independently() {
        let (workers, results, _term_out) = spawn_workers();
        let tmp = std::env::temp_dir();

        workers
            .scan_tx
            .send(ScanRequest {
                workspace_id: 0,
                pane: PaneId::Left,
                path: tmp.clone(),
            })
            .unwrap();
        workers
            .scan_tx
            .send(ScanRequest {
                workspace_id: 0,
                pane: PaneId::Right,
                path: tmp.clone(),
            })
            .unwrap();
        workers
            .find_tx
            .send(FindRequest {
                workspace_id: 0,
                pane: PaneId::Left,
                root: tmp,
                max_depth: 1,
            })
            .unwrap();

        let mut received = 0usize;
        for _ in 0..3 {
            if results
                .recv_timeout(std::time::Duration::from_secs(5))
                .is_ok()
            {
                received += 1;
            }
        }
        assert_eq!(received, 3);
    }
}
