/// A single entry in the command palette.
#[derive(Clone, Debug)]
pub struct PaletteEntry {
    pub label: &'static str,
    pub hint: &'static str,
    pub category: &'static str,
    pub action: crate::action::Action,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MatchKind {
    Prefix,
    Substring,
    Subsequence,
    NoMatch,
}

/// State for the command palette overlay.
#[derive(Clone, Debug)]
pub struct PaletteState {
    pub query: String,
    pub selection: usize,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self::new()
    }
}

impl PaletteState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            selection: 0,
        }
    }
}

/// Returns all palette entries in display order.
/// Sorted by category: Navigation, File Ops, Editor, Preview, View / Layout, System.
pub fn all_entries() -> Vec<PaletteEntry> {
    use crate::action::Action;
    use crate::config::ThemePreset;
    use crate::state::PaneLayout;

    vec![
        // Navigation
        PaletteEntry {
            category: "Navigation",
            label: "Open / enter selection",
            hint: "Enter",
            action: Action::EnterSelection,
        },
        PaletteEntry {
            category: "Navigation",
            label: "Go to parent directory",
            hint: "Backspace",
            action: Action::NavigateToParent,
        },
        PaletteEntry {
            category: "Navigation",
            label: "Navigate back",
            hint: "Alt+Left",
            action: Action::NavigateBack,
        },
        PaletteEntry {
            category: "Navigation",
            label: "Navigate forward",
            hint: "Alt+Right",
            action: Action::NavigateForward,
        },
        PaletteEntry {
            category: "Navigation",
            label: "Switch pane",
            hint: "Tab",
            action: Action::FocusNextPane,
        },
        PaletteEntry {
            category: "Navigation",
            label: "Refresh pane",
            hint: "r",
            action: Action::Refresh,
        },
        PaletteEntry {
            category: "Navigation",
            label: "Toggle hidden files",
            hint: "",
            action: Action::ToggleHiddenFiles,
        },
        PaletteEntry {
            category: "Navigation",
            label: "Cycle sort mode (name/size/date/ext)",
            hint: "s",
            action: Action::CycleSortMode,
        },
        PaletteEntry {
            category: "Navigation",
            label: "Toggle mark on selection",
            hint: "Space",
            action: Action::ToggleMark,
        },
        PaletteEntry {
            category: "Navigation",
            label: "Clear all marks",
            hint: "Shift+M",
            action: Action::ClearMarks,
        },
        // File operations
        PaletteEntry {
            category: "File Ops",
            label: "Copy file",
            hint: "F5",
            action: Action::OpenCopyPrompt,
        },
        PaletteEntry {
            category: "File Ops",
            label: "Move file",
            hint: "Shift+F6",
            action: Action::OpenMovePrompt,
        },
        PaletteEntry {
            category: "File Ops",
            label: "Rename file",
            hint: "F6",
            action: Action::OpenRenamePrompt,
        },
        PaletteEntry {
            category: "File Ops",
            label: "Delete file",
            hint: "F8",
            action: Action::OpenDeletePrompt,
        },
        PaletteEntry {
            category: "File Ops",
            label: "New file",
            hint: "Ins",
            action: Action::OpenNewFilePrompt,
        },
        PaletteEntry {
            category: "File Ops",
            label: "New directory",
            hint: "Shift+F7",
            action: Action::OpenNewDirectoryPrompt,
        },
        // Editor
        PaletteEntry {
            category: "Editor",
            label: "Open file in editor",
            hint: "F4",
            action: Action::OpenSelectedInEditor,
        },
        PaletteEntry {
            category: "Editor",
            label: "Save editor",
            hint: "Ctrl+S",
            action: Action::SaveEditor,
        },
        PaletteEntry {
            category: "Editor",
            label: "Discard editor changes",
            hint: "Ctrl+D",
            action: Action::DiscardEditorChanges,
        },
        PaletteEntry {
            category: "Editor",
            label: "Close editor",
            hint: "Esc",
            action: Action::CloseEditor,
        },
        // Preview
        PaletteEntry {
            category: "Preview",
            label: "Toggle preview panel",
            hint: "F3",
            action: Action::TogglePreviewPanel,
        },
        PaletteEntry {
            category: "Preview",
            label: "Cycle focus (left / right / preview)",
            hint: "Ctrl+W",
            action: Action::CycleFocus,
        },
        // View / Layout
        PaletteEntry {
            category: "View / Layout",
            label: "Open settings panel",
            hint: "Ctrl+O",
            action: Action::OpenSettingsPanel,
        },
        PaletteEntry {
            category: "View / Layout",
            label: "Layout: side by side",
            hint: "",
            action: Action::SetPaneLayout(PaneLayout::SideBySide),
        },
        PaletteEntry {
            category: "View / Layout",
            label: "Layout: stacked",
            hint: "",
            action: Action::SetPaneLayout(PaneLayout::Stacked),
        },
        PaletteEntry {
            category: "View / Layout",
            label: "Theme: fjord",
            hint: "",
            action: Action::SetTheme(ThemePreset::Fjord),
        },
        PaletteEntry {
            category: "View / Layout",
            label: "Theme: sandbar",
            hint: "",
            action: Action::SetTheme(ThemePreset::Sandbar),
        },
        PaletteEntry {
            category: "View / Layout",
            label: "Theme: oxide",
            hint: "",
            action: Action::SetTheme(ThemePreset::Oxide),
        },
        // System
        PaletteEntry {
            category: "System",
            label: "Help",
            hint: "F1",
            action: Action::OpenHelpDialog,
        },
        PaletteEntry {
            category: "System",
            label: "About Zeta",
            hint: "",
            action: Action::OpenAboutDialog,
        },
        PaletteEntry {
            category: "System",
            label: "Quit",
            hint: "Ctrl+Q",
            action: Action::Quit,
        },
    ]
}

