use std::sync::Arc;

#[cfg(test)]
use image;
use ratatui::style::{Color, Modifier};

#[cfg(test)]
use crate::highlight::HighlightToken;
use crate::highlight::{normalize_preview_text, HighlightedLine};

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

use ratatui_image::protocol::StatefulProtocol;

/// Pre-decoded image with protocol-specific encoding for terminal graphics rendering.
/// `ratatui-image` auto-detects Kitty → Sixels → iTerm2 → halfblock based on the terminal.
pub struct ImagePreviewData {
    pub filename: String,
    pub orig_width: u32,
    pub orig_height: u32,
    /// Shared protocol state. `Arc` allows O(1) clone (ViewBuffer cache).
    /// `Mutex` provides interior mutability so `render_stateful_widget` can
    /// mutate the protocol from an immutable `&ViewBuffer` borrow on the UI thread.
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
    /// Only called from the UI render thread — contention is impossible.
    pub fn lock_protocol(&self) -> std::sync::MutexGuard<'_, StatefulProtocol> {
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

/// A read-only scrollable view of syntax-highlighted file content.
/// Used by the preview panel. Scroll state is owned here.
///
/// `lines` is wrapped in `Arc<[_]>` so that `ViewBuffer::clone()` is O(1) —
/// the highlighted content is immutable after construction, and the preview
/// cache can hold a shared reference without paying a full deep-copy on every
/// cache hit during navigation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewBuffer {
    pub lines: Arc<[HighlightedLine]>,
    pub scroll_row: usize,
    pub total_lines: usize,
    /// Raw Markdown source. When `Some`, the buffer represents a markdown
    /// document to be rendered by `tui_markdown` at display time.
    /// `lines` is empty for markdown buffers.
    pub markdown_source: Option<String>,
    /// Pre-decoded image for halfblock rendering; `None` for non-image buffers.
    pub image_data: Option<Arc<ImagePreviewData>>,
    pub archive_data: Option<Arc<ArchiveListing>>,
    pub hex_dump_data: Option<Arc<HexDumpData>>,
}

impl ViewBuffer {
    /// Build from a raw Markdown string — rendered as wrapped plain text in the
    /// preview panel. Kept as a distinct variant so the renderer can apply
    /// markdown-specific layout (no gutter, word-wrap) and a future tui-markdown
    /// integration can be dropped in with a one-line change.
    pub fn from_markdown(source: String) -> Self {
        let total_lines = source.lines().count().max(1);
        Self {
            lines: Arc::from([]),
            scroll_row: 0,
            total_lines,
            markdown_source: Some(source),
            image_data: None,
            archive_data: None,
            hex_dump_data: None,
        }
    }

    /// Returns `true` if this buffer holds raw Markdown source.
    pub fn is_markdown(&self) -> bool {
        self.markdown_source.is_some()
    }

    /// Returns the raw Markdown source, or `None` for non-markdown buffers.
    pub fn markdown_source(&self) -> Option<&str> {
        self.markdown_source.as_deref()
    }

    /// Returns `true` if this buffer represents a decoded image.
    pub fn is_image(&self) -> bool {
        self.image_data.is_some()
    }

    /// Returns `true` if this buffer holds an archive file listing.
    pub fn is_archive(&self) -> bool {
        self.archive_data.is_some()
    }

    /// Returns `true` if this buffer holds a hex dump.
    pub fn is_hex_dump(&self) -> bool {
        self.hex_dump_data.is_some()
    }

    /// Build from pre-decoded image pixel data.
    pub fn from_image_data(data: ImagePreviewData) -> Self {
        // Estimate total cell-rows: half the pixel height (halfblock = 2 rows/cell)
        let total_lines = (data.orig_height / 2 + 2) as usize;
        Self {
            lines: Arc::from([]),
            scroll_row: 0,
            total_lines,
            markdown_source: None,
            image_data: Some(Arc::new(data)),
            archive_data: None,
            hex_dump_data: None,
        }
    }

    /// Build a sanitized read-only preview buffer from raw text.
    pub fn from_render_text(text: &str) -> Self {
        Self::from_plain(text)
    }

    /// Build from pre-highlighted lines (from syntect).
    pub fn from_highlighted(lines: Vec<HighlightedLine>) -> Self {
        let total_lines = lines.len();
        Self {
            lines: lines.into(), // Vec<HighlightedLine> → Arc<[HighlightedLine]>, O(1) clone
            scroll_row: 0,
            total_lines,
            markdown_source: None,
            image_data: None,
            archive_data: None,
            hex_dump_data: None,
        }
    }

    /// Build from plain text — each line becomes a single unstyled token.
    pub fn from_plain(text: &str) -> Self {
        let text = normalize_preview_text(text);
        let lines: Vec<HighlightedLine> = text
            .lines()
            .map(|l| vec![(Color::Reset, Modifier::empty(), Box::from(l))])
            .collect();
        let total_lines = lines.len();
        Self {
            lines: lines.into(), // Vec → Arc<[_]>
            scroll_row: 0,
            total_lines,
            markdown_source: None,
            image_data: None,
            archive_data: None,
            hex_dump_data: None,
        }
    }

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

