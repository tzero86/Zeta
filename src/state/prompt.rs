use std::path::Path;
use std::path::PathBuf;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptState {
    pub kind: PromptKind,
    pub title: &'static str,
    pub base_path: PathBuf,
    pub source_path: Option<PathBuf>,
    /// For batch operations: the full set of source paths. Empty in single-file mode.
    pub source_paths: Vec<PathBuf>,
    pub value: String,
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
            value,
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
