use std::path::Path;
use std::path::PathBuf;

use tui_input::Input;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromptKind {
    Copy,
    Trash,
    Delete,
    Move,
    NewDirectory,
    NewFile,
    Rename,
    /// Navigate the active pane to a typed path.
    GoTo,
    /// Rename all marked files using a pattern ({n}, {name}, {ext}).
    BulkRename,
}

#[derive(Clone, Debug)]
pub struct PromptState {
    pub kind: PromptKind,
    pub title: &'static str,
    pub base_path: PathBuf,
    pub source_path: Option<PathBuf>,
    /// For batch operations: the full set of source paths. Empty in single-file mode.
    pub source_paths: Vec<PathBuf>,
    /// The editable input field (tracks text + cursor position).
    pub input: Input,
}

impl PartialEq for PromptState {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.title == other.title
            && self.base_path == other.base_path
            && self.source_path == other.source_path
            && self.source_paths == other.source_paths
            && self.input.value() == other.input.value()
    }
}

impl Eq for PromptState {}

impl PromptState {
    /// The current text value of the input field.
    pub fn value(&self) -> &str {
        self.input.value()
    }
}

impl PromptKind {
    pub fn is_confirmation_only(self) -> bool {
        matches!(self, Self::Trash | Self::Delete)
    }
}

impl PromptState {
    pub fn new(kind: PromptKind, title: &'static str, base_path: PathBuf) -> Self {
        Self::with_value(kind, title, base_path, None, String::new())
    }

    pub fn with_value(
        kind: PromptKind,
        title: &'static str,
        base_path: PathBuf,
        source_path: Option<PathBuf>,
        value: String,
    ) -> Self {
        Self {
            kind,
            title,
            base_path,
            source_path,
            source_paths: Vec::new(),
            input: Input::from(value),
        }
    }
}

pub fn resolve_prompt_target(prompt: &PromptState, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        prompt.base_path.join(path)
    }
}

pub(crate) fn prompt_base_path(path: &Path) -> PathBuf {
    path.parent().map(Path::to_path_buf).unwrap_or_default()
}
