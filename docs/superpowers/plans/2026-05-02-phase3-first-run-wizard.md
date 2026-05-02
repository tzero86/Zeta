# Phase 3 — First-Run Wizard + Annotated Config

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When Zeta launches for the first time (no config file exists), show a two-step TUI wizard that lets the user pick a theme and then shows a keyboard cheatsheet, writing a fully-annotated `config.toml` on completion.

**Architecture:**
- `ConfigSource::Default` (returned by `AppConfig::load_default_location()` when no file exists) is the trigger. `AppState::bootstrap()` detects it and sets `show_wizard: bool = true`. `initial_commands()` opens the `FirstRunWizard` modal by directly setting `self.overlay.modal` before returning. The wizard is a full-screen modal overlay; the normal pane UI renders underneath (dim).
- The wizard has two sequential steps: `ThemePicker` (live preview as you scroll the list, Enter confirms) → `Cheatsheet` (scrollable key reference, Enter/Esc finishes). On finish the selected theme is applied, an annotated config is written to disk, and the modal is closed.
- Annotated config generation is a dedicated `fn generate_annotated_config(config: &AppConfig) -> String` in `src/config.rs` that builds TOML text with `#` comment lines by hand (avoiding `basic_toml`'s comment-free output).

**Tech Stack:** Rust stable, ratatui 0.30, crossterm, existing `ModalState`/`OverlayState` patterns, `basic_toml` (already in `Cargo.toml`).

---

## File Map

| File | Change |
|------|--------|
| `src/state/wizard.rs` | **CREATE** — `WizardStep`, `WizardState` |
| `src/state/mod.rs` | `show_wizard` field; open modal in `initial_commands()`; handle `WizardAction*` actions; `apply_wizard()` handler |
| `src/state/overlay.rs` | Add `ModalState::FirstRunWizard(WizardState)` |
| `src/state/types.rs` | Add `ModalKind::FirstRunWizard` |
| `src/action.rs` | Add `Action::WizardMoveDown`, `WizardMoveUp`, `WizardConfirm`, `WizardClose`; `fn from_wizard_key_event()` |
| `src/app.rs` | Route `FocusLayer::Modal(ModalKind::FirstRunWizard)` → `from_wizard_key_event()` |
| `src/config.rs` | Add `fn generate_annotated_config(config: &AppConfig) -> String` |
| `src/ui/wizard.rs` | **CREATE** — `render_first_run_wizard()` |
| `src/ui/mod.rs` | Call `render_first_run_wizard()` when modal is `FirstRunWizard` |
| `tests/wizard_integration.rs` | **CREATE** — integration tests |

---

## Task 1 — `WizardState` in `src/state/wizard.rs`

**Files:**
- Create: `src/state/wizard.rs`

- [ ] **Step 1: Write the failing test**

```rust
// src/state/wizard.rs (bottom of file, inside #[cfg(test)] mod tests)
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
```

- [ ] **Step 2: Run test to verify it fails**

```
cargo test --lib wizard -- --nocapture
```
Expected: compile error — `WizardState` not defined.

- [ ] **Step 3: Create `src/state/wizard.rs`**

```rust
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
        w.move_up();
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
```

- [ ] **Step 4: Run tests to verify they pass**

```
cargo test --lib wizard -- --nocapture
```
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/state/wizard.rs
git commit -m "feat(wizard): add WizardState and WizardStep"
```

---

## Task 2 — Wire `WizardState` into `ModalState` and `ModalKind`

**Files:**
- Modify: `src/state/overlay.rs`
- Modify: `src/state/types.rs`

- [ ] **Step 1: Write failing test** (compile-time — just verify variant missing)

```
cargo test --lib overlay -- --nocapture
```
Expected: compiles and passes (baseline check before changes).

- [ ] **Step 2: Add `FirstRunWizard` to `ModalState` in `src/state/overlay.rs`**

At the top of the file, add the import:
```rust
use crate::state::wizard::WizardState;
```

Inside `pub enum ModalState { ... }`, add after the last variant:
```rust
    FirstRunWizard(WizardState),
```

- [ ] **Step 3: Add `FirstRunWizard` to `ModalKind` in `src/state/types.rs`**

Inside `pub enum ModalKind { ... }`, add:
```rust
    FirstRunWizard,
```

- [ ] **Step 4: Add `ModalKind` mapping to `OverlayState` in `src/state/overlay.rs`**

Find the method `pub fn modal_kind(&self) -> Option<ModalKind>` (or similar). If it is a `match` over `ModalState`, add the arm:
```rust
Some(ModalState::FirstRunWizard(_)) => Some(ModalKind::FirstRunWizard),
```

Search for existing pattern: `Some(ModalState::Settings(_)) => Some(ModalKind::Settings)` and add the new arm in the same block.

- [ ] **Step 5: Add `wizard_state()` and `wizard_state_mut()` accessors to `OverlayState`**

```rust
pub fn wizard_state(&self) -> Option<&WizardState> {
    if let Some(ModalState::FirstRunWizard(s)) = &self.modal {
        Some(s)
    } else {
        None
    }
}

pub fn wizard_state_mut(&mut self) -> Option<&mut WizardState> {
    if let Some(ModalState::FirstRunWizard(s)) = &mut self.modal {
        Some(s)
    } else {
        None
    }
}
```

- [ ] **Step 6: `cargo check` passes**

```
cargo check
```
Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add src/state/overlay.rs src/state/types.rs
git commit -m "feat(wizard): add FirstRunWizard modal variant and ModalKind"
```

---

## Task 3 — Wizard actions in `src/action.rs`

**Files:**
- Modify: `src/action.rs`

- [ ] **Step 1: Write the failing test**

```rust
// Add to the existing test module in src/action.rs
#[test]
fn wizard_key_event_up_maps_to_move_up() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(Action::from_wizard_key_event(key), Some(Action::WizardMoveUp));
}

#[test]
fn wizard_key_event_enter_maps_to_confirm() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(Action::from_wizard_key_event(key), Some(Action::WizardConfirm));
}
```

- [ ] **Step 2: Run test to verify it fails**

```
cargo test --lib action::tests::wizard -- --nocapture
```
Expected: compile error — variants not defined.

- [ ] **Step 3: Add wizard action variants to `pub enum Action`**

Inside `pub enum Action { ... }`, add:
```rust
    WizardMoveDown,
    WizardMoveUp,
    /// On ThemePicker: advance to cheatsheet. On Cheatsheet: finish wizard.
    WizardConfirm,
    /// Close wizard without writing config (Esc).
    WizardClose,
```

- [ ] **Step 4: Add `fn from_wizard_key_event`**

After the existing `from_*_key_event` functions, add:

```rust
pub fn from_wizard_key_event(key_event: KeyEvent) -> Option<Self> {
    use crossterm::event::{KeyCode, KeyModifiers};
    match key_event.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Self::WizardMoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Self::WizardMoveDown),
        KeyCode::Enter => Some(Self::WizardConfirm),
        KeyCode::Esc => Some(Self::WizardClose),
        _ => None,
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

```
cargo test --lib action::tests::wizard -- --nocapture
```
Expected: 2 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/action.rs
git commit -m "feat(wizard): add wizard action variants and key handler"
```

---

## Task 4 — Route wizard keys in `src/app.rs`

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Baseline check**

```
cargo check
```
Expected: clean (before change).

- [ ] **Step 2: Add `FocusLayer::Modal(ModalKind::FirstRunWizard)` arm**

Find the `match focus {` block in `fn action_from_key_event(...)` around line 816. It has arms like:
```rust
FocusLayer::Modal(ModalKind::Settings) => { ... }
```

Add after the last `FocusLayer::Modal(...)` arm:
```rust
FocusLayer::Modal(ModalKind::FirstRunWizard) => {
    Action::from_wizard_key_event(key_event)
}
```

- [ ] **Step 3: `cargo check` passes**

```
cargo check
```
Expected: no errors or warnings about non-exhaustive match.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs
git commit -m "feat(wizard): route FirstRunWizard keys to from_wizard_key_event"
```

---

## Task 5 — State handling: `show_wizard`, `initial_commands`, `apply_wizard`

**Files:**
- Modify: `src/state/mod.rs`

- [ ] **Step 1: Write the failing tests**

```rust
// Add to test module in src/state/mod.rs

#[test]
fn bootstrap_with_default_config_sets_show_wizard() {
    use crate::config::{AppConfig, ConfigSource, LoadedConfig};
    use std::time::Instant;
    let loaded = LoadedConfig {
        config: AppConfig::default(),
        path: PathBuf::from(""),
        source: ConfigSource::Default,
    };
    let state = AppState::bootstrap(loaded, Instant::now()).unwrap();
    assert!(state.show_wizard);
}

#[test]
fn bootstrap_with_file_config_does_not_set_show_wizard() {
    use crate::config::{AppConfig, ConfigSource, LoadedConfig};
    use std::time::Instant;
    let loaded = LoadedConfig {
        config: AppConfig::default(),
        path: PathBuf::from(""),
        source: ConfigSource::File,
    };
    let state = AppState::bootstrap(loaded, Instant::now()).unwrap();
    assert!(!state.show_wizard);
}
```

- [ ] **Step 2: Run to verify they fail**

```
cargo test --lib state::tests::bootstrap_with_default -- --nocapture
```
Expected: compile error — `show_wizard` field not defined.

- [ ] **Step 3: Add `show_wizard` field to `AppState`**

Find the `pub struct AppState {` definition. The struct fields are grouped with a comment `// Shared config/theme/runtime shell state.` around line 249. Add after `debug`:
```rust
    /// `true` when no config file was found on startup; opens the first-run wizard.
    show_wizard: bool,
```

- [ ] **Step 4: Initialize `show_wizard` in `AppState::bootstrap`**

In the `Ok(Self { ... })` block, add:
```rust
    show_wizard: loaded_config.source == ConfigSource::Default,
```

Also add the import at the top of `src/state/mod.rs` (it is already used but confirm `ConfigSource` is in scope — it comes from `use crate::config::{..., ConfigSource, ...}`):
```rust
use crate::config::{
    AppConfig, ConfigSource, EditorConfig, IconMode, LoadedConfig, PaneLayout, ResolvedTheme,
    RuntimeKeymap, ThemePalette, ThemePreset,
};
```
(Check what's already imported and just add `ConfigSource` if missing.)

- [ ] **Step 5: Open wizard modal in `initial_commands`**

Find `pub fn initial_commands(&mut self) -> Vec<Command>`. Before the `commands` vec is returned, add:
```rust
if self.show_wizard {
    use crate::state::wizard::WizardState;
    self.overlay.modal = Some(crate::state::overlay::ModalState::FirstRunWizard(
        WizardState::new(),
    ));
}
```

- [ ] **Step 6: Add `apply_wizard` private handler**

After the last `fn apply_*` handler (before `pub fn apply_job_result`), add:

```rust
/// Handles WizardMoveDown, WizardMoveUp, WizardConfirm, WizardClose.
fn apply_wizard(&mut self, action: &Action) -> Result<Vec<Command>> {
    match action {
        Action::WizardMoveDown => {
            if let Some(w) = self.overlay.wizard_state_mut() {
                w.move_down();
                // Live theme preview on ThemePicker step.
                if w.step == crate::state::wizard::WizardStep::ThemePicker {
                    let preset = w.selected_preset();
                    self.theme = ThemePalette::from_preset(preset);
                }
            }
        }
        Action::WizardMoveUp => {
            if let Some(w) = self.overlay.wizard_state_mut() {
                w.move_up();
                if w.step == crate::state::wizard::WizardStep::ThemePicker {
                    let preset = w.selected_preset();
                    self.theme = ThemePalette::from_preset(preset);
                }
            }
        }
        Action::WizardConfirm => {
            let step = self.overlay.wizard_state().map(|w| w.step);
            match step {
                Some(crate::state::wizard::WizardStep::ThemePicker) => {
                    if let Some(w) = self.overlay.wizard_state_mut() {
                        w.advance();
                    }
                }
                Some(crate::state::wizard::WizardStep::Cheatsheet) => {
                    self.finish_wizard();
                }
                None => {}
            }
        }
        Action::WizardClose => {
            self.finish_wizard();
        }
        _ => {}
    }
    Ok(vec![])
}

fn finish_wizard(&mut self) {
    // Commit theme choice to config.
    if let Some(preset) = self.overlay.wizard_state().map(|w| w.selected_preset()) {
        self.theme = ThemePalette::from_preset(preset);
        self.config.theme.preset = preset.as_str().to_string();
    }
    self.overlay.close_all();
    self.show_wizard = false;

    // Write annotated config to disk.
    let path_str = self.config_path.clone();
    let path = std::path::PathBuf::from(&path_str);
    if !path_str.is_empty() {
        let text = crate::config::generate_annotated_config(&self.config);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        match std::fs::write(&path, text) {
            Ok(()) => self.set_status_success(format!(
                "Welcome to Zeta! Config saved to {}",
                path.display()
            )),
            Err(e) => self.set_status_error(format!("could not write config: {e}")),
        }
    }
}
```

- [ ] **Step 7: Dispatch wizard actions from `apply_view`**

In `pub fn apply_view`, find the dispatch section that calls the handler methods. Add a guard before the main match (similar to how `apply_git_diff` is guarded):

```rust
if matches!(
    action,
    Action::WizardMoveDown
        | Action::WizardMoveUp
        | Action::WizardConfirm
        | Action::WizardClose
) {
    return self.apply_wizard(&action);
}
```

- [ ] **Step 8: Run tests**

```
cargo test --lib state::tests::bootstrap_with_default -- --nocapture
cargo test --lib state::tests::bootstrap_with_file -- --nocapture
```
Expected: both pass.

- [ ] **Step 9: Commit**

```bash
git add src/state/mod.rs
git commit -m "feat(wizard): wire show_wizard field, initial_commands trigger, apply_wizard handler"
```

---

## Task 6 — `generate_annotated_config` in `src/config.rs`

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Write the failing tests**

```rust
// Add to test module in src/config.rs (or create one)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotated_config_contains_section_headers() {
        let cfg = AppConfig::default();
        let text = generate_annotated_config(&cfg);
        assert!(text.contains("[theme]"), "missing [theme] section");
        assert!(text.contains("[keymap]"), "missing [keymap] section");
        assert!(text.contains("[editor]"), "missing [editor] section");
    }

    #[test]
    fn annotated_config_contains_comments() {
        let cfg = AppConfig::default();
        let text = generate_annotated_config(&cfg);
        assert!(text.contains("# "), "expected at least one comment line");
    }

    #[test]
    fn annotated_config_theme_preset_round_trips() {
        let mut cfg = AppConfig::default();
        cfg.theme.preset = "dracula".to_string();
        let text = generate_annotated_config(&cfg);
        assert!(text.contains("preset = \"dracula\""));
    }

    #[test]
    fn annotated_config_is_valid_toml() {
        let cfg = AppConfig::default();
        let text = generate_annotated_config(&cfg);
        let result: Result<AppConfig, _> = basic_toml::from_str(&text);
        assert!(result.is_ok(), "generated config is not valid TOML: {result:?}");
    }
}
```

- [ ] **Step 2: Run to verify they fail**

```
cargo test --lib config::tests -- --nocapture
```
Expected: compile error — `generate_annotated_config` not defined.

- [ ] **Step 3: Add `pub fn generate_annotated_config` to `src/config.rs`**

Add after `AppConfig::save`:

```rust
/// Build a human-readable, fully-commented TOML string for `config`.
///
/// This is used by the first-run wizard to write an annotated `config.toml`
/// that teaches the user what every field does.  Unlike `basic_toml::to_string`,
/// this function adds inline `#` doc comments above each setting.
pub fn generate_annotated_config(config: &AppConfig) -> String {
    let icon_mode = match config.icon_mode {
        IconMode::Unicode => "unicode",
        IconMode::Ascii => "ascii",
        IconMode::NerdFont => "nerd_font",
    };
    let pane_layout = match config.pane_layout {
        PaneLayout::Equal => "equal",
        PaneLayout::LeftWide => "left_wide",
        PaneLayout::RightWide => "right_wide",
    };

    let mut openers = String::new();
    for opener in &config.openers {
        let exts: Vec<String> = opener.extensions.iter().map(|e| format!("{e:?}")).collect();
        let exts_str = format!("[{}]", exts.join(", "));
        openers.push_str(&format!(
            "\n[[openers]]\nname = {:?}\ncommand = {:?}\nextensions = {}\n",
            opener.name, opener.command, exts_str
        ));
    }

    format!(
        r#"# Zeta configuration file
# Documentation: https://github.com/tzero86/zeta
# Changes take effect immediately (live reload on save).

# Icon style: "unicode" | "ascii" | "nerd_font"
# "nerd_font" requires a Nerd Font in your terminal.
icon_mode = "{icon_mode}"

# Default pane split: "equal" | "left_wide" | "right_wide"
pane_layout = "{pane_layout}"

# Open the preview panel on startup.
preview_panel_open = {preview_panel_open}

# Auto-preview the highlighted file (requires preview panel open).
preview_on_selection = {preview_on_selection}

# Open an embedded terminal pane on startup.
terminal_open_by_default = {terminal_open_by_default}

# Check GitHub releases for updates when Zeta starts.
check_updates_on_startup = {check_updates_on_startup}

[theme]
# Theme preset. Available: zeta | catppuccin_mocha | dracula | fjord |
#   matrix | monochrome | neon | norton | oxide | sandbar
preset = "{theme_preset}"

# Label shown in the bottom-left status bar corner.
status_bar_label = "{status_bar_label}"

[keymap]
# Key binding to quit Zeta.
quit = "{quit}"

# Key binding to switch focus between left and right panes.
switch_pane = "{switch_pane}"

# Key binding to refresh the active pane directory listing.
refresh = "{refresh}"

# Key bindings to switch between the four workspaces.
workspace = ["{ws0}", "{ws1}", "{ws2}", "{ws3}"]

[editor]
# Number of spaces a tab character expands to in the embedded editor.
tab_width = {tab_width}

# Soft-wrap lines at the viewport edge in the embedded editor.
word_wrap = {word_wrap}
{openers}"#,
        icon_mode = icon_mode,
        pane_layout = pane_layout,
        preview_panel_open = config.preview_panel_open,
        preview_on_selection = config.preview_on_selection,
        terminal_open_by_default = config.terminal_open_by_default,
        check_updates_on_startup = config.check_updates_on_startup,
        theme_preset = config.theme.preset,
        status_bar_label = config.theme.status_bar_label,
        quit = config.keymap.quit,
        switch_pane = config.keymap.switch_pane,
        refresh = config.keymap.refresh,
        ws0 = config.keymap.workspace[0],
        ws1 = config.keymap.workspace[1],
        ws2 = config.keymap.workspace[2],
        ws3 = config.keymap.workspace[3],
        tab_width = config.editor.tab_width,
        word_wrap = config.editor.word_wrap,
        openers = openers,
    )
}
```

> **Note:** `KeymapConfig` fields (`quit`, `switch_pane`, `refresh`, `workspace`) must be `String`/`Display`. Check the actual field types in `KeymapConfig` — if they are `KeyBinding` (a struct), call `.to_string()` or `.display()` on them. Look at how the settings panel displays them (around line 3252 of `src/state/mod.rs`) to find the right method and adjust the format string accordingly.

- [ ] **Step 4: Run tests**

```
cargo test --lib config::tests -- --nocapture
```
Expected: 4 tests pass. Fix any compile errors relating to `KeymapConfig` field types.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(wizard): generate_annotated_config writes commented TOML"
```

---

## Task 7 — Wizard UI renderer `src/ui/wizard.rs`

**Files:**
- Create: `src/ui/wizard.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Baseline check**

```
cargo check
```
Expected: clean.

- [ ] **Step 2: Create `src/ui/wizard.rs`**

```rust
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::state::wizard::{WizardState, WizardStep, WIZARD_THEMES};

/// Key reference rows shown in the cheatsheet step.
/// Each entry is (key, description).
const CHEATSHEET: &[(&str, &str)] = &[
    ("↑ / ↓", "Navigate files"),
    ("Enter", "Open file / enter directory"),
    ("Backspace", "Go up one directory"),
    ("Tab", "Switch pane focus"),
    ("Space", "Toggle mark on file"),
    ("F1", "Help dialog"),
    ("F2", "Toggle embedded terminal"),
    ("F3", "Toggle preview panel"),
    ("F4", "Open file in editor"),
    ("F5", "Copy selected files"),
    ("F6", "Rename"),
    ("Shift+F6", "Move"),
    ("F7", "New directory"),
    ("F8", "Move to trash"),
    ("Shift+F8", "Permanently delete"),
    ("F9", "Toggle diff mode"),
    ("F10 / q", "Quit"),
    ("Ctrl+P", "Command palette"),
    ("Ctrl+F", "Find files"),
    ("F11", "Toggle editor fullscreen"),
    ("F12", "Debug panel"),
    ("Shift+M", "Clear marks"),
    ("m", "Add bookmark (pane context)"),
    ("F12 / Ctrl+,", "Settings panel"),
];

/// Render the first-run wizard as a centred modal overlay.
pub fn render_first_run_wizard(frame: &mut Frame<'_>, area: Rect, state: &WizardState, palette: ThemePalette) {
    // Dim background
    use ratatui::buffer::Buffer;
    use ratatui::widgets::Widget;
    struct Dim;
    impl Widget for Dim {
        fn render(self, area: Rect, buf: &mut Buffer) {
            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_style(Style::default().add_modifier(Modifier::DIM));
                    }
                }
            }
        }
    }
    frame.render_widget(Dim, area);

    // Centre a modal box: 60% wide, 80% tall, at least 60×20.
    let width = (area.width * 6 / 10).max(60).min(area.width.saturating_sub(4));
    let height = (area.height * 8 / 10).max(20).min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let modal = Rect { x, y, width, height };

    frame.render_widget(Clear, modal);

    match state.step {
        WizardStep::ThemePicker => render_theme_picker(frame, modal, state, palette),
        WizardStep::Cheatsheet => render_cheatsheet(frame, modal, state, palette),
    }
}

