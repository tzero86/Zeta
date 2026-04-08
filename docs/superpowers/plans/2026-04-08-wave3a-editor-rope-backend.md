# Wave 3A — Editor Performance: Rope-Backed Buffer + Lightweight Undo Stack

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the editor performance regression introduced in Wave 1C. Replace `tui_textarea::TextArea` as the backing store with `ropey::Rope` (already in `Cargo.toml`) and a hand-rolled lightweight undo/redo stack. Keep every public method signature on `EditorBuffer` identical so no callers change.

---

## Background — Why TextArea Is Slow for Large Files

Wave 1C replaced `ropey::Rope` with `tui_textarea::TextArea<'static>` to gain undo/redo and clipboard support cheaply. The trade-off bit us:

1. **History snapshots.** tui-textarea records undo entries as full copies of affected lines. For files with long lines or many edits, this causes heap pressure and allocation spikes on every keystroke.
2. **Wrong use case.** `TextArea` is designed as a TUI *widget* (search box, single-line input). It holds lines as `Vec<String>` and recalculates internal state on every `input()` call — including re-counting characters for cursor management. On a 50 000-line file this is O(1) per operation but with a constant that is noticeably high because the struct is not designed to be mutated at 60 fps in an editor loop.
3. **`insert_str` cursor behaviour.** Our compatibility `insert(char_idx, text)` method calls `move_cursor(Jump(row, col))` followed by `insert_str`. The cursor jump involves a linear scan of the internal line slice to validate bounds, which is O(line_count) in degenerate cases.

**Rope fixes all of this.** `ropey::Rope` uses a B-tree of chunks, giving O(log n) char-index → line-col conversion, O(log n) insertion and deletion, and O(1) line count. We build a minimal undo stack that stores only the *delta* (char range + replacement text), making history entries tiny regardless of file size.

---

## Architecture

```
EditorBuffer
├── text: ropey::Rope          ← primary storage (O(log n) ops)
├── cursor_char_idx: usize     ← single source of truth for cursor
├── history: UndoStack         ← new module: delta-based undo/redo
│   ├── entries: Vec<Edit>
│   ├── head: usize
│   └── Edit { char_start, removed, inserted }
├── path, is_dirty, search_*   ← unchanged public fields
└── scroll_col: usize          ← unchanged
```

`UndoStack` is a private sub-module in `src/editor.rs`. It is not exposed publicly. The two new public methods `undo()` and `redo()` drive it.

**No other files change.** `tui-textarea` is removed from `Cargo.toml` entirely — it is not used anywhere else.

---

## Public Interface (unchanged — zero caller changes required)

```rust
// Fields — same names and types as before Wave 1C
pub path: Option<PathBuf>
pub is_dirty: bool
pub search_query: String
pub search_active: bool
pub search_match_idx: usize
pub scroll_col: usize

// Methods — identical signatures
fn open(path: &Path) -> Result<Self, EditorError>
fn insert(&mut self, char_idx: usize, text: &str)          // compat
fn insert_char(&mut self, ch: char)
fn insert_newline(&mut self)
fn backspace(&mut self)
fn move_left(&mut self)
fn move_right(&mut self)
fn move_up(&mut self)
fn move_down(&mut self)
fn undo(&mut self)                                          // new in Wave 1C
fn redo(&mut self)                                          // new in Wave 1C
fn save(&mut self) -> Result<(), EditorError>
fn contents(&self) -> String
fn visible_lines(&self) -> Vec<String>
fn visible_highlighted_window(...) -> (usize, Vec<HighlightedLine>)
fn visible_line_window(height: usize) -> (usize, Vec<String>)
fn cursor_line_col(&self) -> (usize, usize)
fn clamp_horizontal_scroll(&mut self, viewport_cols: usize)
fn render_state(&mut self, ...) -> EditorRenderState
fn find_matches(&self, query: &str) -> Vec<(usize, usize)>
fn search_next(&mut self)
fn search_prev(&mut self)
```

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `Cargo.toml` | Remove `tui-textarea`; `ropey` is already present |
| Modify | `src/editor.rs` | Replace TextArea backend with Rope + UndoStack |

