use std::path::PathBuf;

use anyhow::Result;

use crate::action::{Action, Command};
use crate::preview::ViewBuffer;
use crate::state::types::PaneFocus;

#[derive(Clone, Debug, Default)]
pub struct PreviewState {
    pub view: Option<(PathBuf, ViewBuffer)>,
    pub panel_open: bool,
    pub preview_on_selection: bool,
}

impl PreviewState {
    pub fn new(panel_open: bool, preview_on_selection: bool) -> Self {
        Self {
            view: None,
            panel_open,
            preview_on_selection,
        }
    }

    pub fn apply(
        &mut self,
        action: &Action,
        focus: &PaneFocus,
    ) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        match action {
            Action::ClearPreview => {
                self.view = None;
            }
            Action::TogglePreviewPanel => {
                self.panel_open = !self.panel_open;
            }
            Action::PreviewFile { path } => {
                commands.push(Command::PreviewFile { path: path.clone() });
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
        if let Some((ref current, ref mut buf)) = self.view {
            if *current == path {
                buf.reset_scroll();
                return;
            }
        }
        self.view = Some((path, view));
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
        s.apply(&Action::TogglePreviewPanel, &PaneFocus::Left).unwrap();
        assert!(s.panel_open);
        s.apply(&Action::TogglePreviewPanel, &PaneFocus::Left).unwrap();
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
        // scroll while pane focused — should have no effect
        s.apply(&Action::ScrollPreviewDown, &PaneFocus::Left).unwrap();
        let row = s.view.as_ref().unwrap().1.scroll_row;
        assert_eq!(row, 0);
        // scroll while preview focused — should move
        s.apply(&Action::ScrollPreviewDown, &PaneFocus::Preview).unwrap();
        let row = s.view.as_ref().unwrap().1.scroll_row;
        assert_eq!(row, 1);
    }
}
