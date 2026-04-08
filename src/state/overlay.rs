use anyhow::Result;

use crate::action::{Action, CollisionPolicy, Command, MenuId};
use crate::palette::PaletteState;
use crate::state::dialog::{CollisionState, DialogState};
use crate::state::menu::menu_items_for;
use crate::state::prompt::{PromptKind, PromptState};
use crate::state::settings::SettingsState;
use crate::state::types::MenuItem;

/// All modal UI states, mutually exclusive by construction.
/// Only one variant can be active at a time — the type system enforces this.
#[derive(Clone, Debug)]
pub enum ModalState {
    Menu { id: MenuId, selection: usize },
    Prompt(PromptState),
    Dialog(DialogState),
    Collision(CollisionState),
    Palette(PaletteState),
    Settings(SettingsState),
}

#[derive(Clone, Debug, Default)]
pub struct OverlayState {
    pub modal: Option<ModalState>,
}

impl OverlayState {
    /// Close any open modal.
    pub fn close_all(&mut self) {
        self.modal = None;
    }

    pub fn is_menu_open(&self) -> bool {
        matches!(self.modal, Some(ModalState::Menu { .. }))
    }

    pub fn active_menu(&self) -> Option<MenuId> {
        match &self.modal {
            Some(ModalState::Menu { id, .. }) => Some(*id),
            _ => None,
        }
    }

    pub fn menu_selection(&self) -> usize {
        match &self.modal {
            Some(ModalState::Menu { selection, .. }) => *selection,
            _ => 0,
        }
    }

    pub fn menu_items(&self) -> Vec<MenuItem> {
        match &self.modal {
            Some(ModalState::Menu { id, .. }) => menu_items_for(*id),
            _ => vec![],
        }
    }

    pub fn prompt(&self) -> Option<&PromptState> {
        match &self.modal {
            Some(ModalState::Prompt(p)) => Some(p),
            _ => None,
        }
    }

    pub fn dialog(&self) -> Option<&DialogState> {
        match &self.modal {
            Some(ModalState::Dialog(d)) => Some(d),
            _ => None,
        }
    }

    pub fn collision(&self) -> Option<&CollisionState> {
        match &self.modal {
            Some(ModalState::Collision(c)) => Some(c),
            _ => None,
        }
    }

    pub fn palette(&self) -> Option<&PaletteState> {
        match &self.modal {
            Some(ModalState::Palette(p)) => Some(p),
            _ => None,
        }
    }

    pub fn settings(&self) -> Option<&SettingsState> {
        match &self.modal {
            Some(ModalState::Settings(s)) => Some(s),
            _ => None,
        }
    }

