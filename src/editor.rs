use std::fs as std_fs;
use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier};
use ratatui::text::Line;
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
    /// True when word-wrap is active; the UI should not apply horizontal scroll.
    pub word_wrap: bool,
    /// Visual column of cursor (tabs expanded); replaces raw char col for cursor drawing.
    pub cursor_visual_col: Option<usize>,
}

// ---------------------------------------------------------------------------
// Undo stack — delta-based, O(delta_len) per entry regardless of file size
// ---------------------------------------------------------------------------

/// A single reversible edit.
#[derive(Clone, Debug)]
struct Edit {
    /// Char index where the change begins.
    char_start: usize,
    /// Text that was present at `char_start` before the edit (empty for
    /// pure insertions).
    removed: String,
    /// Text that replaced it (empty for pure deletions).
    inserted: String,
    /// Cursor position before this edit — restored on undo.
    cursor_before: usize,
    /// Cursor position after this edit — restored on redo.
    cursor_after: usize,
}

#[derive(Clone, Debug, Default)]
struct UndoStack {
    entries: Vec<Edit>,
    /// Next write slot. `head == entries.len()` means nothing to redo.
    head: usize,
}

impl UndoStack {
    /// Record a new edit, discarding any redo history above `head`.
    fn push(&mut self, edit: Edit) {
        self.entries.truncate(self.head);
        self.entries.push(edit);
        self.head = self.entries.len();
    }

    /// Return the edit to undo and decrement head, or `None` if at bottom.
    fn pop_undo(&mut self) -> Option<&Edit> {
        if self.head == 0 {
            return None;
        }
        self.head -= 1;
        Some(&self.entries[self.head])
    }

    /// Return the edit to redo and increment head, or `None` if at top.
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
    text: Rope,
    cursor_char_idx: usize,
    history: UndoStack,
    /// Increments on every text-changing operation (insert, delete, undo, redo).
    /// Used as the cache key for `highlight_cache` — scrolling alone never
    /// increments this, so highlighting is free during scroll.
    edit_version: usize,
    /// Full-file syntax-highlighted lines, keyed by `(edit_version, theme, tab_width)`.
    /// `None` until first render. Recomputed only when text actually changes.
    highlight_cache: Option<(usize, String, u8, Vec<HighlightedLine>)>,
    /// Word-wrap-expanded highlighted lines, keyed by `(edit_version, theme, tab_width, cols)`.
    /// Computed from `highlight_cache` by splitting each logical line at `cols` chars.
    wrap_highlight_cache: Option<(usize, String, u8, usize, Vec<HighlightedLine>)>,
    /// Parsed markdown preview lines, keyed by `(edit_version, panel_width, theme)`.
    /// `None` until first render. Recomputed only when text, panel width, or theme changes.
    md_preview_cache: Option<(usize, u16, String, Vec<Line<'static>>)>,
    sel_anchor: Option<usize>,
}

impl EditorBuffer {
    pub fn from_text(path: PathBuf, contents: String) -> Self {
        Self {
            path: Some(path),
            text: Rope::from_str(&contents),
            ..Self::default()
        }
    }

    pub fn open(path: &Path) -> Result<Self, EditorError> {
        let bytes = std_fs::read(path).map_err(|source| EditorError::ReadFile {
            path: path.display().to_string(),
            source,
        })?;
        let contents = String::from_utf8_lossy(&bytes);
        Ok(Self::from_text(path.to_path_buf(), contents.into_owned()))
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
        self.edit_version += 1;
    }

    // -----------------------------------------------------------------------
    // Text mutation
    // -----------------------------------------------------------------------