fn render_theme_picker(frame: &mut Frame<'_>, area: Rect, state: &WizardState, palette: ThemePalette) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" 🎨 Welcome to Zeta — Choose a Theme ", Style::default()
                .fg(palette.text_primary)
                .add_modifier(Modifier::BOLD)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette.border_focus))
        .style(Style::default().bg(palette.surface_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(2)])
        .split(inner);

    let items: Vec<ListItem> = WIZARD_THEMES
        .iter()
        .map(|(label, _)| {
            ListItem::new(format!("  {label}  "))
                .style(Style::default().fg(palette.text_primary))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.theme_selection));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(palette.selection_bg)
                .fg(palette.selection_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, chunks[0], &mut list_state);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("  ↑/↓ ", Style::default().fg(palette.key_hint_fg)),
        Span::styled("select   ", Style::default().fg(palette.text_muted)),
        Span::styled("Enter ", Style::default().fg(palette.key_hint_fg)),
        Span::styled("confirm   ", Style::default().fg(palette.text_muted)),
        Span::styled("Esc ", Style::default().fg(palette.key_hint_fg)),
        Span::styled("skip", Style::default().fg(palette.text_muted)),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[1]);
}

fn render_cheatsheet(frame: &mut Frame<'_>, area: Rect, state: &WizardState, palette: ThemePalette) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ⌨  Keyboard Reference ", Style::default()
                .fg(palette.text_primary)
                .add_modifier(Modifier::BOLD)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette.border_focus))
        .style(Style::default().bg(palette.surface_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(2)])
        .split(inner);

    // Build visible rows with scroll offset applied.
    let visible_height = chunks[0].height as usize;
    let start = state.cheatsheet_scroll.min(CHEATSHEET.len().saturating_sub(visible_height));
    let rows: Vec<ListItem> = CHEATSHEET
        .iter()
        .skip(start)
        .take(visible_height)
        .map(|(key, desc)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {key:<14}", key = key), Style::default().fg(palette.key_hint_fg)),
                Span::styled(format!("{desc}"), Style::default().fg(palette.text_primary)),
            ]))
        })
        .collect();

    let list = List::new(rows);
    frame.render_widget(list, chunks[0]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("  ↑/↓ ", Style::default().fg(palette.key_hint_fg)),
        Span::styled("scroll   ", Style::default().fg(palette.text_muted)),
        Span::styled("Enter / Esc ", Style::default().fg(palette.key_hint_fg)),
        Span::styled("start using Zeta", Style::default().fg(palette.text_muted)),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[1]);
}
```

- [ ] **Step 3: Register the module and hook into `src/ui/mod.rs`**

At the top of `src/ui/mod.rs`, add:
```rust
pub mod wizard;
```

Find the `render_overlay` function (or equivalent) where `ModalState::Settings(...)` is rendered. It should have a match block or a series of `if let` checks. Add a branch for the wizard:

```rust
if let Some(crate::state::overlay::ModalState::FirstRunWizard(wizard_state)) = &state.overlay().modal {
    wizard::render_first_run_wizard(frame, area, wizard_state, palette);
    return;
}
```

Add this branch *before* any existing overlay rendering so the wizard takes full priority and the early return prevents other overlays from drawing. Locate where other modals like Settings are rendered and insert the wizard check at the top of that function.

- [ ] **Step 4: `cargo check` passes**

```
cargo check
```
Expected: no errors.

- [ ] **Step 5: `cargo clippy` passes**

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
Fix any warnings (unused imports, etc.).

- [ ] **Step 6: Commit**

```bash
git add src/ui/wizard.rs src/ui/mod.rs
git commit -m "feat(wizard): first-run wizard UI (theme picker + cheatsheet)"
```

---

## Task 8 — Integration tests

**Files:**
- Create: `tests/wizard_integration.rs`

- [ ] **Step 1: Write the integration tests**

```rust
// tests/wizard_integration.rs
use std::path::PathBuf;
use std::time::Instant;

