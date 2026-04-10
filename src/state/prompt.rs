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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptState {
    pub kind: PromptKind,
    pub title: &'static str,
    pub base_path: PathBuf,
    pub source_path: Option<PathBuf>,
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