    pub fn insert_char(&mut self, ch: char) {
        self.sel_anchor = None;
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
        self.edit_version += 1;
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    /// Insert a string at the current cursor position.
    /// Normalises CRLF to LF before insertion.
    pub fn insert_str_at_cursor(&mut self, text: &str) {
        let normalized: String = text.chars().filter(|&c| c != '\r').collect();
        if normalized.is_empty() {
            return;
        }
        self.insert(self.cursor_char_idx, &normalized);
    }

    pub fn backspace(&mut self) {
        if self.sel_anchor.is_some() {
            self.delete_selection();
            return;
        }
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
        self.edit_version += 1;
    }

    pub fn move_left(&mut self) {
        self.sel_anchor = None;
        self.cursor_char_idx = self.cursor_char_idx.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.sel_anchor = None;
        self.cursor_char_idx = (self.cursor_char_idx + 1).min(self.text.len_chars());
    }

    pub fn move_up(&mut self) {
        self.sel_anchor = None;
        let (line, col) = self.cursor_line_col();
        if line == 0 {
            return;
        }
        self.cursor_char_idx = self.line_col_to_char(line - 1, col);
    }

    pub fn move_down(&mut self) {
        self.sel_anchor = None;
        let (line, col) = self.cursor_line_col();
        if line + 1 >= self.text.len_lines() {
            return;
        }
        self.cursor_char_idx = self.line_col_to_char(line + 1, col);
    }

    // -----------------------------------------------------------------------
    // Selection extension (Shift+arrow)
    // -----------------------------------------------------------------------

    /// Extend the selection leftward. Sets the anchor at the current cursor
    /// position if no selection is active, then moves the cursor left.
    pub fn extend_left(&mut self) {
        if self.sel_anchor.is_none() {
            self.sel_anchor = Some(self.cursor_char_idx);
        }
        self.cursor_char_idx = self.cursor_char_idx.saturating_sub(1);
    }

    /// Extend the selection rightward.
    pub fn extend_right(&mut self) {
        if self.sel_anchor.is_none() {
            self.sel_anchor = Some(self.cursor_char_idx);
        }
        self.cursor_char_idx = (self.cursor_char_idx + 1).min(self.text.len_chars());
    }

    /// Extend the selection upward.
    pub fn extend_up(&mut self) {
        if self.sel_anchor.is_none() {
            self.sel_anchor = Some(self.cursor_char_idx);
        }
        let (line, col) = self.cursor_line_col();
        if line == 0 {
            return;
        }
        self.cursor_char_idx = self.line_col_to_char(line - 1, col);
    }

    /// Extend the selection downward.
    pub fn extend_down(&mut self) {
        if self.sel_anchor.is_none() {
            self.sel_anchor = Some(self.cursor_char_idx);
        }
        let (line, col) = self.cursor_line_col();
        if line + 1 >= self.text.len_lines() {
            return;
        }
        self.cursor_char_idx = self.line_col_to_char(line + 1, col);
    }

    // -----------------------------------------------------------------------
    // Undo / redo
    // -----------------------------------------------------------------------

    /// Undo the most recent edit.
    pub fn undo(&mut self) {
        // Clone to avoid borrowing `self` while mutating it.
        self.sel_anchor = None;
        let Some(edit) = self.history.pop_undo().cloned() else {
            return;
        };
        let insert_end = edit.char_start + edit.inserted.chars().count();
        if insert_end > edit.char_start {
            self.text.remove(edit.char_start..insert_end);
        }
        if !edit.removed.is_empty() {
            self.text.insert(edit.char_start, &edit.removed);
        }
        self.cursor_char_idx = edit.cursor_before;
        self.is_dirty = self.text.len_chars() > 0;
    }

    /// Redo the most recently undone edit.
    pub fn redo(&mut self) {
        self.sel_anchor = None;
        let Some(edit) = self.history.pop_redo().cloned() else {
            return;
        };
        let removed_end = edit.char_start + edit.removed.chars().count();
        if removed_end > edit.char_start {
            self.text.remove(edit.char_start..removed_end);
        }
        if !edit.inserted.is_empty() {
            self.text.insert(edit.char_start, &edit.inserted);
        }
        self.cursor_char_idx = edit.cursor_after;
        self.is_dirty = true;
        self.edit_version += 1;
    }

    // -----------------------------------------------------------------------
    // Selection
    // -----------------------------------------------------------------------

    /// Clear the selection anchor without affecting the cursor.
    pub fn clear_selection(&mut self) {
        self.sel_anchor = None;
    }

    /// Returns the selection as `(start, end)` char indices (start < end),
    /// or `None` if no selection anchor is set.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let anchor = self.sel_anchor?;
        let cursor = self.cursor_char_idx;
        if anchor <= cursor {
            Some((anchor, cursor))
        } else {
            Some((cursor, anchor))
        }
    }