---

## Task 1: Restore ropey::Rope as text backend, add UndoStack

**Files:**
- Modify: `Cargo.toml`, `src/editor.rs`

- [ ] **Step 1.1: Write the failing performance-proxy test**

Add to the `tests` module in `src/editor.rs`:

```rust
#[test]
fn large_file_insert_does_not_degrade_linearly() {
    // Proxy test: inserting into a 10 000-line buffer must complete within
    // 50 ms total (not per-keystroke). This will time out under TextArea.
    use std::time::Instant;
    let mut buf = EditorBuffer::default();
    let big = (0..10_000).map(|i| format!("line {i}\n")).collect::<String>();
    buf.insert(0, &big);
    let start = Instant::now();
    for ch in "hello world".chars() {
        buf.insert_char(ch);
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 50,
        "11 keystrokes took {}ms — expected < 50ms",
        elapsed.as_millis()
    );
}
```

- [ ] **Step 1.2: Confirm it fails (times out or takes > 50 ms)**

```bash
cargo test large_file_insert_does_not_degrade -- --nocapture 2>&1 | tail -10
```

Expected: test fails (exceeds 50 ms) or takes noticeably long.

- [ ] **Step 1.3: Remove `tui-textarea` from `Cargo.toml`**

```toml
# Remove this line:
tui-textarea = { version = "0.7", features = ["crossterm"] }
```

`ropey = "1.6"` is already in `[dependencies]` from before Wave 1C. No addition needed.

- [ ] **Step 1.4: Rewrite `src/editor.rs`**

Replace the entire file with the Rope + UndoStack implementation below.

**Key design points:**
- `cursor_char_idx` is the single cursor position as a Rope char index (same as pre-Wave-1C).
- `UndoStack` stores `Edit { char_start: usize, removed: String, inserted: String }`. An edit records what was at `char_start..char_start+inserted.len()` before and after the change. Undo replays in reverse; redo replays forward.
- Every mutating method (`insert_char`, `backspace`, etc.) pushes an `Edit` onto the stack and advances `head`.
- `undo()`/`redo()` replay edits in reverse/forward order and restore `cursor_char_idx`.

