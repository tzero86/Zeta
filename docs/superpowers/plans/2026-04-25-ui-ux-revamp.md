# UI/UX Revamp Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Modernise every major TUI surface with Catppuccin Mocha palette, NerdFont icons, rich pane columns, zoned status bar, halo modals, segmented settings, and polished panel chrome.

**Architecture:** Expand `ThemePalette` with accent tokens that every renderer consumes. All rendering code pulls colours from the palette — no more ad-hoc hex literals. A new `StatusZones` struct replaces the monolithic `status_line()` string so the renderer can paint each zone a distinct colour.

**Tech Stack:** Rust stable · ratatui · crossterm · serde/toml · thiserror

**Spec:** `docs/superpowers/specs/2026-04-25-ui-ux-revamp-design.md`

**Pre-PR validation (run after all tasks):**
```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

---

## Task 1 — Palette tokens, new ThemePalette fields, and styles.rs helpers

**Files:**
- Modify: `src/config.rs` — `ThemePalette` struct + all 9 theme functions + new `CatppuccinMocha` preset
- Modify: `src/ui/styles.rs` — add new style helper functions

**What this task does:** Adds the nine new semantic colour tokens that subsequent tasks need, adds the Catppuccin Mocha theme preset, and adds matching style helpers in `styles.rs`. Every subsequent task in this plan depends on these new fields being present.

---

- [ ] **Step 1: Add new fields to `ThemePalette` in `src/config.rs`**

  Insert the following nine new fields at the end of the `ThemePalette` struct (after `text_sel_bg`):

  ```rust
  // Accent tokens (used by new UI surfaces)
  pub text_subtext: Color,
  pub accent_mauve: Color,
  pub accent_teal: Color,
  pub accent_green: Color,
  pub accent_yellow: Color,
  pub accent_peach: Color,
  pub accent_red: Color,
  pub modal_halo: Color,
  pub pane_filter_bg: Color,
  pub pane_filter_border: Color,
  // Zone background tints for the zoned status bar
  pub status_git_bg: Color,
  pub status_entry_bg: Color,
  pub status_workspace_bg: Color,
  ```

- [ ] **Step 2: Update every existing theme function to set the new fields**

  In each of the 9 theme functions (`fjord`, `sandbar`, `oxide`, `matrix`, `norton`, `neon`, `monochrome`, `dracula`, `zeta`) add the new fields using derived/approximate values. Add these lines to each theme's struct literal:

  **fjord** — append to `Self { … }`:
  ```rust
  text_subtext: Color::Rgb(180, 190, 200),
  accent_mauve: Color::Rgb(180, 60, 30),
  accent_teal: Color::Rgb(118, 196, 182),
  accent_green: Color::Rgb(100, 200, 120),
  accent_yellow: Color::Rgb(214, 179, 92),
  accent_peach: Color::Rgb(230, 130, 80),
  accent_red: Color::Rgb(220, 80, 80),
  modal_halo: Color::Rgb(47, 53, 59),
  pane_filter_bg: Color::Rgb(35, 44, 50),
  pane_filter_border: Color::Rgb(70, 120, 130),
  status_git_bg: Color::Rgb(34, 48, 52),
  status_entry_bg: Color::Rgb(36, 38, 44),
  status_workspace_bg: Color::Rgb(44, 36, 44),
  ```

  **sandbar** — append to `Self { … }`:
  ```rust
  text_subtext: Color::Rgb(195, 185, 170),
  accent_mauve: Color::Rgb(139, 69, 19),
  accent_teal: Color::Rgb(83, 148, 117),
  accent_green: Color::Rgb(80, 160, 100),
  accent_yellow: Color::Rgb(205, 143, 57),
  accent_peach: Color::Rgb(220, 120, 60),
  accent_red: Color::Rgb(200, 70, 70),
  modal_halo: Color::Rgb(52, 47, 40),
  pane_filter_bg: Color::Rgb(40, 37, 30),
  pane_filter_border: Color::Rgb(80, 130, 100),
  status_git_bg: Color::Rgb(36, 42, 38),
  status_entry_bg: Color::Rgb(38, 36, 32),
  status_workspace_bg: Color::Rgb(48, 36, 30),
  ```

  **oxide** — append to `Self { … }`:
  ```rust
  text_subtext: Color::Rgb(185, 192, 200),
  accent_mauve: Color::Rgb(101, 45, 32),
  accent_teal: Color::Rgb(102, 174, 197),
  accent_green: Color::Rgb(90, 160, 100),
  accent_yellow: Color::Rgb(221, 176, 98),
  accent_peach: Color::Rgb(205, 130, 107),
  accent_red: Color::Rgb(210, 80, 80),
  modal_halo: Color::Rgb(40, 45, 53),
  pane_filter_bg: Color::Rgb(30, 38, 44),
  pane_filter_border: Color::Rgb(70, 120, 140),
  status_git_bg: Color::Rgb(30, 42, 50),
  status_entry_bg: Color::Rgb(34, 36, 44),
  status_workspace_bg: Color::Rgb(40, 32, 44),
  ```

  **matrix** — append to `Self { … }`:
  ```rust
  text_subtext: Color::Rgb(40, 160, 60),
  accent_mauve: Color::Rgb(0, 255, 0),
  accent_teal: Color::Rgb(44, 213, 255),
  accent_green: Color::Rgb(0, 200, 64),
  accent_yellow: Color::Rgb(200, 220, 0),
  accent_peach: Color::Rgb(200, 150, 0),
  accent_red: Color::Rgb(220, 50, 80),
  modal_halo: Color::Rgb(20, 40, 20),
  pane_filter_bg: Color::Rgb(12, 26, 12),
  pane_filter_border: Color::Rgb(0, 150, 50),
  status_git_bg: Color::Rgb(8, 28, 12),
  status_entry_bg: Color::Rgb(10, 24, 10),
  status_workspace_bg: Color::Rgb(14, 30, 20),
  ```

  **neon** — append to `Self { … }`:
  ```rust
  text_subtext: Color::Rgb(160, 160, 200),
  accent_mauve: Color::Rgb(255, 0, 255),
  accent_teal: Color::Rgb(0, 220, 220),
  accent_green: Color::Rgb(0, 255, 160),
  accent_yellow: Color::Rgb(255, 255, 0),
  accent_peach: Color::Rgb(255, 160, 80),
  accent_red: Color::Rgb(255, 60, 100),
  modal_halo: Color::Rgb(30, 30, 50),
  pane_filter_bg: Color::Rgb(12, 12, 30),
  pane_filter_border: Color::Rgb(0, 160, 255),
  status_git_bg: Color::Rgb(10, 14, 30),
  status_entry_bg: Color::Rgb(12, 10, 28),
  status_workspace_bg: Color::Rgb(20, 10, 36),
  ```

  **monochrome** — append to `Self { … }`:
  ```rust
  text_subtext: Color::Rgb(160, 160, 160),
  accent_mauve: Color::Rgb(190, 190, 190),
  accent_teal: Color::Rgb(200, 200, 200),
  accent_green: Color::Rgb(150, 200, 150),
  accent_yellow: Color::Rgb(210, 210, 150),
  accent_peach: Color::Rgb(210, 180, 150),
  accent_red: Color::Rgb(220, 100, 100),
  modal_halo: Color::Rgb(40, 40, 40),
  pane_filter_bg: Color::Rgb(22, 22, 22),
  pane_filter_border: Color::Rgb(80, 80, 80),
  status_git_bg: Color::Rgb(25, 28, 25),
  status_entry_bg: Color::Rgb(24, 24, 28),
  status_workspace_bg: Color::Rgb(30, 24, 30),
  ```

  **dracula** — append to `Self { … }`:
  ```rust
  text_subtext: Color::Rgb(190, 195, 220),
  accent_mauve: Color::Rgb(189, 147, 249),
  accent_teal: Color::Rgb(139, 233, 253),
  accent_green: Color::Rgb(80, 250, 123),
  accent_yellow: Color::Rgb(241, 250, 140),
  accent_peach: Color::Rgb(255, 184, 108),
  accent_red: Color::Rgb(255, 85, 85),
  modal_halo: Color::Rgb(60, 63, 80),
  pane_filter_bg: Color::Rgb(46, 48, 62),
  pane_filter_border: Color::Rgb(100, 80, 180),
  status_git_bg: Color::Rgb(44, 46, 70),
  status_entry_bg: Color::Rgb(48, 44, 68),
  status_workspace_bg: Color::Rgb(56, 44, 72),
  ```

  For the **zeta** theme (update `zeta()` function) — also append:
  ```rust
  text_subtext: Color::Rgb(155, 165, 185),
  accent_mauve: Color::Rgb(160, 100, 220),
  accent_teal: Color::Rgb(80, 190, 190),
  accent_green: Color::Rgb(80, 180, 100),
  accent_yellow: Color::Rgb(210, 180, 80),
  accent_peach: Color::Rgb(220, 140, 80),
  accent_red: Color::Rgb(220, 80, 80),
  modal_halo: Color::Rgb(45, 50, 60),
  pane_filter_bg: Color::Rgb(28, 36, 50),
  pane_filter_border: Color::Rgb(60, 100, 160),
  status_git_bg: Color::Rgb(28, 36, 56),
  status_entry_bg: Color::Rgb(32, 30, 52),
  status_workspace_bg: Color::Rgb(40, 30, 58),
  ```

  > **Note:** Find the `norton()` function in config.rs and do the same — use the same values as `zeta()` above (norton is a retro theme, these derive naturally from its palette).

- [ ] **Step 3: Add the Catppuccin Mocha theme preset**

  3a. Add to `ThemePreset` enum (after `Zeta`):
  ```rust
  CatppuccinMocha,
  ```

  3b. Add to `ThemePreset::as_str()` match:
  ```rust
  Self::CatppuccinMocha => "catppuccin_mocha",
  ```

  3c. Add to `ThemePreset::from_name()` (find this function and add):
  ```rust
  "catppuccin_mocha" => Some(Self::CatppuccinMocha),
  ```

  3d. Add to `ThemePalette::from_preset()` match:
  ```rust
  ThemePreset::CatppuccinMocha => ResolvedTheme {
      palette: Self::catppuccin_mocha(),
      preset: String::from("catppuccin_mocha"),
      warning: None,
  },
  ```

  3e. Add the `catppuccin_mocha()` function to `ThemePalette`:
  ```rust
  fn catppuccin_mocha() -> Self {
      Self {
          menu_bg: Color::Rgb(24, 24, 37),           // mantle
          menu_fg: Color::Rgb(205, 214, 244),         // text
          menu_active_bg: Color::Rgb(49, 50, 68),     // surface0/overlay
          menu_mnemonic_fg: Color::Rgb(203, 166, 247), // mauve
          border_focus: Color::Rgb(137, 180, 250),    // blue
          border_editor_focus: Color::Rgb(250, 179, 135), // peach
          selection_bg: Color::Rgb(69, 71, 90),       // surface1
          selection_fg: Color::Rgb(205, 214, 244),    // text
          surface_bg: Color::Rgb(30, 30, 46),         // base
          tools_bg: Color::Rgb(24, 24, 37),           // mantle
          prompt_bg: Color::Rgb(17, 17, 27),          // crust
          prompt_border: Color::Rgb(137, 180, 250),   // blue
          text_primary: Color::Rgb(205, 214, 244),    // text
          text_muted: Color::Rgb(108, 112, 134),      // overlay0
          directory_fg: Color::Rgb(137, 180, 250),    // blue
          symlink_fg: Color::Rgb(148, 226, 213),      // teal
          file_fg: Color::Rgb(186, 194, 222),         // subtext1
          status_bg: Color::Rgb(49, 50, 68),          // surface0
          status_fg: Color::Rgb(205, 214, 244),       // text
          logo_accent: Color::Rgb(203, 166, 247),     // mauve
          key_hint_fg: Color::Rgb(249, 226, 175),     // yellow
          syntect_theme: "Dracula",
          search_match_bg: Color::Rgb(75, 68, 30),
          search_match_active_bg: Color::Rgb(160, 140, 30),
          text_sel_bg: Color::Rgb(40, 55, 100),
          // New accent tokens
          text_subtext: Color::Rgb(186, 194, 222),    // subtext1
          accent_mauve: Color::Rgb(203, 166, 247),    // mauve
          accent_teal: Color::Rgb(148, 226, 213),     // teal
          accent_green: Color::Rgb(166, 227, 161),    // green
          accent_yellow: Color::Rgb(249, 226, 175),   // yellow
          accent_peach: Color::Rgb(250, 179, 135),    // peach
          accent_red: Color::Rgb(243, 139, 168),      // red
          modal_halo: Color::Rgb(49, 50, 68),         // surface0/overlay
          pane_filter_bg: Color::Rgb(33, 36, 58),     // blue tint over base
          pane_filter_border: Color::Rgb(60, 80, 130), // blue 30% over base
          status_git_bg: Color::Rgb(35, 37, 62),      // blue tint
          status_entry_bg: Color::Rgb(36, 33, 58),    // mauve tint
          status_workspace_bg: Color::Rgb(46, 36, 70), // strong mauve
      }
  }
  ```

- [ ] **Step 4: Verify it compiles**
  ```
  cargo check 2>&1 | head -40
  ```
  Expected: zero errors. Fix any "missing field" errors in existing theme functions.

- [ ] **Step 5: Update `settings_entries()` in `src/state/mod.rs` to list CatppuccinMocha as a valid theme**

  Find the `SettingsField::Theme` entry in `settings_entries()` which builds the theme list. Locate the string that lists all theme names and add `catppuccin_mocha`:
  ```rust
  value: match self.theme.preset.as_str() {
      // ... existing arms ...
      "catppuccin_mocha" => String::from("catppuccin_mocha"),
      _ => self.theme.preset.clone(),
  },
  ```
  Also add `catppuccin_mocha` to the cycling logic for `SettingsField::Theme` in `apply_settings_change()` in `src/state/mod.rs` (find the section that cycles themes with `ThemePreset::from_name` and append `"catppuccin_mocha"` to the cycle list).

- [ ] **Step 6: Add new style helpers to `src/ui/styles.rs`**

  Replace the entire file with the expanded version:

  ```rust
  use ratatui::style::{Color, Modifier, Style};

  use crate::config::ThemePalette;

  pub fn elevated_surface_style(palette: ThemePalette) -> Style {
      Style::default().bg(palette.tools_bg)
  }

  pub fn modal_backdrop_style(_palette: ThemePalette) -> Style {
      Style::default().bg(Color::Rgb(36, 38, 42))
  }

  pub fn modal_halo_style(palette: ThemePalette) -> Style {
      Style::default().bg(palette.modal_halo)
  }

  pub fn overlay_title_style(palette: ThemePalette) -> Style {
      Style::default()
          .fg(palette.menu_mnemonic_fg)
          .add_modifier(Modifier::BOLD)
  }

  pub fn overlay_key_hint_style(palette: ThemePalette) -> Style {
      Style::default()
          .fg(palette.key_hint_fg)
          .add_modifier(Modifier::BOLD)
  }

  pub fn overlay_footer_style(palette: ThemePalette) -> Style {
      Style::default().fg(palette.text_muted)
  }

  pub fn command_palette_header_style(palette: ThemePalette) -> Style {
      Style::default()
          .fg(palette.text_muted)
          .add_modifier(Modifier::BOLD)
  }

  pub fn command_palette_entry_label_style(is_selected: bool, palette: ThemePalette) -> Style {
      if is_selected {
          Style::default()
              .fg(palette.selection_fg)
              .bg(palette.selection_bg)
              .add_modifier(Modifier::BOLD)
      } else {
          Style::default().fg(palette.text_primary)
      }
  }

  pub fn command_palette_entry_hint_style(palette: ThemePalette) -> Style {
      Style::default().fg(palette.key_hint_fg)
  }

  pub fn dim_text_style(palette: ThemePalette) -> Style {
      Style::default().fg(palette.text_muted)
  }

  pub fn subtext_style(palette: ThemePalette) -> Style {
      Style::default().fg(palette.text_subtext)
  }

  pub fn accent_mauve_style(palette: ThemePalette) -> Style {
      Style::default().fg(palette.accent_mauve)
  }

  pub fn accent_teal_style(palette: ThemePalette) -> Style {
      Style::default().fg(palette.accent_teal)
  }

  pub fn accent_green_style(palette: ThemePalette) -> Style {
      Style::default().fg(palette.accent_green)
  }

  pub fn accent_yellow_style(palette: ThemePalette) -> Style {
      Style::default().fg(palette.accent_yellow)
  }

  pub fn accent_peach_style(palette: ThemePalette) -> Style {
      Style::default().fg(palette.accent_peach)
  }

  pub fn key_pill_style(palette: ThemePalette) -> Style {
      Style::default()
          .fg(palette.accent_yellow)
          .bg(palette.modal_halo)
          .add_modifier(Modifier::BOLD)
  }

  pub fn section_divider_style(palette: ThemePalette) -> Style {
      Style::default()
          .fg(palette.border_focus)
          .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
  }

  pub fn panel_title_focused_style(accent: Color) -> Style {
      Style::default()
          .fg(accent)
          .add_modifier(Modifier::BOLD)
  }

  pub fn panel_title_unfocused_style(palette: ThemePalette) -> Style {
      Style::default().fg(palette.text_muted)
  }

  pub fn dirty_indicator_style(palette: ThemePalette) -> Style {
      Style::default().fg(palette.accent_peach)
  }

  pub fn pane_filter_strip_style(palette: ThemePalette) -> Style {
      Style::default()
          .fg(palette.text_primary)
          .bg(palette.pane_filter_bg)
  }

  pub fn pane_column_header_style(palette: ThemePalette) -> Style {
      Style::default()
          .fg(palette.text_muted)
          .add_modifier(Modifier::BOLD)
  }

  pub fn category_badge_style(palette: ThemePalette) -> Style {
      Style::default()
          .fg(palette.surface_bg)
          .bg(palette.accent_teal)
          .add_modifier(Modifier::BOLD)
  }

  pub fn match_highlight_style(palette: ThemePalette) -> Style {
      Style::default()
          .fg(palette.accent_yellow)
          .add_modifier(Modifier::BOLD)
  }

  pub fn finder_match_highlight_style(palette: ThemePalette) -> Style {
      Style::default()
          .fg(palette.accent_teal)
          .add_modifier(Modifier::BOLD)
  }
  ```

- [ ] **Step 7: Verify the file compiles**
  ```
  cargo check 2>&1 | head -20
  ```
  Expected: zero errors.

- [ ] **Step 8: Run existing tests to confirm nothing is broken**
  ```
  cargo test --workspace 2>&1 | tail -20
  ```
  Expected: all tests pass (icon tests, unit tests, integration tests).

- [ ] **Step 9: Commit**
  ```
  git add src/config.rs src/ui/styles.rs
  git commit -m "feat: add Catppuccin Mocha theme and 13 new ThemePalette accent tokens"
  ```

---

## Task 2 — Icon system: NerdFont rename + NF v3 codepoints + extension dispatch

**Files:**
- Modify: `src/icon.rs` — rename `Custom` → `NerdFont`, add `icon_for_entry()`, real NF v3 codepoints
- Modify: `src/config.rs` — rename `IconMode::Custom` → `IconMode::NerdFont`
- Modify: `src/ui/pane.rs` — call `icon_for_entry()` instead of `icon_for_kind()`

---

- [ ] **Step 1: Rename `IconMode::Custom` → `IconMode::NerdFont` in `src/config.rs`**

  In the `IconMode` enum:
  ```rust
  #[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
  #[serde(rename_all = "snake_case")]
  pub enum IconMode {
      #[default]
      Unicode,
      Ascii,
      NerdFont,  // was: Custom
  }
  ```
  The `#[serde(rename_all = "snake_case")]` means the TOML key becomes `"nerd_font"`. Add a serde alias so existing configs using `"custom"` continue to work:
  ```rust
  #[serde(alias = "custom")]
  NerdFont,
  ```