use zeta::config::{AppConfig, ConfigSource, LoadedConfig};
use zeta::state::{AppState, ModalKind};

fn make_state(source: ConfigSource) -> AppState {
    let loaded = LoadedConfig {
        config: AppConfig::default(),
        path: PathBuf::from(""),
        source,
    };
    AppState::bootstrap(loaded, Instant::now()).expect("bootstrap failed")
}

#[test]
fn first_launch_opens_wizard_modal() {
    let mut state = make_state(ConfigSource::Default);
    let _cmds = state.initial_commands();
    assert_eq!(
        state.modal_kind(),
        Some(ModalKind::FirstRunWizard),
        "expected FirstRunWizard modal after first-run bootstrap"
    );
}

#[test]
fn subsequent_launch_does_not_open_wizard() {
    let mut state = make_state(ConfigSource::File);
    let _cmds = state.initial_commands();
    assert_ne!(
        state.modal_kind(),
        Some(ModalKind::FirstRunWizard),
        "expected no wizard when config file already exists"
    );
}

#[test]
fn annotated_config_round_trips() {
    use zeta::config::generate_annotated_config;
    let cfg = AppConfig::default();
    let text = generate_annotated_config(&cfg);
    let parsed: AppConfig = basic_toml::from_str(&text)
        .expect("generated annotated config must be valid TOML");
    assert_eq!(cfg.theme.preset, parsed.theme.preset);
    assert_eq!(cfg.editor.tab_width, parsed.editor.tab_width);
}
```

- [ ] **Step 2: Ensure `AppState::modal_kind()` is public**

In `src/state/mod.rs`, add or confirm this public accessor near the other overlay-related accessors:
```rust
pub fn modal_kind(&self) -> Option<ModalKind> {
    self.overlay.modal_kind()
}
```

Also ensure `generate_annotated_config` is `pub` in `src/config.rs` (it already is per Task 6).

- [ ] **Step 3: Run integration tests**

```
cargo test --tests wizard_integration -- --nocapture
```
Expected: 3 tests pass.

- [ ] **Step 4: Commit**

```bash
git add tests/wizard_integration.rs
git commit -m "test(wizard): integration tests for first-run wizard bootstrap and config round-trip"
```

---

## Task 9 — Pre-PR validation, branch, and PR

**Files:** none new

- [ ] **Step 1: Create and switch to feature branch** *(do this first, before Task 1)*

```bash
git checkout main && git pull --ff-only
git checkout -b feat/phase3-first-run-wizard
```

- [ ] **Step 2: Format check**

```
cargo fmt --all -- --check
```
If it fails, run `cargo fmt --all` then re-check.

- [ ] **Step 3: Clippy (zero warnings)**

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
Fix all warnings before proceeding.

- [ ] **Step 4: Full test suite**

```
cargo test --workspace
```
Expected: all existing tests plus new wizard tests pass. The two pre-existing failures (`route_mouse_left_click_on_workspace_pill_{2,4}`) are acceptable — they are unrelated to this phase.

- [ ] **Step 5: Smoke-test manually** *(optional but recommended)*

```bash
cargo build && rm -f ~/.config/zeta/config.toml && cargo run --
```
Expected: wizard opens on first run, theme preview updates live as you scroll, Enter advances to cheatsheet, Enter again closes wizard and writes `~/.config/zeta/config.toml` with `#` comment lines.

