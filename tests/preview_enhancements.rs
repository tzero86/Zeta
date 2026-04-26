//! Integration tests for preview enhancement features:
//! terminal graphics protocol image rendering, archive listings, hex dump.

use std::io::Write;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn png_1x1_bytes() -> Vec<u8> {
    // Generate a valid 1×1 RGBA PNG using the image crate.
    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
    let mut buf = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .unwrap();
    buf
}

fn make_zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let opts = zip::write::FileOptions::default();
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
    let zip_bytes = make_zip_bytes(&[("README.md", b"# hello"), ("src/main.rs", b"fn main() {}")]);
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