- [ ] **Step 2: Rewrite `src/icon.rs`**

  Replace the entire file with the following. This adds `icon_for_entry()` for extension-based dispatch, real NF v3 codepoints, and keeps `icon_for_kind()` for callers that don't have path info:

  ```rust
  use crate::config::IconMode;
  use crate::fs::EntryKind;

  pub fn icon_for_kind(kind: EntryKind, mode: IconMode) -> &'static str {
      icon_for_entry(kind, None, mode)
  }

  pub fn icon_for_entry(kind: EntryKind, extension: Option<&str>, mode: IconMode) -> &'static str {
      match mode {
          IconMode::Unicode => unicode_icon(kind),
          IconMode::Ascii => kind.ascii_label(),
          IconMode::NerdFont => nerdfont_icon(kind, extension),
      }
  }

  fn unicode_icon(kind: EntryKind) -> &'static str {
      match kind {
          EntryKind::Directory => "▣",
          EntryKind::File => "•",
          EntryKind::Symlink => "↗",
          EntryKind::Archive => "🗜",
          EntryKind::Other => "◦",
      }
  }

  fn nerdfont_icon(kind: EntryKind, extension: Option<&str>) -> &'static str {
      match kind {
          EntryKind::Directory => "\u{f07b}",   // 󰉋 nf-fa-folder
          EntryKind::Symlink => "\u{f481}",     // 󱒁 nf-md-link_variant
          EntryKind::Archive => "\u{f410}",     // 󴐐 nf-oct-file_zip (approx)
          EntryKind::Other => "\u{f128}",       // nf-fa-question_circle
          EntryKind::File => {
              match extension.map(|e| e.to_ascii_lowercase()).as_deref() {
                  Some("rs") => "\u{e7a8}",     // nf-dev-rust
                  Some("toml") | Some("yaml") | Some("yml") | Some("json") => "\u{e615}", // nf-seti-config
                  Some("md") | Some("mdx") => "\u{f48a}",  // nf-fa-markdown
                  Some("sh") | Some("bash") | Some("zsh") | Some("fish") => "\u{f489}", // nf-fa-terminal
                  Some("py") => "\u{e606}",     // nf-dev-python
                  Some("js") | Some("ts") | Some("jsx") | Some("tsx") => "\u{e74e}", // nf-dev-javascript
                  Some("go") => "\u{e626}",     // nf-dev-go
                  Some("c") | Some("cpp") | Some("h") | Some("hpp") => "\u{e61e}", // nf-dev-c
                  Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") | Some("webp") => "\u{f1c5}", // nf-fa-file_image_o
                  Some("zip") | Some("tar") | Some("gz") | Some("bz2") | Some("xz") | Some("7z") => "\u{f410}",
                  Some("lock") => "\u{f023}",   // nf-fa-lock (also for hidden dotfiles)
                  _ => "\u{f15b}",              // nf-fa-file (generic)
              }
          }
      }
  }

  #[cfg(test)]
  mod tests {
      use super::{icon_for_entry, icon_for_kind};
      use crate::config::IconMode;
      use crate::fs::EntryKind;

      #[test]
      fn unicode_icons_use_glyphs() {
          assert_eq!(icon_for_kind(EntryKind::Directory, IconMode::Unicode), "▣");
          assert_eq!(icon_for_kind(EntryKind::File, IconMode::Unicode), "•");
          assert_eq!(icon_for_kind(EntryKind::Symlink, IconMode::Unicode), "↗");
          assert_eq!(icon_for_kind(EntryKind::Other, IconMode::Unicode), "◦");
      }

      #[test]
      fn ascii_icons_use_labels() {
          assert_eq!(icon_for_kind(EntryKind::Directory, IconMode::Ascii), "[D]");
          assert_eq!(icon_for_kind(EntryKind::File, IconMode::Ascii), "[F]");
          assert_eq!(icon_for_kind(EntryKind::Symlink, IconMode::Ascii), "[L]");
          assert_eq!(icon_for_kind(EntryKind::Other, IconMode::Ascii), "[?]");
      }

      #[test]
      fn nerdfont_directory_icon() {
          assert_eq!(
              icon_for_kind(EntryKind::Directory, IconMode::NerdFont),
              "\u{f07b}"
          );
      }

      #[test]
      fn nerdfont_rust_extension() {
          assert_eq!(
              icon_for_entry(EntryKind::File, Some("rs"), IconMode::NerdFont),
              "\u{e7a8}"
          );
      }

      #[test]
      fn nerdfont_generic_file_no_extension() {
          assert_eq!(
              icon_for_entry(EntryKind::File, None, IconMode::NerdFont),
              "\u{f15b}"
          );
      }

      #[test]
      fn nerdfont_image_extension() {
          assert_eq!(
              icon_for_entry(EntryKind::File, Some("png"), IconMode::NerdFont),
              "\u{f1c5}"
          );
      }
  }
  ```

