use anyhow::Result;

use crate::action::{Action, Command};
use crate::pane::{PaneId, PaneState};
use crate::state::types::{PaneFocus, PaneLayout};

#[derive(Clone, Debug)]
pub struct PaneSetState {
    pub left: PaneState,
    pub right: PaneState,
    pub focus: PaneFocus,
    pub pane_layout: PaneLayout,
}

impl PaneSetState {
    pub fn new(left: PaneState, right: PaneState) -> Self {
        Self {
            left,
            right,
            focus: PaneFocus::Left,
            pane_layout: PaneLayout::default(),
        }
    }

    pub fn active_pane(&self) -> &PaneState {
        match self.focus {
            PaneFocus::Left | PaneFocus::Preview => &self.left,
            PaneFocus::Right => &self.right,
        }
    }

    pub fn active_pane_mut(&mut self) -> &mut PaneState {
        match self.focus {
            PaneFocus::Left | PaneFocus::Preview => &mut self.left,
            PaneFocus::Right => &mut self.right,
        }
    }

    pub fn inactive_pane(&self) -> &PaneState {
        match self.focus {
            PaneFocus::Left | PaneFocus::Preview => &self.right,
            PaneFocus::Right => &self.left,
        }
    }

    pub fn focused_pane_id(&self) -> PaneId {
        match self.focus {
            PaneFocus::Left | PaneFocus::Preview => PaneId::Left,
            PaneFocus::Right => PaneId::Right,
        }
    }

    pub fn pane(&self, id: PaneId) -> &PaneState {
        match id {
            PaneId::Left => &self.left,
            PaneId::Right => &self.right,
        }
    }

    pub fn pane_mut(&mut self, id: PaneId) -> &mut PaneState {
        match id {
            PaneId::Left => &mut self.left,
            PaneId::Right => &mut self.right,
        }
    }