    /// Returns the text covered by the current selection, or `None`.
    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection_range()?;
        Some(self.text.slice(start..end).to_string())
    }

    /// Delete the selected text, record an undo entry, clear the selection,
    /// and move the cursor to the deletion point. Returns `true` if text was
    /// actually deleted.
    pub fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selection_range() else {
            return false;
        };
        if start == end {
            self.sel_anchor = None;
            return false;
        }
        let removed: String = self.text.slice(start..end).to_string();
        let edit = Edit {
            char_start: start,
            removed,
            inserted: String::new(),
            cursor_before: self.cursor_char_idx,
            cursor_after: start,
        };
        self.text.remove(start..end);
        self.cursor_char_idx = start;
        self.history.push(edit);
        self.sel_anchor = None;
        self.is_dirty = true;
        self.edit_version += 1;
        true
    }

    /// Set selection to the entire document.
    pub fn select_all(&mut self) {
        self.sel_anchor = Some(0);
        self.cursor_char_idx = self.text.len_chars();
    }

    /// Map a char index within an original (non-tab-expanded) line string to
    /// the display column, accounting for tab stops.
    fn char_to_display_col(line: &str, char_idx: usize, tab_width: usize) -> usize {
        if tab_width <= 1 {
            return char_idx;
        }
        let mut display = 0usize;
        for (i, ch) in line.chars().enumerate() {
            if i >= char_idx {
                break;
            }
            if ch == '\t' {
                display += tab_width - (display % tab_width);
            } else {
                display += 1;
            }
        }
        display
    }

    /// For each visible row, return the display-char range covered by the
    /// current selection, or `None` if that row has no selected text.
    ///
    /// `visible_start` and `visible_lines` come from `EditorRenderState`.
    /// In word-wrap mode the mapping is approximate (whole rows selected);
    /// shift+arrow selection is not yet supported so this is correct for
    /// SelectAll.
    pub fn visible_selection_display_ranges(
        &self,
        visible_start: usize,
        visible_lines: &[String],
        tab_width: u8,
        word_wrap: bool,
    ) -> Vec<Option<(usize, usize)>> {
        let Some((sel_start, sel_end)) = self.selection_range() else {
            return vec![None; visible_lines.len()];
        };
        if sel_start == sel_end {
            return vec![None; visible_lines.len()];
        }
        if word_wrap {
            // Word-wrap: each visible row is a sub-chunk of a logical line;
            // mapping back to doc chars is non-trivial. For SelectAll (the
            // only supported selection) every visible row is fully selected.
            return visible_lines
                .iter()
                .map(|row| {
                    let len = row.chars().count();
                    if len > 0 {
                        Some((0, len))
                    } else {
                        None
                    }
                })
                .collect();
        }
        // Non-wrap: visible_lines[i] corresponds to logical line visible_start + i.
        let total_lines = self.text.len_lines();
        let tab_w = tab_width as usize;
        visible_lines
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let logi = visible_start + i;
                if logi >= total_lines {
                    return None;
                }
                let line_char_start = self.text.line_to_char(logi);
                let raw_line = self.text.line(logi).to_string();
                let raw_line_no_nl = raw_line.trim_end_matches('\n');
                let raw_len = raw_line_no_nl.chars().count();
                let line_char_end = line_char_start + raw_len;
                // No overlap with selection.
                if sel_end <= line_char_start || sel_start >= line_char_end {
                    return None;
                }
                let rel_start = sel_start.saturating_sub(line_char_start);
                let rel_end = (sel_end - line_char_start).min(raw_len);
                let disp_start = Self::char_to_display_col(raw_line_no_nl, rel_start, tab_w);
                let disp_end = Self::char_to_display_col(raw_line_no_nl, rel_end, tab_w);
                if disp_start < disp_end {
                    Some((disp_start, disp_end))
                } else {
                    None
                }
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // Persistence
    // -----------------------------------------------------------------------

    pub fn save(&mut self) -> Result<(), EditorError> {
        let path = self.path.clone().ok_or(EditorError::MissingPath)?;
        std_fs::write(&path, self.text.to_string()).map_err(|source| EditorError::WriteFile {
            path: path.display().to_string(),
            source,
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

    /// Expand `\t` to the next tab stop for display.
    fn expand_tabs(s: &str, tab_width: usize) -> String {
        if tab_width <= 1 || !s.contains('\t') {
            return s.to_string();
        }
        let mut out = String::with_capacity(s.len());
        let mut col = 0usize;
        for ch in s.chars() {
            if ch == '\t' {
                let spaces = tab_width - (col % tab_width);
                out.push_str(&" ".repeat(spaces));
                col += spaces;
            } else {
                out.push(ch);
                col += 1;
            }
        }
        out
    }

    /// Visual column of the cursor accounting for tab expansion.
    pub fn cursor_visual_col(&self, tab_width: usize) -> usize {
        let (_, char_col) = self.cursor_line_col();
        if tab_width <= 1 {
            return char_col;
        }
        let (line_idx, _) = self.cursor_line_col();
        let line = self.text.line(line_idx).to_string();
        let mut vis = 0usize;
        for (i, ch) in line.chars().enumerate() {
            if i >= char_col {
                break;
            }
            if ch == '\t' {
                vis += tab_width - (vis % tab_width);
            } else {
                vis += 1;
            }
        }
        vis
    }

    /// Number of visual rows a display string occupies given `cols` viewport width.
    fn visual_row_count(s: &str, cols: usize) -> usize {
        if cols == 0 {
            return 1;
        }
        // Each wrapped segment counts as one row; empty line still occupies one row.
        let len = s.chars().count();
        if len == 0 {
            1
        } else {
            len.div_ceil(cols)
        }
    }
    /// Returns syntax-highlighted lines for the visible viewport.
    ///
    /// The full-file highlight result is cached by `edit_version` so repeated
    /// calls during scrolling (no text change) cost only a slice — not a full
    /// Split a single highlighted line into word-wrapped visual rows at `cols` chars.
    /// Tabs must be pre-expanded (via `expand_tabs`) before highlighting; each char
    /// in the tokens maps to one display column for typical code content.
    fn wrap_highlighted_line(tokens: &HighlightedLine, cols: usize) -> Vec<HighlightedLine> {
        if cols == 0 || tokens.is_empty() {
            return vec![tokens.clone()];
        }
        let mut rows: Vec<HighlightedLine> = Vec::new();
        let mut current: HighlightedLine = Vec::new();
        let mut col = 0usize;
        for (color, modifier, text) in tokens {
            let chars: Vec<char> = text.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                let avail = cols - col;
                let n = avail.min(chars.len() - i);
                let chunk: String = chars[i..i + n].iter().collect();
                if !chunk.is_empty() {
                    current.push((*color, *modifier, chunk.into()));
                }
                col += n;
                i += n;
                if col >= cols {
                    rows.push(std::mem::take(&mut current));
                    col = 0;
                }
            }
        }
        if !current.is_empty() {
            rows.push(current);
        }
        // Always emit at least one row so empty lines still occupy a visual row.
        if rows.is_empty() {
            rows.push(Vec::new());
        }
        rows
    }

    /// Return the highlighted lines visible in the current viewport, caching the
    /// syntect result so that repeated calls during scrolling (no text change)
    /// cost only a slice — not a full syntect re-parse.
    ///
    /// When `word_wrap` is true and `viewport_cols > 0`, each logical line is
    /// split into wrapped visual rows; the wrap result is cached separately from
    /// the base highlight cache so that a resize (cols change) forces a re-wrap
    /// but not a full re-highlight.
    pub fn visible_highlighted_window(
        &mut self,
        height: usize,
        syntect_theme: &str,
        fallback_color: Color,
        tab_width: u8,
        word_wrap: bool,
        viewport_cols: usize,
    ) -> Vec<HighlightedLine> {
        if height == 0 {
            return Vec::new();
        }

        let ext = self
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .map(str::to_owned);

        // Rebuild the base per-logical-line highlight cache when text/theme/tab_width changes.
        let cache_valid = self.highlight_cache.as_ref().is_some_and(|(v, t, tw, _)| {
            *v == self.edit_version && t == syntect_theme && *tw == tab_width
        });
        if !cache_valid {
            let raw = normalize_preview_text(&self.text.to_string());
            let text = Self::expand_tabs(&raw, tab_width as usize);
            let all_lines = match highlight_text(&text, ext.as_deref(), syntect_theme) {
                Some(lines) => lines,
                None => self
                    .text
                    .lines()
                    .map(|l| {
                        let s = normalize_preview_text(&l.to_string());
                        let s = s.strip_suffix('\n').unwrap_or(&s).to_string();
                        let s = Self::expand_tabs(&s, tab_width as usize);
                        vec![(fallback_color, Modifier::empty(), s.into())]
                    })
                    .collect(),
            };
            self.highlight_cache = Some((
                self.edit_version,
                syntect_theme.to_string(),
                tab_width,
                all_lines,
            ));
            // Any prior wrap cache is now stale.
            self.wrap_highlight_cache = None;
        }

        if word_wrap && viewport_cols > 0 {
            // Rebuild the wrap cache when text or viewport width changes.
            let wrap_valid =
                self.wrap_highlight_cache
                    .as_ref()
                    .is_some_and(|(v, t, tw, cols, _)| {
                        *v == self.edit_version
                            && t == syntect_theme
                            && *tw == tab_width
                            && *cols == viewport_cols
                    });
            if !wrap_valid {
                let logical_lines = &self.highlight_cache.as_ref().unwrap().3;
                let mut wrapped: Vec<HighlightedLine> = Vec::new();
                for line in logical_lines {
                    wrapped.extend(Self::wrap_highlighted_line(line, viewport_cols));
                }
                self.wrap_highlight_cache = Some((
                    self.edit_version,
                    syntect_theme.to_string(),
                    tab_width,
                    viewport_cols,
                    wrapped,
                ));
            }
            // Determine the first visual row to show using the same start logic as
            // visible_line_window_h (word_wrap=true path).
            let (logical_start, _, _) =
                self.visible_line_window_h(height, viewport_cols, tab_width, true);
            // Map logical_start to a visual row offset in the wrap cache.
            let all_logical = &self.highlight_cache.as_ref().unwrap().3;
            let visual_start: usize = all_logical[..logical_start]
                .iter()
                .map(|line| Self::wrap_highlighted_line(line, viewport_cols).len())
                .sum();
            let all_wrapped = &self.wrap_highlight_cache.as_ref().unwrap().4;
            let end = (visual_start + height).min(all_wrapped.len());
            all_wrapped[visual_start..end].to_vec()
        } else {
            // Non-wrap: slice the logical-line cache directly.
            let (start, _, _) = self.visible_line_window_h(height, 0, tab_width, false);
            let all_lines = &self.highlight_cache.as_ref().unwrap().3;
            let end = (start + height).min(all_lines.len());
            all_lines[start..end].to_vec()
        }
    }

    // ---------------------------------------------------------------------------
    // Markdown preview cache
    // ---------------------------------------------------------------------------

    /// Return the cached parsed markdown lines if the cache is valid for the
    /// current `edit_version`, `panel_width`, and `theme`. Returns `None` on miss.
    pub fn md_preview_cached(&self, panel_width: u16, theme: &str) -> Option<&Vec<Line<'static>>> {
        self.md_preview_cache
            .as_ref()
            .filter(|(v, w, t, _)| *v == self.edit_version && *w == panel_width && t == theme)
            .map(|(_, _, _, lines)| lines)
    }

    /// Store parsed markdown lines in the cache, keyed by the current
    /// `edit_version`, `panel_width`, and `theme`.
    pub fn set_md_preview_cache(
        &mut self,
        panel_width: u16,
        theme: &str,
        lines: Vec<Line<'static>>,
    ) {
        self.md_preview_cache = Some((self.edit_version, panel_width, theme.to_string(), lines));
    }

    /// Compute the visible line window for rendering `viewport_rows` rows.
    /// When `word_wrap` is true, lines are soft-wrapped at `viewport_cols`.
    /// Returns `(visible_start, visual_lines, cursor_visual_row)`.
    pub fn visible_line_window_h(
        &self,
        viewport_rows: usize,
        viewport_cols: usize,
        tab_width: u8,
        word_wrap: bool,
    ) -> (usize, Vec<String>, usize) {
        if viewport_rows == 0 {
            return (0, Vec::new(), 0);
        }
        let total = self.text.len_lines();
        let (cursor_line, _) = self.cursor_line_col();
        let tab_w = tab_width as usize;
        let vis_col = self.cursor_visual_col(tab_w);

        // Determine the logical-line start of the visible window.
        let start = if !word_wrap || viewport_cols == 0 {
            // Non-wrap: cursor sits on the last visible line (matches original behaviour).
            (cursor_line + 1).saturating_sub(viewport_rows)
        } else {
            // Wrap: walk backwards from cursor_line until we have enough visual rows
            // to show `viewport_rows` lines, keeping cursor near the bottom third.
            let cursor_wrap_row = vis_col / viewport_cols.max(1);
            // Visual rows the cursor line contributes above the cursor position.
            let rows_needed_above = cursor_wrap_row.min(viewport_rows.saturating_sub(1));
            let target_start_visual = viewport_rows.saturating_sub(rows_needed_above + 1);
            // Walk backwards from cursor_line, accumulating visual rows.
            let mut rows_above = 0usize;
            let mut s = cursor_line;
            while s > 0 && rows_above < target_start_visual {
                s -= 1;
                let raw = normalize_preview_text(&self.text.line(s).to_string());
                let line_str = {
                    let r = raw.strip_suffix('\n').unwrap_or(&raw);
                    Self::expand_tabs(r, tab_w)
                };
                rows_above += Self::visual_row_count(&line_str, viewport_cols);
            }
            s
        };

        // Collect visual lines from start until viewport is filled.
        let mut visual_lines: Vec<String> = Vec::new();
        let mut cursor_visual_row: usize = 0;
        let mut found_cursor = false;
        let mut logi = start;
        while visual_lines.len() < viewport_rows && logi < total {
            let raw = normalize_preview_text(&self.text.line(logi).to_string());
            let line = {
                let s = raw.strip_suffix('\n').unwrap_or(&raw);
                Self::expand_tabs(s, tab_w)
            };
            if word_wrap && viewport_cols > 0 && line.chars().count() > viewport_cols {
                // Emit wrapped sub-rows.
                let chars: Vec<char> = line.chars().collect();
                let mut sub_start = 0;
                let mut sub_row = 0usize;
                while sub_start < chars.len() && visual_lines.len() < viewport_rows {
                    let sub_end = (sub_start + viewport_cols).min(chars.len());
                    let sub: String = chars[sub_start..sub_end].iter().collect();
                    if logi == cursor_line && !found_cursor {
                        let cursor_sub = vis_col / viewport_cols;
                        if sub_row == cursor_sub {
                            cursor_visual_row = visual_lines.len();
                            found_cursor = true;
                        }
                    }
                    visual_lines.push(sub);
                    sub_start = sub_end;
                    sub_row += 1;
                }
            } else {
                if logi == cursor_line && !found_cursor {
                    cursor_visual_row = visual_lines.len();
                    found_cursor = true;
                }
                visual_lines.push(line);
            }
            logi += 1;
        }

        (start, visual_lines, cursor_visual_row)
    }

    pub fn cursor_line_col(&self) -> (usize, usize) {
        let safe = self.cursor_char_idx.min(self.text.len_chars());
        let line = self.text.char_to_line(safe);
        let line_start = self.text.line_to_char(line);
        (line, safe.saturating_sub(line_start))
    }

    pub fn line_count(&self) -> usize {
        self.text.len_lines()
    }

    pub fn clamp_horizontal_scroll(&mut self, viewport_cols: usize, tab_width: u8) {
        let col = self.cursor_visual_col(tab_width as usize);
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
        tab_width: u8,
        word_wrap: bool,
    ) -> EditorRenderState {
        if !word_wrap {
            self.clamp_horizontal_scroll(viewport_cols, tab_width);
        }
        let (visible_start, visible_lines, cursor_wrap_row) =
            self.visible_line_window_h(viewport_rows, viewport_cols, tab_width, word_wrap);
        let vis_col = self.cursor_visual_col(tab_width as usize);
        EditorRenderState {
            visible_start,
            visible_lines,
            cursor_visible_row: if is_active {
                Some(cursor_wrap_row)
            } else {
                None
            },
            scroll_col: if word_wrap { 0 } else { self.scroll_col },
            word_wrap,
            cursor_visual_col: if is_active { Some(vis_col) } else { None },
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

    /// For each visible row, return char-column match ranges for the current search query,
    /// plus the active match as `(row_idx, col_start, col_end)`.
    ///
    /// `visible_rows` are the rendered row strings (tabs expanded, newlines stripped) in
    /// order. `cursor_visual_row` is `render_state.cursor_visible_row`: the visual row
    /// that contains the cursor (which `search_next`/`search_prev` place at the match).
    #[allow(clippy::type_complexity)]
    pub fn visible_search_matches(
        &self,
        visible_rows: &[String],
        cursor_visual_row: Option<usize>,
    ) -> (Vec<Vec<(usize, usize)>>, Option<(usize, usize, usize)>) {
        if !self.search_active || self.search_query.is_empty() {
            return (vec![Vec::new(); visible_rows.len()], None);
        }
        let query_lower = self.search_query.to_lowercase();
        let query_char_count = query_lower.chars().count();
        let mut row_matches: Vec<Vec<(usize, usize)>> = Vec::with_capacity(visible_rows.len());
        let mut active_match: Option<(usize, usize, usize)> = None;
        for (row_idx, row) in visible_rows.iter().enumerate() {
            let row_chars: Vec<char> = row.to_lowercase().chars().collect();
            let mut matches_in_row: Vec<(usize, usize)> = Vec::new();
            let mut i = 0;
            while i + query_char_count <= row_chars.len() {
                let slice: String = row_chars[i..i + query_char_count].iter().collect();
                if slice == query_lower {
                    matches_in_row.push((i, i + query_char_count));
                    i += query_char_count;
                } else {
                    i += 1;
                }
            }
            // The first match on the cursor row is treated as the active match.
            // `search_next`/`search_prev` move the cursor to the match start, so
            // the cursor visual row is always the active-match row.
            if cursor_visual_row == Some(row_idx) && active_match.is_none() {
                if let Some(&(s, e)) = matches_in_row.first() {
                    active_match = Some((row_idx, s, e));
                }
            }
            row_matches.push(matches_in_row);
        }
        (row_matches, active_match)
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

    pub fn replace_next(&mut self, replacement: &str) -> bool {
        let matches = self.find_matches(&self.search_query.clone());
        if matches.is_empty() {
            return false;
        }
        let index = self.search_match_idx.min(matches.len() - 1);
        let (start, end) = matches[index];
        self.replace_span(start, end, replacement);
        self.search_match_idx = index;
        self.search_next();
        true
    }

    pub fn replace_all(&mut self, replacement: &str) -> usize {
        let matches = self.find_matches(&self.search_query.clone());
        let count = matches.len();
        for (start, end) in matches.into_iter().rev() {
            self.replace_span(start, end, replacement);
        }
        if count == 0 {
            self.search_match_idx = 0;
        }
        count
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn replace_span(&mut self, start: usize, end: usize, replacement: &str) {
        let removed = self.text.slice(start..end).to_string();
        let edit = Edit {
            char_start: start,
            removed,
            inserted: replacement.to_string(),
            cursor_before: self.cursor_char_idx,
            cursor_after: start + replacement.chars().count(),
        };
        self.text.remove(start..end);
        if !replacement.is_empty() {
            self.text.insert(start, replacement);
        }
        self.cursor_char_idx = edit.cursor_after;
        self.history.push(edit);
        self.is_dirty = true;
        self.edit_version += 1;
    }

    fn line_col_to_char(&self, line: usize, col: usize) -> usize {
        let line_start = self.text.line_to_char(line);
        let line_len = self.visible_line_len(line);
        line_start + col.min(line_len)
    }

    fn visible_line_len(&self, line: usize) -> usize {
        let slice = self.text.line(line);
        let len = slice.len_chars();
        if len > 0 && slice.char(len - 1) == '\n' {
            len - 1
        } else {
            len
        }
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
    ReadFile {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to write editor file {path}: {source}")]
    WriteFile {
        path: String,
        source: std::io::Error,
    },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    // --- open / save -------------------------------------------------------

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

    // --- cursor / mutation -------------------------------------------------

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
            if ch == '\n' {
                buf.insert_newline();
            } else {
                buf.insert_char(ch);
            }
        }
        let (start, visible, _) = buf.visible_line_window_h(2, 80, 4, false);
        assert_eq!(start, 2);
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn horizontal_scroll_advances_when_cursor_moves_right_past_viewport() {
        let mut buf = EditorBuffer::default();
        for _ in 0..20 {
            buf.insert_char('x');
        }
        buf.clamp_horizontal_scroll(10, 4);
        let (_, col) = buf.cursor_line_col();
        assert!(
            col >= buf.scroll_col && col < buf.scroll_col + 10,
            "cursor col {col} should be inside [{}, {})",
            buf.scroll_col,
            buf.scroll_col + 10
        );
        assert!(buf.scroll_col > 0);
    }

    #[test]
    fn horizontal_scroll_retreats_when_cursor_moves_left_past_scroll_origin() {
        let mut buf = EditorBuffer::default();
        for _ in 0..20 {
            buf.insert_char('x');
        }
        buf.clamp_horizontal_scroll(10, 4);
        assert!(buf.scroll_col > 0, "precondition: scroll_col > 0");
        for _ in 0..20 {
            buf.move_left();
        }
        buf.clamp_horizontal_scroll(10, 4);
        assert_eq!(buf.scroll_col, 0);
    }

    // --- undo / redo -------------------------------------------------------

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
        buf.undo(); // removes 'b'
        buf.insert_char('c'); // branch — 'b' redo is gone
        buf.redo(); // no-op
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

    // --- search ------------------------------------------------------------

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

    #[test]
    fn search_next_jumps_to_unicode_match() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "ありがとう ありがとう");
        buf.search_query = String::from("ありがとう");
        buf.cursor_char_idx = 0;
        buf.search_next();
        assert_eq!(buf.cursor_char_idx, 6);
        assert_eq!(buf.search_match_idx, 1);
    }

    #[test]
    fn replace_next_replaces_current_match_and_advances() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo foo");
        buf.search_query = String::from("foo");
        buf.search_next();
        assert!(buf.replace_next("bar"));
        assert_eq!(buf.contents(), "bar foo");
    }

    #[test]
    fn replace_all_replaces_every_occurrence() {
        let mut buf = EditorBuffer::default();
        buf.insert(0, "foo foo foo");
        buf.search_query = String::from("foo");
        let count = buf.replace_all("bar");
        assert_eq!(count, 3);
        assert_eq!(buf.contents(), "bar bar bar");
    }

    // --- performance -------------------------------------------------------

    #[test]
    fn large_file_insert_does_not_degrade_linearly() {
        let mut buf = EditorBuffer::default();
        let big = (0..10_000)
            .map(|i| format!("line {i}\n"))
            .collect::<String>();
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