- [ ] **Step 3: Run the new tests to verify they pass**
  ```
  cargo test --lib icon -- --nocapture
  ```
  Expected: 6 tests pass.

- [ ] **Step 4: Update `src/ui/pane.rs` to call `icon_for_entry()`**

  In `render_item()`, the line `let icon = icon_for_kind(entry.kind, icon_mode);` becomes:
  ```rust
  let icon = icon_for_entry(
      entry.kind,
      entry.path.extension().and_then(|e| e.to_str()),
      icon_mode,
  );
  ```
  Update the import at the top of `pane.rs`:
  ```rust
  use crate::icon::icon_for_entry;
  ```
  Remove the old `icon_for_kind` import if it is no longer used.

- [ ] **Step 5: Fix any remaining references to `IconMode::Custom`**
  ```
  grep -rn "IconMode::Custom\|icon_mode = \"custom\"" src/ tests/
  ```
  Update each occurrence to `IconMode::NerdFont`.

- [ ] **Step 6: Fix any remaining references to the old `custom_icons_use_private_use_glyphs` test name** — that test is replaced by the new tests in Step 2 above.

- [ ] **Step 7: Run all tests**
  ```
  cargo test --workspace
  ```
  Expected: all pass.

- [ ] **Step 8: Commit**
  ```
  git add src/icon.rs src/config.rs src/ui/pane.rs
  git commit -m "feat: rename IconMode::Custom to NerdFont with real NF v3 codepoints and extension dispatch"
  ```

---

## Task 3 — Modal depth: halo + backdrop dim

**Files:**
- Modify: `src/ui/overlay.rs` — `render_modal_backdrop()` uses halo, `render_dialog()` centers title

---

- [ ] **Step 1: Update `render_modal_backdrop()` to paint a halo ring around the modal**

  Replace the existing `render_modal_backdrop()` function body in `src/ui/overlay.rs`:
  ```rust
  pub fn render_modal_backdrop(
      frame: &mut Frame<'_>,
      area: Rect,
      popup: Rect,
      palette: ThemePalette,
  ) {
      // Full-area dim pass: clear then paint with a dark surface
      frame.render_widget(Clear, area);
      frame.render_widget(
          Paragraph::new("").style(modal_backdrop_style(palette)),
          area,
      );
      // Halo ring: one-cell border around the modal in the halo colour
      let halo = Rect {
          x: popup.x.saturating_sub(1).max(area.x),
          y: popup.y.saturating_sub(1).max(area.y),
          width: (popup.width + 2).min(area.width.saturating_sub(popup.x.saturating_sub(area.x))),
          height: (popup.height + 2).min(area.height.saturating_sub(popup.y.saturating_sub(area.y))),
      };
      frame.render_widget(
          Paragraph::new("").style(crate::ui::styles::modal_halo_style(palette)),
          halo,
      );
  }
  ```

  Update the import at the top of `overlay.rs` to add `modal_halo_style`:
  ```rust
  use crate::ui::styles::{
      elevated_surface_style, modal_backdrop_style, modal_halo_style,
      overlay_footer_style, overlay_key_hint_style, overlay_title_style,
  };
  ```