```rust
use std::fs as std_fs;
use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier};
use ropey::Rope;
use thiserror::Error;

use crate::highlight::{highlight_text, normalize_preview_text, HighlightedLine};

// ---------------------------------------------------------------------------
// Public render state
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorRenderState {
    pub visible_start: usize,
    pub visible_lines: Vec<String>,
    pub cursor_visible_row: Option<usize>,
    pub scroll_col: usize,
}

// ---------------------------------------------------------------------------
// Undo stack
// ---------------------------------------------------------------------------

/// A single reversible edit.
#[derive(Clone, Debug)]
struct Edit {
    /// Char index where the change begins.
    char_start: usize,
    /// Text that was removed (empty for pure insertions).
    removed: String,
    /// Text that was inserted (empty for pure deletions).
    inserted: String,
    /// Cursor position *before* this edit (restored on undo).
    cursor_before: usize,
    /// Cursor position *after* this edit (restored on redo).
    cursor_after: usize,
}

#[derive(Clone, Debug, Default)]
struct UndoStack {
    entries: Vec<Edit>,
    /// Points to the next slot to write into.
    /// `head == entries.len()` means "nothing to redo".
    head: usize,
}

impl UndoStack {
    /// Record a new edit, discarding any redo history above `head`.
    fn push(&mut self, edit: Edit) {
        self.entries.truncate(self.head);
        self.entries.push(edit);
        self.head = self.entries.len();
    }

    /// Returns the edit to undo and decrements head, or `None` if at bottom.
    fn pop_undo(&mut self) -> Option<&Edit> {
        if self.head == 0 {
            return None;
        }
        self.head -= 1;
        Some(&self.entries[self.head])
    }

    /// Returns the edit to redo and increments head, or `None` if at top.
    fn pop_redo(&mut self) -> Option<&Edit> {
        if self.head == self.entries.len() {
            return None;
        }
        let edit = &self.entries[self.head];
        self.head += 1;
        Some(edit)
    }
}

// ---------------------------------------------------------------------------
// EditorBuffer
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct EditorBuffer {
    pub path: Option<PathBuf>,
    pub is_dirty: bool,
    pub search_query: String,
    pub search_active: bool,
    pub search_match_idx: usize,
    pub scroll_col: usize,
    // Private — implementation detail
    text: Rope,
    cursor_char_idx: usize,
    history: UndoStack,
}

impl EditorBuffer {
    pub fn open(path: &Path) -> Result<Self, EditorError> {
        let bytes = std_fs::read(path).map_err(|source| EditorError::ReadFile {
            path: path.display().to_string(),
            source,
        })?;
        let contents = String::from_utf8_lossy(&bytes);
        Ok(Self {
            path: Some(path.to_path_buf()),
            text: Rope::from_str(&contents),
            ..Self::default()
        })
    }

    // -----------------------------------------------------------------------
    // Compatibility insert — used by integration tests in state/mod.rs
    // -----------------------------------------------------------------------

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        let idx = char_idx.min(self.text.len_chars());
        let edit = Edit {
            char_start: idx,
            removed: String::new(),
            inserted: text.to_string(),
            cursor_before: self.cursor_char_idx,
            cursor_after: idx + text.chars().count(),
        };
        self.text.insert(idx, text);
        self.cursor_char_idx = edit.cursor_after;
        self.history.push(edit);
        self.is_dirty = true;
    }

    // -----------------------------------------------------------------------
    // Text mutation
    // -----------------------------------------------------------------------

    pub fn insert_char(&mut self, ch: char) {
        let idx = self.cursor_char_idx.min(self.text.len_chars());
        let edit = Edit {
            char_start: idx,
            removed: String::new(),
            inserted: ch.to_string(),
            cursor_before: self.cursor_char_idx,
            cursor_after: idx + 1,
        };
        self.text.insert_char(idx, ch);
        self.cursor_char_idx = edit.cursor_after;
        self.history.push(edit);
        self.is_dirty = true;
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn backspace(&mut self) {
        if self.cursor_char_idx == 0 {
            return;
        }
        let start = self.cursor_char_idx - 1;
        let removed_char = self.text.char(start).to_string();
        let edit = Edit {
            char_start: start,
            removed: removed_char,
            inserted: String::new(),
            cursor_before: self.cursor_char_idx,
            cursor_after: start,
        };
        self.text.remove(start..self.cursor_char_idx);
        self.cursor_char_idx = start;
        self.history.push(edit);
        self.is_dirty = true;
    }

    pub fn move_left(&mut self) {
        self.cursor_char_idx = self.cursor_char_idx.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.cursor_char_idx =
            (self.cursor_char_idx + 1).min(self.text.len_chars());
    }

    pub fn move_up(&mut self) {
        let (line, col) = self.cursor_line_col();
        if line == 0 {
            return;
        }
        self.cursor_char_idx = self.line_col_to_char(line - 1, col);
    }

    pub fn move_down(&mut self) {
        let (line, col) = self.cursor_line_col();
        if line + 1 >= self.text.len_lines() {
            return;
        }
        self.cursor_char_idx = self.line_col_to_char(line + 1, col);
    }

    // -----------------------------------------------------------------------
    // Undo / redo
    // -----------------------------------------------------------------------

    pub fn undo(&mut self) {
        let Some(edit) = self.history.pop_undo() else { return };
        // Reverse the edit: remove what was inserted, re-insert what was removed.
        let insert_end = edit.char_start + edit.inserted.chars().count();
        if insert_end > edit.char_start {
            self.text.remove(edit.char_start..insert_end);
        }
        if !edit.removed.is_empty() {
            self.text.insert(edit.char_start, &edit.removed);
        }
        self.cursor_char_idx = edit.cursor_before;
        self.is_dirty = self.history.head > 0 || self.text.len_chars() > 0;
    }

    pub fn redo(&mut self) {
        let Some(edit) = self.history.pop_redo() else { return };
        // Re-apply: remove what was removed, insert what was inserted.
        let removed_end = edit.char_start + edit.removed.chars().count();
        if removed_end > edit.char_start {
            self.text.remove(edit.char_start..removed_end);
        }
        if !edit.inserted.is_empty() {
            self.text.insert(edit.char_start, &edit.inserted);
        }
        self.cursor_char_idx = edit.cursor_after;
        self.is_dirty = true;
    }

    // -----------------------------------------------------------------------
    // Persistence
    // -----------------------------------------------------------------------

    pub fn save(&mut self) -> Result<(), EditorError> {
        let path = self.path.clone().ok_or(EditorError::MissingPath)?;
        std_fs::write(&path, self.text.to_string()).map_err(|source| {
            EditorError::WriteFile { path: path.display().to_string(), source }
        })?;
        self.is_dirty = false;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Content accessors
    // -----------------------------------------------------------------------

    pub fn contents(&self) -> String {
        self.text.to_string()
    }

    pub fn visible_lines(&self) -> Vec<String> {
        self.text
            .lines()
            .map(|l| {
                let s = normalize_preview_text(&l.to_string());
                s.strip_suffix('\n').unwrap_or(&s).to_string()
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    pub fn visible_highlighted_window(
        &self,
        height: usize,
        syntect_theme: &str,
        fallback_color: Color,
    ) -> (usize, Vec<HighlightedLine>) {
        if height == 0 {
            return (0, Vec::new());
        }
        let text = normalize_preview_text(&self.text.to_string());
        let ext = self
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str());
        let (start, _) = self.visible_line_window(height);
        let lines = match highlight_text(&text, ext, syntect_theme) {
            Some(lines) => lines.into_iter().skip(start).take(height).collect(),
            None => {
                let (_, window) = self.visible_line_window(height);
                window
                    .into_iter()
                    .map(|l| vec![(fallback_color, Modifier::empty(), l)])
                    .collect()
            }
        };
        (start, lines)
    }

    pub fn visible_line_window(&self, height: usize) -> (usize, Vec<String>) {
        if height == 0 {
            return (0, Vec::new());
        }
        let all = self.visible_lines();
        let (cursor_line, _) = self.cursor_line_col();
        let start = if cursor_line + 1 > height { cursor_line + 1 - height } else { 0 };
        let visible = all.into_iter().skip(start).take(height).collect();
        (start, visible)
    }

    pub fn cursor_line_col(&self) -> (usize, usize) {
        let safe = self.cursor_char_idx.min(self.text.len_chars());
        let line = self.text.char_to_line(safe);
        let line_start = self.text.line_to_char(line);
        (line, safe.saturating_sub(line_start))
    }

    pub fn clamp_horizontal_scroll(&mut self, viewport_cols: usize) {
        let (_, col) = self.cursor_line_col();
        if col < self.scroll_col {
            self.scroll_col = col;
        } else if viewport_cols > 0 && col >= self.scroll_col + viewport_cols {
            self.scroll_col = col.saturating_sub(viewport_cols) + 1;
        }
    }

    pub fn render_state(
        &mut self,
        viewport_rows: usize,
        viewport_cols: usize,
        is_active: bool,
    ) -> EditorRenderState {
        self.clamp_horizontal_scroll(viewport_cols);
        let (visible_start, visible_lines) = self.visible_line_window(viewport_rows);
        EditorRenderState {
            visible_start,
            visible_lines,
            cursor_visible_row: if is_active {
                Some(self.cursor_line_col().0.saturating_sub(visible_start))
            } else {
                None
            },
            scroll_col: self.scroll_col,
        }
    }

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------

    pub fn find_matches(&self, query: &str) -> Vec<(usize, usize)> {
        if query.is_empty() {
            return vec![];
        }
        let text = self.text.to_string();
        let query_lower = query.to_lowercase();
        let text_lower = text.to_lowercase();
        let mut matches = Vec::new();
        let mut byte_start = 0;
        while byte_start < text_lower.len() {
            let Some(byte_pos) = text_lower[byte_start..].find(&query_lower) else {
                break;
            };
            let abs_byte = byte_start + byte_pos;
            let end_byte = abs_byte + query_lower.len();
            let start_char = self.text.byte_to_char(abs_byte);
            let end_char = self.text.byte_to_char(end_byte);
            matches.push((start_char, end_char));
            let next_char_len = text_lower[abs_byte..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            byte_start = abs_byte + next_char_len;
        }
        matches
    }

    pub fn search_next(&mut self) {
        let matches = self.find_matches(&self.search_query.clone());
        if matches.is_empty() {
            self.search_match_idx = 0;
            return;
        }
        let next = matches
            .iter()
            .position(|(s, _)| *s > self.cursor_char_idx)
            .unwrap_or(0);
        self.search_match_idx = next;
        self.cursor_char_idx = matches[next].0;
    }

    pub fn search_prev(&mut self) {
        let matches = self.find_matches(&self.search_query.clone());
        if matches.is_empty() {
            self.search_match_idx = 0;
            return;
        }
        let prev = matches
            .iter()
            .rposition(|(s, _)| *s < self.cursor_char_idx)
            .unwrap_or(matches.len() - 1);
        self.search_match_idx = prev;
        self.cursor_char_idx = matches[prev].0;
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn line_col_to_char(&self, line: usize, col: usize) -> usize {
        let line_start = self.text.line_to_char(line);
        let line_len = self.visible_line_len(line);
        line_start + col.min(line_len)
    }

    fn visible_line_len(&self, line: usize) -> usize {
        let slice = self.text.line(line);
        let len = slice.len_chars();
        if len > 0 && slice.char(len - 1) == '\n' { len - 1 } else { len }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("editor buffer has no file path")]
    MissingPath,
    #[error("failed to read editor file {path}: {source}")]
    ReadFile { path: String, source: std::io::Error },
    #[error("failed to write editor file {path}: {source}")]
    WriteFile { path: String, source: std::io::Error },
}
```

