/// A single entry in the command palette.
#[derive(Clone, Debug)]
pub struct PaletteEntry {
    pub label: &'static str,
    pub hint: &'static str,
    pub action: crate::action::Action,
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
            label: "Open / enter selection",
            hint: "Enter",
            action: Action::EnterSelection,
        },
        PaletteEntry {
            label: "Go to parent directory",
            hint: "Backspace",
            action: Action::NavigateToParent,
        },
        PaletteEntry {
            label: "Navigate back",
            hint: "Alt+Left",
            action: Action::NavigateBack,
        },
        PaletteEntry {
            label: "Navigate forward",
            hint: "Alt+Right",
            action: Action::NavigateForward,
        },
        PaletteEntry {
            label: "Switch pane",
            hint: "Tab",
            action: Action::FocusNextPane,
        },
        PaletteEntry {
            label: "Refresh pane",
            hint: "r",
            action: Action::Refresh,
        },
        PaletteEntry {
            label: "Toggle hidden files",
            hint: "",
            action: Action::ToggleHiddenFiles,
        },
        PaletteEntry {
            label: "Cycle sort mode (name/size/date/ext)",
            hint: "s",
            action: Action::CycleSortMode,
        },
        PaletteEntry {
            label: "Toggle mark on selection",
            hint: "Space",
            action: Action::ToggleMark,
        },
        PaletteEntry {
            label: "Clear all marks",
            hint: "Shift+M",
            action: Action::ClearMarks,
        },
        // File operations
        PaletteEntry {
            label: "Copy file",
            hint: "F5",
            action: Action::OpenCopyPrompt,
        },
        PaletteEntry {
            label: "Move file",
            hint: "Shift+F6",
            action: Action::OpenMovePrompt,
        },
        PaletteEntry {
            label: "Rename file",
            hint: "F6",
            action: Action::OpenRenamePrompt,
        },
        PaletteEntry {
            label: "Delete file",
            hint: "F8",
            action: Action::OpenDeletePrompt,
        },
        PaletteEntry {
            label: "New file",
            hint: "Ins",
            action: Action::OpenNewFilePrompt,
        },
        PaletteEntry {
            label: "New directory",
            hint: "Shift+F7",
            action: Action::OpenNewDirectoryPrompt,
        },
        // Editor
        PaletteEntry {
            label: "Open file in editor",
            hint: "F4",
            action: Action::OpenSelectedInEditor,
        },
        PaletteEntry {
            label: "Save editor",
            hint: "Ctrl+S",
            action: Action::SaveEditor,
        },
        PaletteEntry {
            label: "Discard editor changes",
            hint: "Ctrl+D",
            action: Action::DiscardEditorChanges,
        },
        PaletteEntry {
            label: "Close editor",
            hint: "Esc",
            action: Action::CloseEditor,
        },
        // Preview
        PaletteEntry {
            label: "Toggle preview panel",
            hint: "F3",
            action: Action::TogglePreviewPanel,
        },
        PaletteEntry {
            label: "Focus preview panel",
            hint: "Shift+F3",
            action: Action::FocusPreviewPanel,
        },
        // View / Layout
        PaletteEntry {
            label: "Layout: side by side",
            hint: "",
            action: Action::SetPaneLayout(PaneLayout::SideBySide),
        },
        PaletteEntry {
            label: "Layout: stacked",
            hint: "",
            action: Action::SetPaneLayout(PaneLayout::Stacked),
        },
        PaletteEntry {
            label: "Theme: fjord",
            hint: "",
            action: Action::SetTheme(ThemePreset::Fjord),
        },
        PaletteEntry {
            label: "Theme: sandbar",
            hint: "",
            action: Action::SetTheme(ThemePreset::Sandbar),
        },
        PaletteEntry {
            label: "Theme: oxide",
            hint: "",
            action: Action::SetTheme(ThemePreset::Oxide),
        },
        // System
        PaletteEntry {
            label: "Help",
            hint: "F1",
            action: Action::OpenHelpDialog,
        },
        PaletteEntry {
            label: "About Zeta",
            hint: "",
            action: Action::OpenAboutDialog,
        },
        PaletteEntry {
            label: "Quit",
            hint: "Ctrl+Q",
            action: Action::Quit,
        },
    ]
}

/// Returns entries whose label contains all chars of `query` in order
/// (case-insensitive subsequence match).
pub fn filter_entries<'a>(entries: &'a [PaletteEntry], query: &str) -> Vec<&'a PaletteEntry> {
    if query.is_empty() {
        return entries.iter().collect();
    }
    let q: Vec<char> = query.to_lowercase().chars().collect();
    entries
        .iter()
        .filter(|e| {
            let label = e.label.to_lowercase();
            let mut qi = 0;
            for ch in label.chars() {
                if qi < q.len() && ch == q[qi] {
                    qi += 1;
                }
            }
            qi == q.len()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{all_entries, filter_entries};

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
}