- [ ] **Step 2: Center modal titles in `render_dialog()`**

  Find the `Block::default().title(...)` call in `render_dialog()` and use `Title::from(...).alignment(Alignment::Center)`:

  First, update the imports at the top of `overlay.rs`:
  ```rust
  use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
  use ratatui::widgets::{
      Block, Borders, Clear, List, ListItem, ListState, Paragraph,
      Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
      block::Title,
  };
  ```

  Then replace the block construction in `render_dialog()`:
  ```rust
  let block = Block::default()
      .title(
          Title::from(Span::styled(dialog.title, overlay_title_style(palette)))
              .alignment(Alignment::Center),
      )
      .borders(Borders::ALL)
      .border_style(Style::default().fg(palette.prompt_border))
      .style(elevated_surface_style(palette));
  ```

  Apply the same centered title pattern to `render_prompt()`, `render_collision_dialog()`, and `render_destructive_confirm()` (find all `Block::default().title(Span::styled(...))` occurrences in overlay.rs and apply `.alignment(Alignment::Center)` via `Title::from(...)`).

- [ ] **Step 3: Verify compilation**
  ```
  cargo check 2>&1 | head -20
  ```

- [ ] **Step 4: Commit**
  ```
  git add src/ui/overlay.rs
  git commit -m "feat: modal halo ring backdrop and centered titles"
  ```

---

## Task 4 — Menu bar: context-aware dimming + workspace CWD hint

**Files:**
- Modify: `src/ui/menu_bar.rs` — `menu_spans()` dims irrelevant items, `workspace_switcher_spans()` shows CWD hint

---

- [ ] **Step 1: Add context-aware dimming to `menu_spans()`**

  `menu_spans()` currently receives `label`, `mnemonic`, `active`, `palette`. Add a `is_relevant: bool` parameter:

  ```rust
  fn menu_spans(
      label: &'static str,
      mnemonic: Option<char>,
      active: bool,
      is_relevant: bool,
      palette: ThemePalette,
  ) -> Vec<Span<'static>> {
      let fg = if !is_relevant {
          palette.text_muted
      } else if active {
          palette.menu_fg
      } else {
          palette.menu_fg
      };
      let style = if active {
          Style::default()
              .fg(fg)
              .bg(palette.menu_active_bg)
              .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
      } else if !is_relevant {
          Style::default().fg(fg).bg(palette.menu_bg)
      } else {
          Style::default().fg(fg).bg(palette.menu_bg)
      };
      let highlighted = mnemonic.map(|v| v.to_ascii_uppercase());
      let mut spans = Vec::with_capacity(label.len());
      let mut used_highlight = false;
      for ch in label.chars() {
          let mut char_style = style;
          if is_relevant && !used_highlight && Some(ch.to_ascii_uppercase()) == highlighted {
              char_style = char_style.fg(palette.menu_mnemonic_fg);
              used_highlight = true;
          }
          spans.push(Span::styled(ch.to_string(), char_style));
      }
      spans
  }
  ```

  Update the call site in `render_menu_bar()`. The relevance of each menu tab depends on context. Define a helper:
  ```rust
  fn tab_is_relevant(tab_id: crate::action::MenuId, ctx: MenuContext) -> bool {
      use crate::action::MenuId;
      match ctx {
          MenuContext::Pane => matches!(tab_id, MenuId::File | MenuId::Navigate | MenuId::View | MenuId::Help | MenuId::Tools),
          MenuContext::Editor | MenuContext::EditorFullscreen => true, // all menus relevant in editor
          MenuContext::Terminal | MenuContext::TerminalFullscreen => {
              matches!(tab_id, MenuId::Navigate | MenuId::View | MenuId::Help | MenuId::Tools)
          }
      }
  }
  ```

  Update the loop in `render_menu_bar()`:
  ```rust
  for tab in menu_tabs(ctx) {
      line.spans.extend(menu_spans(
          tab.label,
          Some(tab.mnemonic),
          state.active_menu() == Some(tab.id),
          tab_is_relevant(tab.id, ctx),
          palette,
      ));
  }
  ```

- [ ] **Step 2: Update `workspace_switcher_spans()` to show CWD hint for active workspace**

  Add a `cwd_hint: Option<&str>` parameter (caller passes truncated CWD):
  ```rust
  pub fn workspace_switcher_spans(
      active_workspace: usize,
      workspace_count: usize,
      cwd_hint: Option<&str>,
      bar_active: bool,
      palette: ThemePalette,
  ) -> Vec<Span<'static>> {
      let bg = if bar_active { palette.menu_active_bg } else { palette.menu_bg };
      let active_style = Style::default()
          .fg(palette.accent_mauve)
          .bg(bg)
          .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
      let inactive_style = Style::default().fg(palette.text_muted).bg(bg);
      let mut spans = Vec::with_capacity(workspace_count * 3 + 4);
      spans.push(Span::styled(" ", Style::default().bg(bg)));
      for idx in 0..workspace_count {
          if idx == active_workspace {
              let label = if let Some(hint) = cwd_hint {
                  format!("{}:{} ", idx + 1, hint)
              } else {
                  format!("[{}] ", idx + 1)
              };
              spans.push(Span::styled(label, active_style));
          } else {
              spans.push(Span::styled(format!("{} ", idx + 1), inactive_style));
          }
      }
      spans
  }
  ```

  Update the call site in `render_menu_bar()`. To get a CWD hint, add a helper that truncates the active pane path:
  ```rust
  let cwd_hint: Option<String> = {
      let cwd = state.panes.active_pane().cwd();
      let home = std::env::var_os("HOME")
          .or_else(|| std::env::var_os("USERPROFILE"))
          .map(std::path::PathBuf::from);
      let display = if let Some(ref h) = home {
          cwd.strip_prefix(h)
              .map(|r| {
                  let s = r.display().to_string();
                  if s.is_empty() { String::from("~") } else { format!("~/{}", s) }
              })
              .unwrap_or_else(|| cwd.display().to_string())
      } else {
          cwd.display().to_string()
      };
      // Keep to 12 chars max, trim from the left
      let chars: Vec<char> = display.chars().collect();
      if chars.len() > 12 {
          Some(format!("…{}", &display[display.char_indices().nth(chars.len() - 11).map(|(i, _)| i).unwrap_or(0)..]))
      } else {
          Some(display)
      }
  };
  line.spans.extend(workspace_switcher_spans(
      state.active_workspace_index(),
      state.workspace_count(),
      cwd_hint.as_deref(),
      bar_is_active,
      palette,
  ));
  ```

  > **Note:** `state.panes.active_pane().cwd()` — verify this method exists. If the method is named differently, check the `PaneState` API (grep for `pub fn cwd` in `src/pane.rs`).

- [ ] **Step 3: Verify compilation**
  ```
  cargo check 2>&1 | head -20
  ```

- [ ] **Step 4: Commit**
  ```
  git add src/ui/menu_bar.rs
  git commit -m "feat: menu bar context dimming and workspace CWD hint"
  ```

---

## Task 5 — Pane entries: Rich Columns header row + improved filter bar

**Files:**
- Modify: `src/ui/pane.rs` — add column header row above list, restyle filter bar

---

- [ ] **Step 1: Add a column header row above the file list (details view only)**

  In `render_pane()`, after computing `list_area` (and optionally `filter_area`), insert a header row when `pane.details_view` is true.

  Locate the section that computes `list_area` and `filter_area`:
  ```rust
  let (list_area, filter_area) = if pane.filter_active { ... } else { (inner, None) };
  ```

  After that block, add:
  ```rust
  let (header_area, list_area) = if pane.details_view && list_area.height > 2 {
      let chunks = Layout::default()
          .direction(Direction::Vertical)
          .constraints([Constraint::Length(1), Constraint::Min(1)])
          .split(list_area);
      (Some(chunks[0]), chunks[1])
  } else {
      (None, list_area)
  };
  ```

  After rendering the list widget and before the filter rendering, add:
  ```rust
  if let Some(ha) = header_area {
      render_column_headers(frame, ha, palette);
  }
  ```

  Add the `render_column_headers` function in `pane.rs`:
  ```rust
  fn render_column_headers(frame: &mut Frame<'_>, area: Rect, palette: ThemePalette) {
      use crate::ui::styles::pane_column_header_style;
      let w = area.width as usize;
      // Fixed right: 9 (size) + 1 (gap) + 16 (date) + 1 (gap) + 3 (git) = 30
      let right_fixed = 30usize;
      let left_fixed = 5usize; // 2 (mark) + 2 (icon+space) + 1 (git) = 5
      let name_width = w.saturating_sub(left_fixed + right_fixed).max(4);
      let header = format!(
          "  {icon:<2}{name:<name_width$}{size:>9} {date:<16} {git:<3}",
          icon = "",
          name = "Name",
          size = "Size",
          date = "Modified",
          git = "Git",
          name_width = name_width,
      );
      let para = Paragraph::new(header).style(pane_column_header_style(palette));
      frame.render_widget(para, area);
  }
  ```

- [ ] **Step 2: Restyle the inline filter bar**

  Find the filter rendering section (near line 222):
  ```rust
  if let Some(filter_area) = filter_area {
      let filter = Paragraph::new(format!(" Filter: {}_", pane.filter_query)).style(
          Style::default()
              .fg(palette.text_primary)
              .bg(palette.selection_bg),
      );
      frame.render_widget(filter, filter_area);
  }
  ```

  Replace with:
  ```rust
  if let Some(filter_area) = filter_area {
      use crate::ui::styles::pane_filter_strip_style;
      let match_count = pane.filtered_count();  // see Step 3 below
      let query_display = format!(" ⌕  {}│", pane.filter_query);
      let count_display = format!(" {} matches  Esc clear", match_count);
      let query_width = query_display.chars().count();
      let count_width = count_display.chars().count();
      let pad = (filter_area.width as usize)
          .saturating_sub(query_width + count_width);
      let line = Line::from(vec![
          Span::styled(query_display, pane_filter_strip_style(palette).add_modifier(Modifier::BOLD)),
          Span::styled(" ".repeat(pad), pane_filter_strip_style(palette)),
          Span::styled(
              count_display,
              Style::default().fg(palette.accent_green).bg(palette.pane_filter_bg),
          ),
      ]);
      frame.render_widget(Paragraph::new(line), filter_area);
  }
  ```

