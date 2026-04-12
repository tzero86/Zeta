use std::io::Read;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

use crossbeam_channel::{bounded, Receiver, Sender};

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
    pub pane: PaneId,
    pub path: PathBuf,
    pub session_id: SessionId,
}

#[derive(Clone, Debug)]
pub struct SftpFileOpRequest {
    pub operation: FileOperation,
    pub src_session: Option<SessionId>,
    pub dst_session: Option<SessionId>,
    pub refresh: Vec<RefreshTarget>,
    pub collision: CollisionPolicy,
}

pub enum SftpRequest {
    Connect {
        address: String,
        auth_method: crate::state::ssh::SshAuthMethod,
        credential: String,
        reply: crossbeam_channel::Sender<Result<SessionId, String>>,
    },
    Disconnect {
        session_id: SessionId,
    },
    Scan(SftpScanRequest),
    FileOp(SftpFileOpRequest),
}

#[derive(Clone, Debug)]
pub struct ArchiveListRequest {
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
    pub pane: PaneId,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct FileOpRequest {
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
    pub path: PathBuf,
    pub syntect_theme: String,
    pub archive: Option<PathBuf>,
    pub inner_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct EditorLoadRequest {
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct GitStatusRequest {
    pub pane: PaneId,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct FindRequest {
    pub pane: PaneId,
    pub root: PathBuf,
    pub max_depth: usize,
}

#[derive(Clone, Debug)]
pub struct WatchRequest {
    pub paths: Vec<PathBuf>,
}

// ---------------------------------------------------------------------------
// Result types (unchanged public surface)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobResult {
    DirectoryScanned {
        pane: PaneId,
        path: PathBuf,
        entries: Vec<EntryInfo>,
        elapsed_ms: u128,
    },
    ArchiveListed {
        pane: PaneId,
        archive_path: PathBuf,
        inner_path: PathBuf,
        entries: Vec<EntryInfo>,
        elapsed_ms: u128,
    },
    FileOperationCompleted {
        message: String,
        refreshed: Vec<RefreshedPane>,
        elapsed_ms: u128,
    },
    FileOperationCollision {
        operation: FileOperation,
        refresh: Vec<RefreshTarget>,
        path: PathBuf,
        elapsed_ms: u128,
    },
    FileOperationProgress {
        status: FileOperationStatus,
    },
    JobFailed {
        pane: PaneId,
        path: PathBuf,
        message: String,
        elapsed_ms: u128,
    },
    PreviewLoaded {
        path: PathBuf,
        view: crate::preview::ViewBuffer,
    },
    EditorLoaded {
        path: PathBuf,
        contents: String,
    },
    EditorLoadFailed {
        path: PathBuf,
        message: String,
    },
    /// Git status fetched successfully for a pane's working directory.
    GitStatusLoaded {
        pane: PaneId,
        status: crate::git::RepoStatus,
    },
    /// The path is not inside a git repository (or git is not available).
    GitStatusAbsent {
        pane: PaneId,
    },
    FindResults {
        pane: PaneId,
        root: PathBuf,
        entries: Vec<PathBuf>,
    },
    DirectoryChanged {
        path: PathBuf,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefreshedPane {
    pub pane: PaneId,
    pub path: PathBuf,
    pub entries: Vec<EntryInfo>,
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
}

/// Spawn three dedicated background workers that all fan results into a single
/// `Receiver<JobResult>`. Each worker processes its queue sequentially; because
/// the queues are independent, a slow file operation never delays a scan.
pub fn spawn_workers() -> (WorkerChannels, Receiver<JobResult>) {
    let (result_tx, result_rx) = bounded::<JobResult>(64);

    // --- Scan worker ---
    let (scan_tx, scan_rx) = bounded::<ScanRequest>(16);
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
                            pane: req.pane,
                            path: req.path,
                            entries,
                            elapsed_ms: started_at.elapsed().as_millis(),
                        },
                        Err(err) => JobResult::JobFailed {
                            pane: req.pane,
                            path: req.path,
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
    let (file_op_tx, file_op_rx) = bounded::<FileOpRequest>(8);
    {
        let result_tx = result_tx.clone();
        thread::Builder::new()
            .name("zeta-file-op".into())
            .spawn(move || {
                for req in file_op_rx {
                    let outcome =
                        run_file_operation(req.operation, req.refresh, req.collision, &result_tx);
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
    let (preview_tx, preview_rx) = bounded::<PreviewRequest>(8);
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
                            path: req.path,
                            contents: String::from_utf8_lossy(&bytes).into_owned(),
                        },
                        Err(error) => JobResult::EditorLoadFailed {
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
                            pane: req.pane,
                            status,
                        },
                        None => JobResult::GitStatusAbsent { pane: req.pane },
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
                                    });
                                }
                            }

                            entries.sort_by(|l, r| {
                                l.kind
                                    .cmp(&r.kind)
                                    .then_with(|| l.name.to_lowercase().cmp(&r.name.to_lowercase()))
                            });
                            let _ = result_tx.send(JobResult::ArchiveListed {
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
                                pane: req.pane,
                                path: archive_path,
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
                            address,
                            auth_method,
                            credential,
                            reply,
                        } => {
                            let result: Result<(SessionId, crate::fs::sftp::SftpBackend), String> =
                                connect_sftp(&address, auth_method, &credential, None);
                            match result {
                                Ok((session_id, backend)) => {
                                    sessions.insert(session_id.clone(), backend);
                                    let _ = reply.send(Ok(session_id));
                                }
                                Err(e) => {
                                    let _ = reply.send(Err(e));
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
                                        pane: req.pane,
                                        path: req.path.clone(),
                                        entries,
                                        elapsed_ms: started_at.elapsed().as_millis(),
                                    },
                                    Err(e) => JobResult::JobFailed {
                                        pane: req.pane,
                                        path: req.path,
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

                            let job_result = match result {
                                Ok(msg) => JobResult::FileOperationCompleted {
                                    message: msg,
                                    refreshed: vec![],
                                    elapsed_ms: started_at.elapsed().as_millis(),
                                },
                                Err((path, msg)) => JobResult::JobFailed {
                                    pane,
                                    path,
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
        },
        result_rx,
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

    loop {
        while let Ok(req) = watch_rx.try_recv() {
            for path in &watched_paths {
                let _ = watcher.unwatch(path);
            }
            watched_paths.clear();
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

/// Verify SSH host key against known_hosts for security
fn verify_host_key(
    host: &str,
    port: u16,
    session: &ssh2::Session,
    known_hosts_file: Option<&std::path::Path>,
) -> Result<(), String> {
    let mut known_hosts = session
        .known_hosts()
        .map_err(|e| format!("Connection failed: Failed to initialize known_hosts: {}", e))?;

    // Attempt to read known_hosts file
    let default_path = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("~"))
        .join(".ssh")
        .join("known_hosts");

    let known_hosts_path = known_hosts_file.unwrap_or(&default_path);

    if known_hosts_path.exists() {
        known_hosts
            .read_file(known_hosts_path, ssh2::KnownHostFileKind::OpenSSH)
            .map_err(|e| format!("Connection failed: Failed to read known_hosts file: {}", e))?;
    } else {
        return Err(
            "Connection failed: ~/.ssh/known_hosts file not found. Host key verification required."
                .to_string(),
        );
    }

    let (key, _key_type) = session
        .host_key()
        .ok_or("Connection failed: Server did not provide a host key")?;

    match known_hosts.check_port(host, port, key) {
        ssh2::CheckResult::Match => Ok(()),
        ssh2::CheckResult::NotFound => Err(format!(
            "Connection failed: Host key for {}:{} not found in known_hosts",
            host, port
        )),
        ssh2::CheckResult::Mismatch => {
            Err("WARNING: Host key changed! Please investigate manually.".to_string())
        }
        ssh2::CheckResult::Failure => {
            Err("Connection failed: Host key verification failed".to_string())
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

/// Connect to SSH host and create SftpBackend
fn connect_sftp(
    address: &str,
    auth_method: crate::state::ssh::SshAuthMethod,
    credential: &str,
    known_hosts_file: Option<&std::path::Path>,
) -> Result<(SessionId, crate::fs::sftp::SftpBackend), String> {
    use std::net::TcpStream;

    let (user, host, port) = parse_ssh_address(address)?;

    // Create session ID for tracking
    let session_id = format!("{}@{}:{}", user, host, port);

    // Connect to SSH server
    let tcp = TcpStream::connect((host.as_str(), port))
        .map_err(|e| format!("Connection failed: {}", e))?;

    let mut session =
        ssh2::Session::new().map_err(|e| format!("Failed to create session: {}", e))?;

    session.set_tcp_stream(tcp);
    session
        .handshake()
        .map_err(|e| format!("Handshake failed: {}", e))?;

    // Verify host key (security best practice)
    verify_host_key(&host, port, &session, known_hosts_file)?;

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
                session
                    .userauth_password(&user, credential)
                    .map_err(|e| format!("Authentication failed: {}", e))?;
            }
            crate::state::ssh::SshAuthMethod::KeyFile => {
                session
                    .userauth_pubkey_file(&user, None, std::path::Path::new(credential), None)
                    .map_err(|e| format!("Key authentication failed: {}", e))?;
            }
            crate::state::ssh::SshAuthMethod::Agent => {
                return Err(
                    "Agent authentication failed and no other credential provided".to_string(),
                );
            }
        }
    }

    // Create SFTP backend
    let backend = crate::fs::sftp::SftpBackend::new(session, std::path::PathBuf::from("/"))
        .map_err(|e| format!("Failed to initialize SFTP: {}", e))?;

    Ok((session_id, backend))
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
    operation: FileOperation,
    refresh: Vec<RefreshTarget>,
    collision: CollisionPolicy,
    result_tx: &Sender<JobResult>,
) -> Result<JobResult, JobResult> {
    let started_at = Instant::now();
    let operation_label = describe_operation(&operation);
    let primary_path = primary_path(&operation);
    let failure_pane = refresh
        .first()
        .map(|target| target.pane)
        .unwrap_or(PaneId::Left);

    let op_result = match &operation {
        FileOperation::Copy {
            source,
            destination,
        } => run_copy_with_progress(source, destination, collision, result_tx),
        FileOperation::CreateDirectory { path } => create_directory(path, collision),
        FileOperation::CreateFile { path } => create_file(path, collision),
        FileOperation::Delete { path } => delete_path(path),
        FileOperation::Trash { path } => trash_path(path),
        FileOperation::Move {
            source,
            destination,
        } => run_move_with_progress(source, destination, collision, result_tx),
        FileOperation::Rename {
            source,
            destination,
        } => rename_path(source, destination, collision),
        FileOperation::ExtractArchive {
            archive,
            inner_path,
            destination,
        } => run_extract_archive(archive, inner_path, destination, result_tx),
    };

    if let Err(error) = op_result {
        return match error {
            FileSystemError::PathExists { path } => Err(JobResult::FileOperationCollision {
                operation,
                refresh,
                path: PathBuf::from(path),
                elapsed_ms: started_at.elapsed().as_millis(),
            }),
            other => Err(JobResult::JobFailed {
                pane: failure_pane,
                path: primary_path,
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
                    pane: target.pane,
                    path: target.path,
                    message: error.to_string(),
                    elapsed_ms: started_at.elapsed().as_millis(),
                });
            }
        }
    }

    Ok(JobResult::FileOperationCompleted {
        message: operation_label,
        refreshed,
        elapsed_ms: started_at.elapsed().as_millis(),
    })
}

fn run_copy_with_progress(
    source: &Path,
    destination: &Path,
    collision: CollisionPolicy,
    result_tx: &Sender<JobResult>,
) -> Result<(), FileSystemError> {
    copy_path_with_progress(source, destination, collision, &mut |progress| {
        let _ = send_progress_update(
            result_tx,
            "copy",
            progress.completed,
            progress.total,
            progress.current_path,
        );
    })
}

fn run_move_with_progress(
    source: &Path,
    destination: &Path,
    collision: CollisionPolicy,
    result_tx: &Sender<JobResult>,
) -> Result<(), FileSystemError> {
    match rename_path(source, destination, collision) {
        Ok(()) => {
            let _ = send_progress_update(result_tx, "move", 1, 1, destination.to_path_buf());
            Ok(())
        }
        Err(error) if is_cross_device_error(&error) => {
            let total = count_path_entries(source)?.saturating_add(1);
            let _ = send_progress_update(result_tx, "move", 0, total, source.to_path_buf());

            copy_path_with_progress(source, destination, collision, &mut |progress| {
                let _ = send_progress_update(
                    result_tx,
                    "move",
                    progress.completed,
                    total,
                    progress.current_path,
                );
            })?;

            delete_path(source)?;
            let _ =
                send_progress_update(result_tx, "move", total, total, destination.to_path_buf());
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn send_progress_update(
    result_tx: &Sender<JobResult>,
    operation: &'static str,
    completed: u64,
    total: u64,
    current_path: PathBuf,
) -> Result<(), ()> {
    result_tx
        .send(JobResult::FileOperationProgress {
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

fn primary_path(operation: &FileOperation) -> PathBuf {
    match operation {
        FileOperation::Copy { source, .. } => source.clone(),
        FileOperation::CreateDirectory { path } => path.clone(),
        FileOperation::CreateFile { path } => path.clone(),
        FileOperation::Delete { path } => path.clone(),
        FileOperation::Trash { path } => path.clone(),
        FileOperation::Move { source, .. } => source.clone(),
        FileOperation::Rename { source, .. } => source.clone(),
        FileOperation::ExtractArchive { archive, .. } => archive.clone(),
    }
}

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

fn run_extract_archive(
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

            let out_path = destination.join(rel);
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
            let _ = send_progress_update(result_tx, "extract", 1, 1, out_path);
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
            let out_path = destination.join(rel);
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
            let _ = send_progress_update(result_tx, "extract", 1, 1, out_path);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_worker_responds_to_request() {
        let (workers, results) = spawn_workers();
        let tmp = std::env::temp_dir();
        workers
            .git_tx
            .send(GitStatusRequest {
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
                    pane: PaneId::Left,
                    ..
                } | JobResult::GitStatusAbsent { pane: PaneId::Left }
            ),
            "unexpected result: {result:?}"
        );
    }

    #[test]
    fn worker_channels_can_send_and_receive_scan_request() {
        let (workers, results) = spawn_workers();
        let tmp = std::env::temp_dir();
        workers
            .scan_tx
            .send(ScanRequest {
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
        let (workers, results) = spawn_workers();
        let tmp = std::env::temp_dir();
        workers
            .find_tx
            .send(FindRequest {
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

        let (workers, results) = spawn_workers();
        workers
            .archive_tx
            .send(ArchiveListRequest {
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

        let (workers, results) = spawn_workers();
        workers
            .file_op_tx
            .send(FileOpRequest {
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
        let (workers, results) = spawn_workers();
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
        let (workers, results) = spawn_workers();
        let tmp = std::env::temp_dir();

        workers
            .scan_tx
            .send(ScanRequest {
                pane: PaneId::Left,
                path: tmp.clone(),
            })
            .unwrap();
        workers
            .scan_tx
            .send(ScanRequest {
                pane: PaneId::Right,
                path: tmp.clone(),
            })
            .unwrap();
        workers
            .find_tx
            .send(FindRequest {
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
