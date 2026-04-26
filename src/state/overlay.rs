use anyhow::Result;
use crossterm::event::Event;
use tui_input::backend::crossterm::EventHandler;

use crate::action::{Action, CollisionPolicy, Command, MenuId};
use crate::finder::FileFinderState;
use crate::palette::PaletteState;
use crate::state::bookmarks::BookmarksState;
use crate::state::dialog::{CollisionState, DialogState};
use crate::state::menu::menu_items_for;
use crate::state::prompt::PromptState;
use crate::state::settings::SettingsState;
use crate::state::ssh::SshConnectionState;
use crate::state::types::MenuItem;

fn flyout_trigger(item: &MenuItem) -> Option<MenuId> {
    if let Action::OpenMenu(id) = item.action {
        Some(id)
    } else {
        None
    }
}

/// All modal UI states, mutually exclusive by construction.
/// Only one variant can be active at a time — the type system enforces this.
#[derive(Clone, Debug)]
pub enum ModalState {
    Menu {
        id: MenuId,
        selection: usize,
        /// Active flyout submenu: (submenu_id, submenu_selection).
        /// When Some, keyboard navigation targets the flyout.
        flyout: Option<(MenuId, usize)>,
    },
    Prompt(PromptState),
    Dialog(DialogState),
    Collision(CollisionState),
    DestructiveConfirm(crate::state::dialog::DestructiveConfirmState),
    Palette(PaletteState),
    Settings(SettingsState),
    Bookmarks(BookmarksState),
    FileFinder(FileFinderState),
    SshConnect(crate::state::ssh::SshConnectionState),
    SshTrustPrompt {
        host: String,
        port: u16,
        fingerprints: crate::state::ssh::HostKeyFingerprints,
        address: String,
        auth_method: crate::state::ssh::SshAuthMethod,
        credential: String,
        pane: crate::pane::PaneId,
    },
    /// Context menu for opening a file with a specific program.
    OpenWith {
        /// `(display_name, command)` — command is empty string for the OS default.
        items: Vec<(String, String)>,
        selection: usize,
        target: std::path::PathBuf,
    },
}

