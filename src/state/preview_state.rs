use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::action::{Action, Command};
use crate::preview::ViewBuffer;
use crate::state::types::PaneFocus;

const PREVIEW_DEBOUNCE_MS: u64 = 150;
const PREVIEW_CACHE_SIZE: usize = 8;

#[derive(Clone, Debug, Default)]
pub struct PreviewState {
    pub view: Option<(PathBuf, ViewBuffer)>,
    pub panel_open: bool,
    pub preview_on_selection: bool,
    requested_path: Option<PathBuf>,
    pending_request: Option<(PathBuf, Instant)>,
    cache: VecDeque<(PathBuf, ViewBuffer)>,
}

impl PreviewState {
    pub fn new(panel_open: bool, preview_on_selection: bool) -> Self {
        Self {
            view: None,
            panel_open,
            preview_on_selection,
            requested_path: None,
            pending_request: None,
            cache: VecDeque::new(),
        }
    }

    pub fn should_auto_preview(&self) -> bool {
        self.panel_open && self.preview_on_selection
    }

    pub fn request_debounced_preview(&mut self, path: PathBuf) {
        self.requested_path = Some(path.clone());
        if let Some((_, cached)) = self
            .cache
            .iter()
            .find(|(cached_path, _)| *cached_path == path)
        {
            self.view = Some((path, cached.clone()));
            self.pending_request = None;
            return;
        }
        self.pending_request = Some((
            path,
            Instant::now() + Duration::from_millis(PREVIEW_DEBOUNCE_MS),
        ));
    }

    pub fn preview_command_due(&mut self) -> Option<Command> {
        // Check expiry without taking ownership — only clone the Instant, not the PathBuf.
        if Instant::now() < self.pending_request.as_ref()?.1 {
            return None;
        }
        // Deadline passed: take the pending request (avoids PathBuf clone).
        let (path, _) = self.pending_request.take()?;
        Some(Command::PreviewFile { path })
    }

    fn cache_view(&mut self, path: PathBuf, view: ViewBuffer) {
        self.cache.retain(|(cached_path, _)| *cached_path != path);
        self.cache.push_front((path, view));
        while self.cache.len() > PREVIEW_CACHE_SIZE {
            self.cache.pop_back();
        }
    }

