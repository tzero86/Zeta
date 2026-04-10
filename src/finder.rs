use std::path::{Path, PathBuf};

use crate::pane::PaneId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileFinderState {
    pub pane: PaneId,
    pub root: PathBuf,
    pub query: String,
    pub selection: usize,
    pub all_entries: Vec<PathBuf>,
    pub filtered: Vec<PathBuf>,
}

impl FileFinderState {
    pub fn new(pane: PaneId, root: PathBuf) -> Self {
        Self {
            pane,
            root,
            query: String::new(),
            selection: 0,
            all_entries: Vec::new(),
            filtered: Vec::new(),
        }
    }

    pub fn set_results(&mut self, entries: Vec<PathBuf>) {
        self.all_entries = entries;
        self.refilter();
    }

    pub fn input(&mut self, ch: char) {
        self.query.push(ch);
        self.refilter();
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.refilter();
    }

    pub fn move_down(&mut self) {
        if !self.filtered.is_empty() {
            self.selection = (self.selection + 1).min(self.filtered.len() - 1);
        }
    }

    pub fn move_up(&mut self) {
        self.selection = self.selection.saturating_sub(1);
    }

    pub fn selected(&self) -> Option<&PathBuf> {
        self.filtered.get(self.selection)
    }

    pub fn relative_display_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .display()
            .to_string()
    }

    fn refilter(&mut self) {
        self.filtered = self
            .all_entries
            .iter()
            .filter(|path| {
                let candidate = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                subsequence_match(&self.query, candidate)
            })
            .cloned()
            .collect();
        self.selection = self.selection.min(self.filtered.len().saturating_sub(1));
    }
}

pub fn subsequence_match(query: &str, candidate: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let query = query.to_lowercase();
    let candidate = candidate.to_lowercase();
    let mut query_chars = query.chars();
    let mut next = query_chars.next();

    for ch in candidate.chars() {
        if Some(ch) == next {
            next = query_chars.next();
            if next.is_none() {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsequence_match_accepts_empty_query() {
        assert!(subsequence_match("", "Cargo.toml"));
    }

    #[test]
    fn subsequence_match_is_case_insensitive() {
        assert!(subsequence_match("rdm", "README.md"));
    }

    #[test]
    fn finder_state_filters_results() {
        let mut state = FileFinderState::new(PaneId::Left, PathBuf::from("."));
        state.set_results(vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("README.md"),
        ]);
        state.input('r');
        state.input('m');
        assert_eq!(state.filtered, vec![PathBuf::from("README.md")]);
    }
}