pub fn match_kind(label: &str, query: &str) -> MatchKind {
    if query.is_empty() {
        return MatchKind::Prefix;
    }

    let query_lower = query.to_lowercase();
    let label_lower = label.to_lowercase();

    if label_lower.starts_with(&query_lower) {
        MatchKind::Prefix
    } else if label_lower.contains(&query_lower) {
        MatchKind::Substring
    } else if is_subsequence(&label_lower, &query_lower) {
        MatchKind::Subsequence
    } else {
        MatchKind::NoMatch
    }
}

fn is_subsequence(label: &str, query: &str) -> bool {
    let mut query_chars = query.chars();
    let mut next_query = query_chars.next();

    for ch in label.chars() {
        if Some(ch) == next_query {
            next_query = query_chars.next();
            if next_query.is_none() {
                return true;
            }
        }
    }

    query.is_empty() || next_query.is_none()
}

fn category_order(category: &str) -> usize {
    match category {
        "Navigation" => 0,
        "File Ops" => 1,
        "Editor" => 2,
        "Preview" => 3,
        "View / Layout" => 4,
        "System" => 5,
        _ => usize::MAX,
    }
}

/// Returns entries matching `query`, ordered by match quality first.
/// Prefix matches come before substring matches, then subsequence matches.
pub fn filter_entries<'a>(entries: &'a [PaletteEntry], query: &str) -> Vec<&'a PaletteEntry> {
    let mut matched: Vec<(MatchKind, usize, usize, &'a PaletteEntry)> = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let kind = match_kind(entry.label, query);
            (kind != MatchKind::NoMatch).then_some((
                kind,
                category_order(entry.category),
                index,
                entry,
            ))
        })
        .collect();

    matched.sort_by_key(|(kind, category, index, _)| (*kind, *category, *index));
    matched.into_iter().map(|(_, _, _, entry)| entry).collect()
}

#[cfg(test)]
mod tests {
    use super::{all_entries, filter_entries, match_kind, MatchKind};

    #[test]
    fn filter_empty_query_returns_all() {
        let entries = all_entries();
        let filtered = filter_entries(&entries, "");
        assert_eq!(filtered.len(), entries.len());
    }

    #[test]
    fn filter_subsequence_matches_label() {
        let entries = all_entries();
        // "quit" should match "Quit"
        let filtered = filter_entries(&entries, "quit");
        assert!(filtered.iter().any(|e| e.label == "Quit"));
    }

    #[test]
    fn filter_subsequence_no_match_returns_empty() {
        let entries = all_entries();
        let filtered = filter_entries(&entries, "zzzzzzzzzzzzzzz");
        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_case_insensitive() {
        let entries = all_entries();
        let lower = filter_entries(&entries, "copy");
        let upper = filter_entries(&entries, "COPY");
        assert_eq!(lower.len(), upper.len());
    }

    #[test]
    fn prefix_matches_rank_before_substring() {
        assert_eq!(match_kind("Copy file", "co"), MatchKind::Prefix);
        assert_eq!(
            match_kind("Open / enter selection", "enter"),
            MatchKind::Substring
        );
    }
}