    pub fn apply(&mut self, action: &Action, focus: &PaneFocus) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        match action {
            Action::ClearPreview => {
                self.view = None;
                self.requested_path = None;
                self.pending_request = None;
            }
            Action::TogglePreviewPanel => {
                self.panel_open = !self.panel_open;
                if !self.panel_open {
                    self.pending_request = None;
                }
            }
            Action::PreviewFile { path } => {
                self.requested_path = Some(path.clone());
                if let Some((_, cached)) = self
                    .cache
                    .iter()
                    .find(|(cached_path, _)| cached_path == path)
                {
                    self.view = Some((path.clone(), cached.clone()));
                } else {
                    commands.push(Command::PreviewFile { path: path.clone() });
                }
            }
            Action::FocusPreviewPanel => {
                // Focus toggling handled at AppState level (needs full focus context)
            }
            Action::ScrollPreviewDown => {
                if *focus == PaneFocus::Preview {
                    if let Some((_, v)) = self.view.as_mut() {
                        v.scroll_down(1);
                    }
                }
            }
            Action::ScrollPreviewUp => {
                if *focus == PaneFocus::Preview {
                    if let Some((_, v)) = self.view.as_mut() {
                        v.scroll_up(1);
                    }
                }
            }
            Action::ScrollPreviewPageDown => {
                if *focus == PaneFocus::Preview {
                    if let Some((_, v)) = self.view.as_mut() {
                        v.scroll_down(20);
                    }
                }
            }
            Action::ScrollPreviewPageUp => {
                if *focus == PaneFocus::Preview {
                    if let Some((_, v)) = self.view.as_mut() {
                        v.scroll_up(20);
                    }
                }
            }
            _ => {}
        }
        Ok(commands)
    }

    pub fn apply_job_loaded(&mut self, path: PathBuf, view: ViewBuffer) {
        if self.requested_path.as_ref() != Some(&path) {
            return;
        }
        let cached = view.clone();
        if let Some((ref current, ref mut buf)) = self.view {
            if *current == path {
                *buf = view;
                buf.reset_scroll();
                self.cache_view(path, cached);
                return;
            }
        }
        self.view = Some((path.clone(), view));
        self.cache_view(path, cached);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::ViewBuffer;
    use std::path::PathBuf;

    #[test]
    fn toggle_panel_flips_state() {
        let mut s = PreviewState::new(false, true);
        s.apply(&Action::TogglePreviewPanel, &PaneFocus::Left)
            .unwrap();
        assert!(s.panel_open);
        s.apply(&Action::TogglePreviewPanel, &PaneFocus::Left)
            .unwrap();
        assert!(!s.panel_open);
    }

    #[test]
    fn clear_preview_removes_view() {
        let mut s = PreviewState::new(true, true);
        s.view = Some((PathBuf::from("/tmp/a.txt"), ViewBuffer::from_plain("hello")));
        s.apply(&Action::ClearPreview, &PaneFocus::Left).unwrap();
        assert!(s.view.is_none());
    }

    #[test]
    fn scroll_only_applies_when_preview_focused() {
        let mut s = PreviewState::new(true, true);
        s.view = Some((
            PathBuf::from("/tmp/a.txt"),
            ViewBuffer::from_plain("line1\nline2\nline3"),
        ));
        s.apply(&Action::ScrollPreviewDown, &PaneFocus::Left)
            .unwrap();
        let row = s.view.as_ref().unwrap().1.scroll_row;
        assert_eq!(row, 0);
        s.apply(&Action::ScrollPreviewDown, &PaneFocus::Preview)
            .unwrap();
        let row = s.view.as_ref().unwrap().1.scroll_row;
        assert_eq!(row, 1);
    }

    #[test]
    fn markdown_preview_can_scroll() {
        let mut s = PreviewState::new(true, true);
        let source = (1..=50)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        s.view = Some((
            PathBuf::from("/tmp/a.md"),
            ViewBuffer::from_markdown(source),
        ));
        // Initial scroll_row is 0
        assert_eq!(s.view.as_ref().unwrap().1.scroll_row, 0);
        // Scroll down 5 times → scroll_row should advance
        for _ in 0..5 {
            s.apply(&Action::ScrollPreviewDown, &PaneFocus::Preview)
                .unwrap();
        }
        assert_eq!(s.view.as_ref().unwrap().1.scroll_row, 5);
        // Page down (20) → scroll_row = 25
        s.apply(&Action::ScrollPreviewPageDown, &PaneFocus::Preview)
            .unwrap();
        assert_eq!(s.view.as_ref().unwrap().1.scroll_row, 25);
    }

    #[test]
    fn stale_preview_result_is_ignored() {
        let mut s = PreviewState::new(true, true);
        s.request_debounced_preview(PathBuf::from("/tmp/b.txt"));
        s.apply_job_loaded(PathBuf::from("/tmp/a.txt"), ViewBuffer::from_plain("old"));
        assert!(s.view.is_none());
    }

    #[test]
    fn cached_preview_is_reused_immediately() {
        let mut s = PreviewState::new(true, true);
        let path = PathBuf::from("/tmp/a.txt");
        s.request_debounced_preview(path.clone());
        s.apply_job_loaded(path.clone(), ViewBuffer::from_plain("hello"));
        s.view = None;
        s.request_debounced_preview(path.clone());
        assert_eq!(s.view.as_ref().map(|(_, v)| v.total_lines), Some(1));
        assert!(s.preview_command_due().is_none());
    }
}
