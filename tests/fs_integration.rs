use std::fs;
use std::path::{Path, PathBuf};

use zeta::action::CollisionPolicy;
use zeta::fs::{
    copy_path, create_directory, create_file, delete_path, looks_like_binary, rename_path,
    scan_directory, suggest_non_conflicting_path, EntryKind, FileSystemError,
};

// ---------------------------------------------------------------------------
// Temp-directory helpers
// ---------------------------------------------------------------------------

fn temp_dir_path(prefix: &str) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    std::env::temp_dir().join(format!("zeta_test_{}_{}", prefix, ts))
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(prefix: &str) -> Self {
        let p = temp_dir_path(prefix);
        fs::create_dir_all(&p).expect("temp dir should be created");
        Self(p)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn child(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// ---------------------------------------------------------------------------
// Category 1 — scan_directory
// ---------------------------------------------------------------------------

#[test]
fn scan_empty_directory_returns_empty() {
    let dir = TempDir::new("scan_empty");
    let entries = scan_directory(dir.path()).expect("scan should succeed");
    assert!(entries.is_empty(), "expected no entries in empty dir");
}

#[test]
fn scan_returns_files_and_directories() {
    let dir = TempDir::new("scan_mixed");
    fs::create_dir(dir.child("sub_dir")).expect("subdir should be created");
    fs::write(dir.child("file.txt"), "hello").expect("file should be written");

    let entries = scan_directory(dir.path()).expect("scan should succeed");

    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(
        names.contains(&"sub_dir"),
        "scan should include subdirectory"
    );
    assert!(names.contains(&"file.txt"), "scan should include file");

    let dir_entry = entries
        .iter()
        .find(|e| e.name == "sub_dir")
        .expect("directory entry should exist");
    let file_entry = entries
        .iter()
        .find(|e| e.name == "file.txt")
        .expect("file entry should exist");

    assert_eq!(dir_entry.kind, EntryKind::Directory);
    assert_eq!(file_entry.kind, EntryKind::File);
}

#[test]
fn scan_includes_hidden_files() {
    let dir = TempDir::new("scan_hidden");
    fs::write(dir.child(".hidden"), "secret").expect("hidden file should be written");
    fs::write(dir.child("visible.txt"), "visible").expect("visible file should be written");

    let entries = scan_directory(dir.path()).expect("scan should succeed");
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();

    assert!(
        names.contains(&".hidden"),
        "raw scan should include hidden files"
    );
    assert!(
        names.contains(&"visible.txt"),
        "raw scan should include visible files"
    );
}

#[test]
fn scan_non_directory_returns_error() {
    let dir = TempDir::new("scan_non_dir");
    let file_path = dir.child("notadir.txt");
    fs::write(&file_path, "content").expect("file should be written");

    let result = scan_directory(&file_path);
    assert!(
        result.is_err(),
        "scanning a file path should return an error"
    );
}

#[test]
fn scan_handles_unicode_filenames() {
    let dir = TempDir::new("scan_unicode");
    let unicode_name = "日本語ファイル.txt";
    fs::write(dir.child(unicode_name), "こんにちは").expect("unicode file should be written");

    let entries = scan_directory(dir.path()).expect("scan should succeed");
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();

    assert!(
        names.contains(&unicode_name),
        "unicode filename should round-trip through scan"
    );
}

// ---------------------------------------------------------------------------
// Category 2 — copy_path / create_file / create_directory
// ---------------------------------------------------------------------------

#[test]
fn copy_creates_destination_file() {
    let dir = TempDir::new("copy_basic");
    let src = dir.child("source.txt");
    let dst = dir.child("dest.txt");
    fs::write(&src, "data").expect("source should be written");

    copy_path(&src, &dst, CollisionPolicy::Fail).expect("copy should succeed");

    assert!(dst.exists(), "destination file should exist after copy");
}

#[test]
fn copy_fail_policy_rejects_existing_destination() {
    let dir = TempDir::new("copy_fail");
    let src = dir.child("source.txt");
    let dst = dir.child("dest.txt");
    fs::write(&src, "new").expect("source should be written");
    fs::write(&dst, "existing").expect("destination should be written");

    let err = copy_path(&src, &dst, CollisionPolicy::Fail)
        .expect_err("Fail policy should reject existing destination");
    assert!(
        matches!(err, FileSystemError::PathExists { .. }),
        "expected PathExists error, got: {err}"
    );
}

#[test]
fn copy_overwrite_policy_replaces_existing() {
    let dir = TempDir::new("copy_overwrite");
    let src = dir.child("source.txt");
    let dst = dir.child("dest.txt");
    fs::write(&src, "new-content").expect("source should be written");
    fs::write(&dst, "old-content").expect("destination should be written");

    copy_path(&src, &dst, CollisionPolicy::Overwrite).expect("overwrite copy should succeed");

    let result = fs::read_to_string(&dst).expect("destination should be readable");
    assert_eq!(
        result, "new-content",
        "overwrite should replace file content"
    );
}

#[test]
fn create_file_creates_empty_file() {
    let dir = TempDir::new("create_file");
    let path = dir.child("new_file.txt");

    create_file(&path, CollisionPolicy::Fail).expect("file creation should succeed");

    assert!(path.exists(), "created file should exist");
    let metadata = fs::metadata(&path).expect("metadata should be readable");
    assert_eq!(metadata.len(), 0, "newly created file should be empty");
}

#[test]
fn create_file_fail_policy_rejects_existing() {
    let dir = TempDir::new("create_file_fail");
    let path = dir.child("existing.txt");
    fs::write(&path, "already here").expect("file should be pre-created");

    let err = create_file(&path, CollisionPolicy::Fail)
        .expect_err("Fail policy should reject existing file");
    assert!(
        matches!(err, FileSystemError::PathExists { .. }),
        "expected PathExists error, got: {err}"
    );
}

#[test]
fn create_directory_creates_dir() {
    let dir = TempDir::new("create_dir");
    let new_dir = dir.child("brand_new_dir");

    create_directory(&new_dir, CollisionPolicy::Fail).expect("directory creation should succeed");

    assert!(new_dir.exists(), "created directory should exist");
    assert!(new_dir.is_dir(), "created path should be a directory");
}

// ---------------------------------------------------------------------------
// Category 3 — rename_path / delete_path
// ---------------------------------------------------------------------------

#[test]
fn rename_moves_file_to_new_name() {
    let dir = TempDir::new("rename_basic");
    let src = dir.child("old_name.txt");
    let dst = dir.child("new_name.txt");
    fs::write(&src, "content").expect("source should be written");

    rename_path(&src, &dst, CollisionPolicy::Fail).expect("rename should succeed");

    assert!(!src.exists(), "source should no longer exist after rename");
    assert!(dst.exists(), "destination should exist after rename");
}

#[test]
fn rename_rejects_existing_destination() {
    let dir = TempDir::new("rename_fail");
    let src = dir.child("source.txt");
    let dst = dir.child("dest.txt");
    fs::write(&src, "source content").expect("source should be written");
    fs::write(&dst, "dest content").expect("destination should be pre-created");

    let err = rename_path(&src, &dst, CollisionPolicy::Fail)
        .expect_err("Fail policy should reject existing destination");
    assert!(
        matches!(err, FileSystemError::PathExists { .. }),
        "expected PathExists error, got: {err}"
    );
}

#[test]
fn delete_removes_file() {
    let dir = TempDir::new("delete_file");
    let path = dir.child("to_delete.txt");
    fs::write(&path, "goodbye").expect("file should be written");

    delete_path(&path).expect("delete should succeed");

    assert!(!path.exists(), "file should not exist after deletion");
}

#[test]
fn delete_missing_file_returns_error() {
    let dir = TempDir::new("delete_missing");
    let path = dir.child("does_not_exist.txt");

    let err = delete_path(&path).expect_err("deleting missing file should return an error");
    assert!(
        matches!(err, FileSystemError::DeletePath { .. }),
        "expected DeletePath error, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// Category 4 — suggest_non_conflicting_path
// ---------------------------------------------------------------------------

#[test]
fn suggest_non_conflicting_adds_suffix_when_exists() {
    let dir = TempDir::new("suggest_suffix");
    let original = dir.child("note.txt");
    fs::write(&original, "data").expect("original should be written");

    let suggestion = suggest_non_conflicting_path(&original);

    assert_eq!(
        suggestion,
        dir.child("note-1.txt"),
        "should suggest -1 suffix when original exists"
    );
}

#[test]
fn suggest_non_conflicting_increments_when_first_suffix_taken() {
    let dir = TempDir::new("suggest_increment");
    let original = dir.child("note.txt");
    fs::write(&original, "data").expect("original should be written");
    fs::write(dir.child("note-1.txt"), "data").expect("first collision should be written");

    let suggestion = suggest_non_conflicting_path(&original);

    assert_eq!(
        suggestion,
        dir.child("note-2.txt"),
        "should suggest -2 suffix when -1 also exists"
    );
}

// ---------------------------------------------------------------------------
// Category 5 — looks_like_binary
// ---------------------------------------------------------------------------

#[test]
fn looks_like_binary_empty_is_not_binary() {
    assert!(
        !looks_like_binary(&[]),
        "empty byte slice should not be considered binary"
    );
}

#[test]
fn looks_like_binary_null_byte_detected() {
    let bytes = b"hello\x00world";
    assert!(
        looks_like_binary(bytes),
        "bytes containing null should be detected as binary"
    );
}

#[test]
fn looks_like_binary_ascii_text_is_not_binary() {
    let text = b"The quick brown fox jumps over the lazy dog.\n";
    assert!(
        !looks_like_binary(text),
        "plain ASCII text should not be considered binary"
    );
}

// ---------------------------------------------------------------------------
// Category 6 — edge cases
// ---------------------------------------------------------------------------

#[test]
fn copy_preserves_file_contents() {
    let dir = TempDir::new("copy_contents");
    let src = dir.child("source.txt");
    let dst = dir.child("dest.txt");
    let content = "Line 1\nLine 2\nLine 3\n";
    fs::write(&src, content).expect("source should be written");

    copy_path(&src, &dst, CollisionPolicy::Fail).expect("copy should succeed");

    let copied = fs::read_to_string(&dst).expect("destination should be readable");
    assert_eq!(
        copied, content,
        "copied file should have identical contents"
    );
}

#[test]
fn copy_to_missing_parent_returns_error() {
    let dir = TempDir::new("copy_no_parent");
    let src = dir.child("source.txt");
    // Destination parent directory does not exist.
    let dst = dir.child("nonexistent_subdir").join("dest.txt");
    fs::write(&src, "data").expect("source should be written");

    let result = copy_path(&src, &dst, CollisionPolicy::Fail);
    assert!(
        result.is_err(),
        "copy to a path with a missing parent directory should fail"
    );
}

#[test]
fn copy_preserves_unicode_content() {
    let dir = TempDir::new("copy_unicode");
    let src = dir.child("unicode_src.txt");
    let dst = dir.child("unicode_dst.txt");
    let content = "こんにちは世界 — Héllo Wörld — 🦀";
    fs::write(&src, content).expect("unicode source should be written");

    copy_path(&src, &dst, CollisionPolicy::Fail).expect("copy of unicode content should succeed");

    let copied = fs::read_to_string(&dst).expect("destination should be readable");
    assert_eq!(
        copied, content,
        "unicode content should survive a copy round-trip"
    );
}