#[derive(Clone, Debug)]
pub struct OverlayState {
    pub modal: Option<ModalState>,
    pub menu_context: crate::state::menu::MenuContext,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            modal: None,
            menu_context: crate::state::menu::MenuContext::Pane,
        }
    }
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
            Some(ModalState::Menu { id, .. }) => menu_items_for(*id, self.menu_context),
            _ => vec![],
        }
    }

    /// Returns the active flyout state as `(submenu_id, submenu_selection)` if open.
    pub fn menu_flyout(&self) -> Option<(MenuId, usize)> {
        match &self.modal {
            Some(ModalState::Menu { flyout, .. }) => *flyout,
            _ => None,
        }
    }

    /// Returns the flyout submenu items if a flyout is open.
    pub fn menu_flyout_items(&self) -> Vec<MenuItem> {
        match &self.modal {
            Some(ModalState::Menu { flyout, .. }) => {
                if let Some((flyout_id, _)) = flyout {
                    menu_items_for(*flyout_id, self.menu_context)
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    /// Returns the flyout submenu selection index if a flyout is open.
    pub fn menu_flyout_selection(&self) -> usize {
        match &self.modal {
            Some(ModalState::Menu { flyout, .. }) => flyout.map(|(_, sel)| sel).unwrap_or(0),
            _ => 0,
        }
    }

    pub fn set_menu_context(&mut self, ctx: crate::state::menu::MenuContext) {
        self.menu_context = ctx;
    }

    /// Legacy helper kept for compatibility — sets context to Editor or Pane.
    pub fn set_editor_menu_mode(&mut self, enabled: bool) {
        self.menu_context = if enabled {
            crate::state::menu::MenuContext::Editor
        } else {
            crate::state::menu::MenuContext::Pane
        };
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

    pub fn destructive_confirm(&self) -> Option<&crate::state::dialog::DestructiveConfirmState> {
        match &self.modal {
            Some(ModalState::DestructiveConfirm(d)) => Some(d),
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

    pub fn bookmarks(&self) -> Option<&BookmarksState> {
        match &self.modal {
            Some(ModalState::Bookmarks(b)) => Some(b),
            _ => None,
        }
    }

    pub fn bookmarks_mut(&mut self) -> Option<&mut BookmarksState> {
        match &mut self.modal {
            Some(ModalState::Bookmarks(b)) => Some(b),
            _ => None,
        }
    }

    pub fn file_finder(&self) -> Option<&FileFinderState> {
        match &self.modal {
            Some(ModalState::FileFinder(f)) => Some(f),
            _ => None,
        }
    }

    pub fn file_finder_mut(&mut self) -> Option<&mut FileFinderState> {
        match &mut self.modal {
            Some(ModalState::FileFinder(f)) => Some(f),
            _ => None,
        }
    }

    pub fn open_ssh_connect(&mut self, state: SshConnectionState) {
        self.close_all();
        self.modal = Some(ModalState::SshConnect(state));
    }

    pub fn close_ssh_connect(&mut self) {
        if matches!(self.modal, Some(ModalState::SshConnect(_))) {
            self.modal = None;
        }
    }

    pub fn ssh_connect(&self) -> Option<&SshConnectionState> {
        match &self.modal {
            Some(ModalState::SshConnect(s)) => Some(s),
            _ => None,
        }
    }

    pub fn ssh_connect_mut(&mut self) -> Option<&mut SshConnectionState> {
        match &mut self.modal {
            Some(ModalState::SshConnect(s)) => Some(s),
            _ => None,
        }
    }

    pub fn ssh_trust_prompt(&self) -> Option<&crate::state::ssh::SshAuthMethod> {
        // Returns a reference to auth_method so callers can check if the prompt is active.
        // Full data is accessed by matching on the modal directly.
        match &self.modal {
            Some(ModalState::SshTrustPrompt { auth_method, .. }) => Some(auth_method),
            _ => None,
        }
    }

    pub fn is_ssh_trust_prompt(&self) -> bool {
        matches!(self.modal, Some(ModalState::SshTrustPrompt { .. }))
    }

    pub fn is_open_with(&self) -> bool {
        matches!(self.modal, Some(ModalState::OpenWith { .. }))
    }

    /// Return the open-with menu state as `(items, selection, target)` if active.
    #[allow(clippy::type_complexity)]
    pub fn open_with(&self) -> Option<(&[(String, String)], usize, &std::path::Path)> {
        match &self.modal {
            Some(ModalState::OpenWith {
                items,
                selection,
                target,
            }) => Some((items.as_slice(), *selection, target.as_path())),
            _ => None,
        }
    }

    /// Open the SSH host-trust prompt with the pending connection parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn open_ssh_trust_prompt(
        &mut self,
        host: String,
        port: u16,
        fingerprints: crate::state::ssh::HostKeyFingerprints,
        address: String,
        auth_method: crate::state::ssh::SshAuthMethod,
        credential: String,
        pane: crate::pane::PaneId,
    ) {
        self.close_all();
        self.modal = Some(ModalState::SshTrustPrompt {
            host,
            port,
            fingerprints,
            address,
            auth_method,
            credential,
            pane,
        });
    }

    pub fn open_file_finder(&mut self, state: FileFinderState) {
        self.close_all();
        self.modal = Some(ModalState::FileFinder(state));
    }

    pub fn open_bookmarks(&mut self, state: BookmarksState) {
        self.close_all();
        self.modal = Some(ModalState::Bookmarks(state));
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

            // ── Destructive Confirm ──────────────────────────────────────────
            Action::DestructiveConfirmYes => {
                if let Some(ModalState::DestructiveConfirm(state)) = self.modal.take() {
                    for operation in state.operations {
                        commands.push(crate::action::Command::RunFileOperation {
                            operation,
                            refresh: state.refresh_targets.clone(),
                            collision: crate::action::CollisionPolicy::Fail,
                        });
                    }
                }
            }
            Action::DestructiveConfirmNo => {
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
            Action::ScrollDialogDown => {
                if let Some(ModalState::Dialog(d)) = self.modal.as_mut() {
                    d.scroll_down(1);
                }
            }
            Action::ScrollDialogUp => {
                if let Some(ModalState::Dialog(d)) = self.modal.as_mut() {
                    d.scroll_up(1);
                }
            }
            Action::ScrollDialogPageDown => {
                if let Some(ModalState::Dialog(d)) = self.modal.as_mut() {
                    d.scroll_down(10);
                }
            }
            Action::ScrollDialogPageUp => {
                if let Some(ModalState::Dialog(d)) = self.modal.as_mut() {
                    d.scroll_up(10);
                }
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
                    flyout: None,
                });
            }
            Action::MenuActivate => {
                let snapshot = if let Some(ModalState::Menu {
                    id,
                    selection,
                    flyout,
                }) = &self.modal
                {
                    Some((*id, *selection, *flyout))
                } else {
                    None
                };
                if let Some((id, sel, flyout_opt)) = snapshot {
                    if let Some((flyout_id, flyout_sel)) = flyout_opt {
                        if let Some(item) = menu_items_for(flyout_id, self.menu_context)
                            .get(flyout_sel)
                            .cloned()
                        {
                            self.close_all();
                            commands.push(Command::DispatchAction(item.action.clone()));
                        }
                    } else {
                        let items = menu_items_for(id, self.menu_context);
                        if let Some(item) = items.get(sel).cloned() {
                            if let Some(sub_id) = flyout_trigger(&item) {
                                if let Some(ModalState::Menu { flyout, .. }) = self.modal.as_mut() {
                                    *flyout = Some((sub_id, 0));
                                }
                            } else {
                                self.close_all();
                                commands.push(Command::DispatchAction(item.action.clone()));
                            }
                        }
                    }
                }
            }
            Action::MenuEnterFlyout => {
                let snapshot = if let Some(ModalState::Menu {
                    id,
                    selection,
                    flyout,
                }) = &self.modal
                {
                    Some((*id, *selection, flyout.is_some()))
                } else {
                    None
                };
                if let Some((id, sel, flyout_open)) = snapshot {
                    if flyout_open {
                        commands.push(Command::DispatchAction(Action::MenuActivate));
                    } else {
                        let items = menu_items_for(id, self.menu_context);
                        if let Some(sub_id) = items.get(sel).and_then(flyout_trigger) {
                            if let Some(ModalState::Menu { flyout, .. }) = self.modal.as_mut() {
                                *flyout = Some((sub_id, 0));
                            }
                        } else {
                            // Not a trigger — switch to next tab directly
                            if let Some(ModalState::Menu {
                                id,
                                selection,
                                flyout,
                            }) = self.modal.as_mut()
                            {
                                *flyout = None;
                                let tabs = crate::state::menu::menu_tabs(self.menu_context);
                                if let Some(pos) = tabs.iter().position(|tab| tab.id == *id) {
                                    *id = tabs[(pos + 1) % tabs.len()].id;
                                }
                                *selection = 0;
                            }
                        }
                    }
                }
            }
            Action::MenuExitFlyout => {
                let flyout_is_open = if let Some(ModalState::Menu { flyout, .. }) = &self.modal {
                    flyout.is_some()
                } else {
                    false
                };
                if let Some(ModalState::Menu { .. }) = &self.modal {
                    if flyout_is_open {
                        if let Some(ModalState::Menu { flyout, .. }) = self.modal.as_mut() {
                            *flyout = None;
                        }
                    } else {
                        // No flyout open — switch to previous tab directly
                        if let Some(ModalState::Menu {
                            id,
                            selection,
                            flyout,
                        }) = self.modal.as_mut()
                        {
                            *flyout = None;
                            let tabs = crate::state::menu::menu_tabs(self.menu_context);
                            if let Some(pos) = tabs.iter().position(|tab| tab.id == *id) {
                                *id = tabs[(pos + tabs.len() - 1) % tabs.len()].id;
                            }
                            *selection = 0;
                        }
                    }
                }
            }
            Action::MenuClickItem(index) => {
                if let Some(ModalState::Menu { id, .. }) = &self.modal {
                    let id = *id;
                    let items = menu_items_for(id, self.menu_context);
                    if *index < items.len() {
                        let item = items[*index].clone();
                        self.close_all();
                        commands.push(Command::DispatchAction(item.action.clone()));
                    }
                }
            }
            Action::MenuSetSelection(index) => {
                if let Some(ModalState::Menu { selection, .. }) = &mut self.modal {
                    *selection = *index;
                }
            }
            Action::MenuMnemonic(ch) => {
                let search_id = if let Some(ModalState::Menu { id, flyout, .. }) = &self.modal {
                    Some(flyout.map(|(fid, _)| fid).unwrap_or(*id))
                } else {
                    None
                };
                if let Some(search_id) = search_id {
                    if let Some(item) = menu_items_for(search_id, self.menu_context)
                        .into_iter()
                        .find(|item| item.mnemonic.eq_ignore_ascii_case(ch))
                    {
                        self.close_all();
                        commands.push(Command::DispatchAction(item.action.clone()));
                    }
                }
            }
            Action::MenuMoveDown => {
                let snapshot = if let Some(ModalState::Menu {
                    id,
                    selection,
                    flyout,
                }) = &self.modal
                {
                    Some((*id, *selection, *flyout))
                } else {
                    None
                };
                if let Some((id, selection, flyout_opt)) = snapshot {
                    if let Some((flyout_id, flyout_sel)) = flyout_opt {
                        let flyout_len = menu_items_for(flyout_id, self.menu_context).len();
                        if flyout_len > 0 {
                            let new_sel = (flyout_sel + 1).min(flyout_len.saturating_sub(1));
                            if let Some(ModalState::Menu { flyout, .. }) = self.modal.as_mut() {
                                if let Some((_, s)) = flyout.as_mut() {
                                    *s = new_sel;
                                }
                            }
                        }
                    } else {
                        let items = menu_items_for(id, self.menu_context);
                        let len = items.len();
                        if len > 0 {
                            let new_sel = (selection + 1).min(len.saturating_sub(1));
                            let trigger = items.get(new_sel).and_then(flyout_trigger);
                            if let Some(ModalState::Menu {
                                selection, flyout, ..
                            }) = self.modal.as_mut()
                            {
                                *selection = new_sel;
                                *flyout = trigger.map(|sub_id| (sub_id, 0));
                            }
                        }
                    }
                }
            }
            Action::MenuMoveUp => {
                let snapshot = if let Some(ModalState::Menu {
                    id,
                    selection,
                    flyout,
                }) = &self.modal
                {
                    Some((*id, *selection, *flyout))
                } else {
                    None
                };
                if let Some((id, selection, flyout_opt)) = snapshot {
                    if let Some((_, flyout_sel)) = flyout_opt {
                        if flyout_sel > 0 {
                            if let Some(ModalState::Menu { flyout, .. }) = self.modal.as_mut() {
                                if let Some((_, s)) = flyout.as_mut() {
                                    *s -= 1;
                                }
                            }
                        } else {
                            if let Some(ModalState::Menu { flyout, .. }) = self.modal.as_mut() {
                                *flyout = None;
                            }
                        }
                    } else {
                        let new_sel = selection.saturating_sub(1);
                        let items = menu_items_for(id, self.menu_context);
                        let trigger = items.get(new_sel).and_then(flyout_trigger);
                        if let Some(ModalState::Menu {
                            selection, flyout, ..
                        }) = self.modal.as_mut()
                        {
                            *selection = new_sel;
                            *flyout = trigger.map(|sub_id| (sub_id, 0));
                        }
                    }
                }
            }
            Action::MenuNext => {
                if let Some(ModalState::Menu {
                    id,
                    selection,
                    flyout,
                }) = self.modal.as_mut()
                {
                    *flyout = None;
                    let tabs = crate::state::menu::menu_tabs(self.menu_context);
                    if let Some(pos) = tabs.iter().position(|tab| tab.id == *id) {
                        *id = tabs[(pos + 1) % tabs.len()].id;
                    }
                    *selection = 0;
                }
            }
            Action::MenuPrevious => {
                if let Some(ModalState::Menu {
                    id,
                    selection,
                    flyout,
                }) = self.modal.as_mut()
                {
                    *flyout = None;
                    let tabs = crate::state::menu::menu_tabs(self.menu_context);
                    if let Some(pos) = tabs.iter().position(|tab| tab.id == *id) {
                        *id = tabs[(pos + tabs.len() - 1) % tabs.len()].id;
                    }
                    *selection = 0;
                }
            }

            // ── File op prompts ──────────────────────────────────────────────
            Action::OpenCopyPrompt
            | Action::OpenDeletePrompt
            | Action::OpenPermanentDeletePrompt
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
                // Legacy variant — kept so existing tests compile.
                // Routes through tui-input for consistent cursor handling.
                if let Some(ModalState::Prompt(p)) = self.modal.as_mut() {
                    if !p.kind.is_confirmation_only() {
                        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
                        p.input.handle_event(&Event::Key(KeyEvent::new(
                            KeyCode::Backspace,
                            KeyModifiers::NONE,
                        )));
                    }
                }
            }
            Action::PromptInput(ch) => {
                // Legacy variant — kept so existing tests compile.
                if let Some(ModalState::Prompt(p)) = self.modal.as_mut() {
                    if !p.kind.is_confirmation_only() {
                        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
                        p.input.handle_event(&Event::Key(KeyEvent::new(
                            KeyCode::Char(*ch),
                            KeyModifiers::NONE,
                        )));
                    }
                }
            }
            Action::PromptKey(key_event) => {
                if let Some(ModalState::Prompt(p)) = self.modal.as_mut() {
                    if !p.kind.is_confirmation_only() {
                        p.input.handle_event(&Event::Key(*key_event));
                    }
                }
            }
            Action::PromptCancel => {
                self.close_all();
            }
            Action::PromptSubmit => {
                // Full submission logic needs pane context — handled in AppState::apply_view
            }

            // ── Settings ─────────────────────────────────────────────────────
            Action::OpenSettingsPanel => {
                self.close_all();
                self.modal = Some(ModalState::Settings(SettingsState::new()));
            }
            Action::OpenBookmarks => {
                self.close_all();
                self.modal = Some(ModalState::Bookmarks(BookmarksState::new()));
            }
            Action::CloseBookmarks => {
                self.close_all();
            }
            Action::BookmarkMoveDown => {
                if let Some(ModalState::Bookmarks(b)) = self.modal.as_mut() {
                    b.selection = b.selection.saturating_add(1);
                }
            }
            Action::BookmarkMoveUp => {
                if let Some(ModalState::Bookmarks(b)) = self.modal.as_mut() {
                    b.selection = b.selection.saturating_sub(1);
                }
            }
            Action::BookmarkConfirm => {
                if let Some(ModalState::Bookmarks(b)) = self.modal.as_ref() {
                    commands.push(Command::DispatchAction(Action::BookmarkSelect(b.selection)));
                }
            }
            Action::BookmarkDeleteCurrent => {
                if let Some(ModalState::Bookmarks(b)) = self.modal.as_ref() {
                    commands.push(Command::DispatchAction(Action::DeleteBookmark(b.selection)));
                }
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
                selection: 0,
                flyout: None,
            })
        ));
    }

    #[test]
    fn open_settings_closes_any_existing_modal() {
        let mut s = OverlayState::default();
        s.modal = Some(ModalState::Menu {
            id: MenuId::File,
            selection: 0,
            flyout: None,
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
    fn open_menu_has_no_flyout() {
        let mut s = OverlayState::default();
        s.apply(&Action::OpenMenu(MenuId::View)).unwrap();
        assert!(
            s.menu_flyout().is_none(),
            "newly opened menu must have no flyout"
        );
    }

    #[test]
    fn menu_activate_emits_dispatch_action() {
        let mut s = OverlayState::default();
        s.modal = Some(ModalState::Menu {
            id: MenuId::File,
            selection: 0,
            flyout: None,
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

    fn open_view_menu() -> OverlayState {
        let mut s = OverlayState::default();
        s.apply(&Action::OpenMenu(MenuId::View)).unwrap();
        s
    }

    #[test]
    fn move_down_to_themes_trigger_opens_flyout() {
        let mut s = open_view_menu();
        // View menu pane items: 0=Toggle Hidden, 1=Settings, 2=Layout:Side, 3=Layout:Stacked, 4=Themes..., 5=Toggle Details
        s.apply(&Action::MenuMoveDown).unwrap(); // sel=1
        s.apply(&Action::MenuMoveDown).unwrap(); // sel=2
        s.apply(&Action::MenuMoveDown).unwrap(); // sel=3
        s.apply(&Action::MenuMoveDown).unwrap(); // sel=4 (Themes...)
        let flyout = s.menu_flyout();
        assert!(
            flyout.is_some(),
            "flyout must open when landing on Themes trigger"
        );
        assert_eq!(flyout.unwrap().0, MenuId::Themes);
        assert_eq!(flyout.unwrap().1, 0, "flyout selection starts at 0");
    }

    #[test]
    fn move_up_away_from_trigger_closes_flyout() {
        let mut s = open_view_menu();
        for _ in 0..4 {
            s.apply(&Action::MenuMoveDown).unwrap();
        }
        assert!(s.menu_flyout().is_some());
        s.apply(&Action::MenuMoveUp).unwrap();
        assert!(
            s.menu_flyout().is_none(),
            "flyout must close when leaving trigger item"
        );
    }

    #[test]
    fn move_down_while_flyout_open_moves_flyout_selection() {
        let mut s = open_view_menu();
        for _ in 0..4 {
            s.apply(&Action::MenuMoveDown).unwrap();
        }
        assert_eq!(s.menu_flyout().unwrap().1, 0);
        s.apply(&Action::MenuMoveDown).unwrap();
        assert_eq!(
            s.menu_flyout().unwrap().1,
            1,
            "down key should advance flyout selection"
        );
    }

    #[test]
    fn enter_flyout_on_trigger_opens_flyout() {
        let mut s = open_view_menu();
        for _ in 0..4 {
            s.apply(&Action::MenuMoveDown).unwrap();
        }
        let flyout_before = s.menu_flyout();
        s.apply(&Action::MenuEnterFlyout).unwrap();
        // flyout stays open, focus now in flyout
        assert_eq!(
            s.menu_flyout(),
            flyout_before,
            "flyout stays open after MenuEnterFlyout"
        );
    }

    #[test]
    fn enter_flyout_not_on_trigger_switches_tab() {
        let mut s = open_view_menu();
        // selection=0, not a trigger item
        let initial_menu = s.active_menu();
        s.apply(&Action::MenuEnterFlyout).unwrap();
        assert_ne!(
            s.active_menu(),
            initial_menu,
            "MenuEnterFlyout on non-trigger must switch tab"
        );
        assert!(s.menu_flyout().is_none());
    }

    #[test]
    fn exit_flyout_when_open_collapses_flyout() {
        let mut s = open_view_menu();
        for _ in 0..4 {
            s.apply(&Action::MenuMoveDown).unwrap();
        }
        assert!(s.menu_flyout().is_some());
        s.apply(&Action::MenuExitFlyout).unwrap();
        assert!(
            s.menu_flyout().is_none(),
            "MenuExitFlyout must close flyout"
        );
        assert!(s.is_menu_open(), "parent menu must remain open");
    }

    #[test]
    fn exit_flyout_when_closed_switches_prev_tab() {
        let mut s = open_view_menu();
        let initial_menu = s.active_menu();
        s.apply(&Action::MenuExitFlyout).unwrap();
        assert_ne!(
            s.active_menu(),
            initial_menu,
            "MenuExitFlyout with no flyout switches to prev tab"
        );
    }

    #[test]
    fn menu_activate_on_flyout_item_dispatches_action() {
        let mut s = open_view_menu();
        for _ in 0..4 {
            s.apply(&Action::MenuMoveDown).unwrap();
        }
        assert!(s.menu_flyout().is_some());
        let cmds = s.apply(&Action::MenuActivate).unwrap();
        assert!(
            !cmds.is_empty(),
            "MenuActivate on flyout item must dispatch action"
        );
        assert!(
            s.modal.is_none(),
            "menu must close after activating flyout item"
        );
    }

    #[test]
    fn menu_next_clears_flyout() {
        let mut s = open_view_menu();
        for _ in 0..4 {
            s.apply(&Action::MenuMoveDown).unwrap();
        }
        assert!(s.menu_flyout().is_some());
        s.apply(&Action::MenuNext).unwrap();
        assert!(s.menu_flyout().is_none(), "MenuNext must clear flyout");
    }
}
