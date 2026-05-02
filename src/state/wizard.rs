use crate::config::ThemePreset;

/// All available themes shown in the wizard, in display order.
/// Each tuple is (display label, ThemePreset).
pub const WIZARD_THEMES: &[(&str, ThemePreset)] = &[
    ("Zeta (default dark)", ThemePreset::Zeta),
    ("Catppuccin Mocha", ThemePreset::CatppuccinMocha),
    ("Dracula", ThemePreset::Dracula),
    ("Fjord", ThemePreset::Fjord),
    ("Matrix", ThemePreset::Matrix),
    ("Monochrome", ThemePreset::Monochrome),
    ("Neon", ThemePreset::Neon),
    ("Norton (classic)", ThemePreset::Norton),
    ("Oxide", ThemePreset::Oxide),
    ("Sandbar", ThemePreset::Sandbar),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WizardStep {
    ThemePicker,
    Cheatsheet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WizardState {
    pub step: WizardStep,
    /// Index into `WIZARD_THEMES`.
    pub theme_selection: usize,
    /// Scroll offset for the cheatsheet page.
    pub cheatsheet_scroll: usize,
}

impl WizardState {
    pub fn new() -> Self {
        Self {
            step: WizardStep::ThemePicker,
            theme_selection: 0,
            cheatsheet_scroll: 0,
        }
    }

    /// Advance from ThemePicker → Cheatsheet (no-op on Cheatsheet).
    pub fn advance(&mut self) {
        if self.step == WizardStep::ThemePicker {
            self.step = WizardStep::Cheatsheet;
        }
    }

    pub fn move_down(&mut self) {
        match self.step {
            WizardStep::ThemePicker => {
                if self.theme_selection + 1 < WIZARD_THEMES.len() {
                    self.theme_selection += 1;
                }
            }
            WizardStep::Cheatsheet => {
                self.cheatsheet_scroll = self.cheatsheet_scroll.saturating_add(1);
            }
        }
    }

    pub fn move_up(&mut self) {
        match self.step {
            WizardStep::ThemePicker => {
                self.theme_selection = self.theme_selection.saturating_sub(1);
            }
            WizardStep::Cheatsheet => {
                self.cheatsheet_scroll = self.cheatsheet_scroll.saturating_sub(1);
            }
        }
    }

    /// The theme preset currently highlighted.
    pub fn selected_preset(&self) -> ThemePreset {
        WIZARD_THEMES[self.theme_selection].1
    }
}

impl Default for WizardState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wizard_starts_on_theme_picker() {
        let w = WizardState::new();
        assert_eq!(w.step, WizardStep::ThemePicker);
        assert_eq!(w.theme_selection, 0);
    }

    #[test]
    fn wizard_advance_goes_to_cheatsheet() {
        let mut w = WizardState::new();
        w.advance();
        assert_eq!(w.step, WizardStep::Cheatsheet);
    }

    #[test]
    fn wizard_theme_clamps_to_last() {
        let mut w = WizardState::new();
        w.move_up(); // no panic at 0
        assert_eq!(w.theme_selection, 0);
        for _ in 0..20 {
            w.move_down();
        }
        assert!(w.theme_selection < WIZARD_THEMES.len());
    }

    #[test]
    fn wizard_selected_preset_matches_list() {
        let w = WizardState::new();
        assert_eq!(w.selected_preset(), WIZARD_THEMES[0].1);
    }
}