    /// Returns the slice of lines visible starting at scroll_row, clamped to height.
    pub fn visible_window(&self, height: usize) -> (usize, &[HighlightedLine]) {
        if self.lines.is_empty() {
            return (0, &[]);
        }
        let start = self.scroll_row.min(self.total_lines.saturating_sub(1));
        let end = (start + height).min(self.total_lines);
        (start, &self.lines[start..end])
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll_row = self
            .scroll_row
            .saturating_add(n)
            .min(self.total_lines.saturating_sub(1));
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_row = self.scroll_row.saturating_sub(n);
    }

    pub fn reset_scroll(&mut self) {
        self.scroll_row = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_highlighted(n: usize) -> Vec<HighlightedLine> {
        (0..n)
            .map(|i| {
                vec![(
                    Color::White,
                    Modifier::empty(),
                    format!("line {i}").into_boxed_str(),
                )]
            })
            .collect()
    }

    #[test]
    fn markdown_variant_stores_raw_text() {
        let vb = ViewBuffer::from_markdown("# Hello\n\nWorld".to_string());
        assert!(vb.is_markdown());
        assert_eq!(vb.markdown_source(), Some("# Hello\n\nWorld"));
    }

    #[test]
    fn non_markdown_variant_returns_none_for_markdown_source() {
        let vb = ViewBuffer::from_plain("hello");
        assert!(!vb.is_markdown());
        assert_eq!(vb.markdown_source(), None);
    }

    #[test]
    fn from_plain_builds_correct_total() {
        let text = "alpha\nbeta\ngamma";
        let buf = ViewBuffer::from_render_text(text);
        assert_eq!(buf.total_lines, 3);
        assert_eq!(buf.scroll_row, 0);
        assert_eq!(buf.lines.len(), 3);

        // Each line should be a single unstyled token.
        let token: &HighlightToken = &buf.lines[1][0];
        assert_eq!(token.0, Color::Reset);
        assert_eq!(token.1, Modifier::empty());
        assert_eq!(token.2.as_ref(), "beta");
    }

    #[test]
    fn preview_prep_strips_control_chars_and_preserves_visible_width() {
        let buf = ViewBuffer::from_plain("alpha\r\nbeta\nchar\tlie\nwide: 測試\u{0007}");

        assert_eq!(buf.lines.len(), 4);
        assert!(buf
            .lines
            .iter()
            .all(|line| line.iter().all(|token| !token.2.contains('\r'))));
        assert!(buf
            .lines
            .iter()
            .all(|line| line.iter().all(|token| !token.2.contains('\u{0007}'))));
        assert_eq!(buf.lines[2][0].2.as_ref(), "char    lie");
        assert!(buf
            .lines
            .iter()
            .any(|line| line.iter().any(|token| token.2.contains("wide: 測試"))));
    }

    #[test]
    fn visible_window_clamps_to_available_lines() {
        let buf = ViewBuffer::from_highlighted(make_highlighted(5));

        // Asking for more lines than available should return all 5.
        let (start, window) = buf.visible_window(10);
        assert_eq!(start, 0);
        assert_eq!(window.len(), 5);

        // Asking for exactly 3 lines returns first 3.
        let (start, window) = buf.visible_window(3);
        assert_eq!(start, 0);
        assert_eq!(window.len(), 3);

        // Empty buffer always returns an empty slice.
        let empty = ViewBuffer::from_highlighted(vec![]);
        let (start, window) = empty.visible_window(10);
        assert_eq!(start, 0);
        assert_eq!(window.len(), 0);
    }

    #[test]
    fn scroll_down_clamps_at_last_line() {
        let mut buf = ViewBuffer::from_highlighted(make_highlighted(5));

        // Scroll far past the end — should clamp to last valid index (4).
        buf.scroll_down(100);
        assert_eq!(buf.scroll_row, 4);

        // Scrolling further should stay clamped.
        buf.scroll_down(1);
        assert_eq!(buf.scroll_row, 4);
    }

    #[test]
    fn scroll_up_clamps_at_zero() {
        let mut buf = ViewBuffer::from_highlighted(make_highlighted(5));
        buf.scroll_down(3);
        assert_eq!(buf.scroll_row, 3);

        buf.scroll_up(1);
        assert_eq!(buf.scroll_row, 2);

        // Scrolling past zero should clamp.
        buf.scroll_up(100);
        assert_eq!(buf.scroll_row, 0);

        // Scrolling up again at zero should stay at zero.
        buf.scroll_up(1);
        assert_eq!(buf.scroll_row, 0);
    }

    #[test]
    fn archive_listing_is_detected() {
        let listing = ArchiveListing {
            format: ArchiveFormat::Zip,
            entries: vec![
                ArchiveEntry {
                    path: "README.md".into(),
                    size: 1024,
                    compressed_size: Some(512),
                    is_dir: false,
                },
                ArchiveEntry {
                    path: "src/".into(),
                    size: 0,
                    compressed_size: Some(0),
                    is_dir: true,
                },
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
            rows: vec![HexRow {
                offset: "00000000".into(),
                hex_part: "ff d8 ff e0 00 10 4a 46  49 46 00 01 01 00 00 01".into(),
                ascii_part: "..JFIF......".into(),
            }],
            total_bytes: 16,
            truncated: false,
        };
        let vb = ViewBuffer::from_hex_dump(data);
        assert!(vb.is_hex_dump());
        assert!(!vb.is_archive());
        assert!(!vb.is_image());
        assert_eq!(vb.total_lines, 3); // 1 row + 2 (header + footer)
    }

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
}
