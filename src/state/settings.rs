use crate::config::{IconMode, ThemePreset};

use super::PaneLayout;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SettingsTab {
    #[default]
    Appearance,
    Panels,
    Editor,
    Keymaps,
}

impl SettingsTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Appearance => "1 Appearance",
            Self::Panels => "2 Panels",
            Self::Editor => "3 Editor",
            Self::Keymaps => "4 Keymaps",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Appearance => Self::Panels,
            Self::Panels => Self::Editor,
            Self::Editor => Self::Keymaps,
            Self::Keymaps => Self::Appearance,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Appearance => Self::Keymaps,
            Self::Panels => Self::Appearance,
            Self::Editor => Self::Panels,
            Self::Keymaps => Self::Editor,
        }
    }

    pub fn from_number(n: usize) -> Option<Self> {
        match n {
            1 => Some(Self::Appearance),
            2 => Some(Self::Panels),
            3 => Some(Self::Editor),
            4 => Some(Self::Keymaps),
            _ => None,
        }
    }
}

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
    pub active_tab: SettingsTab,
    /// When `Some(idx)`, the settings panel is waiting for the user to press
    /// the new key combo for the entry at `idx`. Any key event becomes the
    /// new binding; Esc cancels.
    pub rebind_mode: Option<usize>,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            selection: 0,
            active_tab: SettingsTab::default(),
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

#[cfg(test)]
mod tests {
    use super::SettingsTab;

    #[test]
    fn settings_tab_cycles_forward() {
        assert_eq!(SettingsTab::Appearance.next(), SettingsTab::Panels);
        assert_eq!(SettingsTab::Keymaps.next(), SettingsTab::Appearance);
    }

    #[test]
    fn settings_tab_from_number() {
        assert_eq!(SettingsTab::from_number(1), Some(SettingsTab::Appearance));
        assert_eq!(SettingsTab::from_number(5), None);
    }
}