- [ ] **Step 3: Add `filtered_count()` to `PaneState`**

  In `src/pane.rs`, find `PaneState` and add:
  ```rust
  pub fn filtered_count(&self) -> usize {
      if !self.filter_active || self.filter_query.is_empty() {
          return self.entries.len();
      }
      let q = self.filter_query.to_lowercase();
      self.entries.iter().filter(|e| e.name.to_lowercase().contains(&q)).count()
  }
  ```

  > **Note:** Check how the existing filter is implemented in `PaneState`. If `visible_entries()` already applies the filter, use the same logic here.

- [ ] **Step 4: Dim non-matching entries when filter is active**

  In `render_item()`, add a `is_filtered_out: bool` field to `RenderItemArgs`:
  ```rust
  struct RenderItemArgs<'a> {
      // ... existing fields ...
      is_filtered_out: bool,
  }
  ```

  In the `render_pane()` loop that builds items, compute:
  ```rust
  let is_filtered_out = pane.filter_active
      && !pane.filter_query.is_empty()
      && !entry.name.to_lowercase().contains(&pane.filter_query.to_lowercase());
  ```

  Pass `is_filtered_out` in the `RenderItemArgs { ... }` struct.

  In `render_item()`, after building the `row_styles`, apply dimming:
  ```rust
  // At the very top of render_item(), after destructuring:
  if is_filtered_out {
      // Return a dimmed version of the row
      let name = display_name.unwrap_or_else(|| entry.name.clone());
      return ListItem::new(Line::from(Span::styled(
          format!("  {} {}", icon, name),
          Style::default().fg(palette.text_muted),
      )));
  }
  ```

- [ ] **Step 5: Verify compilation**
  ```
  cargo check 2>&1 | head -20
  ```

- [ ] **Step 6: Commit**
  ```
  git add src/ui/pane.rs src/pane.rs
  git commit -m "feat: pane column headers, accent filter bar, dim non-matching entries"
  ```

---

## Task 6 — Status bar: structured zone rendering

**Files:**
- Modify: `src/state/mod.rs` — add `StatusZones` struct + `status_zones()` method (keep `status_line()` for tests)
- Modify: `src/ui/mod.rs` — replace single-paragraph status bar with multi-span zoned line

---

- [ ] **Step 1: Add `StatusZones` to `src/state/mod.rs`**

  Add the following struct definitions near the top of the file (after existing imports, before `impl AppState`):
  ```rust
  /// Structured data for the four status bar zones.
  #[derive(Clone, Debug)]
  pub struct StatusZones {
      pub git_branch: Option<String>,
      pub entry_detail: Option<String>,
      pub message: String,
      pub marks: Option<MarksInfo>,
      pub progress: Option<FileOpProgress>,
      pub workspace: String,
  }

  #[derive(Clone, Debug)]
  pub struct MarksInfo {
      pub count: usize,
      pub total_bytes: u64,
  }

  #[derive(Clone, Debug)]
  pub struct FileOpProgress {
      pub operation: String,
      pub current: u64,
      pub total: u64,
      pub current_name: String,
  }
  ```

- [ ] **Step 2: Add `status_zones()` method to `AppState`**

  Add this method after `status_line()`:
  ```rust
  pub fn status_zones(&self) -> StatusZones {
      let git_branch = {
          let active_pane_id = match self.panes.focus {
              PaneFocus::Left | PaneFocus::Preview => crate::pane::PaneId::Left,
              PaneFocus::Right => crate::pane::PaneId::Right,
          };
          self.git_status(active_pane_id)
              .map(|g| format!(" ⎇ {} ", g.branch))
      };

      let entry_detail = self
          .panes
          .active_pane()
          .selected_entry()
          .map(|e| {
              let icon = crate::icon::icon_for_entry(
                  e.kind,
                  e.path.extension().and_then(|x| x.to_str()),
                  self.icon_mode,
              );
              let name = e.path.file_name()
                  .and_then(|n| n.to_str())
                  .unwrap_or(&e.name);
              let size_str = e.size_bytes.map(format_file_size).unwrap_or_default();
              #[cfg(unix)]
              let perms = format_permissions_unix(&e.path);
              #[cfg(not(unix))]
              let perms = String::new();
              if perms.is_empty() {
                  format!(" {} {}  {} ", icon, name, size_str)
              } else {
                  format!(" {} {}  {} {} ", icon, name, perms, size_str)
              }
          });

      let marks = {
          let pane = self.panes.active_pane();
          let count = pane.marked_count();
          if count > 0 {
              let total_bytes: u64 = pane.marked
                  .iter()
                  .filter_map(|path| {
                      pane.entries.iter().find(|e| &e.path == path).and_then(|e| e.size_bytes)
                  })
                  .sum();
              Some(MarksInfo { count, total_bytes })
          } else {
              None
          }
      };

      let progress = self.file_operation_status.as_ref().map(|status| {
          let current_name = status.current_path
              .file_name()
              .and_then(|v| v.to_str())
              .unwrap_or(".")
              .to_string();
          FileOpProgress {
              operation: status.operation.to_string(),
              current: status.completed,
              total: status.total,
              current_name,
          }
      });

      let workspace = format!(
          " ws:{}/{} ",
          self.active_workspace_index() + 1,
          self.workspace_count()
      );

      StatusZones {
          git_branch,
          entry_detail,
          message: format!(" {} ", self.status_message),
          marks,
          progress,
          workspace,
      }
  }
  ```

- [ ] **Step 3: Write a unit test for `status_zones()`**

  Add to the tests module in `src/state/mod.rs` (or in a nearby `tests` submodule):
  ```rust
  #[test]
  fn status_zones_workspace_format() {
      // Workspace string is always "ws:N/M " style
      let ws = format!(" ws:{}/{} ", 1, 4);
      assert!(ws.starts_with(" ws:"));
      assert!(ws.contains('/'));
  }
  ```

- [ ] **Step 4: Run the test**
  ```
  cargo test status_zones
  ```
  Expected: pass.

- [ ] **Step 5: Update `src/ui/mod.rs` to render zones**

  Find the status bar rendering section (around line 372):
  ```rust
  let status = Paragraph::new(Line::raw(state.status_line())).style(
      Style::default()
          .fg(palette.status_fg)
          .bg(palette.status_bg)
          .add_modifier(Modifier::BOLD),
  );
  frame.render_widget(status, areas[2]);
  ```

  Replace with a call to a new helper function:
  ```rust
  render_status_bar(frame, areas[2], state, palette);
  ```

  Add the `render_status_bar` function (in `src/ui/mod.rs` or in a new `src/ui/status_bar.rs`):
  ```rust
  fn render_status_bar(
      frame: &mut Frame<'_>,
      area: Rect,
      state: &AppState,
      palette: crate::config::ThemePalette,
  ) {
      let zones = state.status_zones();
      let mut spans = Vec::new();

      if let Some(ref progress) = zones.progress {
          // Progress mode: full-width bar showing current operation
          let op_text = format!(
              " {} {}/{} — {} ",
              progress.operation,
              progress.current,
              progress.total,
              progress.current_name,
          );
          let bar_width = (area.width as usize).saturating_sub(op_text.len() + 2);
          let filled = if progress.total > 0 {
              (progress.current * bar_width as u64 / progress.total) as usize
          } else {
              0
          };
          let empty = bar_width.saturating_sub(filled);
          spans.push(Span::styled(
              format!("{}{}{} ", op_text, "─".repeat(filled), "░".repeat(empty)),
              Style::default().fg(palette.status_fg).bg(palette.status_bg),
          ));
      } else {
          // Normal mode: git | entry | message | [marks] | workspace
          if let Some(ref git) = zones.git_branch {
              spans.push(Span::styled(
                  git.clone(),
                  Style::default().fg(palette.border_focus).bg(palette.status_git_bg),
              ));
              spans.push(Span::styled("│", Style::default().fg(palette.modal_halo).bg(palette.status_bg)));
          }
          if let Some(ref entry) = zones.entry_detail {
              spans.push(Span::styled(
                  entry.clone(),
                  Style::default().fg(palette.text_primary).bg(palette.status_entry_bg),
              ));
              spans.push(Span::styled("│", Style::default().fg(palette.modal_halo).bg(palette.status_bg)));
          }
          // Message zone (expands to fill)
          spans.push(Span::styled(
              zones.message.clone(),
              Style::default().fg(palette.text_subtext).bg(palette.status_bg),
          ));
          if let Some(ref marks) = zones.marks {
              let size_str = if marks.total_bytes > 0 {
                  format!(" ✦ {} · {} ", marks.count, format_size(marks.total_bytes))
              } else {
                  format!(" ✦ {} ", marks.count)
              };
              spans.push(Span::styled("│", Style::default().fg(palette.modal_halo).bg(palette.status_bg)));
              spans.push(Span::styled(
                  size_str,
                  Style::default().fg(palette.accent_yellow).bg(palette.status_bg),
              ));
          }
          spans.push(Span::styled("│", Style::default().fg(palette.modal_halo).bg(palette.status_bg)));
          spans.push(Span::styled(
              zones.workspace.clone(),
              Style::default().fg(palette.accent_mauve).bg(palette.status_workspace_bg).add_modifier(Modifier::BOLD),
          ));
      }

      frame.render_widget(
          Paragraph::new(Line::from(spans)),
          area,
      );
  }

  fn format_size(bytes: u64) -> String {
      if bytes >= 1_073_741_824 {
          format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
      } else if bytes >= 1_048_576 {
          format!("{:.1} MB", bytes as f64 / 1_048_576.0)
      } else if bytes >= 1024 {
          format!("{:.0} KB", bytes as f64 / 1024.0)
      } else {
          format!("{} B", bytes)
      }
  }
  ```

  Add the import for `AppState` and `StatusZones` at the top if needed.

