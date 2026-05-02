use crate::action::Action;
use crate::fs::FileSystemError;
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
    /// Inline rename is active on the focused pane.
    PaneInlineRename,
    /// The editor panel is focused.
    Editor,
    /// The preview panel is focused.
    Preview,
    /// Keyboard focus is on the markdown preview split within the editor panel.
    MarkdownPreview,
    /// The embedded terminal emulator is focused.
    Terminal,
    /// The git diff viewer file list pane has focus.
    GitDiffFileList,
    /// The git diff viewer diff content pane has focus.
    GitDiffContent,
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
    SshTrustPrompt,
    OpenWith,
    DestructiveConfirm,
}

/// Structured application-level error type.
///
/// Wraps lower-level errors with a short context string so that user-facing
/// messages can explain *what* the app was trying to do when the error
/// occurred, not just what the OS says.
///
/// Use the constructor helpers rather than constructing variants directly.
#[derive(Debug, thiserror::Error)]
pub enum ZetaError {
    #[error("{context}: {source}")]
    Fs {
        context: String,
        source: FileSystemError,
    },
    #[error("{context}: {source}")]
    Io {
        context: String,
        source: std::io::Error,
    },
    #[error("{message}")]
    Other { message: String },
}

impl ZetaError {
    pub fn fs(source: FileSystemError, context: impl Into<String>) -> Self {
        Self::Fs {
            context: context.into(),
            source,
        }
    }

    pub fn io(source: std::io::Error, context: impl Into<String>) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
        }
    }
}

/// Severity of a status bar message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageKind {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

/// A status bar message with optional severity for colored display.
#[derive(Debug, Clone, Default)]
pub struct StatusMessage {
    pub text: String,
    pub kind: MessageKind,
}

impl StatusMessage {
    pub fn info(text: impl Into<String>) -> Self {
        Self { text: text.into(), kind: MessageKind::Info }
    }

    pub fn success(text: impl Into<String>) -> Self {
        Self { text: text.into(), kind: MessageKind::Success }
    }

    pub fn warning(text: impl Into<String>) -> Self {
        Self { text: text.into(), kind: MessageKind::Warning }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self { text: text.into(), kind: MessageKind::Error }
    }
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

    #[test]
    fn zeta_error_fs_displays_context() {
        let source = FileSystemError::Other {
            message: "permission denied".into(),
        };
        let err = ZetaError::fs(source, "directory scan failed");
        assert!(err.to_string().contains("directory scan failed"));
        assert!(err.to_string().contains("permission denied"));
    }

    #[test]
    fn zeta_error_other_displays_message() {
        let err = ZetaError::other("something went wrong");
        assert_eq!(err.to_string(), "something went wrong");
    }

    #[test]
    fn status_message_default_is_empty_info() {
        let m = StatusMessage::default();
        assert_eq!(m.text, "");
        assert!(matches!(m.kind, MessageKind::Info));
    }

    #[test]
    fn status_message_error_constructor() {
        let m = StatusMessage::error("disk full");
        assert_eq!(m.text, "disk full");
        assert!(matches!(m.kind, MessageKind::Error));
    }

    #[test]
    fn status_message_warning_constructor() {
        let m = StatusMessage::warning("read-only");
        assert!(matches!(m.kind, MessageKind::Warning));
    }
}
