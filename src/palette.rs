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
/// Sorted by category: Navigation, File Ops, Editor, Preview, View / Layout, Appearance, System.
pub fn all_entries() -> Vec<PaletteEntry> {
    use crate::action::{Action, MenuId};
    use crate::config::ThemePreset;
    use crate::state::PaneLayout;

    vec![
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
            label: "Add bookmark for current directory",
            hint: "Ctrl+B",
            action: Action::AddBookmark,
        },
        PaletteEntry {
            category: "Navigation",
            label: "Open bookmarks",
            hint: "Ctrl+Shift+B",
            action: Action::OpenBookmarks,
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
        PaletteEntry {
            category: "Workspaces",
            label: "Switch to workspace 1",
            hint: "Ctrl+1",
            action: Action::SwitchToWorkspace(0),
        },
        PaletteEntry {
            category: "Workspaces",
            label: "Switch to workspace 2",
            hint: "Ctrl+2",
            action: Action::SwitchToWorkspace(1),
        },
        PaletteEntry {
            category: "Workspaces",
            label: "Switch to workspace 3",
            hint: "Ctrl+3",
            action: Action::SwitchToWorkspace(2),
        },
        PaletteEntry {
            category: "Workspaces",
            label: "Switch to workspace 4",
            hint: "Ctrl+4",
            action: Action::SwitchToWorkspace(3),
        },
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
            label: "Move file to trash",
            hint: "F8",
            action: Action::OpenDeletePrompt,
        },
        PaletteEntry {
            category: "File Ops",
            label: "Delete file permanently",
            hint: "Shift+F8",
            action: Action::OpenPermanentDeletePrompt,
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
        PaletteEntry {
            category: "File Ops",
            label: "Connect via SSH",
            hint: "ssh",
            action: Action::OpenSshConnect,
        },
        PaletteEntry {
            category: "System",
            label: "Open shell in current directory",
            hint: "F2",
            action: Action::OpenShell,
        },
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
            category: "Appearance",
            label: "Themes: Open selection menu",
            hint: "view > themes",
            action: Action::OpenMenu(MenuId::Themes),
        },
        PaletteEntry {
            category: "Appearance",
            label: "Theme: Zeta (default)",
            hint: "zeta",
            action: Action::SetTheme(ThemePreset::Zeta),
        },
        PaletteEntry {
            category: "Appearance",
            label: "Theme: Neon",
            hint: "neon",
            action: Action::SetTheme(ThemePreset::Neon),
        },
        PaletteEntry {
            category: "Appearance",
            label: "Theme: Monochrome (B/W)",
            hint: "monochrome",
            action: Action::SetTheme(ThemePreset::Monochrome),
        },
        PaletteEntry {
            category: "Appearance",
            label: "Theme: Matrix",
            hint: "matrix",
            action: Action::SetTheme(ThemePreset::Matrix),
        },
        PaletteEntry {
            category: "Appearance",
            label: "Theme: Norton Commander (classic)",
            hint: "norton",
            action: Action::SetTheme(ThemePreset::Norton),
        },
        PaletteEntry {
            category: "Appearance",
            label: "Theme: Fjord",
            hint: "fjord",
            action: Action::SetTheme(ThemePreset::Fjord),
        },
        PaletteEntry {
            category: "Appearance",
            label: "Theme: Sandbar",
            hint: "sandbar",
            action: Action::SetTheme(ThemePreset::Sandbar),
        },
        PaletteEntry {
            category: "Appearance",
            label: "Theme: Oxide",
            hint: "oxide",
            action: Action::SetTheme(ThemePreset::Oxide),
        },
        PaletteEntry {
            category: "Appearance",
            label: "Theme: Dracula",
            hint: "dracula",
            action: Action::SetTheme(ThemePreset::Dracula),
        },
        PaletteEntry {
            category: "System",
            label: "Command palette",
            hint: "Shift+P",
            action: Action::OpenCommandPalette,
        },
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
        "Workspaces" => 0,
        "Navigation" => 1,
        "File Ops" => 2,
        "Editor" => 3,
        "Preview" => 4,
        "View / Layout" => 5,
        "Appearance" => 6,
        "System" => 7,
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
    use crate::action::Action;

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

    #[test]
    fn workspace_entries_have_dedicated_category_and_sort_first() {
        let entries = all_entries();
        let filtered = filter_entries(&entries, "");

        assert_eq!(
            filtered.first().map(|entry| entry.category),
            Some("Workspaces")
        );
        assert!(filtered
            .iter()
            .take(4)
            .all(|entry| entry.category == "Workspaces"));
    }

    #[test]
    fn palette_includes_workspace_switch_entries() {
        let entries = all_entries();

        assert!(entries.iter().any(|entry| {
            entry.category == "Workspaces"
                && entry.label == "Switch to workspace 1"
                && entry.hint == "Ctrl+1"
                && entry.action == Action::SwitchToWorkspace(0)
        }));
        assert!(entries.iter().any(|entry| {
            entry.category == "Workspaces"
                && entry.label == "Switch to workspace 4"
                && entry.hint == "Ctrl+4"
                && entry.action == Action::SwitchToWorkspace(3)
        }));
    }
}