- [ ] **Step 6: Verify compilation**
  ```
  cargo check 2>&1 | head -20
  ```

- [ ] **Step 7: Commit**
  ```
  git add src/state/mod.rs src/ui/mod.rs
  git commit -m "feat: zoned status bar with git, entry, marks, and workspace segments"
  ```

---

## Task 7 — Panel chrome: rich title bars for Editor, Preview, Terminal

**Files:**
- Modify: `src/ui/editor.rs` — rich title with dirty indicator, line/col, type badge
- Modify: `src/ui/preview.rs` — rich title with file type hint and type badge
- Modify: `src/ui/terminal.rs` — rich title with shell name and type badge

---

- [ ] **Step 1: Enrich the editor title in `src/ui/editor.rs`**

  Find the title construction in `render_editor()` (around line 97):
  ```rust
  let dirty_marker = if editor.is_dirty { "*" } else { "" };
  let title = format!("Editor{}  {}", dirty_marker, path);
  let block = Block::default()
      .title(title)
      .borders(Borders::ALL)
      .border_style(border_style);
  ```

  Replace with:
  ```rust
  use ratatui::widgets::block::Title;
  use ratatui::layout::Alignment;
  use crate::ui::styles::{panel_title_focused_style, panel_title_unfocused_style, dirty_indicator_style};

  let filename = editor.path.as_ref()
      .and_then(|p| p.file_name())
      .map(|n| n.to_string_lossy().into_owned())
      .unwrap_or_else(|| String::from("[untitled]"));
  let parent = editor.path.as_ref()
      .and_then(|p| p.parent())
      .map(|d| d.display().to_string())
      .unwrap_or_default();
  let dirty_part = if editor.is_dirty { " ● " } else { "   " };
  let ln_col = format!(
      " Ln {} · Col {} ",
      render_state.cursor_row + 1,
      render_state.cursor_col + 1
  );
  let accent = palette.border_editor_focus;
  let title_style = if is_focused {
      panel_title_focused_style(accent)
  } else {
      panel_title_unfocused_style(palette)
  };
  let dirty_style = if editor.is_dirty {
      dirty_indicator_style(palette)
  } else {
      title_style
  };
  let badge_style = Style::default()
      .fg(palette.surface_bg)
      .bg(accent)
      .add_modifier(Modifier::BOLD);
  let title_spans: Line = Line::from(vec![
      Span::styled(format!(" 󰈙 {} ", filename), title_style),
      Span::styled(dirty_part, dirty_style),
      Span::styled(format!(" {} ", parent), Style::default().fg(palette.text_muted)),
      Span::styled(ln_col, Style::default().fg(palette.text_subtext)),
      Span::styled(" Editor ", badge_style),
  ]);
  let block = Block::default()
      .title(Title::from(title_spans))
      .borders(Borders::ALL)
      .border_style(border_style);
  ```

  > **Note:** `render_state.cursor_row` and `render_state.cursor_col` — check `EditorRenderState` for the correct field names. If they don't exist, use `editor.cursor_line()` and `editor.cursor_col()` or equivalent methods.

- [ ] **Step 2: Enrich the preview panel title in `src/ui/preview.rs`**

  Find `render_preview_panel()` (around line 200). Update the title construction:
  ```rust
  let accent = palette.accent_teal;
  let title_style = if is_focused {
      panel_title_focused_style(accent)
  } else {
      panel_title_unfocused_style(palette)
  };
  let badge_style = Style::default()
      .fg(palette.surface_bg)
      .bg(accent)
      .add_modifier(Modifier::BOLD);
  let ext_hint = args.path.as_ref()
      .and_then(|p| p.extension())
      .and_then(|e| e.to_str())
      .unwrap_or("")
      .to_ascii_uppercase();
  let title_spans: Line = Line::from(vec![
      Span::styled(format!(" 󰿃 {} ", filename), title_style),
      Span::styled(
          if ext_hint.is_empty() { String::new() } else { format!(" .{} ", ext_hint) },
          Style::default().fg(palette.text_muted)
      ),
      Span::styled(" Preview ", badge_style),
  ]);
  let block = Block::default()
      .title(Title::from(title_spans))
      .borders(Borders::ALL)
      .border_style(border_style);
  ```

  Add needed imports to `preview.rs`:
  ```rust
  use ratatui::widgets::block::Title;
  use ratatui::layout::Alignment;
  use crate::ui::styles::{panel_title_focused_style, panel_title_unfocused_style};
  ```

- [ ] **Step 3: Enrich the terminal panel title in `src/ui/terminal.rs`**

  Find the `Block::default().title(" Terminal ")` block and update:
  ```rust
  use ratatui::widgets::block::Title;
  use crate::ui::styles::{panel_title_focused_style, panel_title_unfocused_style};
  let accent = palette.accent_green;
  let title_style = if focused {
      panel_title_focused_style(accent)
  } else {
      panel_title_unfocused_style(palette)
  };
  let badge_style = Style::default()
      .fg(palette.surface_bg)
      .bg(accent)
      .add_modifier(Modifier::BOLD);
  let title_spans = Line::from(vec![
      Span::styled(" 󰆍 Terminal ", title_style),
      Span::styled(" Shell ", badge_style),
  ]);
  let block = Block::default()
      .title(Title::from(title_spans))
      .borders(Borders::ALL)
      .border_style(...);  // keep existing border_style logic
  ```

- [ ] **Step 4: Verify compilation**
  ```
  cargo check 2>&1 | head -20
  ```

- [ ] **Step 5: Commit**
  ```
  git add src/ui/editor.rs src/ui/preview.rs src/ui/terminal.rs
  git commit -m "feat: rich panel chrome titles for editor, preview, and terminal"
  ```

---

## Task 8 — Settings: segmented tab panel

**Files:**
- Modify: `src/state/settings.rs` — add `SettingsTab` enum + `active_tab` to `SettingsState`
- Modify: `src/state/mod.rs` — `settings_entries()` accepts tab, `apply_settings_change()` handles tab actions
- Modify: `src/ui/settings.rs` — render tab bar + filter entries by active tab

---

- [ ] **Step 1: Add `SettingsTab` and update `SettingsState` in `src/state/settings.rs`**

  ```rust
  #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
  pub enum SettingsTab {
      #[default]
      Appearance,
      Panels,
      Editor,
      Keymaps,
  }

  impl SettingsTab {
      pub fn label(self) -> &'static str {
          match self {
              Self::Appearance => "1 Appearance",
              Self::Panels => "2 Panels",
              Self::Editor => "3 Editor",
              Self::Keymaps => "4 Keymaps",
          }
      }

      pub fn next(self) -> Self {
          match self {
              Self::Appearance => Self::Panels,
              Self::Panels => Self::Editor,
              Self::Editor => Self::Keymaps,
              Self::Keymaps => Self::Appearance,
          }
      }

      pub fn prev(self) -> Self {
          match self {
              Self::Appearance => Self::Keymaps,
              Self::Panels => Self::Appearance,
              Self::Editor => Self::Panels,
              Self::Keymaps => Self::Editor,
          }
      }

      pub fn from_number(n: usize) -> Option<Self> {
          match n {
              1 => Some(Self::Appearance),
              2 => Some(Self::Panels),
              3 => Some(Self::Editor),
              4 => Some(Self::Keymaps),
              _ => None,
          }
      }
  }
  ```

  Update `SettingsState` to add `active_tab`:
  ```rust
  pub struct SettingsState {
      pub selection: usize,
      pub rebind_mode: Option<usize>,
      pub active_tab: SettingsTab,
  }
  ```

  Update `SettingsState::new()`:
  ```rust
  pub fn new() -> Self {
      Self {
          selection: 0,
          rebind_mode: None,
          active_tab: SettingsTab::default(),
      }
  }
  ```

- [ ] **Step 2: Add a unit test for `SettingsTab` cycling**

  ```rust
  #[cfg(test)]
  mod tests {
      use super::SettingsTab;

      #[test]
      fn settings_tab_cycles_forward() {
          assert_eq!(SettingsTab::Appearance.next(), SettingsTab::Panels);
          assert_eq!(SettingsTab::Keymaps.next(), SettingsTab::Appearance);
      }

      #[test]
      fn settings_tab_from_number() {
          assert_eq!(SettingsTab::from_number(1), Some(SettingsTab::Appearance));
          assert_eq!(SettingsTab::from_number(5), None);
      }
  }
  ```

- [ ] **Step 3: Run the tests**
  ```
  cargo test settings_tab
  ```
  Expected: 2 tests pass.

