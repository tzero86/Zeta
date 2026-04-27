# Preview Enhancements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade image rendering to use terminal graphics protocols (Kitty/Sixels/iTerm2 with halfblock fallback) via `ratatui-image`, and add archive file listings and hex dump previews for binary files.

**Architecture:** `ratatui-image` v10's `Picker` auto-detects the best protocol at startup and is stored in `AppState`; it is passed through `PreviewRequest` to background workers that create `StatefulProtocol` objects. `ViewBuffer` (a struct with optional fields) gains two new fields: `archive_data` and `hex_dump_data`. `ImagePreviewData` replaces its custom scale-cache with an `Arc<Mutex<StatefulProtocol>>`.

**Tech Stack:** Rust stable, ratatui 0.29, ratatui-image 10.0.6 (crossterm feature), image 0.25, zip 0.6, tar 0.4 / flate2 / bzip2 / xz2 (all already present in Cargo.toml).

---

## File Map

| File | Change |
|---|---|
| `Cargo.toml` | `ratatui-image` already added — update version pin to `"10"` |
| `src/preview.rs` | Add `ArchiveListing`, `HexRow`, `HexDumpData`; update `ImagePreviewData`; add fields + constructors to `ViewBuffer` |
| `src/state/mod.rs` | Add `image_picker: Picker` to `AppState`; expose `image_picker()` accessor |
| `src/app.rs` | After `TerminalSession::enter()`, initialize picker via `from_query_stdio()`; include picker in `PreviewRequest` |
| `src/jobs.rs` | Add `picker` to `PreviewRequest`; update `load_image_preview()`; add `load_archive_preview()`, `load_hex_dump_preview()`; update `load_preview_from_bytes()` pipeline |
| `src/ui/preview.rs` | Replace halfblock render loop with `StatefulImage`; add `render_archive_preview()`, `render_hex_dump_preview()`; update dispatch in `render_preview_panel()` |
| `tests/preview_enhancements.rs` | New integration test file |

---

## Task 1: Rename branch and pin ratatui-image version

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Rename branch**

```bash
cd /mnt/c/Users/Zero/Documents/coding/Zeta
git branch -m feature/git-diff-viewer feature/preview-enhancements
```

Expected: no output (success).

- [ ] **Step 2: Pin ratatui-image to major version**

In `Cargo.toml`, the `cargo add` command pinned to an exact version. Change it to a compatible major-version pin:

Find this line:
```toml
ratatui-image = { version = "10.0.6", features = ["crossterm"] }
```

Replace with:
```toml
ratatui-image = { version = "10", features = ["crossterm"] }
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo check 2>&1 | tail -5
```

Expected: `Finished` with no errors. If there are errors, run `cargo check 2>&1` and fix before proceeding.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add ratatui-image dependency, rename branch to preview-enhancements"
```

---

## Task 2: Add `ArchiveListing`, `HexRow`, `HexDumpData` to `src/preview.rs`

**Files:**
- Modify: `src/preview.rs`

- [ ] **Step 1: Write failing unit tests for new data structures**

Add these tests inside the `#[cfg(test)] mod tests` block at the bottom of `src/preview.rs`:

```rust
#[test]
fn archive_listing_is_detected() {
    let listing = ArchiveListing {
        format: ArchiveFormat::Zip,
        entries: vec![
            ArchiveEntry { path: "README.md".into(), size: 1024, compressed_size: Some(512), is_dir: false },
            ArchiveEntry { path: "src/".into(), size: 0, compressed_size: Some(0), is_dir: true },
        ],
        total_entries: 2,
    };
    let vb = ViewBuffer::from_archive(listing);
    assert!(vb.is_archive());
    assert!(!vb.is_image());
    assert!(!vb.is_hex_dump());
    assert!(!vb.is_markdown());
}

#[test]
fn hex_dump_is_detected() {
    let data = HexDumpData {
        rows: vec![
            HexRow {
                offset: "00000000".into(),
                hex_part: "ff d8 ff e0 00 10 4a 46  49 46 00 01 01 00 00 01".into(),
                ascii_part: "..JFIF......".into(),
            },
        ],
        total_bytes: 16,
        truncated: false,
    };
    let vb = ViewBuffer::from_hex_dump(data);
    assert!(vb.is_hex_dump());
    assert!(!vb.is_archive());
    assert!(!vb.is_image());
    assert_eq!(vb.total_lines, 3); // 1 row + 2 (header + footer)
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test --lib preview -- --nocapture 2>&1 | tail -15
```

Expected: compile error — `ArchiveListing`, `HexDumpData`, etc. not yet defined.

- [ ] **Step 3: Add new data types to `src/preview.rs`**

Add this block near the top of `src/preview.rs`, after the existing imports and before `ScaleCache`:

```rust
/// Format identifier for archive previews.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    Tar,
    TarGz,
    TarBz2,
    TarXz,
}

impl ArchiveFormat {
    pub fn label(&self) -> &'static str {
        match self {
            ArchiveFormat::Zip => "ZIP",
            ArchiveFormat::Tar => "TAR",
            ArchiveFormat::TarGz => "TAR.GZ",
            ArchiveFormat::TarBz2 => "TAR.BZ2",
            ArchiveFormat::TarXz => "TAR.XZ",
        }
    }
}

/// A single entry inside an archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub path: String,
    pub size: u64,
    /// Present only for ZIP archives.
    pub compressed_size: Option<u64>,
    pub is_dir: bool,
}

/// File listing extracted from an archive for preview.
/// Capped at `MAX_ARCHIVE_ENTRIES` entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveListing {
    pub format: ArchiveFormat,
    pub entries: Vec<ArchiveEntry>,
    /// Total entries in the archive (may exceed `entries.len()` if capped).
    pub total_entries: usize,
}

pub const MAX_ARCHIVE_ENTRIES: usize = 1_000;

/// One row of a hex dump (16 bytes per row).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HexRow {
    /// Pre-formatted offset string, e.g. `"00000000"`.
    pub offset: String,
    /// Pre-formatted hex bytes with spacing, e.g. `"ff d8 ff e0 00 10 4a 46  49 46 00 01 01 00 00 01"`.
    pub hex_part: String,
    /// Printable ASCII representation; non-printable bytes are `.`.
    pub ascii_part: String,
}

/// Pre-computed hex dump of the first 4 KB of a binary file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HexDumpData {
    pub rows: Vec<HexRow>,
    pub total_bytes: usize,
    /// True when the file exceeds 4 KB and only a prefix is shown.
    pub truncated: bool,
}

pub const HEX_DUMP_MAX_BYTES: usize = 4096;
```

