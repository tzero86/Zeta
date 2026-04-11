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
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FocusLayer {
    #[default]
    Pane,
    /// One-line quick-filter input is active for the focused pane.
    PaneFilter,
    /// The editor panel is focused.
    Editor,
    /// The preview panel is focused.
    Preview,
    /// Keyboard focus is on the markdown preview split within the editor panel.
    MarkdownPreview,
    /// A modal overlay is open; only modal-specific keys are processed.
    Modal(ModalKind),
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
    Bookmarks,
    FileFinder,
    SshConnect,
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
