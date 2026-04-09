# Wave 7A — SSH/SFTP Remote Filesystems

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow one pane to connect to a remote host over SFTP. The user opens a connection dialog, enters `user@host[:port]` and an authentication method (password or key file), and the pane begins browsing the remote filesystem. File operations between a remote and local pane copy/move files via SFTP.

This is the most complex wave. It requires a proper filesystem abstraction trait before any of the remote plumbing, so **Task 1 is a refactor** that creates `FsBackend` — a trait implemented by both the existing local backend and the new SFTP backend. All subsequent tasks build on that foundation.

**New dependency:**
- `ssh2 = "0.9"` — libssh2 bindings. Links against the system libssh2 (available via `apt install libssh2-dev` on Linux, bundled on Windows via the crate's build script).

> **Binary size note:** `ssh2` adds ~500 KB to the release binary and requires libssl. On Windows the build script downloads and links libssh2 statically. Accept this cost; SFTP is a meaningful feature for remote developer workflows.

**Jira:** ZTA-97 (ZTA-184 through ZTA-196)

**Wave dependency:** Starts AFTER Wave 6B. This is the most architecturally invasive wave. Allow extra implementation time.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `Cargo.toml` | Add `ssh2 = "0.9"` |
| Create | `src/fs/backend.rs` | `FsBackend` trait |
| Create | `src/fs/local.rs` | `LocalBackend` — wraps existing `src/fs.rs` functions |
| Create | `src/fs/sftp.rs` | `SftpBackend` — ssh2 SFTP implementation |
| Modify | `src/fs.rs` | Re-export trait and backends; keep public API stable |
| Modify | `src/pane.rs` | `PaneState.backend: BackendKind` (Local or Remote) |
| Modify | `src/jobs.rs` | Workers accept `FsBackend`-generic requests; `SftpWorker` owns session |
| Modify | `src/state/types.rs` | `ModalKind::SshConnect` |
| Create | `src/state/ssh.rs` | `SshConnectionState` — dialog state for connection form |
| Modify | `src/state/mod.rs` | `OpenSshConnect`, `SshConnectConfirm`, `SshDisconnect` handlers |
| Modify | `src/action.rs` | New SSH actions |
| Create | `src/ui/ssh.rs` | SSH connection dialog renderer |
| Modify | `src/ui/mod.rs` | `pub mod ssh;`; render ssh dialog; remote breadcrumb in pane title |

---

## Architecture overview

```
PaneState
├── mode: PaneMode (Real | Archive | Remote { session_id, remote_path })
└── entries: Vec<EntryInfo>   — same type, backend-agnostic

WorkerChannels
├── scan_tx   → ScanRequest { backend: BackendKind, … }
├── file_op_tx → FileOpRequest { src_backend, dst_backend, … }
└── sftp_tx   → SftpRequest (connect/disconnect lifecycle management)

SftpWorker
└── owns ssh2::Session — kept alive for the pane's lifetime
    Sessions stored in a HashMap<SessionId, Session>
    Session established on connect, dropped on disconnect or pane close
```

The key constraint: `ssh2::Session` is not `Send` on all platforms. The SFTP worker owns the session and all remote operations go through it via a channel — callers never hold the session directly.

---

## Task 1: `FsBackend` trait (prerequisite refactor)

**Files:** `src/fs/backend.rs`, `src/fs/local.rs`, `src/fs.rs`

This task refactors the existing `fs.rs` functions into a trait. **No user-visible behaviour changes.**

- [ ] **Step 1.1: Define `FsBackend` trait**

```rust
pub trait FsBackend: Send + Sync {
    fn scan_directory(&self, path: &Path) -> Result<Vec<EntryInfo>, FileSystemError>;
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, FileSystemError>;
    fn write_file(&self, path: &Path, contents: &[u8]) -> Result<(), FileSystemError>;
    fn create_directory(&self, path: &Path, collision: CollisionPolicy) -> Result<(), FileSystemError>;
    fn delete_path(&self, path: &Path) -> Result<(), FileSystemError>;
    fn rename_path(&self, src: &Path, dst: &Path, collision: CollisionPolicy) -> Result<(), FileSystemError>;
    fn copy_path(&self, src: &Path, dst: &Path, collision: CollisionPolicy,
                 progress: &mut dyn FnMut(CopyProgress)) -> Result<(), FileSystemError>;
    fn exists(&self, path: &Path) -> bool;
    fn metadata(&self, path: &Path) -> Result<EntryInfo, FileSystemError>;
}
```

- [ ] **Step 1.2: Implement `LocalBackend`**

`LocalBackend` is a zero-size struct that delegates every method to the existing free functions in `src/fs.rs`. No logic changes — this is a pure structural refactor.

```rust
pub struct LocalBackend;

impl FsBackend for LocalBackend {
    fn scan_directory(&self, path: &Path) -> Result<Vec<EntryInfo>, FileSystemError> {
        crate::fs::scan_directory(path)
    }
    // ... delegate all others
}
```

- [ ] **Step 1.3: Update `jobs.rs` to use `LocalBackend`**

Workers that currently call `scan_directory` etc. directly now call `backend.scan_directory`. Pass `Arc<dyn FsBackend>` into each worker. For local operations this is `Arc::new(LocalBackend)`.

- [ ] **Step 1.4: Run full test suite to confirm no regressions**

```bash
cargo test --workspace
```

- [ ] **Step 1.5: Commit**

```bash
git commit -m "refactor(fs): extract FsBackend trait; LocalBackend delegates to existing functions"
```

---

## Task 2: SSH connection dialog

**Files:** `src/state/types.rs`, `src/state/ssh.rs`, `src/action.rs`, `src/state/mod.rs`, `src/ui/ssh.rs`

- [ ] **Step 2.1: Add `ModalKind::SshConnect`**

- [ ] **Step 2.2: `SshConnectionState`**

```rust
pub struct SshConnectionState {
    /// Raw input, e.g. "user@example.com:22"
    pub address: String,
    /// "password" or "key"
    pub auth_method: SshAuthMethod,
    /// Password (if method == Password) or path to private key file
    pub credential: String,
    /// Which field has cursor focus: Address, Credential
    pub focused_field: SshDialogField,
    /// Error message from last failed attempt
    pub error: Option<String>,
}

pub enum SshAuthMethod { Password, KeyFile }
pub enum SshDialogField { Address, Credential }
```

- [ ] **Step 2.3: New actions**

```rust
OpenSshConnect,                     // open dialog
SshDialogInput(char),              // type into focused field
SshDialogBackspace,
SshDialogToggleField,              // Tab — switch between address/credential fields
SshDialogToggleAuthMethod,         // Space — switch password/key
SshConnectConfirm,                 // Enter — attempt connection
SshDisconnect,                     // disconnect remote pane
CloseSshConnect,                   // Esc
```

- [ ] **Step 2.4: Render SSH dialog**

Centred modal with:
```
┌─ SSH Connect ──────────────────────────┐
│                                        │
│  Address:     user@host:22█            │
│  Auth:        [Password] / Key File    │
│  Password:    ···········              │
│                                        │
│  Enter=connect  Tab=switch  Esc=cancel │
│                                        │
│  Error: authentication failed          │
└────────────────────────────────────────┘
```

- [ ] **Step 2.5: Commit**

```bash
git commit -m "feat(ui): SSH connection dialog"
```

---

## Task 3: SftpBackend + SftpWorker

**Files:** `src/fs/sftp.rs`, `src/jobs.rs`

- [ ] **Step 3.1: `SftpBackend`**

```rust
use ssh2::{Session, Sftp};

pub struct SftpBackend {
    sftp: Sftp,
}

impl SftpBackend {
    pub fn connect(address: &str, auth: SshAuth) -> Result<Self, FileSystemError> {
        use std::net::TcpStream;
        let (host, port) = parse_address(address)?;
        let tcp = TcpStream::connect((host.as_str(), port))?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;
        match auth {
            SshAuth::Password { user, password } => {
                session.userauth_password(&user, &password)?;
            }
            SshAuth::KeyFile { user, path } => {
                session.userauth_pubkey_file(&user, None, &path, None)?;
            }
        }
        let sftp = session.sftp()?;
        Ok(Self { sftp })
    }
}

impl FsBackend for SftpBackend {
    fn scan_directory(&self, path: &Path) -> Result<Vec<EntryInfo>, FileSystemError> {
        let entries = self.sftp.readdir(path)?;
        // Convert ssh2::FileStat → EntryInfo
        Ok(entries.into_iter().filter_map(|(p, stat)| {
            Some(EntryInfo {
                name: p.file_name()?.to_string_lossy().into_owned(),
                path: p,
                kind: if stat.is_dir() { EntryKind::Directory } else { EntryKind::File },
                size: Some(stat.size.unwrap_or(0)),
                modified: stat.mtime.map(|t| std::time::SystemTime::UNIX_EPOCH
                    + std::time::Duration::from_secs(t)),
            })
        }).collect())
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>, FileSystemError> {
        use std::io::Read;
        let mut remote_file = self.sftp.open(path)?;
        let mut buf = Vec::new();
        remote_file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    // ... implement remaining trait methods via sftp.create, sftp.unlink, etc.
}
```

- [ ] **Step 3.2: `SftpWorker`**

The SFTP worker is a long-lived thread that owns the `SftpBackend` (and thus the `ssh2::Session`). It receives `SftpWorkerRequest` via channel:

```rust
enum SftpWorkerRequest {
    Connect { address: String, auth: SshAuth, reply: oneshot::Sender<Result<SessionId>> },
    Disconnect { session_id: SessionId },
    Scan { pane: PaneId, path: PathBuf, session_id: SessionId },
    ReadFile { path: PathBuf, session_id: SessionId },
}
```

The worker maintains a `HashMap<SessionId, SftpBackend>`. On `Connect`, it creates a new `SftpBackend` and assigns a `SessionId` (UUID or incrementing counter). On `Disconnect`, it drops the backend (which closes the SSH session).

> **Note on oneshot channels:** `crossbeam-channel` doesn't have oneshot. Use `bounded(1)` as a oneshot equivalent, or add `oneshot = "1"` (tiny crate, 0 dependencies). Evaluate at implementation time.

- [ ] **Step 3.3: Commit**

```bash
git commit -m "feat(fs): SftpBackend implementing FsBackend via ssh2; SftpWorker"
```

---

## Task 4: Remote pane mode + navigation

**Files:** `src/pane.rs`, `src/state/mod.rs`

- [ ] **Step 4.1: Add `PaneMode::Remote` variant**

```rust
pub enum PaneMode {
    Real,
    Archive { source: PathBuf, inner_path: PathBuf },
    Remote { session_id: SessionId, remote_path: PathBuf },
}
```

- [ ] **Step 4.2: State handling for `SshConnectConfirm`**

1. Parse address + credentials from `SshConnectionState`
2. Dispatch `SftpWorkerRequest::Connect` via `sftp_tx`
3. On success result, set active pane's `mode = PaneMode::Remote { session_id, remote_path: PathBuf::from("/") }`
4. Dispatch `SftpWorkerRequest::Scan` for the root path

- [ ] **Step 4.3: Route scan/file-op requests for remote panes to SftpWorker**

In `execute_command`, when `ScanPane` is dispatched, check if the pane is in `Remote` mode. If so, send to `sftp_tx` instead of `scan_tx`.

- [ ] **Step 4.4: Handle `SshDisconnect`**

1. Send `SftpWorkerRequest::Disconnect { session_id }`
2. Reset active pane's `mode = PaneMode::Real`
3. Re-scan the pane's last real `cwd`

- [ ] **Step 4.5: Remote breadcrumb in pane title**

```
Right [23]  user@example.com:/home/user/projects  (remote)
```

- [ ] **Step 4.6: Commit**

```bash
git commit -m "feat(state): remote pane mode, SSH connect/disconnect flow"
```

---

## Task 5: Cross-backend file operations

When copying between a local pane and a remote pane, the file-op worker needs to:
1. **Local → Remote:** read local file bytes, write via SFTP
2. **Remote → Local:** read via SFTP, write to local filesystem
3. **Remote → Remote (same session):** SFTP rename/copy on server
4. **Remote → Remote (different sessions):** stream bytes through local memory

- [ ] **Step 5.1: Add `BackendRef` to `FileOpRequest`**

```rust
pub struct FileOpRequest {
    pub operation: FileOperation,
    pub refresh: Vec<RefreshTarget>,
    pub collision: CollisionPolicy,
    pub src_backend: BackendRef,   // Local or Remote(session_id)
    pub dst_backend: BackendRef,
}
```

- [ ] **Step 5.2: File-op worker resolves backends**

The file-op worker gets an `Arc<HashMap<SessionId, SftpBackend>>` reference (or requests backends via channel from the SFTP worker). At implementation time, decide whether to:

- Option A: Pass `Arc<dyn FsBackend>` directly in the request (simpler, avoids cross-channel complexity)
- Option B: File-op worker sends sub-requests to SFTP worker for each read/write (more correct given ssh2 threading constraints)

> **Recommendation:** Start with Option A — clone the `Arc<SftpBackend>` when the operation starts. If `SftpBackend` is `Send`, this works. If not, use Option B.

- [ ] **Step 5.3: Commit**

```bash
git commit -m "feat(jobs): cross-backend file operations (local↔remote, remote↔remote)"
```

---

## Task 6: Final verification

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Manual smoke test:
- Open SSH connect dialog (`Ctrl+Shift+R` or menu → Navigate → Connect SSH)
- Connect to a known host with password auth
- Browse remote filesystem in one pane, local in the other
- Copy file from remote to local → file appears in local pane
- Disconnect → pane returns to local filesystem

```bash
git commit -m "chore: Wave 7A complete — SSH/SFTP remote filesystem support"
```

---

## Known limitations and future improvements

- No host key verification (security risk in untrusted networks — add `~/.ssh/known_hosts` checking in a follow-up)
- No SSH agent support (password and key file only)
- No directory upload/download progress for large transfers
- No persistent connection pooling across app restarts
- Remote → remote copy streams through local memory (no server-side copy)