- [ ] **Step 4: Add fields and constructors to `ViewBuffer`**

Add two new fields to the `ViewBuffer` struct:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewBuffer {
    pub lines: Arc<[HighlightedLine]>,
    pub scroll_row: usize,
    pub total_lines: usize,
    pub markdown_source: Option<String>,
    pub image_data: Option<Arc<ImagePreviewData>>,
    pub archive_data: Option<Arc<ArchiveListing>>,   // NEW
    pub hex_dump_data: Option<Arc<HexDumpData>>,     // NEW
}
```

Update every existing `Self { ... }` constructor in `ViewBuffer` to include the two new fields as `None`. There are four constructors: `from_markdown`, `from_image_data`, `from_highlighted`, `from_plain`. Each needs:
```rust
archive_data: None,
hex_dump_data: None,
```

Add two new constructors after `from_plain`:

```rust
/// Build from a pre-computed archive listing.
pub fn from_archive(data: ArchiveListing) -> Self {
    let total_lines = data.entries.len() + 2; // header + entries + footer
    Self {
        lines: Arc::from([]),
        scroll_row: 0,
        total_lines,
        markdown_source: None,
        image_data: None,
        archive_data: Some(Arc::new(data)),
        hex_dump_data: None,
    }
}

/// Build from a pre-computed hex dump.
pub fn from_hex_dump(data: HexDumpData) -> Self {
    let total_lines = data.rows.len() + 2; // header + rows + footer
    Self {
        lines: Arc::from([]),
        scroll_row: 0,
        total_lines,
        markdown_source: None,
        image_data: None,
        archive_data: None,
        hex_dump_data: Some(Arc::new(data)),
    }
}
```

Add two new helper methods alongside `is_image()` and `is_markdown()`:

```rust
/// Returns `true` if this buffer holds an archive file listing.
pub fn is_archive(&self) -> bool {
    self.archive_data.is_some()
}

/// Returns `true` if this buffer holds a hex dump.
pub fn is_hex_dump(&self) -> bool {
    self.hex_dump_data.is_some()
}
```

- [ ] **Step 5: Run tests to confirm they pass**

```bash
cargo test --lib preview -- --nocapture 2>&1 | tail -20
```

Expected: all `preview` tests pass. Look for `test preview::tests::archive_listing_is_detected ... ok` and `hex_dump_is_detected ... ok`.

- [ ] **Step 6: Commit**

```bash
git add src/preview.rs
git commit -m "feat(preview): add ArchiveListing, HexDumpData, HexRow types and ViewBuffer constructors"
```

---

## Task 3: Update `ImagePreviewData` to use `StatefulProtocol`

**Files:**
- Modify: `src/preview.rs`

This replaces the custom halfblock scale-cache with `ratatui-image`'s `StatefulProtocol`.

- [ ] **Step 1: Write a failing unit test**

Add to `#[cfg(test)] mod tests` in `src/preview.rs`:

```rust
#[test]
fn image_preview_data_stores_protocol() {
    use ratatui_image::picker::Picker;
    let picker = Picker::halfblocks();
    let img = image::DynamicImage::new_rgba8(4, 4);
    let proto = picker.new_resize_protocol(img);
    let data = ImagePreviewData::new("test.png".into(), 4, 4, proto);
    assert_eq!(data.filename, "test.png");
    assert_eq!(data.orig_width, 4);
    assert_eq!(data.orig_height, 4);
}
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cargo test --lib preview::tests::image_preview_data_stores_protocol -- --nocapture 2>&1 | tail -10
```

Expected: compile error — `ImagePreviewData::new` signature mismatch.

- [ ] **Step 3: Replace `ImagePreviewData` in `src/preview.rs`**

Delete `ScaleCache` struct, remove the `scale_cache` field, and rewrite `ImagePreviewData`:

Replace the entire block from `/// Cached result of scaling` through `impl Eq for ImagePreviewData {}` (lines 10–98 in the original file) with:

```rust
use ratatui_image::protocol::StatefulProtocol;

/// Pre-decoded image with protocol-specific encoding for terminal graphics rendering.
/// The `StatefulProtocol` handles all resize and encode logic; ratatui-image automatically
/// falls back to halfblock Unicode art when the terminal lacks a graphics protocol.
pub struct ImagePreviewData {
    pub filename: String,
    pub orig_width: u32,
    pub orig_height: u32,
    /// Shared protocol state. `Arc` allows O(1) clone (ViewBuffer cache). `Mutex` provides
    /// interior mutability so `render_stateful_widget` can mutate the protocol from an
    /// immutable `&ViewBuffer` borrow on the UI thread.
    protocol: std::sync::Arc<std::sync::Mutex<StatefulProtocol>>,
}

impl ImagePreviewData {
    pub fn new(
        filename: String,
        orig_width: u32,
        orig_height: u32,
        protocol: StatefulProtocol,
    ) -> Self {
        Self {
            filename,
            orig_width,
            orig_height,
            protocol: std::sync::Arc::new(std::sync::Mutex::new(protocol)),
        }
    }

    /// Acquire a lock on the stateful protocol for rendering.
    /// Called exclusively from the UI render thread — contention is impossible.
    pub fn lock_protocol(
        &self,
    ) -> std::sync::MutexGuard<'_, StatefulProtocol> {
        self.protocol.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Clone for ImagePreviewData {
    fn clone(&self) -> Self {
        Self {
            filename: self.filename.clone(),
            orig_width: self.orig_width,
            orig_height: self.orig_height,
            protocol: std::sync::Arc::clone(&self.protocol),
        }
    }
}

impl std::fmt::Debug for ImagePreviewData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImagePreviewData")
            .field("filename", &self.filename)
            .field("orig_width", &self.orig_width)
            .field("orig_height", &self.orig_height)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ImagePreviewData {
    fn eq(&self, other: &Self) -> bool {
        self.filename == other.filename
            && self.orig_width == other.orig_width
            && self.orig_height == other.orig_height
    }
}

impl Eq for ImagePreviewData {}
```

