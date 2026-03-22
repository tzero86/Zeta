use crate::fs::{EntryInfo, EntryKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreviewContent {
    Empty,
    Text(String),
    Binary { bytes: usize },
}

pub fn preview_summary(entry: Option<&EntryInfo>) -> PreviewContent {
    match entry {
        Some(item) if item.kind == EntryKind::Directory => {
            PreviewContent::Text(format!("directory: {}", item.name))
        }
        Some(item) if item.kind == EntryKind::File => {
            PreviewContent::Text(format!("file: {}", item.name))
        }
        Some(item) => PreviewContent::Text(format!("item: {}", item.name)),
        None => PreviewContent::Empty,
    }
}