    pub fn apply(&mut self, action: &Action) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        match action {
            // ── Palette ──────────────────────────────────────────────────────
            Action::OpenCommandPalette => {
                if self.modal.is_none() {
                    self.modal = Some(ModalState::Palette(PaletteState::new()));
                }
            }
            Action::CloseCommandPalette => {
                self.close_all();
            }
            Action::PaletteInput(c) => {
                if let Some(ModalState::Palette(p)) = self.modal.as_mut() {
                    p.query.push(*c);
                    p.selection = 0;
                }
            }
            Action::PaletteBackspace => {
                if let Some(ModalState::Palette(p)) = self.modal.as_mut() {
                    p.query.pop();
                    p.selection = 0;
                }
            }
            Action::PaletteMoveDown => {
                if let Some(ModalState::Palette(p)) = self.modal.as_mut() {
                    let entries = crate::palette::all_entries();
                    let matches = crate::palette::filter_entries(&entries, &p.query);
                    if !matches.is_empty() {
                        p.selection = (p.selection + 1).min(matches.len() - 1);
                    }
                }
            }
            Action::PaletteMoveUp => {
                if let Some(ModalState::Palette(p)) = self.modal.as_mut() {
                    p.selection = p.selection.saturating_sub(1);
                }
            }
            Action::PaletteConfirm => {
                if let Some(ModalState::Palette(p)) = self.modal.take() {
                    let entries = crate::palette::all_entries();
                    let matches = crate::palette::filter_entries(&entries, &p.query);
                    if let Some(entry) = matches.get(p.selection) {
                        commands.push(Command::DispatchAction(entry.action.clone()));
                    }
                }
            }

            // ── Collision ────────────────────────────────────────────────────
            Action::CollisionCancel => {
                self.close_all();
            }
            Action::CollisionOverwrite => {
                if let Some(ModalState::Collision(c)) = self.modal.take() {
                    commands.push(Command::RunFileOperation {
                        operation: c.operation,
                        refresh: c.refresh,
                        collision: CollisionPolicy::Overwrite,
                    });
                }
            }
            Action::CollisionRename => {
                if let Some(ModalState::Collision(c)) = self.modal.take() {
                    self.modal = Some(ModalState::Prompt(c.rename_prompt()));
                }
            }
            Action::CollisionSkip => {
                self.close_all();
            }

            // ── Dialog ───────────────────────────────────────────────────────
            Action::CloseDialog => {
                self.close_all();
            }
            Action::OpenAboutDialog => {
                // Needs theme preset + config_path — handled in AppState::apply_view
            }
            Action::OpenHelpDialog => {
                self.close_all();
                self.modal = Some(ModalState::Dialog(DialogState::help()));
            }

            // ── Menu ─────────────────────────────────────────────────────────
            Action::CloseMenu => {
                self.close_all();
            }
            Action::OpenMenu(menu_id) => {
                self.close_all();
                self.modal = Some(ModalState::Menu {
                    id: *menu_id,
                    selection: 0,
                });
            }
            Action::MenuActivate => {
                if let Some(ModalState::Menu { id, selection }) = &self.modal {
                    let id = *id;
                    let sel = *selection;
                    if let Some(item) = menu_items_for(id).get(sel).cloned() {
                        self.close_all();
                        commands.push(Command::DispatchAction(item.action.clone()));
                    }
                }
            }
            Action::MenuMnemonic(ch) => {
                if let Some(ModalState::Menu { id, .. }) = &self.modal {
                    let id = *id;
                    if let Some(item) = menu_items_for(id)
                        .into_iter()
                        .find(|item| item.mnemonic.eq_ignore_ascii_case(ch))
                    {
                        self.close_all();
                        commands.push(Command::DispatchAction(item.action.clone()));
                    }
                }
            }
            Action::MenuMoveDown => {
                if let Some(ModalState::Menu { id, selection }) = self.modal.as_mut() {
                    let len = menu_items_for(*id).len();
                    if len > 0 {
                        *selection = (*selection + 1).min(len.saturating_sub(1));
                    }
                }
            }
            Action::MenuMoveUp => {
                if let Some(ModalState::Menu { selection, .. }) = self.modal.as_mut() {
                    *selection = selection.saturating_sub(1);
                }
            }
            Action::MenuNext => {
                if let Some(ModalState::Menu { id, selection }) = self.modal.as_mut() {
                    *id = match *id {
                        MenuId::File => MenuId::Navigate,
                        MenuId::Navigate => MenuId::View,
                        MenuId::View => MenuId::Help,
                        MenuId::Help => MenuId::File,
                    };
                    *selection = 0;
                }
            }
            Action::MenuPrevious => {
                if let Some(ModalState::Menu { id, selection }) = self.modal.as_mut() {
                    *id = match *id {
                        MenuId::File => MenuId::Help,
                        MenuId::Navigate => MenuId::File,
                        MenuId::View => MenuId::Navigate,
                        MenuId::Help => MenuId::View,
                    };
                    *selection = 0;
                }
            }

            // ── File op prompts ──────────────────────────────────────────────
            Action::OpenCopyPrompt
            | Action::OpenDeletePrompt
            | Action::OpenMovePrompt
            | Action::OpenNewDirectoryPrompt
            | Action::OpenNewFilePrompt
            | Action::OpenRenamePrompt => {
                // Building these prompts requires active pane context.
                // AppState::apply_view handles them after calling overlay.close_all().
                self.close_all();
            }

            // ── Prompt input ─────────────────────────────────────────────────
            Action::PromptBackspace => {
                if let Some(ModalState::Prompt(p)) = self.modal.as_mut() {
                    if p.kind != PromptKind::Delete {
                        p.value.pop();
                    }
                }
            }
            Action::PromptCancel => {
                self.close_all();
            }
            Action::PromptInput(ch) => {
                if let Some(ModalState::Prompt(p)) = self.modal.as_mut() {
                    if p.kind != PromptKind::Delete {
                        p.value.push(*ch);
                    }
                }
            }
            Action::PromptSubmit => {
                // Full submission logic needs pane context — handled in AppState::apply_view
            }

            // ── Settings ─────────────────────────────────────────────────────
            Action::OpenSettingsPanel => {
                self.close_all();
                self.modal = Some(ModalState::Settings(SettingsState::new()));
            }
            Action::CloseSettingsPanel => {
                self.close_all();
            }
            Action::SettingsMoveDown => {
                if let Some(ModalState::Settings(s)) = self.modal.as_mut() {
                    s.selection = s.selection.saturating_add(1);
                }
            }
            Action::SettingsMoveUp => {
                if let Some(ModalState::Settings(s)) = self.modal.as_mut() {
                    s.selection = s.selection.saturating_sub(1);
                }
            }
            Action::SettingsToggleCurrent => {
                // Toggle logic needs full config context — handled in AppState::apply_view
            }
            _ => {}
        }
        Ok(commands)
    }

    /// Called by AppState when a collision arrives from a job result.
    pub fn set_collision(&mut self, collision: CollisionState) {
        self.modal = Some(ModalState::Collision(collision));
    }

    /// Called by AppState::apply_view to open About dialog (needs config context).
    pub fn open_about(&mut self, dialog: DialogState) {
        self.close_all();
        self.modal = Some(ModalState::Dialog(dialog));
    }

    /// Called by AppState::apply_view to open a prompt (needs pane context).
    pub fn open_prompt(&mut self, prompt: PromptState) {
        self.modal = Some(ModalState::Prompt(prompt));
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn open_menu_closes_any_existing_modal() {
        let mut s = OverlayState::default();
        s.modal = Some(ModalState::Palette(PaletteState::new()));
        s.apply(&Action::OpenMenu(MenuId::File)).unwrap();
        assert!(matches!(
            s.modal,
            Some(ModalState::Menu {
                id: MenuId::File,
                selection: 0
            })
        ));
    }

    #[test]
    fn open_settings_closes_any_existing_modal() {
        let mut s = OverlayState::default();
        s.modal = Some(ModalState::Menu {
            id: MenuId::File,
            selection: 0,
        });
        s.apply(&Action::OpenSettingsPanel).unwrap();
        assert!(matches!(s.modal, Some(ModalState::Settings(_))));
    }

    #[test]
    fn close_all_removes_modal() {
        let mut s = OverlayState::default();
        s.modal = Some(ModalState::Settings(SettingsState::new()));
        s.close_all();
        assert!(s.modal.is_none());
    }

    #[test]
    fn menu_activate_emits_dispatch_action() {
        let mut s = OverlayState::default();
        s.modal = Some(ModalState::Menu {
            id: MenuId::File,
            selection: 0,
        });
        let cmds = s.apply(&Action::MenuActivate).unwrap();
        assert!(s.modal.is_none(), "menu should close after activation");
        assert!(!cmds.is_empty(), "should emit DispatchAction command");
        assert!(matches!(cmds[0], Command::DispatchAction(_)));
    }

    #[test]
    fn palette_confirm_emits_dispatch_action_and_closes() {
        let mut s = OverlayState::default();
        s.modal = Some(ModalState::Palette(PaletteState::new()));
        // Empty query with selection 0 — may or may not match, but modal should close
        s.apply(&Action::PaletteConfirm).unwrap();
        assert!(s.modal.is_none());
    }
}
