# Preview Enhancements Design

**Date:** 2026-04-26  
**Branch:** `feature/preview-enhancements` (renamed from `feature/git-diff-viewer`)  
**Scope:** Terminal graphics protocol image rendering + archive listings + hex dump for binary files

---

## Problem

The preview panel has two gaps:

1. **Images** render via a hand-rolled halfblock Unicode renderer. Modern terminal emulators (Kitty, WezTerm, Ghostty, iTerm2) support pixel-level graphics protocols that produce dramatically better image quality. The existing code does not detect or use these protocols.

2. **Binary files** (non-image) show only `[binary file — N bytes]`. Archives are never listed. Unknown binaries give the user no useful information.

---

## Approach

- **Image rendering:** Replace the custom halfblock renderer with `ratatui-image`, which auto-detects the best available protocol (Kitty → Sixels → iTerm2 → halfblock) and provides a `StatefulWidget` interface. Halfblock remains the final fallback.
- **Binary enhancements:** Add archive file listing (ZIP, tar variants) and a classic hex dump for unknown binaries. Reuse existing `zip`/`tar`/`flate2`/`bzip2`/`xz2` dependencies — no new binary deps required.

---

## Architecture

### New Dependency

```toml
ratatui-image = { version = "2", features = ["crossterm"] }
```

This is the only new dependency. The `image` crate is already present for decoding.

### Protocol Detection Chain

```
Kitty → Sixels → iTerm2 → Halfblock Unicode
```

`ratatui-image`'s `Picker::from_query_stdio()` determines the best available protocol at startup by querying the terminal for font metrics and testing protocol support. Fully automatic — no user configuration required.

### Startup: Picker Initialization

`Picker` is created once on the main thread before the TUI enters raw mode, then stored in `AppState`. It is cheaply cloneable and sent to background worker threads when images are loaded.

```rust
// AppState addition
pub struct AppState {
    pub image_picker: ratatui_image::picker::Picker,
    // ...existing fields
}
```

If `Picker::from_query_stdio()` fails, fall back to `Picker::halfblocks()` — never panic.

### ImagePreviewData Changes

`scale_cache` is removed. `StatefulProtocol` replaces it. The protocol handles resize internally, so the manual viewport-keyed cache becomes unnecessary.

```rust
// BEFORE
pub struct ImagePreviewData {
    raw: DynamicImage,
    scale_cache: Mutex<HashMap<(u32, u32), RgbaImage>>,
}

// AFTER
pub struct ImagePreviewData {
    raw: DynamicImage,
    protocol: Mutex<Box<dyn StatefulProtocol>>,
}
```

`StatefulProtocol` is created in the background worker (`load_image_preview()`) using the cloned `Picker`. The UI thread only calls `render_stateful_widget`.

### Render Change

`render_image_preview()` in `src/ui/preview.rs` shrinks from ~80 lines to ~10:

```rust
fn render_image_preview(frame: &mut Frame, area: Rect, data: &ImagePreviewData) {
    let mut proto = data.protocol.lock().unwrap();
    frame.render_stateful_widget(StatefulImage::default(), area, &mut *proto);
}
```

On terminal resize, `StatefulProtocol` detects the area change and re-encodes on the next render automatically.

### New ViewBuffer Variants

```rust
pub enum ViewBuffer {
    Text(PreviewLines),         // existing
    Highlighted(PreviewLines),  // existing
    Image(ImagePreviewData),    // existing — upgraded internals
    Markdown(String),           // existing
    Archive(ArchiveListing),    // NEW
    HexDump(HexDumpData),       // NEW
}
```

### Archive Listing

**Supported formats:** `.zip`, `.tar`, `.tar.gz`, `.tgz`, `.tar.bz2`, `.tbz2`, `.tar.xz`, `.txz`

```rust
pub struct ArchiveListing {
    pub format: ArchiveFormat,
    pub entries: Vec<ArchiveEntry>,
    pub total_entries: usize,
}

pub enum ArchiveFormat { Zip, Tar, TarGz, TarBz2, TarXz }

pub struct ArchiveEntry {
    pub path: String,
    pub size: u64,
    pub compressed_size: Option<u64>,  // ZIP only
    pub is_dir: bool,
}
```