- [ ] **Step 1.5: Run tests**

```bash
cargo test --lib editor
```

Expected: all tests pass, including `large_file_insert_does_not_degrade_linearly`.

- [ ] **Step 1.6: Confirm tui-textarea is fully unused**

```bash
grep -rn "tui_textarea\|tui-textarea" src/ Cargo.toml
```

Expected: no output.

- [ ] **Step 1.7: Commit**

```bash
git add Cargo.toml Cargo.lock src/editor.rs
git commit -m "perf(editor): replace tui-textarea with ropey Rope + delta-based undo stack"
```

---

## Task 2: Restore full editor test coverage

**Files:**
- Modify: `src/editor.rs` (test module only)

The test suite from before Wave 1C was more comprehensive than what Wave 1C left behind. Restore the full set and add regression tests for the new undo/redo behaviour.

- [ ] **Step 2.1: Add / restore tests**

Replace the `tests` module with:

```rust
#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};
    use super::{EditorBuffer, EditorError};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("zeta-{name}-{ts}.txt"))
    }

    // --- open / save ---

    #[test]
    fn opens_existing_file_contents() {
        let p = temp_path("open");
        fs::write(&p, "hello editor\n").unwrap();
        let buf = EditorBuffer::open(&p).unwrap();
        assert_eq!(buf.contents(), "hello editor\n");
        assert!(!buf.is_dirty);
        fs::remove_file(p).unwrap();
    }

    #[test]
    fn save_persists_changes_and_clears_dirty_flag() {
        let p = temp_path("save");
        fs::write(&p, "hello").unwrap();
        let mut buf = EditorBuffer::open(&p).unwrap();
        buf.insert(buf.text.len_chars(), " world");
        buf.save().unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "hello world");
        assert!(!buf.is_dirty);
        fs::remove_file(p).unwrap();
    }

    #[test]
    fn save_without_path_fails() {
        let mut buf = EditorBuffer::default();
        assert!(matches!(buf.save(), Err(EditorError::MissingPath)));
    }

    // --- cursor / mutation ---

    #[test]
    fn typing_and_backspace_update_cursor() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.backspace();
        assert_eq!(buf.contents(), "a");
        assert_eq!(buf.cursor_line_col(), (0, 1));
        assert!(buf.is_dirty);
    }

    #[test]
    fn cursor_moves_between_lines() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('a');
        buf.insert_newline();
        buf.insert_char('b');
        buf.move_up();
        assert_eq!(buf.cursor_line_col(), (0, 1));
        buf.move_down();
        assert_eq!(buf.cursor_line_col(), (1, 1));
    }

    #[test]
    fn visible_window_follows_cursor() {
        let mut buf = EditorBuffer::default();
        for ch in ['a', '\n', 'b', '\n', 'c', '\n', 'd'] {
            if ch == '\n' { buf.insert_newline(); } else { buf.insert_char(ch); }
        }
        let (start, visible) = buf.visible_line_window(2);
        assert_eq!(start, 2);
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn horizontal_scroll_advances_when_cursor_moves_right_past_viewport() {
        let mut buf = EditorBuffer::default();
        for _ in 0..20 { buf.insert_char('x'); }
        buf.clamp_horizontal_scroll(10);
        let (_, col) = buf.cursor_line_col();
        assert!(col >= buf.scroll_col && col < buf.scroll_col + 10);
        assert!(buf.scroll_col > 0);
    }

    #[test]
    fn horizontal_scroll_retreats_when_cursor_moves_left_past_scroll_origin() {
        let mut buf = EditorBuffer::default();
        for _ in 0..20 { buf.insert_char('x'); }
        buf.clamp_horizontal_scroll(10);
        assert!(buf.scroll_col > 0);
        for _ in 0..20 { buf.move_left(); }
        buf.clamp_horizontal_scroll(10);
        assert_eq!(buf.scroll_col, 0);
    }

    // --- undo / redo ---

    #[test]
    fn undo_restores_previous_content() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.undo();
        assert_eq!(buf.contents(), "a", "undo should remove 'b'");
    }

    #[test]
    fn undo_twice_restores_empty() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.undo();
        buf.undo();
        assert_eq!(buf.contents(), "");
    }

    #[test]
    fn redo_reapplies_undone_change() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('x');
        buf.undo();
        buf.redo();
        assert!(buf.contents().contains('x'));
    }

    #[test]
    fn new_edit_after_undo_clears_redo_history() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.undo();
        buf.insert_char('c'); // branch — 'b' redo history is gone
        buf.redo();           // should be a no-op
        assert_eq!(buf.contents(), "ac");
    }

    #[test]
    fn undo_backspace_restores_deleted_char() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('h');
        buf.insert_char('i');
        buf.backspace();
        assert_eq!(buf.contents(), "h");
        buf.undo();
        assert_eq!(buf.contents(), "hi");
    }

    // --- search ---

    #[test]
    fn find_matches_returns_all_occurrences() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo bar foo baz foo");
        assert_eq!(buf.find_matches("foo"), vec![(0, 3), (8, 11), (16, 19)]);
    }

    #[test]
    fn find_matches_is_case_insensitive() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "Hello hello HELLO");
        assert_eq!(buf.find_matches("hello").len(), 3);
    }

    #[test]
    fn find_matches_empty_query_returns_nothing() {
        let mut buf = EditorBuffer::default();
        buf.insert_char('a');
        assert!(buf.find_matches("").is_empty());
    }

    #[test]
    fn search_next_jumps_to_next_match() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo bar foo");
        buf.search_query = String::from("foo");
        buf.cursor_char_idx = 0;
        buf.search_next();
        assert_eq!(buf.cursor_char_idx, 8);
    }

    #[test]
    fn search_next_wraps_around() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo bar foo");
        buf.search_query = String::from("foo");
        buf.cursor_char_idx = 9;
        buf.search_next();
        assert_eq!(buf.cursor_char_idx, 0);
    }

    #[test]
    fn search_prev_jumps_to_previous_match() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo bar foo");
        buf.search_query = String::from("foo");
        buf.cursor_char_idx = 8;
        buf.search_prev();
        assert_eq!(buf.cursor_char_idx, 0);
    }

    #[test]
    fn search_prev_wraps_around_to_last_match() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo bar foo");
        buf.search_query = String::from("foo");
        buf.cursor_char_idx = 0;
        buf.search_prev();
        assert_eq!(buf.cursor_char_idx, 8);
    }

    #[test]
    fn find_matches_returns_char_spans_for_unicode_text() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "café café");
        assert_eq!(buf.find_matches("café"), vec![(0, 4), (5, 9)]);
    }

    #[test]
    fn find_matches_returns_char_spans_for_japanese_text() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "ありがとう ありがとう");
        assert_eq!(buf.find_matches("ありがとう"), vec![(0, 5), (6, 11)]);
    }

    // --- performance ---

    #[test]
    fn large_file_insert_does_not_degrade_linearly() {
        let mut buf = EditorBuffer::default();
        let big = (0..10_000).map(|i| format!("line {i}\n")).collect::<String>();
        buf.insert(0, &big);
        let start = Instant::now();
        for ch in "hello world".chars() {
            buf.insert_char(ch);
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 50,
            "11 keystrokes took {}ms — expected < 50ms",
            elapsed.as_millis()
        );
    }
}
```