Also remove the now-unused `use image;` import at the top and replace with:
```rust
use image::DynamicImage;
```
(This will be needed later; keep the import even if unused for now to avoid churn.)

Also remove `use ratatui::style::{Color, Modifier};` if the compiler complains (it's still used by `HighlightedLine` lower in the file).

- [ ] **Step 4: Fix `ViewBuffer::from_image_data`**

The method uses `data.orig_height` which still exists. No change needed there. But the `total_lines` estimate used `orig_height / 2` which is now less relevant (ratatui-image handles sizing). Keep it as is — it drives scroll range.

- [ ] **Step 5: Run tests**

```bash
cargo test --lib preview -- --nocapture 2>&1 | tail -20
```

Expected: all preview tests pass including `image_preview_data_stores_protocol`.

- [ ] **Step 6: Commit**

```bash
git add src/preview.rs
git commit -m "feat(preview): replace ImagePreviewData scale_cache with StatefulProtocol"
```

---

## Task 4: Add `image_picker` to `AppState` and initialize in `App::run()`

**Files:**
- Modify: `src/state/mod.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Add `image_picker` field to `AppState` in `src/state/mod.rs`**

In `src/state/mod.rs`, add to the `AppState` struct (after `debug: DebugState`):

```rust
pub struct AppState {
    workspaces: [WorkspaceState; 4],
    active_workspace_idx: usize,
    pub overlay: OverlayState,
    config_path: String,
    config: AppConfig,
    icon_mode: IconMode,
    theme: ResolvedTheme,
    last_size: Option<(u16, u16)>,
    redraw_count: u64,
    startup_time_ms: u128,
    should_quit: bool,
    needs_redraw: bool,
    pub debug_visible: bool,
    pub debug: DebugState,
    /// Detected terminal graphics protocol picker. Initialized to halfblocks at
    /// bootstrap; upgraded to the best available protocol in `App::run()` after
    /// the terminal enters alternate screen.
    pub image_picker: ratatui_image::picker::Picker,   // NEW
}
```

- [ ] **Step 2: Initialize `image_picker` in `AppState::bootstrap()`**

In the `Ok(Self { ... })` return at the end of `AppState::bootstrap()`, add:

```rust
image_picker: ratatui_image::picker::Picker::halfblocks(),
```

- [ ] **Step 3: Initialize picker properly in `App::run()` after terminal enters raw mode**

In `src/app.rs`, change the beginning of `App::run()`:

```rust
pub fn run(&mut self) -> Result<()> {
    let mut terminal = TerminalSession::enter()?;

    // Query terminal for graphics capabilities now that we are in alternate screen.
    // Falls back silently to halfblocks if the query fails or times out.
    self.state.image_picker =
        ratatui_image::picker::Picker::from_query_stdio()
            .unwrap_or_else(|_| ratatui_image::picker::Picker::halfblocks());

    while !self.state.should_quit() {
```

- [ ] **Step 4: Run cargo check**

```bash
cargo check 2>&1 | tail -10
```

Expected: `Finished` with no errors.

- [ ] **Step 5: Commit**

```bash
git add src/state/mod.rs src/app.rs
git commit -m "feat(app): add image_picker to AppState, initialize after terminal enters raw mode"
```

---

## Task 5: Thread `Picker` through `PreviewRequest` and update `load_image_preview()`

**Files:**
- Modify: `src/jobs.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Add `picker` field to `PreviewRequest` in `src/jobs.rs`**

Find the `PreviewRequest` struct (around line 115) and add the picker field:

```rust
#[derive(Clone, Debug)]
pub struct PreviewRequest {
    pub workspace_id: usize,
    pub path: PathBuf,
    pub syntect_theme: String,
    pub archive: Option<PathBuf>,
    pub inner_path: Option<PathBuf>,
    pub picker: ratatui_image::picker::Picker,   // NEW
}
```

- [ ] **Step 2: Update the `PreviewRequest` send-site in `src/app.rs`**

At `src/app.rs:510`, update the send call to include the picker:

```rust
self.workers
    .preview_tx
    .send(PreviewRequest {
        workspace_id,
        path,
        syntect_theme: self.state.theme().palette.syntect_theme.to_string(),
        archive,
        inner_path: inner,
        picker: self.state.image_picker.clone(),   // NEW
    })
    .context("failed to queue background preview job")?;
```

- [ ] **Step 3: Write a failing test for `load_image_preview` with picker**

Add to the `#[cfg(test)]` block in `src/jobs.rs` (find it near the bottom):

```rust
#[test]
fn load_image_preview_returns_image_buffer_with_halfblocks() {
    use ratatui_image::picker::Picker;

    // 1×1 red RGBA PNG
    let png_bytes: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // PNG signature
        0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
        0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41,
        0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00,
        0x00, 0x00, 0x02, 0x00, 0x01, 0xe2, 0x21, 0xbc,
        0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
        0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    let picker = Picker::halfblocks();
    let path = std::path::Path::new("test.png");
    let vb = load_image_preview(png_bytes, path, &picker);
    assert!(vb.is_image(), "expected Image buffer, got something else");
}
```

- [ ] **Step 4: Run test to confirm it fails**

```bash
cargo test --lib load_image_preview_returns_image_buffer_with_halfblocks -- --nocapture 2>&1 | tail -10
```

Expected: compile error — `load_image_preview` does not accept a `Picker` yet.

- [ ] **Step 5: Update `load_image_preview()` signature and body in `src/jobs.rs`**

Find `fn load_image_preview(bytes: &[u8], path: &Path) -> crate::preview::ViewBuffer` (around line 1186) and replace:

```rust
fn load_image_preview(
    bytes: &[u8],
    path: &Path,
    picker: &ratatui_image::picker::Picker,
) -> crate::preview::ViewBuffer {
    use image::ImageReader;
    use std::io::Cursor;

    let reader = match ImageReader::new(Cursor::new(bytes)).with_guessed_format() {
        Ok(r) => r,
        Err(e) => return crate::preview::ViewBuffer::from_plain(&format!("[image: {e}]")),
    };
    let img = match reader.decode() {
        Ok(img) => img,
        Err(e) => return crate::preview::ViewBuffer::from_plain(&format!("[image: {e}]")),
    };

    let orig_w = img.width();
    let orig_h = img.height();

    let filename = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("image")
        .to_owned();

    let protocol = picker.new_resize_protocol(img);

    crate::preview::ViewBuffer::from_image_data(crate::preview::ImagePreviewData::new(
        filename,
        orig_w,
        orig_h,
        protocol,
    ))
}
```

- [ ] **Step 6: Update every call to `load_image_preview` in `src/jobs.rs`**

Search for all calls to `load_image_preview(` in jobs.rs. There is one in `load_preview_from_bytes()`:

```rust
// Around line 1251 — change:
return load_image_preview(bytes, path);
// To:
return load_image_preview(bytes, path, &req_picker);
```

Wait — `load_preview_from_bytes()` doesn't currently receive a picker. We need to thread it through. Change the signature:

```rust
fn load_preview_from_bytes(
    bytes: &[u8],
    path: &Path,
    syntect_theme: &str,
    picker: &ratatui_image::picker::Picker,
) -> crate::preview::ViewBuffer {
```

And update all calls to `load_preview_from_bytes(...)` to pass `picker`. There are two call sites:

1. In `load_preview_content()` (around line 1227):

```rust
fn load_preview_content(
    path: &Path,
    syntect_theme: &str,
    picker: &ratatui_image::picker::Picker,
) -> crate::preview::ViewBuffer {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return crate::preview::ViewBuffer::from_plain("[empty file]"),
    };
    load_preview_from_bytes(&bytes, path, syntect_theme, picker)
}
```

2. In the archive inner-file extraction branch (around line 523–570), the inline call `load_preview_from_bytes(&buf, &inner_path, &req.syntect_theme)` — add `&req.picker`:

```rust
load_preview_from_bytes(&buf, &inner_path, &req.syntect_theme, &req.picker)
```

3. In the preview worker loop (around line 499–501), update the call to `load_preview_content`:

```rust
let view = if req.archive.is_none() {
    load_preview_content(&req.path, &req.syntect_theme, &req.picker)
```

- [ ] **Step 7: Run the test**

```bash
cargo test --lib load_image_preview_returns_image_buffer_with_halfblocks -- --nocapture 2>&1 | tail -10
```

Expected: `test ... ok`.

- [ ] **Step 8: Run all lib tests to check for regressions**

```bash
cargo test --lib 2>&1 | tail -10
```

Expected: all pass.

- [ ] **Step 9: Commit**

```bash
git add src/jobs.rs src/app.rs
git commit -m "feat(jobs): thread Picker through PreviewRequest, update load_image_preview to use StatefulProtocol"
```

---

## Task 6: Add `load_archive_preview()` to `src/jobs.rs`

**Files:**
- Modify: `src/jobs.rs`

- [ ] **Step 1: Write failing unit tests**

Add to the `#[cfg(test)]` block in `src/jobs.rs`:

```rust
#[test]
fn archive_format_detected_by_extension() {
    assert_eq!(
        archive_format_for_ext("zip"),
        Some(crate::preview::ArchiveFormat::Zip)
    );
    assert_eq!(
        archive_format_for_ext("tar"),
        Some(crate::preview::ArchiveFormat::Tar)
    );
    assert_eq!(
        archive_format_for_ext("tgz"),
        Some(crate::preview::ArchiveFormat::TarGz)
    );
    assert_eq!(archive_format_for_ext("rs"), None);
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test --lib archive_format_detected_by_extension -- --nocapture 2>&1 | tail -5
```

Expected: compile error — `archive_format_for_ext` not defined.

- [ ] **Step 3: Add `archive_format_for_ext` and `load_archive_preview` to `src/jobs.rs`**

Add these functions just before `load_image_preview`:

```rust
/// Map a file extension (lowercase) to an `ArchiveFormat`, or `None` if not an archive.
fn archive_format_for_ext(ext: &str) -> Option<crate::preview::ArchiveFormat> {
    match ext {
        "zip" => Some(crate::preview::ArchiveFormat::Zip),
        "tar" => Some(crate::preview::ArchiveFormat::Tar),
        "tgz" => Some(crate::preview::ArchiveFormat::TarGz),
        "tbz2" => Some(crate::preview::ArchiveFormat::TarBz2),
        "txz" => Some(crate::preview::ArchiveFormat::TarXz),
        _ => None,
    }
}

/// Build an `ArchiveListing` for the given archive bytes.
/// Handles compound extensions like `.tar.gz` by inspecting the full filename.
fn load_archive_preview(bytes: &[u8], path: &Path) -> crate::preview::ViewBuffer {
    use crate::preview::{ArchiveEntry, ArchiveListing, MAX_ARCHIVE_ENTRIES};
    use std::io::Cursor;

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Detect archive format — compound extensions take priority.
    let format = if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        crate::preview::ArchiveFormat::TarGz
    } else if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") {
        crate::preview::ArchiveFormat::TarBz2
    } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        crate::preview::ArchiveFormat::TarXz
    } else if name.ends_with(".tar") {
        crate::preview::ArchiveFormat::Tar
    } else {
        crate::preview::ArchiveFormat::Zip
    };

    let mut entries: Vec<ArchiveEntry> = Vec::new();
    let mut total_entries: usize = 0;
    let mut had_error = false;

    match format {
        crate::preview::ArchiveFormat::Zip => {
            let cursor = Cursor::new(bytes);
            match zip::ZipArchive::new(cursor) {
                Ok(mut archive) => {
                    total_entries = archive.len();
                    for i in 0..archive.len().min(MAX_ARCHIVE_ENTRIES) {
                        if let Ok(file) = archive.by_index(i) {
                            entries.push(ArchiveEntry {
                                path: file.name().to_owned(),
                                size: file.size(),
                                compressed_size: Some(file.compressed_size()),
                                is_dir: file.is_dir(),
                            });
                        }
                    }
                }
                Err(_) => had_error = true,
            }
        }
        crate::preview::ArchiveFormat::Tar
        | crate::preview::ArchiveFormat::TarGz
        | crate::preview::ArchiveFormat::TarBz2
        | crate::preview::ArchiveFormat::TarXz => {
            let cursor = Cursor::new(bytes);
            let reader: Box<dyn std::io::Read> = match format {
                crate::preview::ArchiveFormat::TarGz => {
                    Box::new(flate2::read::GzDecoder::new(cursor))
                }
                crate::preview::ArchiveFormat::TarBz2 => {
                    Box::new(bzip2::read::BzDecoder::new(cursor))
                }
                crate::preview::ArchiveFormat::TarXz => {
                    Box::new(xz2::read::XzDecoder::new(cursor))
                }
                _ => Box::new(cursor),
            };
            let mut archive = tar::Archive::new(reader);
            match archive.entries() {
                Ok(iter) => {
                    for entry in iter.flatten() {
                        total_entries += 1;
                        if entries.len() < MAX_ARCHIVE_ENTRIES {
                            let path_str = entry
                                .path()
                                .ok()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default();
                            let size = entry.header().size().unwrap_or(0);
                            let is_dir = entry.header().entry_type().is_dir();
                            entries.push(ArchiveEntry {
                                path: path_str,
                                size,
                                compressed_size: None,
                                is_dir,
                            });
                        }
                    }
                }
                Err(_) => had_error = true,
            }
        }
    }

    if had_error && entries.is_empty() {
        return crate::preview::ViewBuffer::from_plain("⚠ could not read archive");
    }

    let listing = ArchiveListing {
        format,
        entries,
        total_entries,
    };
    let mut vb = crate::preview::ViewBuffer::from_archive(listing);
    if had_error {
        // Partial listing — scroll_row and total_lines already reflect what we have.
        // The renderer will show the ⚠ footer.
        vb.total_lines = vb.total_lines.max(1);
    }
    vb
}
```

- [ ] **Step 4: Update `load_preview_from_bytes` to call `load_archive_preview`**

In `load_preview_from_bytes`, after the `is_image_ext` branch and before `looks_like_binary`, insert:

```rust
// Archive files: show file listing.
let is_archive_ext = {
    let lower = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    lower.ends_with(".zip")
        || lower.ends_with(".tar")
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar.bz2")
        || lower.ends_with(".tbz2")
        || lower.ends_with(".tar.xz")
        || lower.ends_with(".txz")
};
if is_archive_ext {
    return load_archive_preview(bytes, path);
}
```

- [ ] **Step 5: Run the test**

```bash
cargo test --lib archive_format_detected_by_extension -- --nocapture 2>&1 | tail -5
```

Expected: `test ... ok`.

- [ ] **Step 6: Run all lib tests**

```bash
cargo test --lib 2>&1 | tail -10
```

Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add src/jobs.rs
git commit -m "feat(jobs): add load_archive_preview for ZIP and tar variants"
```

---

## Task 7: Add `load_hex_dump_preview()` to `src/jobs.rs`

**Files:**
- Modify: `src/jobs.rs`

- [ ] **Step 1: Write failing unit tests**

Add to the `#[cfg(test)]` block in `src/jobs.rs`:

```rust
#[test]
fn hex_dump_formats_row_correctly() {
    let row = build_hex_row(0, &[0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46,
                                  0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01]);
    assert_eq!(row.offset, "00000000");
    assert!(row.hex_part.contains("ff d8 ff e0"));
    assert!(row.hex_part.contains("49 46 00 01"));
    // Gap between the two 8-byte groups
    assert!(row.hex_part.contains("  "));
    // Non-printable → dot
    assert!(row.ascii_part.contains('.'));
    // 'J' (0x4a) and 'F' (0x46) are printable
    assert!(row.ascii_part.contains('J'));
    assert!(row.ascii_part.contains('F'));
}

#[test]
fn hex_dump_pads_short_final_row() {
    // 3 bytes — should produce a 16-byte-wide hex field with padding
    let row = build_hex_row(16, &[0xde, 0xad, 0xbe]);
    assert_eq!(row.offset, "00000010");
    // hex_part should be padded to the same width as a full row
    assert_eq!(row.hex_part.len(), "ff d8 ff e0 00 10 4a 46  49 46 00 01 01 00 00 01".len());
}

#[test]
fn load_hex_dump_preview_truncates_at_4kb() {
    let data = vec![0u8; 8192]; // 8 KB of null bytes
    let path = std::path::Path::new("blob.bin");
    let vb = load_hex_dump_preview_internal(&data, path);
    assert!(vb.is_hex_dump());
    let dump = vb.hex_dump_data.as_ref().unwrap();
    assert!(dump.truncated);
    assert_eq!(dump.total_bytes, 8192);
    assert_eq!(dump.rows.len(), crate::preview::HEX_DUMP_MAX_BYTES / 16);
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test --lib hex_dump -- --nocapture 2>&1 | tail -10
```

Expected: compile errors — functions not defined yet.

- [ ] **Step 3: Add `build_hex_row` and `load_hex_dump_preview_internal` to `src/jobs.rs`**

Add these functions near `load_archive_preview`:

```rust
/// Build one 16-byte row of a hex dump.
/// `offset` is the byte offset of the first byte in `chunk`.
/// `chunk` may be shorter than 16 bytes for the final row.
pub(crate) fn build_hex_row(offset: usize, chunk: &[u8]) -> crate::preview::HexRow {
    // Format the 16-byte hex field in two 8-byte groups separated by an extra space.
    let mut hex_parts: Vec<String> = Vec::with_capacity(16);
    for (i, b) in chunk.iter().enumerate() {
        hex_parts.push(format!("{:02x}", b));
        if i == 7 {
            hex_parts.push(String::new()); // extra space between groups
        }
    }
    // Pad to full 16-byte width.
    let full_len = 16;
    let actual_bytes = chunk.len();
    for i in actual_bytes..full_len {
        hex_parts.push("  ".into()); // two spaces for missing byte
        if i == 7 {
            hex_parts.push(String::new());
        }
    }
    // Join with single spaces (the empty string entries produce double-spaces at the group boundary).
    let hex_part = hex_parts.join(" ").trim_end().to_owned();
    // Pad hex_part to the reference width of a full row.
    let reference_width = "ff d8 ff e0 00 10 4a 46  49 46 00 01 01 00 00 01".len();
    let hex_part = format!("{:<width$}", hex_part, width = reference_width);

    let ascii_part: String = chunk
        .iter()
        .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
        .collect();

    crate::preview::HexRow {
        offset: format!("{:08x}", offset),
        hex_part,
        ascii_part,
    }
}

/// Build a `ViewBuffer::HexDump` for the first `HEX_DUMP_MAX_BYTES` of `bytes`.
pub(crate) fn load_hex_dump_preview_internal(
    bytes: &[u8],
    _path: &Path,
) -> crate::preview::ViewBuffer {
    use crate::preview::HEX_DUMP_MAX_BYTES;

    let total_bytes = bytes.len();
    let visible = &bytes[..bytes.len().min(HEX_DUMP_MAX_BYTES)];
    let rows: Vec<crate::preview::HexRow> = visible
        .chunks(16)
        .enumerate()
        .map(|(i, chunk)| build_hex_row(i * 16, chunk))
        .collect();

    let data = crate::preview::HexDumpData {
        rows,
        total_bytes,
        truncated: total_bytes > HEX_DUMP_MAX_BYTES,
    };
    crate::preview::ViewBuffer::from_hex_dump(data)
}

fn load_hex_dump_preview(bytes: &[u8], path: &Path) -> crate::preview::ViewBuffer {
    load_hex_dump_preview_internal(bytes, path)
}
```

- [ ] **Step 4: Update `load_preview_from_bytes` to call `load_hex_dump_preview`**

Replace the `looks_like_binary` branch:

```rust
// OLD:
if looks_like_binary(bytes) {
    let size_bytes = std::fs::metadata(path)
        .map(|m| m.len())
        .unwrap_or(bytes.len() as u64);
    let label = format!("[binary file — {size_bytes} bytes]");
    return crate::preview::ViewBuffer::from_plain(&label);
}

// NEW:
if looks_like_binary(bytes) {
    return load_hex_dump_preview(bytes, path);
}
```

- [ ] **Step 5: Run the hex dump tests**

```bash
cargo test --lib hex_dump -- --nocapture 2>&1 | tail -15
```

Expected: all three hex_dump tests pass.

- [ ] **Step 6: Run all lib tests**

```bash
cargo test --lib 2>&1 | tail -10
```

Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add src/jobs.rs
git commit -m "feat(jobs): add hex dump preview for unknown binary files"
```

---

## Task 8: Update `render_image_preview()` to use `StatefulImage`

**Files:**
- Modify: `src/ui/preview.rs`

- [ ] **Step 1: Add a compile-time check via `cargo check`**

```bash
cargo check 2>&1 | grep "error" | head -10
```

At this point `render_image_preview` still calls `data.scaled_for()` which no longer exists. Confirm the error.

- [ ] **Step 2: Replace `render_image_preview` in `src/ui/preview.rs`**

Find the function starting at `fn render_image_preview(` and replace the entire body with:

```rust
fn render_image_preview(
    frame: &mut Frame<'_>,
    area: Rect,
    view: &ViewBuffer,
    palette: ThemePalette,
) {
    let Some(data) = &view.image_data else {
        return;
    };

    // Header row: filename and original dimensions.
    if area.height > 0 {
        let header = Span::styled(
            format!(
                " {}  {}×{}px ",
                data.filename, data.orig_width, data.orig_height
            ),
            Style::default()
                .fg(palette.text_primary)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![header]))
                .style(Style::default().bg(palette.surface_bg)),
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
        );
    }

    let image_area = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };
    if image_area.height == 0 || image_area.width == 0 {
        return;
    }

    let mut proto = data.lock_protocol();
    let image_widget = ratatui_image::StatefulImage::<ratatui_image::protocol::StatefulProtocol>::default();
    frame.render_stateful_widget(image_widget, image_area, &mut *proto);
}
```

- [ ] **Step 3: cargo check**

```bash
cargo check 2>&1 | tail -10
```

Expected: `Finished` with no errors.

- [ ] **Step 4: Run all lib tests**

```bash
cargo test --lib 2>&1 | tail -10
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/ui/preview.rs
git commit -m "feat(ui): replace halfblock render loop with StatefulImage widget"
```

---

## Task 9: Add `render_archive_preview()`, `render_hex_dump_preview()`, and update dispatch

**Files:**
- Modify: `src/ui/preview.rs`

- [ ] **Step 1: Add `render_archive_preview` to `src/ui/preview.rs`**

Add this function after `render_image_preview`:

```rust
fn render_archive_preview(
    frame: &mut Frame<'_>,
    area: Rect,
    view: &ViewBuffer,
    palette: ThemePalette,
) {
    let Some(data) = &view.archive_data else {
        return;
    };
    if area.height == 0 {
        return;
    }

    // Header row
    let header_text = format!(
        " {}  {} ({} entries) ",
        data.format.label(),
        "",
        data.total_entries
    );
    frame.render_widget(
        Paragraph::new(header_text).style(
            Style::default()
                .fg(palette.text_primary)
                .add_modifier(Modifier::BOLD)
                .bg(palette.surface_bg),
        ),
        Rect { x: area.x, y: area.y, width: area.width, height: 1 },
    );

    if area.height < 2 {
        return;
    }

    let list_area = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(2), // reserve 1 row for footer
        ..area
    };

    let start = view.scroll_row.min(data.entries.len().saturating_sub(1));
    let end = (start + list_area.height as usize).min(data.entries.len());

    for (row_idx, entry) in data.entries[start..end].iter().enumerate() {
        let y = list_area.y + row_idx as u16;
        if y >= list_area.y + list_area.height {
            break;
        }

        let (icon, color) = if entry.is_dir {
            (" ", palette.accent_blue)
        } else {
            (" ", palette.text_primary)
        };

        let size_str = if entry.is_dir {
            String::new()
        } else {
            format_file_size(entry.size)
        };

        let line_text = format!("{}{:<width$} {}", icon, entry.path, size_str, width = (area.width as usize).saturating_sub(14));
        frame.render_widget(
            Paragraph::new(line_text).style(Style::default().fg(color).bg(palette.surface_bg)),
            Rect { x: area.x, y, width: area.width, height: 1 },
        );
    }

    // Footer
    let footer_y = area.y + area.height - 1;
    let capped = if data.total_entries > data.entries.len() {
        format!(
            " {} entries (showing first {}) ",
            data.total_entries,
            data.entries.len()
        )
    } else {
        format!(" {} entries ", data.total_entries)
    };
    frame.render_widget(
        Paragraph::new(capped).style(
            Style::default()
                .fg(palette.text_muted)
                .bg(palette.surface_bg),
        ),
        Rect { x: area.x, y: footer_y, width: area.width, height: 1 },
    );
}

fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
```

- [ ] **Step 2: Add `render_hex_dump_preview` to `src/ui/preview.rs`**

Add after `render_archive_preview`:

```rust
fn render_hex_dump_preview(
    frame: &mut Frame<'_>,
    area: Rect,
    view: &ViewBuffer,
    palette: ThemePalette,
) {
    let Some(data) = &view.hex_dump_data else {
        return;
    };
    if area.height == 0 {
        return;
    }

    // Header row
    let header_text = format!(" HEX  {} bytes ", data.total_bytes);
    frame.render_widget(
        Paragraph::new(header_text).style(
            Style::default()
                .fg(palette.text_primary)
                .add_modifier(Modifier::BOLD)
                .bg(palette.surface_bg),
        ),
        Rect { x: area.x, y: area.y, width: area.width, height: 1 },
    );

    if area.height < 2 {
        return;
    }

    let list_area = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(2), // reserve footer
        ..area
    };

    let start = view.scroll_row.min(data.rows.len().saturating_sub(1));
    let end = (start + list_area.height as usize).min(data.rows.len());

    for (row_idx, hex_row) in data.rows[start..end].iter().enumerate() {
        let y = list_area.y + row_idx as u16;
        if y >= list_area.y + list_area.height {
            break;
        }
        let spans = vec![
            Span::styled(
                format!("{} ", hex_row.offset),
                Style::default().fg(palette.text_muted).bg(palette.surface_bg),
            ),
            Span::styled(
                format!("{} ", hex_row.hex_part),
                Style::default().fg(palette.accent_blue).bg(palette.surface_bg),
            ),
            Span::styled(
                format!("|{}|", hex_row.ascii_part),
                Style::default().fg(palette.accent_amber).bg(palette.surface_bg),
            ),
        ];
        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect { x: area.x, y, width: area.width, height: 1 },
        );
    }

    // Footer
    let footer_y = area.y + area.height - 1;
    let footer_text = if data.truncated {
        format!(" Showing first 4 KB of {} bytes ", data.total_bytes)
    } else {
        format!(" {} bytes ", data.total_bytes)
    };
    frame.render_widget(
        Paragraph::new(footer_text).style(
            Style::default()
                .fg(palette.text_muted)
                .bg(palette.surface_bg),
        ),
        Rect { x: area.x, y: footer_y, width: area.width, height: 1 },
    );
}
```

- [ ] **Step 3: Check that `palette.accent_blue` and `palette.accent_amber` exist**

```bash
grep -n "accent_blue\|accent_amber\|accent_teal" src/config.rs | head -10
```

If `accent_amber` doesn't exist, use `palette.text_primary` instead. Use whatever amber/yellow color is available in `ThemePalette`. Substitute accordingly in both render functions.

- [ ] **Step 4: Update dispatch in `render_preview_panel`**

In `render_preview_panel`, update the non-cheap dispatch block. The current chain is:
```rust
} else if v.is_markdown() { ... }
  else if v.is_image() { ... }
  else { render_wrapped_preview_view(...) }
