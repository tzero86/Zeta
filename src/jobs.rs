use std::path::PathBuf;
use std::thread;
use std::time::Instant;

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::fs::{scan_directory, EntryInfo};
use crate::pane::PaneId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobRequest {
    ScanDirectory { pane: PaneId, path: PathBuf },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobResult {
    DirectoryScanned {
        pane: PaneId,
        path: PathBuf,
        entries: Vec<EntryInfo>,
        elapsed_ms: u128,
    },
    JobFailed {
        pane: PaneId,
        path: PathBuf,
        message: String,
        elapsed_ms: u128,
    },
}

pub fn spawn_scan_worker() -> (Sender<JobRequest>, Receiver<JobResult>) {
    let (request_tx, request_rx) = bounded::<JobRequest>(16);
    let (result_tx, result_rx) = bounded::<JobResult>(16);

    thread::spawn(move || run_scan_worker(request_rx, result_tx));

    (request_tx, result_rx)
}

fn run_scan_worker(request_rx: Receiver<JobRequest>, result_tx: Sender<JobResult>) {
    while let Ok(request) = request_rx.recv() {
        match request {
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
        }
    }
}
