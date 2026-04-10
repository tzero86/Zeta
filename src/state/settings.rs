use crate::config::{IconMode, ThemePreset};

use super::PaneLayout;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsState {
    pub selection: usize,
}

impl SettingsState {
    pub fn new() -> Self {
        Self { selection: 0 }
    }
}

impl Default for SettingsState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsField {
    Theme(ThemePreset),
    IconMode(IconMode),
    PaneLayout(PaneLayout),
    PreviewPanel,
    PreviewOnSelection,
    KeymapPlaceholder,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsEntry {
    pub label: &'static str,
    pub value: String,
    pub hint: &'static str,
    pub field: SettingsField,
}
