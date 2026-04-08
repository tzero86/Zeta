use crate::action::Action;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneFocus {
    Left,
    Right,
    Preview,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PaneLayout {
    #[default]
    SideBySide,
    Stacked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuItem {
    pub label: &'static str,
    pub shortcut: &'static str,
    pub mnemonic: char,
    pub action: Action,
}

/// Which input layer currently has keyboard focus.
///
/// Derived from `AppState::focus_layer()` — do not store separately.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusLayer {
    /// No overlay open; directional keys navigate pane entries.
    Pane,
    /// The editor panel is focused.
    Editor,
    /// The preview panel is focused.
    Preview,
    /// A modal overlay is open; only modal-specific keys are processed.
    Modal(ModalKind),
}

impl Default for FocusLayer {
    fn default() -> Self {
        Self::Pane
    }
}

/// Identifies which modal overlay is currently active.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModalKind {
    Menu,
    Prompt,
    Dialog,
    Collision,
    Palette,
    Settings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_layer_modal_wraps_kind() {
        let layer = FocusLayer::Modal(ModalKind::Palette);
        assert!(matches!(layer, FocusLayer::Modal(ModalKind::Palette)));
    }

    #[test]
    fn focus_layer_pane_is_default() {
        assert!(matches!(FocusLayer::default(), FocusLayer::Pane));
    }
}