```

Replace with:
```rust
} else if v.is_markdown() {
    if let Some(source) = v.markdown_source() {
        let widget = Paragraph::new(source)
            .style(Style::default().bg(palette.tools_bg))
            .wrap(Wrap { trim: false })
            .scroll((v.scroll_row as u16, 0));
        frame.render_widget(widget, inner);
    }
} else if v.is_image() {
    render_image_preview(frame, inner, v, palette);
} else if v.is_archive() {
    render_archive_preview(frame, inner, v, palette);
} else if v.is_hex_dump() {
    render_hex_dump_preview(frame, inner, v, palette);
} else {
    let height = inner.height as usize;
    let (first_line_num, window) = v.visible_window(height);
    if window.is_empty() {
        return;
    }
    render_wrapped_preview_view(frame, inner, window, first_line_num + 1, palette);
}
```

Also update the `cheap_mode` branch to handle archive and hex dump (add after the `is_image` check):

```rust
} else if v.is_archive() {
    if let Some(data) = &v.archive_data {
        let text = format!("{} archive ({} entries)", data.format.label(), data.total_entries);
        frame.render_widget(
            Paragraph::new(text).style(Style::default().fg(palette.text_primary).bg(palette.tools_bg)),
            inner,
        );
    }
} else if v.is_hex_dump() {
    if let Some(data) = &v.hex_dump_data {
        let text = format!("Binary file ({} bytes)", data.total_bytes);
        frame.render_widget(
            Paragraph::new(text).style(Style::default().fg(palette.text_muted).bg(palette.tools_bg)),
            inner,
        );
    }
```

- [ ] **Step 5: cargo check**

```bash
cargo check 2>&1 | tail -10
```

Fix any missing palette field references (use `grep "pub " src/config.rs` to see available palette fields).

- [ ] **Step 6: Run all lib tests**

```bash
cargo test --lib 2>&1 | tail -10
```

Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add src/ui/preview.rs
git commit -m "feat(ui): add render_archive_preview and render_hex_dump_preview, update panel dispatch"
```

---

## Task 10: Integration tests

**Files:**
- Create: `tests/preview_enhancements.rs`

- [ ] **Step 1: Create test fixture helpers and test file**

Create `tests/preview_enhancements.rs`:

```rust
//! Integration tests for preview enhancement features:
//! terminal graphics protocol image rendering, archive listings, hex dump.

use std::io::Write;
use tempfile::NamedTempFile;
use zeta::preview::ViewBuffer;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn png_1x1_bytes() -> Vec<u8> {
    // Minimal valid 1×1 RGBA PNG
    vec![
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
        0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
        0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41,
        0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00,
        0x00, 0x00, 0x02, 0x00, 0x01, 0xe2, 0x21, 0xbc,
        0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
        0x44, 0xae, 0x42, 0x60, 0x82,
    ]
}

fn write_temp_file(ext: &str, contents: &[u8]) -> NamedTempFile {
    let mut f = tempfile::Builder::new()
        .suffix(&format!(".{ext}"))
        .tempfile()
        .unwrap();
    f.write_all(contents).unwrap();
    f
}

fn make_zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let opts = zip::write::SimpleFileOptions::default();
        for (name, data) in entries {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    buf
}

// ---------------------------------------------------------------------------
// Image tests
// ---------------------------------------------------------------------------

#[test]
fn png_file_produces_image_buffer_not_hex_dump() {
    let picker = ratatui_image::picker::Picker::halfblocks();
    let path = std::path::Path::new("test.png");
    // Use the internal loader via the test API.
    let vb = zeta::jobs::test_load_image_preview(&png_1x1_bytes(), path, &picker);
    assert!(vb.is_image(), "PNG should produce Image buffer");
    assert!(!vb.is_hex_dump());
}

#[test]
fn halfblocks_picker_always_succeeds() {
    let picker = ratatui_image::picker::Picker::halfblocks();
    let img = image::DynamicImage::new_rgba8(2, 2);
    let _proto = picker.new_resize_protocol(img); // must not panic
}

// ---------------------------------------------------------------------------
// Archive tests
// ---------------------------------------------------------------------------

#[test]
fn zip_archive_produces_archive_listing() {
    let zip_bytes = make_zip_bytes(&[
        ("README.md", b"# hello"),
        ("src/main.rs", b"fn main() {}"),
    ]);
    let path = std::path::Path::new("project.zip");
    let vb = zeta::jobs::test_load_archive_preview(&zip_bytes, path);
    assert!(vb.is_archive(), "ZIP should produce Archive buffer");
    let listing = vb.archive_data.as_ref().unwrap();
    assert_eq!(listing.total_entries, 2);
    assert!(listing.entries.iter().any(|e| e.path == "README.md"));
    assert!(listing.entries.iter().any(|e| e.path == "src/main.rs"));
}

#[test]
fn zip_entry_has_compressed_size() {
    let zip_bytes = make_zip_bytes(&[("hello.txt", b"hello world")]);
    let path = std::path::Path::new("test.zip");
    let vb = zeta::jobs::test_load_archive_preview(&zip_bytes, path);
    let listing = vb.archive_data.as_ref().unwrap();
    assert!(listing.entries[0].compressed_size.is_some());
}

// ---------------------------------------------------------------------------
// Hex dump tests
// ---------------------------------------------------------------------------

#[test]
fn binary_blob_produces_hex_dump() {
    // Null byte triggers binary detection
    let data: Vec<u8> = (0u8..=255).collect();
    let path = std::path::Path::new("blob.bin");
    let vb = zeta::jobs::test_load_hex_dump_preview(&data, path);
    assert!(vb.is_hex_dump());
    let dump = vb.hex_dump_data.as_ref().unwrap();
    assert_eq!(dump.total_bytes, 256);
    assert!(!dump.truncated);
    assert_eq!(dump.rows.len(), 16); // 256 / 16 = 16 rows
}

#[test]
fn hex_dump_truncates_at_4kb() {
    let data = vec![0xffu8; 8192];
    let path = std::path::Path::new("large.bin");
    let vb = zeta::jobs::test_load_hex_dump_preview(&data, path);
    let dump = vb.hex_dump_data.as_ref().unwrap();
    assert!(dump.truncated);
    assert_eq!(dump.total_bytes, 8192);
    assert_eq!(dump.rows.len(), zeta::preview::HEX_DUMP_MAX_BYTES / 16);
}

#[test]
fn hex_dump_offset_column_is_correct() {
    let data: Vec<u8> = vec![0xabu8; 32];
    let path = std::path::Path::new("blob.bin");
    let vb = zeta::jobs::test_load_hex_dump_preview(&data, path);
    let dump = vb.hex_dump_data.as_ref().unwrap();
    assert_eq!(dump.rows[0].offset, "00000000");
    assert_eq!(dump.rows[1].offset, "00000010");
}
```

- [ ] **Step 2: Expose test helpers in `src/jobs.rs` behind `#[cfg(test)]` or a `pub(crate)` gate**

In `src/jobs.rs`, make the internal functions accessible for the integration tests by adding a `pub mod testing` block:

```rust
/// Test-only helpers that expose internal preview functions for integration tests.
#[doc(hidden)]
pub mod testing {
    use std::path::Path;
    use crate::preview::ViewBuffer;

    pub fn load_image_preview(
        bytes: &[u8],
        path: &Path,
        picker: &ratatui_image::picker::Picker,
    ) -> ViewBuffer {
        super::load_image_preview(bytes, path, picker)
    }

    pub fn load_archive_preview(bytes: &[u8], path: &Path) -> ViewBuffer {
        super::load_archive_preview(bytes, path)
    }

    pub fn load_hex_dump_preview(bytes: &[u8], path: &Path) -> ViewBuffer {
        super::load_hex_dump_preview(bytes, path)
    }
}
```

Then update the integration test imports:
```rust
// In tests/preview_enhancements.rs, replace zeta::jobs::test_load_* with:
use zeta::jobs::testing::{load_archive_preview as test_load_archive_preview, ...};
```

Actually, to keep the test file cleaner, expose them directly:

```rust
// In tests/preview_enhancements.rs:
fn load_image_preview_test(bytes: &[u8], path: &std::path::Path, picker: &ratatui_image::picker::Picker) -> ViewBuffer {
    zeta::jobs::testing::load_image_preview(bytes, path, picker)
}
// etc.
```

Or just use the module path directly in the assertions.

- [ ] **Step 3: Run integration tests**

```bash
cargo test --tests preview_enhancements -- --nocapture 2>&1 | tail -20
```

Fix any compilation errors (missing pub exports, wrong paths). Then:

Expected output: all tests pass with lines like:
```
test png_file_produces_image_buffer_not_hex_dump ... ok
test halfblocks_picker_always_succeeds ... ok
test zip_archive_produces_archive_listing ... ok
...
```

- [ ] **Step 4: Run full test suite**

```bash
cargo test --workspace 2>&1 | tail -15
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add tests/preview_enhancements.rs src/jobs.rs
git commit -m "test: add integration tests for image protocol, archive listing, hex dump"
```

---

## Task 11: Pre-PR validation

- [ ] **Step 1: Format check**

```bash
cargo fmt --all -- --check 2>&1
```

If there are formatting issues: `cargo fmt --all` then re-check.

- [ ] **Step 2: Clippy**

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -20
```

Fix any warnings before proceeding.

- [ ] **Step 3: Full test suite**

```bash
cargo test --workspace 2>&1 | tail -15
```

Expected: all tests pass.

- [ ] **Step 4: Final commit (if formatting/clippy fixes were needed)**

```bash
git add -A
git commit -m "chore: fmt and clippy fixes"
```

- [ ] **Step 5: Verify branch name**

```bash
git branch --show-current
```

Expected: `feature/preview-enhancements`
