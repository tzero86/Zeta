use std::path::{Path, PathBuf};
use std::thread;
use std::time::Instant;

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::action::{CollisionPolicy, FileOperation, RefreshTarget};
use crate::fs::{
    copy_path_with_progress, count_path_entries, create_directory, create_file, delete_path,
    looks_like_binary, rename_path, scan_directory, EntryInfo, FileSystemError,
};
use crate::pane::PaneId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobRequest {
    FileOperation {
        operation: FileOperation,
        refresh: Vec<RefreshTarget>,
        collision: CollisionPolicy,
    },
    ScanDirectory {
        pane: PaneId,
        path: PathBuf,
    },
    PreviewFile {
        path: PathBuf,
        syntect_theme: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobResult {
    DirectoryScanned {
        pane: PaneId,
        path: PathBuf,
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

pub fn spawn_scan_worker() -> (Sender<JobRequest>, Receiver<JobResult>) {
    let (request_tx, request_rx) = bounded::<JobRequest>(16);
    let (result_tx, result_rx) = bounded::<JobResult>(32);

    thread::spawn(move || run_scan_worker(request_rx, result_tx));

    (request_tx, result_rx)
}

fn run_scan_worker(request_rx: Receiver<JobRequest>, result_tx: Sender<JobResult>) {
    while let Ok(request) = request_rx.recv() {
        match request {
            JobRequest::FileOperation {
                operation,
                refresh,
                collision,
            } => {
                let result = run_file_operation(operation, refresh, collision, &result_tx);
                match result {
                    Ok(job_result) | Err(job_result) => {
                        if result_tx.send(job_result).is_err() {
                            break;
                        }
                    }
                }
            }
            JobRequest::ScanDirectory { pane, path } => {
                let started_at = Instant::now();

                match scan_directory(&path) {
                    Ok(entries) => {
                        if result_tx
                            .send(JobResult::DirectoryScanned {
                                pane,
                                path,
                                entries,
                                elapsed_ms: started_at.elapsed().as_millis(),
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(error) => {
                        if result_tx
                            .send(JobResult::JobFailed {
                                pane,
                                path,
                                message: error.to_string(),
                                elapsed_ms: started_at.elapsed().as_millis(),
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            JobRequest::PreviewFile {
                path,
                syntect_theme,
            } => {
                let view = load_preview_content(&path, &syntect_theme);
                if result_tx
                    .send(JobResult::PreviewLoaded { path, view })
                    .is_err()
                {
                    break;
                }
            }
        }
    }
}

fn load_preview_content(path: &Path, syntect_theme: &str) -> crate::preview::ViewBuffer {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return crate::preview::ViewBuffer::from_plain("[empty file]"),
    };

    if bytes.is_empty() {
        return crate::preview::ViewBuffer::from_plain("[empty file]");
    }

    if looks_like_binary(&bytes) {
        // Try to get the full file size from metadata; fall back to bytes read.
        let size_bytes = std::fs::metadata(path)
            .map(|m| m.len())
            .unwrap_or(bytes.len() as u64);
        let label = format!("[binary file — {size_bytes} bytes]");
        return crate::preview::ViewBuffer::from_plain(&label);
    }

    // Decode lossily and attempt syntax highlighting (safe — runs in worker thread).
    let text = String::from_utf8_lossy(&bytes);
    let extension = path.extension().and_then(|e| e.to_str());

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
        FileOperation::Move {
            source,
            destination,
        } => run_move_with_progress(source, destination, collision, result_tx),
        FileOperation::Rename {
            source,
            destination,
        } => rename_path(source, destination, collision),
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
    for target in refresh {
        match scan_directory(&target.path) {
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
        FileOperation::Move { source, .. } => source.clone(),
        FileOperation::Rename { source, .. } => source.clone(),
    }
}

fn describe_operation(operation: &FileOperation) -> String {
    match operation {
        FileOperation::Copy { destination, .. } => format!("copied to {}", destination.display()),
        FileOperation::CreateDirectory { path } => format!("created {}", path.display()),
        FileOperation::CreateFile { path } => format!("created {}", path.display()),
        FileOperation::Delete { path } => format!("deleted {}", path.display()),
        FileOperation::Move { destination, .. } => format!("moved to {}", destination.display()),
        FileOperation::Rename { destination, .. } => {
            format!("renamed to {}", destination.display())
        }
    }
}
