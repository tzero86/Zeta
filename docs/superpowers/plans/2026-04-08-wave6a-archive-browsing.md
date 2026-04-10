# Wave 6A — Archive Browsing

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the user navigate into `.zip`, `.tar`, `.tar.gz`, `.tar.bz2`, and `.tar.xz` archives as if they were read-only directories. Pressing Enter on a supported archive opens it in the active pane. Pressing Backspace or navigating up exits back to the real filesystem. Extraction is via F5 (copy) into a real destination directory.

**Architecture:**
- `PaneMode` enum: `Real` (normal filesystem) or `Archive { source: PathBuf, inner_path: PathBuf }`.
- `PaneState` gains a `mode: PaneMode` field. All pane navigation logic branches on this.
- `ArchiveWorker` — sixth background worker. Receives `ArchiveListRequest`, runs listing in a thread (can be slow for large archives), returns `JobResult::ArchiveListed { entries: Vec<EntryInfo> }`.
- Extraction uses the existing `FileOperation::Copy` infrastructure but with a new `ArchiveExtractRequest` that the file-op worker handles.
- `EntryKind::Archive` added to `fs.rs` — used by the pane renderer to show the archive icon and by the entry-open logic.

**New dependencies:**
- `zip = "2"` — ZIP reading (pure Rust, no system lib)
- `tar = "0.4"` — TAR reading
- `flate2 = "1"` — gzip decompression (already a transitive dep via syntect; verify before adding)
- `bzip2 = "0.4"` — bzip2 decompression
- `xz2 = "0.1"` — xz/lzma decompression (links against system liblzma)

> **Binary size note:** `zip` and `tar` + `flate2` are pure Rust and add ~200 KB to the release binary. `bzip2` and `xz2` link C libraries; if binary size is a concern, skip those formats initially and add them later.

**Jira:** ZTA-156 (ZTA-158 through ZTA-163)

**Wave dependency:** Starts AFTER Wave 5C. No other waves depend on this.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `Cargo.toml` | Add `zip`, `tar`, `flate2`, optional `bzip2`, `xz2` |
| Modify | `src/fs.rs` | `EntryKind::Archive`; `list_archive(path) -> Result<Vec<EntryInfo>>`; `extract_archive(src, dest_dir)` |
| Modify | `src/pane.rs` | `PaneMode` enum; `PaneState.mode`; `in_archive()`, `archive_source()` helpers |
| Modify | `src/jobs.rs` | `ArchiveListRequest`, `JobResult::ArchiveListed`, `ArchiveWorker`, `archive_tx` on `WorkerChannels` |
| Modify | `src/state/mod.rs` | Handle `Enter` on archive entry → `OpenArchive`; handle `ArchiveListed` job result; `ExitArchive` action |
| Modify | `src/action.rs` | `OpenArchive { path: PathBuf }`, `ExitArchive` |
| Modify | `src/ui/pane.rs` | Show archive breadcrumb path when in archive mode; `EntryKind::Archive` icon |

---

## Supported formats

| Extension(s) | Crate |
|---|---|
| `.zip` | `zip` |
| `.tar` | `tar` |
| `.tar.gz`, `.tgz` | `tar` + `flate2` |
| `.tar.bz2`, `.tbz2` | `tar` + `bzip2` |
| `.tar.xz`, `.txz` | `tar` + `xz2` |

Detection is by file extension (case-insensitive). Magic-byte detection is a future improvement.

---

## Task 1: EntryKind::Archive + fs helpers

**Files:** `src/fs.rs`

- [ ] **Step 1.1: Add `EntryKind::Archive` variant**

```rust
pub enum EntryKind {
    Directory,
    File,
    Symlink,
    Archive,   // ← new
}
```

- [ ] **Step 1.2: Update `scan_directory` to detect archives**

When building `EntryInfo` for a file, check if its extension matches a supported archive format and set `kind = EntryKind::Archive`.

```rust
fn is_archive_extension(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".zip")
        || lower.ends_with(".tar")
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar.bz2")
        || lower.ends_with(".tbz2")
        || lower.ends_with(".tar.xz")
        || lower.ends_with(".txz")
}
```

- [ ] **Step 1.3: Implement `list_archive(path: &Path) -> Result<Vec<EntryInfo>, FileSystemError>`**

Returns a flat list of `EntryInfo` entries representing the top-level contents of the archive. Directories within the archive are collapsed to their top-level entries only (non-recursive for the initial view).

For ZIP:
```rust
fn list_zip(path: &Path) -> Result<Vec<EntryInfo>> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut seen = std::collections::HashSet::new();
    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        // Extract top-level component only.
        let top = entry.name().split('/').next().unwrap_or("").to_string();
        if top.is_empty() || !seen.insert(top.clone()) { continue; }
        let is_dir = entry.is_dir() || entry.name().contains('/');
        entries.push(EntryInfo {
            name: top.clone(),
            path: path.join(&top),   // virtual path for display
            kind: if is_dir { EntryKind::Directory } else { EntryKind::File },
            size: if is_dir { None } else { Some(entry.size()) },
            modified: None,
        });
    }
    Ok(entries)
}
```

Similar implementations for TAR variants.

- [ ] **Step 1.4: Implement `extract_archive(src: &Path, dest_dir: &Path) -> Result<(), FileSystemError>`**

Extracts the entire archive into `dest_dir`. Used by the file-op worker when copying FROM an archive.

- [ ] **Step 1.5: Tests**

```rust
#[test]
fn archive_extension_detection_is_case_insensitive() { ... }

#[test]
fn list_zip_returns_top_level_entries() {
    // Create a temp zip file, list it, verify entries.
}
```