> **Note:** The tests directly access `buf.text.len_chars()` and `buf.cursor_char_idx` which are private fields. Because the tests live in `mod tests` inside `src/editor.rs` (a child module), they can access private fields. This is intentional — the tests are white-box tests of the buffer internals.

- [ ] **Step 2.2: Run the full editor test suite**

```bash
cargo test --lib editor
```

Expected: all tests pass.

- [ ] **Step 2.3: Run the full workspace test suite**

```bash
cargo test --workspace
```

Expected: same pass count as end of Wave 2B (158/160, same 2 pre-existing path-separator failures).

- [ ] **Step 2.4: Commit**

```bash
git add src/editor.rs
git commit -m "test(editor): restore full test suite + undo/redo + performance regression guard"
```

---

## Task 3: Final verification

**Files:** None modified.

- [ ] **Step 3.1: Run clippy**

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: zero warnings.

- [ ] **Step 3.2: Verify tui-textarea is gone**

```bash
grep -rn "tui_textarea\|tui-textarea" src/ Cargo.toml Cargo.lock
```

Expected: no output from `src/` or `Cargo.toml`. (`Cargo.lock` may retain the entry until `cargo update` is run — that is acceptable.)

- [ ] **Step 3.3: Manual smoke test**

```
cargo run
# Open a file larger than 1 000 lines
# Type freely — should feel instantaneous
# Open search (Ctrl+F), type a query — should update without lag
# Ctrl+Z several times — undo should step back char by char
# Ctrl+Y (or mapped redo key) — redo should reapply
```