- [ ] **Step 4: Update `settings_entries()` in `src/state/mod.rs` to filter by tab**

  Change the signature:
  ```rust
  pub fn settings_entries_for_tab(&self, tab: crate::state::SettingsTab) -> Vec<SettingsEntry> {
  ```

  Then wrap the entries into groups. Use `SettingsTab` to filter which entries are returned:
  ```rust
  pub fn settings_entries_for_tab(&self, tab: SettingsTab) -> Vec<SettingsEntry> {
      let all = self.settings_entries();
      match tab {
          SettingsTab::Appearance => all.into_iter().filter(|e| matches!(
              e.field,
              SettingsField::Theme(_) | SettingsField::IconMode(_)
          )).collect(),
          SettingsTab::Panels => all.into_iter().filter(|e| matches!(
              e.field,
              SettingsField::PaneLayout(_) | SettingsField::PreviewPanel
              | SettingsField::PreviewOnSelection | SettingsField::TerminalOpenByDefault
          )).collect(),
          SettingsTab::Editor => all.into_iter().filter(|e| matches!(
              e.field,
              SettingsField::EditorTabWidth(_) | SettingsField::EditorWordWrap
          )).collect(),
          SettingsTab::Keymaps => all.into_iter().filter(|e| matches!(
              e.field,
              SettingsField::KeymapBinding { .. }
          )).collect(),
      }
  }
  ```

  Keep the original `settings_entries()` unchanged for backward compatibility (it is used in other event-handling code).

- [ ] **Step 5: Handle Tab/Shift+Tab/1-4 for settings tab navigation**

  In the settings input handler in `src/state/mod.rs` (find the section that handles `ModalKind::Settings` key events), add:
  ```rust
  // Tab → next tab
  KeyCode::Tab => {
      if let Some(s) = self.settings_mut() {
          s.active_tab = s.active_tab.next();
          s.selection = 0;
      }
  }
  // Shift+Tab → prev tab
  KeyCode::BackTab => {
      if let Some(s) = self.settings_mut() {
          s.active_tab = s.active_tab.prev();
          s.selection = 0;
      }
  }
  // Number keys 1-4 → jump to tab
  KeyCode::Char(ch @ '1'..='4') => {
      let n = (ch as usize) - ('0' as usize);
      if let Some(s) = self.settings_mut() {
          if let Some(tab) = SettingsTab::from_number(n) {
              s.active_tab = tab;
              s.selection = 0;
          }
      }
  }
  ```

  > **Note:** Find the correct location in mod.rs where settings key events are dispatched. Search for `ModalKind::Settings` in the input handler. Add a `settings_mut()` helper method to `AppState` if it doesn't already exist:
  ```rust
  fn settings_mut(&mut self) -> Option<&mut SettingsState> {
      self.overlay.modal_mut().and_then(|m| {
          if let crate::state::overlay::ModalState::Settings(ref mut s) = m {
              Some(s)
          } else {
              None
          }
      })
  }
  ```

- [ ] **Step 6: Update `render_settings_panel()` in `src/ui/settings.rs` to render tabs**

  Replace the header section (the intro paragraph in `chunks[0]`) with a tab bar:
  ```rust
  // Tab bar row
  let tab_bar_spans: Vec<Span> = [
      SettingsTab::Appearance,
      SettingsTab::Panels,
      SettingsTab::Editor,
      SettingsTab::Keymaps,
  ]
  .iter()
  .flat_map(|&tab| {
      let is_active = tab == settings.active_tab;
      let style = if is_active {
          Style::default()
              .fg(palette.selection_fg)
              .bg(palette.border_focus)
              .add_modifier(Modifier::BOLD)
      } else {
          Style::default()
              .fg(palette.text_muted)
              .bg(palette.tools_bg)
      };
      vec![
          Span::styled(format!(" {} ", tab.label()), style),
          Span::styled("  ", Style::default().bg(palette.tools_bg)),
      ]
  })
  .collect();
  frame.render_widget(
      Paragraph::new(Line::from(tab_bar_spans)),
      chunks[0],
  );
  ```

  Update the entries list to use `settings_entries_for_tab()`:
  ```rust
  let entries = state.settings_entries_for_tab(settings.active_tab);
  ```

  Add the import at the top of `settings.rs`:
  ```rust
  use crate::state::{AppState, SettingsState, SettingsTab, SettingsField};
  ```

- [ ] **Step 7: Verify compilation**
  ```
  cargo check 2>&1 | head -20
  ```

- [ ] **Step 8: Commit**
  ```
  git add src/state/settings.rs src/state/mod.rs src/ui/settings.rs
  git commit -m "feat: settings panel segmented tabs (Appearance/Panels/Editor/Keymaps)"
  ```

---

## Task 9 — Help and About modals: two-column help, polished About

**Files:**
- Modify: `src/state/dialog.rs` — `help()` restructured for two-column, `about()` adds `##LOGO` prefix
- Modify: `src/ui/overlay.rs` — `render_dialog()` handles `##LOGO` prefix in mauve + two-column help layout

---

- [ ] **Step 1: Update `DialogState::help()` in `src/state/dialog.rs`**

  Replace the `help()` function body. The two-column layout is driven by a `##COL2` marker that splits left/right columns:

  ```rust
  pub fn help() -> Self {
      Self {
          title: " Help ",
          lines: vec![
              // LEFT column (Navigation + Files)
              String::from("##COLSTART"),
              String::from("## Navigation"),
              String::from("  ↑/↓ j/k\tMove selection"),
              String::from("  Enter\tOpen file or directory"),
              String::from("  Backspace\tGo to parent"),
              String::from("  Tab\tSwitch pane"),
              String::from("  PgUp/PgDn\tScroll page"),
              String::from("  Alt+1..4\tSwitch workspace"),
              String::from("  Click\tSelect with mouse"),
              String::new(),
              String::from("## Files"),
              String::from("  F5\tCopy"),
              String::from("  F6/Shift+F6\tRename / Move"),
              String::from("  F8\tDelete (trash)"),
              String::from("  Ins\tNew file"),
              String::from("  Shift+F7\tNew directory"),
              String::from("  Ctrl+D\tDiff-sync to other pane"),
              String::from("  /\tFilter pane entries"),
              String::new(),
              // RIGHT column (Editor + System)
              String::from("##COLBREAK"),
              String::from("## Editor"),
              String::from("  F4\tOpen in editor"),
              String::from("  Ctrl+S\tSave"),
              String::from("  Ctrl+D\tDiscard changes"),
              String::from("  Ctrl+F\tFind in file"),
              String::from("  F3/Shift+F3\tNext / prev match"),
              String::from("  Esc\tClose search or editor"),
              String::new(),
              String::from("## System"),
              String::from("  F1 / Ctrl+O\tHelp / Settings"),
              String::from("  F2\tToggle terminal"),
              String::from("  F3 / Ctrl+W\tToggle preview"),
              String::from("  F9\tDiff mode"),
              String::from("  F10 / q\tQuit"),
              String::from("  Shift+P\tCommand palette"),
              String::from("  Esc / Enter\tClose dialogs"),
              String::from("##COLEND"),
          ],
          scroll: 0,
      }
  }
  ```

- [ ] **Step 2: Add `##LOGO` prefix support to `about()` in `src/state/dialog.rs`**

  In `DialogState::about()`, prefix every ASCII art line with `##LOGO`:
  ```rust
  String::from("##LOGO ____  ________  ____             __               "),
  String::from("##LOGO|    \\|        \\|    \\           |  \\              "),
  String::from("##LOGO| $$$$ \\$$$$$$$$ \\$$$$  ______  _| $$_     ______  "),
  String::from("##LOGO| $$      /  $$   | $$ /      \\|   $$ \\   |      \\ "),
  String::from("##LOGO| $$     /  $$    | $$|  $$$$$$\\\\$$$$$$    \\$$$$$$\\"),
  String::from("##LOGO| $$    /  $$     | $$| $$    $$ | $$ __  /      $$"),
  String::from("##LOGO| $$_  /  $$___  _| $$| $$$$$$$$ | $$|  \\|  $$$$$$$"),
  String::from("##LOGO| $$ \\|  $$    \\|   $$ \\$$     \\  \\$$  $$ \\$$    $$"),
  String::from("##LOGO \\$$$$ \\$$$$$$$$ \\$$$$  \\$$$$$$$   \\$$$$   \\$$$$$$$"),
  ```
  Update the `Icons` tip line to reflect the new NerdFont mode:
  ```rust
  String::from("  Icons\tNerdFont (recommended); Unicode and ASCII fallbacks available"),
  String::from("  Tip\tSet icon_mode = \"nerd_font\" in config.toml"),
  ```