- [ ] **Step 1.6: Commit**

```bash
git commit -m "feat(fs): EntryKind::Archive, list_archive, extract_archive"
```

---

## Task 2: PaneMode + pane navigation

**Files:** `src/pane.rs`

- [ ] **Step 2.1: Add `PaneMode`**

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum PaneMode {
    #[default]
    Real,
    Archive {
        /// Path of the archive file on the real filesystem.
        source: PathBuf,
        /// Current inner path within the archive (e.g. `"subdir/"` for nested navigation).
        inner_path: PathBuf,
    },
}
```

- [ ] **Step 2.2: Add to `PaneState`**

```rust
pub struct PaneState {
    // ... existing ...
    pub mode: PaneMode,
}
```

- [ ] **Step 2.3: Add helpers**

```rust
impl PaneState {
    pub fn in_archive(&self) -> bool {
        matches!(self.mode, PaneMode::Archive { .. })
    }

    pub fn archive_source(&self) -> Option<&Path> {
        match &self.mode {
            PaneMode::Archive { source, .. } => Some(source),
            PaneMode::Real => None,
        }
    }
}
```

- [ ] **Step 2.4: Update `navigate_up` / `go_to_parent` logic**

When in archive mode, `navigate_up` pops the inner path component. When at the root of the archive, exit archive mode and return to the real filesystem directory containing the archive.

- [ ] **Step 2.5: Commit**

```bash
git commit -m "feat(pane): add PaneMode::Archive; in_archive() / archive_source() helpers"
```

---

## Task 3: ArchiveWorker + job result

**Files:** `src/jobs.rs`

- [ ] **Step 3.1: Add types**

```rust
#[derive(Clone, Debug)]
pub struct ArchiveListRequest {
    pub pane: PaneId,
    pub archive_path: PathBuf,
    pub inner_path: PathBuf,
}

// In JobResult:
ArchiveListed {
    pane: PaneId,
    archive_path: PathBuf,
    inner_path: PathBuf,
    entries: Vec<crate::fs::EntryInfo>,
},
ArchiveError {
    pane: PaneId,
    message: String,
},
```

- [ ] **Step 3.2: Add `archive_tx` to `WorkerChannels` and spawn ArchiveWorker**

The worker calls `crate::fs::list_archive` and fans the result into `result_tx`.

- [ ] **Step 3.3: Tests**

```rust
#[test]
fn archive_worker_responds_to_request() { ... }
```

- [ ] **Step 3.4: Commit**

```bash
git commit -m "feat(jobs): ArchiveWorker — lists archive contents in background"
```

---

## Task 4: State and action wiring

**Files:** `src/action.rs`, `src/state/mod.rs`, `src/app.rs`

- [ ] **Step 4.1: New actions**

```rust
OpenArchive { path: PathBuf },
ExitArchive,
```

- [ ] **Step 4.2: `EnterSelection` branching — open archive vs. enter directory**

In `apply_view` where `EnterSelection` is handled:

```rust
if entry.kind == EntryKind::Archive {
    commands.push(Command::ListArchive {
        pane: active_pane_id,
        archive_path: entry.path.clone(),
        inner_path: PathBuf::new(),
    });
} else {
    // existing directory scan logic
}
```

- [ ] **Step 4.3: Handle `ArchiveListed` in `apply_job_result`**

```rust
JobResult::ArchiveListed { pane, archive_path, inner_path, entries } => {
    let pane_state = self.panes.pane_mut(pane);
    pane_state.mode = PaneMode::Archive { source: archive_path, inner_path };
    pane_state.set_entries(entries);
    pane_state.selection = 0;
}
```

- [ ] **Step 4.4: Handle `ExitArchive`**

Reset `pane_state.mode = PaneMode::Real` and re-scan the containing directory.

- [ ] **Step 4.5: Commit**

```bash
git commit -m "feat(state): wire OpenArchive / ExitArchive into pane navigation"
```

---

## Task 5: Render archive breadcrumb

**Files:** `src/ui/pane.rs`

- [ ] **Step 5.1: Update pane title when in archive mode**

```rust
let title = if pane.in_archive() {
    format!(
        "{} [{}]  {} :: {}",
        label,
        pane.entries.len(),
        pane.archive_source().unwrap().display(),
        pane.mode_inner_path().display(),
    )
} else {
    // existing title format
};
```

- [ ] **Step 5.2: Archive icon for `EntryKind::Archive`**

Add a distinct icon (e.g. `📦` in unicode mode, `[A]` in ASCII mode) for archive entries in `icon_for_kind`.

- [ ] **Step 5.3: Commit**

```bash
git commit -m "feat(ui): archive breadcrumb in pane title; archive icon"
```

---

## Task 6: Extraction via F5

When an archive pane is the source and a real-filesystem pane is the destination, F5 (copy) should extract the selected entry rather than doing a raw file copy.

- [ ] **Step 6.1: Detect extraction case in `OpenCopyPrompt` handling**

If the active pane is in archive mode, push a `FileOperation::ExtractArchive { archive: PathBuf, inner: PathBuf, destination: PathBuf }` instead of the regular `Copy`.

- [ ] **Step 6.2: Handle `ExtractArchive` in file-op worker**

Calls `crate::fs::extract_archive`.

- [ ] **Step 6.3: Commit**

```bash
git commit -m "feat: archive extraction via F5 into real-filesystem pane"
```

---

## Final verification

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
git commit -m "chore: Wave 6A complete — archive browsing (zip, tar.gz, tar.bz2, tar.xz)"
```