- [ ] **Step 3.4: Final commit**

```bash
git commit -m "chore: Wave 3A complete — ropey Rope backend, O(log n) editor ops, delta undo"
```

---

## Performance Notes

| Operation | Wave 1C (TextArea) | Wave 3A (Rope) |
|---|---|---|
| `insert_char` | O(line_len) copy into history | O(log n) Rope insert + tiny Edit |
| `backspace` | O(line_len) copy into history | O(log n) Rope remove + tiny Edit |
| `cursor_line_col` | O(1) (TextArea tracks internally) | O(log n) `char_to_line` |
| `visible_line_window` | O(height) slice + clone | O(height × line_len) collect |
| `find_matches` | O(total_chars) per call | O(total_chars) per call |
| `undo` / `redo` | O(history_depth × line_len) | O(log n + delta_len) |
| Memory per undo entry | O(changed_line_len) — can be MB | O(changed_chars) — bytes |

`find_matches` remains O(total_chars) — this is acceptable since it only runs on search input, not on every render frame. If search performance on very large files becomes a concern, a background search worker is the right next step.

---

## Jira

**ZTA-87** — Editor performance regression: replace tui-textarea with ropey Rope backend  
Sub-tasks: ZTA-122 (Rope backend), ZTA-123 (delta undo stack), ZTA-124 (performance test)

**Wave dependency:** Starts AFTER Wave 2B is merged. No other waves depend on this — it is a drop-in replacement with an identical public interface.