    pub fn apply(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        match action {
            Action::EnterSelection => {
                if self.active_pane().can_enter_selected() {
                    if let Some(entry) = self.active_pane().selected_entry().cloned() {
                        let pane_id = self.focused_pane_id();
                        let active = self.active_pane_mut();
                        active.clear_marks();
                        active.push_history();
                        if entry.kind == crate::fs::EntryKind::Archive {
                            commands.push(Command::OpenArchive {
                                path: entry.path.clone(),
                                inner: std::path::PathBuf::new(),
                            });
                        } else {
                            commands.push(Command::ScanPane {
                                pane: pane_id,
                                path: entry.path.clone(),
                            });
                        }
                    }
                }
            }
            Action::NavigateToParent => {
                // If we're inside an archive, navigate up within archive or exit archive.
                if self.active_pane().in_archive() {
                    if let crate::pane::PaneMode::Archive { source, inner_path } =
                        &self.active_pane().mode
                    {
                        if inner_path.as_os_str().is_empty() {
                            // at archive root -> exit archive
                            commands.push(Command::DispatchAction(Action::ExitArchive));
                        } else {
                            // navigate to parent inside archive
                            let parent = inner_path
                                .parent()
                                .map(std::path::Path::to_path_buf)
                                .unwrap_or_default();
                            commands.push(Command::OpenArchive {
                                path: source.clone(),
                                inner: parent,
                            });
                        }
                    }
                } else if let Some(path) = self.active_pane().parent_path() {
                    let pane = self.focused_pane_id();
                    let active = self.active_pane_mut();
                    active.clear_marks();
                    active.push_history();
                    commands.push(Command::ScanPane { pane, path });
                }
            }
            Action::NavigateBack => {
                if let Some(path) = self.active_pane_mut().pop_back() {
                    let pane_id = self.focused_pane_id();
                    self.active_pane_mut().clear_marks();
                    commands.push(Command::ScanPane {
                        pane: pane_id,
                        path,
                    });
                }
            }
            Action::NavigateForward => {
                if let Some(path) = self.active_pane_mut().pop_forward() {
                    let pane_id = self.focused_pane_id();
                    self.active_pane_mut().clear_marks();
                    commands.push(Command::ScanPane {
                        pane: pane_id,
                        path,
                    });
                }
            }
            Action::FocusNextPane => {
                self.focus = match self.focus {
                    PaneFocus::Left | PaneFocus::Preview => PaneFocus::Right,
                    PaneFocus::Right => PaneFocus::Left,
                };
            }
            Action::MoveSelectionDown => {
                self.active_pane_mut().move_selection_down();
            }
            Action::MoveSelectionUp => {
                self.active_pane_mut().move_selection_up();
            }
            Action::ToggleMark => {
                self.active_pane_mut().toggle_mark_selected();
            }
            Action::ClearMarks => {
                self.active_pane_mut().clear_marks();
            }
            Action::Refresh => {
                let pane = self.focused_pane_id();
                let path = self.active_pane().cwd.clone();
                commands.push(Command::ScanPane { pane, path });
            }
            Action::CycleSortMode => {
                let pane = self.active_pane_mut();
                pane.sort_mode = pane.sort_mode.next();
                pane.selection = 0;
                pane.scroll_offset = 0;
                pane.refresh_filter();
            }
            Action::PaneClick { left_pane, row } => {
                use crate::state::types::PaneFocus;
                // Switch focus to clicked pane if needed.
                let target_focus = if *left_pane {
                    PaneFocus::Left
                } else {
                    PaneFocus::Right
                };
                self.focus = target_focus;
                // Move selection to the clicked row (clamped to entry count).
                let pane = self.active_pane_mut();
                let count = pane.filtered_len_pub();
                if count > 0 {
                    pane.selection = (*row).min(count.saturating_sub(1));
                    pane.scroll_offset = pane.scroll_offset.min(pane.selection);
                }
            }
            Action::PaneDoubleClick { left_pane, row } => {
                use crate::state::types::PaneFocus;
                let target_focus = if *left_pane {
                    PaneFocus::Left
                } else {
                    PaneFocus::Right
                };
                self.focus = target_focus;
                {
                    let pane = self.active_pane_mut();
                    let count = pane.filtered_len_pub();
                    if count > 0 {
                        pane.selection = (*row).min(count.saturating_sub(1));
                    }
                }
                // Now try to enter the selection.
                if let Some(entry) = self.active_pane().selected_entry().cloned() {
                    if entry.kind == crate::fs::EntryKind::Directory {
                        let pane_id = self.focused_pane_id();
                        self.active_pane_mut().clear_marks();
                        self.active_pane_mut().push_history();
                        commands.push(Command::ScanPane {
                            pane: pane_id,
                            path: entry.path,
                        });
                    } else if entry.kind == crate::fs::EntryKind::Archive {
                        commands.push(Command::OpenArchive {
                            path: entry.path,
                            inner: std::path::PathBuf::new(),
                        });
                    }
                }
            }

            Action::ToggleHiddenFiles => {
                let new_value = !self.active_pane().show_hidden;
                self.active_pane_mut().set_show_hidden(new_value)?;
            }
            _ => {}
        }
        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane::PaneState;

    fn make_state() -> PaneSetState {
        let cwd = std::env::temp_dir();
        PaneSetState::new(
            PaneState::empty("Left", cwd.clone()),
            PaneState::empty("Right", cwd),
        )
    }

    #[test]
    fn focus_next_pane_cycles_left_to_right() {
        let mut s = make_state();
        assert_eq!(s.focus, PaneFocus::Left);
        s.apply(&Action::FocusNextPane).unwrap();
        assert_eq!(s.focus, PaneFocus::Right);
    }

    #[test]
    fn focus_next_pane_cycles_right_to_left() {
        let mut s = make_state();
        s.focus = PaneFocus::Right;
        s.apply(&Action::FocusNextPane).unwrap();
        assert_eq!(s.focus, PaneFocus::Left);
    }

    #[test]
    fn inactive_pane_returns_opposite_of_focus() {
        let cwd = std::env::temp_dir();
        let mut s = PaneSetState::new(
            PaneState::empty("Left", cwd.clone()),
            PaneState::empty("Right", cwd.clone()),
        );
        s.focus = PaneFocus::Left;
        assert_eq!(s.inactive_pane().title, "Right");
        s.focus = PaneFocus::Right;
        assert_eq!(s.inactive_pane().title, "Left");
    }
}