- [ ] **Step 6: Open PR**

```bash
gh pr create \
  --base main \
  --head feat/phase3-first-run-wizard \
  --title "feat: Phase 3 — first-run wizard + annotated config" \
  --body "## What

Adds a two-step first-run wizard that opens automatically when no \`config.toml\` is found:
1. **Theme Picker** — live preview of all 10 themes with keyboard navigation
2. **Cheatsheet** — scrollable keyboard reference

On completion, writes a fully-annotated \`config.toml\` with inline \`#\` doc comments for every setting.

## How

- \`ConfigSource::Default\` detection triggers \`show_wizard = true\` in \`AppState::bootstrap\`
- \`initial_commands()\` opens \`ModalState::FirstRunWizard\`
- New \`src/state/wizard.rs\` holds pure logic (no I/O, fully unit-tested)
- New \`src/ui/wizard.rs\` renders theme-list + cheatsheet overlays
- \`generate_annotated_config()\` in \`src/config.rs\` produces commented TOML
- 7 unit tests + 3 integration tests

## Pre-PR checklist
- [x] \`cargo fmt --all -- --check\`
- [x] \`cargo clippy --workspace --all-targets --all-features -- -D warnings\`
- [x] \`cargo test --workspace\`"
```

---

## Self-review checklist

- **Spec coverage:** all 3 requirements covered — theme picker ✅, cheatsheet ✅, annotated config write ✅
- **Placeholder scan:** no TBD/TODO; all code blocks are complete
- **Note on `KeymapConfig` fields:** Task 6 Step 3 includes a callout to check the actual field types. The subagent must inspect `src/config.rs` lines for `KeymapConfig` struct definition before writing the format string.
- **Type consistency:** `WizardState` defined in Task 1; used correctly in Tasks 2–8 via the same `move_down`, `move_up`, `advance`, `selected_preset` API. `ModalState::FirstRunWizard(WizardState)` set in Task 2, opened in Task 5, rendered in Task 7, tested in Task 8.
- **`modal_kind()` exposure:** Task 8 explicitly ensures `AppState::modal_kind()` is public.
- **Branch order:** Task 9 Step 1 (create branch) must happen first — remind subagent to reorder.