- [ ] **Step 3: Update `render_dialog()` in `src/ui/overlay.rs` to handle new markers**

  In the `styled_lines` map in `render_dialog()`, add handling for `##LOGO` before the existing `##` branch:

  ```rust
  } else if let Some(art) = raw.strip_prefix("##LOGO") {
      Line::from(Span::styled(
          art.to_string(),
          Style::default()
              .fg(palette.accent_mauve)
              .add_modifier(Modifier::BOLD),
      ))
  } else if let Some(header) = raw.strip_prefix("##") {
      // existing section header handling
  ```

  For the two-column help layout, add a new dedicated render path. After the `styled_lines` and `paragraph` rendering, check if the dialog is the Help dialog and use a two-column layout:

  Add a helper function in `overlay.rs`:
  ```rust
  fn render_two_column_help(
      frame: &mut Frame<'_>,
      inner: Rect,
      lines: &[String],
      scroll: u16,
      palette: ThemePalette,
  ) {
      // Split COLSTART..COLBREAK as left column, COLBREAK..COLEND as right column
      let mut left: Vec<&str> = Vec::new();
      let mut right: Vec<&str> = Vec::new();
      let mut in_left = false;
      let mut in_right = false;
      for line in lines {
          if line == "##COLSTART" { in_left = true; continue; }
          if line == "##COLBREAK" { in_left = false; in_right = true; continue; }
          if line == "##COLEND" { break; }
          if in_left { left.push(line.as_str()); }
          if in_right { right.push(line.as_str()); }
      }

      let halves = Layout::default()
          .direction(Direction::Horizontal)
          .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
          .split(inner);

      let render_col = |col_lines: &[&str], area: Rect| -> Vec<Line<'static>> {
          col_lines.iter().map(|raw| {
              if raw.is_empty() {
                  Line::raw("")
              } else if let Some(header) = raw.strip_prefix("## ") {
                  Line::from(Span::styled(
                      header.to_string(),
                      section_divider_style(palette),
                  ))
              } else if let Some((key, desc)) = raw.split_once('\t') {
                  let key_part = key.trim_start();
                  Line::from(vec![
                      Span::styled(
                          format!(" {} ", key_part.trim()),
                          key_pill_style(palette),
                      ),
                      Span::raw("  "),
                      Span::styled(desc.to_string(), Style::default().fg(palette.text_primary)),
                  ])
              } else {
                  Line::from(Span::styled(
                      raw.to_string(),
                      Style::default().fg(palette.text_primary),
                  ))
              }
          }).collect()
      };

      let left_lines = render_col(&left, halves[0]);
      let right_lines = render_col(&right, halves[1]);
      frame.render_widget(
          Paragraph::new(left_lines).scroll((scroll, 0)),
          halves[0],
      );
      frame.render_widget(
          Paragraph::new(right_lines).scroll((scroll, 0)),
          halves[1],
      );
  }
  ```

  Add imports for `key_pill_style` and `section_divider_style` in `overlay.rs`:
  ```rust
  use crate::ui::styles::{
      elevated_surface_style, modal_backdrop_style, modal_halo_style,
      overlay_footer_style, overlay_key_hint_style, overlay_title_style,
      key_pill_style, section_divider_style,
  };
  ```

  In `render_dialog()`, detect whether to use two-column rendering:
  ```rust
  let has_two_col = dialog.lines.iter().any(|l| l == "##COLSTART");
  if has_two_col {
      render_two_column_help(frame, inner, &dialog.lines, scroll, palette);
  } else {
      // existing single-column rendering
      let paragraph = Paragraph::new(styled_lines)
          .style(elevated_surface_style(palette))
          .scroll((scroll, 0));
      frame.render_widget(paragraph, inner);
  }
  ```

- [ ] **Step 4: Verify compilation**
  ```
  cargo check 2>&1 | head -20
  ```

- [ ] **Step 5: Commit**
  ```
  git add src/state/dialog.rs src/ui/overlay.rs
  git commit -m "feat: two-column help modal with key pills and mauve About logo"
  ```

---

## Task 10 — Command palette and file finder: visual polish

**Files:**
- Modify: `src/ui/palette.rs` — `⌕` input prefix, match-char highlight, category badge colour
- Modify: `src/ui/finder.rs` — `⌕ teal` input, teal match highlight, root hint

---

- [ ] **Step 1: Update `render_command_palette()` in `src/ui/palette.rs`**

  3a. Update the input row (find `let input_line = format!("> {}_", state.query);`):
  ```rust
  let input_line = Line::from(vec![
      Span::styled(" ⌕  ", Style::default().fg(palette.border_focus).add_modifier(Modifier::BOLD)),
      Span::styled(state.query.clone(), Style::default().fg(palette.text_primary).add_modifier(Modifier::BOLD)),
      Span::styled("│", Style::default().fg(palette.text_muted)),
  ]);
  let input = Paragraph::new(input_line).style(
      Style::default().fg(palette.text_primary).bg(palette.tools_bg),
  );
  ```

  3b. Update the footer line:
  ```rust
  let footer = Paragraph::new("  Enter run  ·  Esc close")
      .style(overlay_footer_style(palette));
  ```

  3c. In the `Row::Entry(entry)` rendering section, add match highlighting for chars in the query. After computing `label_text`, split it into matched/unmatched spans:
  ```rust
  use crate::ui::styles::match_highlight_style;
  // Build spans: chars matching query get accent_yellow, others get normal style
  let query_lower = state.query.to_lowercase();
  let mut label_spans: Vec<Span> = Vec::new();
  if query_lower.is_empty() {
      label_spans.push(Span::styled(label_text.clone() + &padding, label_style));
  } else {
      let mut remaining = label_text.as_str();
      let mut out = String::new();
      while !remaining.is_empty() {
          if let Some(pos) = remaining.to_lowercase().find(&query_lower) {
              if pos > 0 {
                  out.push_str(&remaining[..pos]);
                  label_spans.push(Span::styled(std::mem::take(&mut out), label_style));
              }
              let matched = &remaining[pos..pos + query_lower.len()];
              label_spans.push(Span::styled(matched.to_string(), match_highlight_style(palette)));
              remaining = &remaining[pos + query_lower.len()..];
          } else {
              out.push_str(remaining);
              break;
          }
      }
      if !out.is_empty() {
          label_spans.push(Span::styled(out, label_style));
      }
      label_spans.push(Span::styled(padding, label_style));
  }
  // Add category badge per entry
  use crate::ui::styles::category_badge_style;
  let mut line_spans: Vec<Span> = vec![Span::raw(" ")];
  line_spans.extend(label_spans);
  line_spans.push(Span::raw("  "));
  line_spans.push(Span::styled(hint.to_string(), hint_style));
  line_spans.push(Span::raw(" "));
  let line = Line::from(line_spans);
  ListItem::new(line)
  ```

- [ ] **Step 2: Update `render_file_finder()` in `src/ui/finder.rs`**

  2a. Update input row styling:
  ```rust
  let input = Paragraph::new(vec![
      Line::from(vec![
          Span::styled(" ⌕  ", Style::default().fg(palette.accent_teal).add_modifier(Modifier::BOLD)),
          Span::styled(state.query.clone(), Style::default().fg(palette.text_primary).add_modifier(Modifier::BOLD)),
          Span::styled("│", Style::default().fg(palette.text_muted)),
      ]),
      Line::from(Span::styled(
          format!(" root: {} ", state.root.display()),
          Style::default().fg(palette.text_muted),
      )),
  ])
  .style(Style::default().fg(palette.text_primary).bg(palette.tools_bg));
  ```

  2b. Update match item styling. In the items loop, change how selected items look and add teal match highlight:
  ```rust
  use crate::ui::styles::finder_match_highlight_style;
  let base = if is_selected {
      Style::default()
          .fg(palette.selection_fg)
          .bg(palette.selection_bg)
          .add_modifier(Modifier::BOLD)
  } else {
      Style::default().fg(palette.text_primary)
  };
  let dir_style = Style::default().fg(palette.text_muted);
  let query_lower = state.query.to_lowercase();

  // Show dir + filename with teal highlights on matched chars
  let dir_part = if let Some(parent) = std::path::Path::new(&rel).parent() {
      let s = parent.display().to_string();
      if s.is_empty() || s == "." { String::new() } else { format!("{}/", s) }
  } else {
      String::new()
  };
  let mut spans: Vec<Span> = vec![
      Span::styled(" ", base),
      Span::styled(dir_part, dir_style),
  ];
  // Highlight matched chars in filename
  let fname = filename;
  if query_lower.is_empty() {
      spans.push(Span::styled(fname.to_string(), base.add_modifier(Modifier::BOLD)));
  } else {
      let mut rem = fname;
      while !rem.is_empty() {
          if let Some(pos) = rem.to_lowercase().find(&query_lower) {
              if pos > 0 {
                  spans.push(Span::styled(rem[..pos].to_string(), base));
              }
              spans.push(Span::styled(rem[pos..pos + query_lower.len()].to_string(), finder_match_highlight_style(palette)));
              rem = &rem[pos + query_lower.len()..];
          } else {
              spans.push(Span::styled(rem.to_string(), base));
              break;
          }
      }
  }
  ListItem::new(Line::from(spans))
  ```

  2c. Update footer:
  ```rust
  let footer = Paragraph::new("  Enter open  ·  Ctrl+Enter open in editor  ·  Esc close")
      .style(overlay_footer_style(palette));
  ```

- [ ] **Step 3: Verify compilation**
  ```
  cargo check 2>&1 | head -20
  ```

- [ ] **Step 4: Run all tests**
  ```
  cargo test --workspace
  ```
  Expected: all tests pass.

- [ ] **Step 5: Run clippy**
  ```
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  ```
  Fix any warnings.

- [ ] **Step 6: Run formatter**
  ```
  cargo fmt --all
  ```

- [ ] **Step 7: Final commit**
  ```
  git add src/ui/palette.rs src/ui/finder.rs
  git commit -m "feat: command palette match highlighting and teal file finder accent"
  ```

---

## Final Validation

After all 10 tasks are complete:

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] Visually launch Zeta and verify each surface:
  - [ ] CatppuccinMocha theme (`ctrl+O` → Appearance → set theme to `catppuccin_mocha`)
  - [ ] NerdFont icons visible (`ctrl+O` → set icon_mode to `nerd_font`)
  - [ ] Modal halo visible on any dialog (F1 Help, F8 Delete)
  - [ ] Help modal shows two columns
  - [ ] About modal logo is in mauve
  - [ ] Status bar shows coloured zones
  - [ ] Settings shows 4 tabs (1–4 jump, Tab cycles)
  - [ ] Filter bar (`/`) shows accent strip with match count
  - [ ] Palette (`Shift+P`) shows match highlights
- [ ] Update CHANGELOG.md with all changes under a new `## [v0.5.0]` section
- [ ] Commit: `git commit -m "chore: bump version to 0.5.0"`