Loaded by new `load_archive_preview()` in `src/jobs.rs`. Capped at 1,000 entries. Directories styled in blue; files in default color. Footer shows total entry count and aggregate uncompressed size.

### Hex Dump

```rust
pub struct HexDumpData {
    pub lines: Vec<String>,   // pre-rendered at load time
    pub total_bytes: usize,
}
```

Format per line: `00000000  ff d8 ff e0 00 10 4a 46  49 46 00 01 01 00 00 01  |..JFIF......|`

- Offset: dim/grey
- Hex bytes: alternating blue/green per 4-byte group
- ASCII sidebar: printable chars in amber, non-printable as `.`
- First 4 KB shown (256 rows). Footer: `Showing first 4 KB of N bytes`

Pre-rendered as `Vec<String>` at load time — zero computation during scroll.

### Updated Detection Pipeline (`src/jobs.rs`)

```
load_preview_from_bytes()
├─ image ext (png/jpg/jpeg/gif/bmp/webp)     → load_image_preview()    [upgraded]
├─ archive ext (zip/tar/tgz/tar.gz/...)      → load_archive_preview()  [NEW]
├─ looks_like_binary() (null-byte heuristic) → load_hex_dump_preview() [NEW]
└─ else                                      → syntax highlight or plain text
```

---

## Error Handling

| Failure | Behaviour |
|---|---|
| Picker query fails | Fall back to `Picker::halfblocks()` — never crash |
| Protocol encode error | Show `⚠ image render error` in preview area; log detail |
| Corrupt archive | Show partial listing + `⚠ truncated (read error)` footer |
| Archive > 1,000 entries | Show first 1,000 + `… N more entries` footer |
| Hex dump > 4 KB | Show first 4 KB + `Showing first 4 KB of N bytes` footer |
| Permission denied | Existing error path unchanged |
| Unsupported image format | Falls through to hex dump path |

---

## Performance

- `Picker::from_query_stdio()` runs once at startup (~1 ms); stored and cloned cheaply.
- `StatefulProtocol` is created off the UI thread (background worker). Re-encoding on resize is lazy — happens on next render call, not synchronously.
- Archive listing capped at 1,000 entries; always loaded in background.
- Hex dump pre-renders all lines at load time; scroll is pure text output.
- Existing 8-entry LRU preview cache is unchanged — all new `ViewBuffer` variants fit the same slot.

---

## Config Changes

None. Protocol detection is fully automatic. A future `image_protocol = "auto" | "halfblock"` config key is an acceptable follow-up if users need to force a downgrade, but is explicitly out of scope here.

---

## Testing Plan

### Unit Tests (adjacent to modules)

- Hex dump: correct offset/hex/ASCII for known byte sequences
- Hex dump: non-printable bytes render as `.`
- Hex dump: last row shorter than 16 bytes pads correctly
- Archive format detection: correct `ArchiveFormat` enum per extension
- Archive listing: entry cap enforced at 1,000
- `ViewBuffer` helpers: `is_image()`, `is_archive()`, `is_hex_dump()`

### Integration Tests (`tests/`)

- Load a real ZIP fixture → `ArchiveListing` with correct entries and sizes
- Load a real `.tar.gz` fixture → same
- Load a PNG → `ViewBuffer::Image`, not `HexDump`
- Load a random binary blob → `ViewBuffer::HexDump`
- Corrupt ZIP → partial listing with error footer text present
- Halfblock picker always succeeds (no terminal required in test environment)

---

## Files Changed

| File | Change |
|---|---|
| `Cargo.toml` | Add `ratatui-image = { version = "2", features = ["crossterm"] }` |
| `src/app.rs` | Initialize `Picker` at startup, store in `AppState` |
| `src/preview.rs` | Replace `scale_cache` with `StatefulProtocol`; add `ArchiveListing`, `HexDumpData`, new `ViewBuffer` variants |
| `src/jobs.rs` | Upgrade `load_image_preview()`; add `load_archive_preview()`, `load_hex_dump_preview()` |
| `src/ui/preview.rs` | Simplify `render_image_preview()`; add `render_archive_preview()`, `render_hex_dump_preview()` |
| `tests/preview_enhancements.rs` | New integration test file |
