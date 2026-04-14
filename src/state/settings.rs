use crate::config::{IconMode, ThemePreset};

use super::PaneLayout;

/// Identifies which keymap field a rebindable settings entry controls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeymapField {
    Quit,
    SwitchPane,
    Refresh,
    Workspace(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsState {
    pub selection: usize,
    /// When `Some(idx)`, the settings panel is waiting for the user to press
    /// the new key combo for the entry at `idx`. Any key event becomes the
    /// new binding; Esc cancels.
    pub rebind_mode: Option<usize>,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            selection: 0,
            rebind_mode: None,
        }
    }
}

impl Default for SettingsState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettingsField {
    Theme(ThemePreset),
    IconMode(IconMode),
    PaneLayout(PaneLayout),
    PreviewPanel,
    PreviewOnSelection,
    EditorTabWidth(u8),
    EditorWordWrap,
    TerminalOpenByDefault,
    /// A rebindable key mapping.  The `current` string is what is stored in
    /// `KeymapConfig` (e.g. `"alt+1"`) and is displayed to the user.
    KeymapBinding {
        field: KeymapField,
        current: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsEntry {
    pub label: &'static str,
    pub value: String,
    pub hint: &'static str,
    pub field: SettingsField,
}
