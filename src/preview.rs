use std::sync::{Arc, Mutex};

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

/// Cached result of scaling `ImagePreviewData::pixels` to a specific viewport.
#[derive(Debug)]
struct ScaleCache {
    target_w: u32,
    target_h: u32,
    scaled: image::RgbaImage,
}

/// Pre-decoded image pixels for viewport-adaptive halfblock rendering.
#[derive(Debug)]
pub struct ImagePreviewData {
    pub filename: String,
    pub orig_width: u32,
    pub orig_height: u32,
    /// Decoded RGBA pixels (possibly pre-scaled, max 800px wide).
    /// Scaled to exact viewport size at render time; result cached in
    /// `scale_cache` so the resize only runs when the viewport changes.
    pub pixels: Arc<image::RgbaImage>,
    /// Last viewport-scaled render, keyed by (target_w, target_h).
    /// Mutex is used so `ImagePreviewData` is Sync (required for Arc<...>).
    /// The lock is only ever acquired from the UI render thread, so contention
    /// is impossible and the overhead is negligible.
    scale_cache: Mutex<Option<ScaleCache>>,
}

impl ImagePreviewData {
    pub fn new(
        filename: String,
        orig_width: u32,
        orig_height: u32,
        pixels: Arc<image::RgbaImage>,
    ) -> Self {
        Self {
            filename,
            orig_width,
            orig_height,
            pixels,
            scale_cache: Mutex::new(None),
        }
    }

    /// Return the image scaled to `(target_w, target_h)`.
    /// Uses the cached result if dimensions haven't changed; otherwise resamples
    /// with `Triangle` (bilinear) — fast enough for the UI thread, visually
    /// indistinguishable from Lanczos3 at terminal halfblock resolution.
    pub fn scaled_for(&self, target_w: u32, target_h: u32) -> image::RgbaImage {
        let mut cache = self.scale_cache.lock().unwrap_or_else(|e| e.into_inner());
        if !cache
            .as_ref()
            .is_some_and(|c| c.target_w == target_w && c.target_h == target_h)
        {
            let scaled = image::imageops::resize(
                self.pixels.as_ref(),
                target_w,
                target_h,
                image::imageops::FilterType::Triangle,
            );
            *cache = Some(ScaleCache {
                target_w,
                target_h,
                scaled,
            });
        }
        cache.as_ref().unwrap().scaled.clone()
    }
}

impl Clone for ImagePreviewData {
    fn clone(&self) -> Self {
        // Cloning intentionally drops the render cache; the new instance will
        // repopulate it on first render.
        Self::new(
            self.filename.clone(),
            self.orig_width,
            self.orig_height,
            Arc::clone(&self.pixels),
        )
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
}
