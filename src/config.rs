use std::env;
use std::fs as std_fs;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::action::KeyBinding;
use crate::state::PaneLayout;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemePreset {
    Fjord,
    Sandbar,
    Oxide,
    Matrix,
    Norton,
    Neon,
    Monochrome,
    Dracula,
    Zeta,
    CatppuccinMocha,
}

impl ThemePreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fjord => "fjord",
            Self::Sandbar => "sandbar",
            Self::Oxide => "oxide",
            Self::Matrix => "matrix",
            Self::Norton => "norton",
            Self::Neon => "neon",
            Self::Monochrome => "monochrome",
            Self::Dracula => "dracula",
            Self::Zeta => "zeta",
            Self::CatppuccinMocha => "catppuccin_mocha",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppConfig {
    pub theme: ThemeConfig,
    pub keymap: KeymapConfig,
    #[serde(default)]
    pub icon_mode: IconMode,
    #[serde(default)]
    pub pane_layout: PaneLayout,
    #[serde(default)]
    pub preview_panel_open: bool,
    #[serde(default = "default_preview_on_selection")]
    pub preview_on_selection: bool,
    #[serde(default)]
    pub bookmarks: Vec<PathBuf>,
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub terminal_open_by_default: bool,
    #[serde(default)]
    pub openers: Vec<OpenerConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            keymap: KeymapConfig::default(),
            icon_mode: IconMode::default(),
            pane_layout: PaneLayout::default(),
            preview_panel_open: false,
            preview_on_selection: true,
            bookmarks: Vec::new(),
            editor: EditorConfig::default(),
            terminal_open_by_default: false,
            openers: Vec::new(),
        }
    }
}

/// A custom "open with" program entry defined in the user config.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenerConfig {
    /// Display name shown in the open-with menu.
    pub name: String,
    /// Shell command to run. Use `{}` as a placeholder for the file path;
    /// if absent, the path is appended as the last argument.
    pub command: String,
    /// File extensions this opener applies to (e.g. `["rs", "toml"]`).
    /// An empty list means the opener matches every file type.
    #[serde(default)]
    pub extensions: Vec<String>,
}

impl OpenerConfig {
    /// Return `true` if this opener applies to the given file extension.
    pub fn matches_extension(&self, ext: &str) -> bool {
        self.extensions.is_empty() || self.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
    }
}

/// Per-editor settings (tab width, word wrap).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EditorConfig {
    /// Number of spaces a `\t` expands to in the editor view (default 4).
    #[serde(default = "default_tab_width")]
    pub tab_width: u8,
    /// Soft-wrap long lines at the viewport boundary.
    #[serde(default)]
    pub word_wrap: bool,
}

fn default_tab_width() -> u8 {
    4
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_width: 4,
            word_wrap: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IconMode {
    #[default]
    Unicode,
    Ascii,
    Custom,
}

impl AppConfig {
    pub fn load_default_location() -> Result<LoadedConfig, ConfigError> {
        let path = resolve_config_path()?;

        if !path.exists() {
            return Ok(LoadedConfig {
                config: Self::default(),
                path,
                source: ConfigSource::Default,
            });
        }

        let config = Self::load(&path)?;

        Ok(LoadedConfig {
            config,
            path,
            source: ConfigSource::File,
        })
    }

    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = std_fs::read_to_string(path).map_err(|source| ConfigError::ReadFile {
            path: path.display().to_string(),
            source,
        })?;

        toml::from_str(&raw).map_err(ConfigError::Parse)
    }

    pub fn compile_keymap(&self) -> Result<RuntimeKeymap, ConfigError> {
        self.keymap.compile()
    }

    pub fn resolve_theme(&self) -> ResolvedTheme {
        ThemePalette::resolve(&self.theme)
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let raw = toml::to_string_pretty(self)?;

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std_fs::create_dir_all(parent).map_err(|source| ConfigError::CreateDir {
                    path: parent.display().to_string(),
                    source,
                })?;
            }
        }

        std_fs::write(path, raw).map_err(|source| ConfigError::WriteFile {
            path: path.display().to_string(),
            source,
        })
    }
}

fn default_preview_on_selection() -> bool {
    true
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedConfig {
    pub config: AppConfig,
    pub path: PathBuf,
    pub source: ConfigSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeKeymap {
    pub quit: KeyBinding,
    pub switch_pane: KeyBinding,
    pub refresh: KeyBinding,
    /// Configurable workspace-switch bindings, one per workspace slot (0-based)..
    pub workspace: [KeyBinding; 4],
}

impl Default for RuntimeKeymap {
    fn default() -> Self {
        KeymapConfig::default()
            .compile()
            .expect("default keymap should always compile")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigSource {
    Default,
    File,
}

impl ConfigSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::File => "file",
        }
    }
}

pub fn resolve_config_path() -> Result<PathBuf, ConfigError> {
    let env_override = env::var_os("ZETA_CONFIG").map(PathBuf::from);
    let xdg_home = env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    let appdata = env::var_os("APPDATA").map(PathBuf::from);
    let home = env::var_os("HOME").map(PathBuf::from);
    let user_profile = env::var_os("USERPROFILE").map(PathBuf::from);

    resolve_config_path_from_env(env_override, xdg_home, appdata, home, user_profile)
}

fn resolve_config_path_from_env(
    env_override: Option<PathBuf>,
    xdg_home: Option<PathBuf>,
    appdata: Option<PathBuf>,
    home: Option<PathBuf>,
    user_profile: Option<PathBuf>,
) -> Result<PathBuf, ConfigError> {
    if let Some(path) = env_override {
        return Ok(path);
    }

    if let Some(path) = xdg_home {
        return Ok(path.join("zeta").join("config.toml"));
    }

    if let Some(path) = appdata {
        return Ok(path.join("zeta").join("config.toml"));
    }

    if let Some(path) = home {
        return Ok(path.join(".config").join("zeta").join("config.toml"));
    }

    if let Some(path) = user_profile {
        return Ok(path
            .join("AppData")
            .join("Roaming")
            .join("zeta")
            .join("config.toml"));
    }

    Err(ConfigError::NoConfigHome)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ThemeConfig {
    pub preset: String,
    pub status_bar_label: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            preset: String::from("zeta"),
            status_bar_label: String::from("Zeta"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThemePalette {
    pub menu_bg: Color,
    pub menu_fg: Color,
    pub menu_active_bg: Color,
    pub menu_mnemonic_fg: Color,
    pub border_focus: Color,
    pub border_editor_focus: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub surface_bg: Color,
    pub tools_bg: Color,
    pub prompt_bg: Color,
    pub prompt_border: Color,
    pub text_primary: Color,
    pub text_muted: Color,
    pub directory_fg: Color,
    pub symlink_fg: Color,
    pub file_fg: Color,
    pub status_bg: Color,
    pub status_fg: Color,
    pub logo_accent: Color,
    pub key_hint_fg: Color,
    /// Syntect theme name used for preview-panel syntax highlighting.
    pub syntect_theme: &'static str,
    /// Background for all visible search matches.
    pub search_match_bg: Color,
    /// Background for the active (cursor) search match.
    pub search_match_active_bg: Color,
    /// Background for text selection highlighting in the editor.
    pub text_sel_bg: Color,
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedTheme {
    pub palette: ThemePalette,
    pub preset: String,
    pub warning: Option<String>,
}

impl ThemePalette {
    pub fn resolve(config: &ThemeConfig) -> ResolvedTheme {
        match ThemePreset::from_name(&config.preset) {
            Some(preset) => Self::from_preset(preset),
            None => ResolvedTheme {
                palette: Self::zeta(),
                preset: String::from("zeta"),
                warning: Some(format!(
                    "unknown theme preset '{}', using zeta",
                    config.preset
                )),
            },
        }
    }

    pub fn from_preset(preset: ThemePreset) -> ResolvedTheme {
        match preset {
            ThemePreset::Fjord => ResolvedTheme {
                palette: Self::fjord(),
                preset: String::from("fjord"),
                warning: None,
            },
            ThemePreset::Sandbar => ResolvedTheme {
                palette: Self::sandbar(),
                preset: String::from("sandbar"),
                warning: None,
            },
            ThemePreset::Oxide => ResolvedTheme {
                palette: Self::oxide(),
                preset: String::from("oxide"),
                warning: None,
            },
            ThemePreset::Matrix => ResolvedTheme {
                palette: Self::matrix(),
                preset: String::from("matrix"),
                warning: None,
            },
            ThemePreset::Norton => ResolvedTheme {
                palette: Self::norton(),
                preset: String::from("norton"),
                warning: None,
            },
            ThemePreset::Neon => ResolvedTheme {
                palette: Self::neon(),
                preset: String::from("neon"),
                warning: None,
            },
            ThemePreset::Monochrome => ResolvedTheme {
                palette: Self::monochrome(),
                preset: String::from("monochrome"),
                warning: None,
            },
            ThemePreset::Dracula => ResolvedTheme {
                palette: Self::dracula(),
                preset: String::from("dracula"),
                warning: None,
            },
            ThemePreset::Zeta => ResolvedTheme {
                palette: Self::zeta(),
                preset: String::from("zeta"),
                warning: None,
            },
            ThemePreset::CatppuccinMocha => ResolvedTheme {
                palette: Self::catppuccin_mocha(),
                preset: String::from("catppuccin_mocha"),
                warning: None,
            },
        }
    }

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

    fn fjord() -> Self {
        Self {
            menu_bg: Color::Rgb(212, 196, 168),
            menu_fg: Color::Black,
            menu_active_bg: Color::Rgb(240, 223, 193),
            menu_mnemonic_fg: Color::Rgb(120, 34, 17),
            border_focus: Color::Rgb(118, 196, 182),
            border_editor_focus: Color::Rgb(230, 188, 98),
            selection_bg: Color::Rgb(47, 58, 66),
            selection_fg: Color::White,
            surface_bg: Color::Rgb(30, 34, 38),
            tools_bg: Color::Rgb(38, 43, 48),
            prompt_bg: Color::Rgb(24, 27, 30),
            prompt_border: Color::Rgb(212, 196, 168),
            text_primary: Color::White,
            text_muted: Color::DarkGray,
            directory_fg: Color::Rgb(118, 196, 182),
            symlink_fg: Color::Rgb(214, 179, 92),
            file_fg: Color::Gray,
            status_bg: Color::Cyan,
            status_fg: Color::Black,
            logo_accent: Color::Rgb(180, 60, 30),
            key_hint_fg: Color::Rgb(118, 196, 182),
            syntect_theme: "base16-ocean.dark",
            search_match_bg: Color::Rgb(80, 64, 0),
            search_match_active_bg: Color::Rgb(185, 140, 10),
            text_sel_bg: Color::Rgb(35, 85, 145),
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
        }
    }

    fn sandbar() -> Self {
        Self {
            menu_bg: Color::Rgb(224, 207, 175),
            menu_fg: Color::Black,
            menu_active_bg: Color::Rgb(246, 232, 203),
            menu_mnemonic_fg: Color::Rgb(139, 69, 19),
            border_focus: Color::Rgb(83, 148, 117),
            border_editor_focus: Color::Rgb(205, 143, 57),
            selection_bg: Color::Rgb(72, 82, 90),
            selection_fg: Color::White,
            surface_bg: Color::Rgb(36, 33, 29),
            tools_bg: Color::Rgb(44, 40, 34),
            prompt_bg: Color::Rgb(28, 26, 22),
            prompt_border: Color::Rgb(224, 207, 175),
            text_primary: Color::Rgb(241, 236, 228),
            text_muted: Color::Rgb(128, 118, 106),
            directory_fg: Color::Rgb(140, 201, 157),
            symlink_fg: Color::Rgb(227, 196, 109),
            file_fg: Color::Rgb(222, 218, 210),
            status_bg: Color::Rgb(114, 164, 199),
            status_fg: Color::Black,
            logo_accent: Color::Rgb(190, 80, 20),
            key_hint_fg: Color::Rgb(83, 148, 117),
            syntect_theme: "Solarized (light)",
            search_match_bg: Color::Rgb(80, 64, 0),
            search_match_active_bg: Color::Rgb(185, 140, 10),
            text_sel_bg: Color::Rgb(35, 85, 145),
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
        }
    }

    fn oxide() -> Self {
        Self {
            menu_bg: Color::Rgb(189, 178, 166),
            menu_fg: Color::Black,
            menu_active_bg: Color::Rgb(224, 216, 205),
            menu_mnemonic_fg: Color::Rgb(101, 45, 32),
            border_focus: Color::Rgb(102, 174, 197),
            border_editor_focus: Color::Rgb(205, 130, 107),
            selection_bg: Color::Rgb(61, 67, 79),
            selection_fg: Color::White,
            surface_bg: Color::Rgb(27, 31, 36),
            tools_bg: Color::Rgb(33, 38, 44),
            prompt_bg: Color::Rgb(20, 24, 28),
            prompt_border: Color::Rgb(189, 178, 166),
            text_primary: Color::Rgb(233, 236, 239),
            text_muted: Color::Rgb(122, 129, 138),
            directory_fg: Color::Rgb(102, 174, 197),
            symlink_fg: Color::Rgb(221, 176, 98),
            file_fg: Color::Rgb(203, 210, 217),
            status_bg: Color::Rgb(116, 181, 201),
            status_fg: Color::Black,
            logo_accent: Color::Rgb(170, 70, 50),
            key_hint_fg: Color::Rgb(102, 174, 197),
            syntect_theme: "base16-mocha.dark",
            search_match_bg: Color::Rgb(80, 64, 0),
            search_match_active_bg: Color::Rgb(185, 140, 10),
            text_sel_bg: Color::Rgb(35, 85, 145),
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
        }
    }

    fn matrix() -> Self {
        // Matrix-style colors: black bg, neon green, blue-violet accents
        Self {
            menu_bg: Color::Black,
            menu_fg: Color::Rgb(0, 255, 128),
            menu_active_bg: Color::Rgb(10, 28, 10),
            menu_mnemonic_fg: Color::Rgb(0, 255, 0),
            border_focus: Color::Rgb(0, 200, 64),
            border_editor_focus: Color::Rgb(44, 213, 255),
            selection_bg: Color::Rgb(0, 40, 0),
            selection_fg: Color::Rgb(0, 255, 128),
            surface_bg: Color::Rgb(8, 18, 8),
            tools_bg: Color::Rgb(12, 32, 16),
            prompt_bg: Color::Rgb(12, 32, 16),
            prompt_border: Color::Rgb(0, 255, 64),
            text_primary: Color::Rgb(0, 255, 128),
            text_muted: Color::Rgb(40, 120, 40),
            directory_fg: Color::Rgb(100, 255, 180),
            symlink_fg: Color::Rgb(44, 213, 255),
            file_fg: Color::Rgb(120, 190, 120),
            status_bg: Color::Rgb(0, 255, 96),
            status_fg: Color::Black,
            logo_accent: Color::Rgb(0, 255, 64),
            key_hint_fg: Color::Rgb(44, 213, 255),
            syntect_theme: "Dracula",
            search_match_bg: Color::Rgb(80, 64, 0),
            search_match_active_bg: Color::Rgb(185, 140, 10),
            text_sel_bg: Color::Rgb(35, 85, 145),
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
        }
    }

    fn neon() -> Self {
        // High-vibrancy Neon theme: Deep Black, Cyan, Magenta, Yellow
        Self {
            menu_bg: Color::Rgb(10, 10, 15),
            menu_fg: Color::Rgb(0, 255, 255), // Cyan
            menu_active_bg: Color::Rgb(30, 30, 45),
            menu_mnemonic_fg: Color::Rgb(255, 0, 255), // Magenta
            border_focus: Color::Rgb(0, 255, 255),     // Cyan
            border_editor_focus: Color::Rgb(255, 255, 0), // Yellow
            selection_bg: Color::Rgb(0, 100, 100),     // Darker cyan for selection background
            selection_fg: Color::White,                // White for selection text
            surface_bg: Color::Rgb(8, 8, 12),          // Very dark blue-gray
            tools_bg: Color::Rgb(20, 20, 30),          // Slightly lighter for hierarchy
            prompt_bg: Color::Rgb(15, 15, 25),         // Consistent with tools_bg
            prompt_border: Color::Rgb(255, 0, 255),
            text_primary: Color::Rgb(220, 220, 240),
            text_muted: Color::Rgb(100, 100, 130),
            directory_fg: Color::Rgb(0, 255, 255),
            symlink_fg: Color::Rgb(255, 0, 255),
            file_fg: Color::Rgb(200, 200, 220),
            status_bg: Color::Rgb(0, 255, 255),
            status_fg: Color::Black,
            logo_accent: Color::Rgb(255, 0, 255),
            key_hint_fg: Color::Rgb(0, 255, 255),
            syntect_theme: "base16-ocean.dark",
            search_match_bg: Color::Rgb(80, 64, 0),
            search_match_active_bg: Color::Rgb(185, 140, 10),
            text_sel_bg: Color::Rgb(35, 85, 145),
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
        }
    }

    fn monochrome() -> Self {
        // Minimalist B&W theme with functional status colors
        Self {
            menu_bg: Color::Rgb(20, 20, 20),
            menu_fg: Color::White,
            menu_active_bg: Color::Rgb(50, 50, 50),
            menu_mnemonic_fg: Color::Rgb(180, 180, 180),
            border_focus: Color::White,
            border_editor_focus: Color::White,
            selection_bg: Color::White,
            selection_fg: Color::Black,
            surface_bg: Color::Rgb(10, 10, 10), // Very dark gray
            tools_bg: Color::Rgb(25, 25, 25),   // Slightly lighter for hierarchy
            prompt_bg: Color::Rgb(20, 20, 20),  // Consistent with tools_bg
            prompt_border: Color::White,
            text_primary: Color::White,
            text_muted: Color::Rgb(100, 100, 100),
            directory_fg: Color::White,
            symlink_fg: Color::Rgb(180, 180, 180),
            file_fg: Color::Rgb(200, 200, 200),
            status_bg: Color::Rgb(30, 30, 30), // Subtle dark gray
            status_fg: Color::White,           // White text for status bar
            logo_accent: Color::Green,         // Green for git-like status or accent
            key_hint_fg: Color::Rgb(150, 150, 150), // Gray for key hints
            syntect_theme: "base16-ocean.dark",
            search_match_bg: Color::Rgb(80, 64, 0),
            search_match_active_bg: Color::Rgb(185, 140, 10),
            text_sel_bg: Color::Rgb(35, 85, 145),
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
        }
    }

    fn dracula() -> Self {
        Self {
            menu_bg: Color::Rgb(40, 42, 54),               // #282A36
            menu_fg: Color::Rgb(248, 248, 242),            // #F8F8F2
            menu_active_bg: Color::Rgb(68, 71, 90),        // #44475A
            menu_mnemonic_fg: Color::Rgb(255, 121, 198),   // #FF79C6
            border_focus: Color::Rgb(189, 147, 249),       // #BD93F9
            border_editor_focus: Color::Rgb(80, 250, 123), // #50FA7B
            selection_bg: Color::Rgb(68, 71, 90),          // #44475A
            selection_fg: Color::Rgb(248, 248, 242),       // #F8F8F2
            surface_bg: Color::Rgb(40, 42, 54),            // #282A36
            tools_bg: Color::Rgb(52, 55, 70),              // #343746
            prompt_bg: Color::Rgb(33, 34, 44),             // #21222C
            prompt_border: Color::Rgb(189, 147, 249),      // #BD93F9
            text_primary: Color::Rgb(248, 248, 242),       // #F8F8F2
            text_muted: Color::Rgb(98, 114, 164),          // #6272A4
            directory_fg: Color::Rgb(139, 233, 253),       // #8BE9FD
            symlink_fg: Color::Rgb(80, 250, 123),          // #50FA7B
            file_fg: Color::Rgb(248, 248, 242),            // #F8F8F2
            status_bg: Color::Rgb(255, 184, 108),          // #FFB86C
            status_fg: Color::Rgb(40, 42, 54),             // #282A36
            logo_accent: Color::Rgb(255, 121, 198),        // #FF79C6
            key_hint_fg: Color::Rgb(241, 250, 140),        // #F1FA8C
            syntect_theme: "Dracula",
            search_match_bg: Color::Rgb(80, 64, 0),
            search_match_active_bg: Color::Rgb(185, 140, 10),
            text_sel_bg: Color::Rgb(35, 85, 145),
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
        }
    }

    fn norton() -> Self {
        // Classic Norton Commander: navy blue, gold, white, blue
        Self {
            menu_bg: Color::Rgb(0, 14, 64),
            menu_fg: Color::White,
            menu_active_bg: Color::Rgb(59, 87, 183),
            menu_mnemonic_fg: Color::Rgb(255, 220, 60),
            border_focus: Color::Rgb(243, 205, 57),
            border_editor_focus: Color::Rgb(112, 181, 255),
            selection_bg: Color::Rgb(255, 220, 60),
            selection_fg: Color::Black,
            surface_bg: Color::Rgb(7, 23, 70),
            tools_bg: Color::Rgb(32, 42, 87),
            prompt_bg: Color::Rgb(62, 61, 100),
            prompt_border: Color::Rgb(112, 181, 255),
            text_primary: Color::White,
            text_muted: Color::Rgb(222, 222, 169),
            directory_fg: Color::Rgb(112, 225, 255),
            symlink_fg: Color::Rgb(44, 213, 255),
            file_fg: Color::White,
            status_bg: Color::Rgb(59, 87, 183),
            status_fg: Color::White,
            logo_accent: Color::Rgb(255, 220, 60),
            key_hint_fg: Color::Rgb(243, 205, 57),
            syntect_theme: "base16-ocean.dark",
            search_match_bg: Color::Rgb(80, 64, 0),
            search_match_active_bg: Color::Rgb(185, 140, 10),
            text_sel_bg: Color::Rgb(35, 85, 145),
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
        }
    }
    fn zeta() -> Self {
        // Official Zeta theme: deep slate + periwinkle blue.
        // Signature accent: #82aaff (periwinkle). Secondary: #ff9e64 (amber).
        Self {
            menu_bg: Color::Rgb(26, 27, 46),
            menu_fg: Color::Rgb(200, 211, 245),
            menu_active_bg: Color::Rgb(45, 48, 96),
            menu_mnemonic_fg: Color::Rgb(255, 158, 100),
            border_focus: Color::Rgb(130, 170, 255),
            border_editor_focus: Color::Rgb(195, 232, 141),
            selection_bg: Color::Rgb(45, 48, 96),
            selection_fg: Color::Rgb(200, 211, 245),
            surface_bg: Color::Rgb(26, 27, 46),
            tools_bg: Color::Rgb(30, 32, 48),
            prompt_bg: Color::Rgb(22, 22, 37),
            prompt_border: Color::Rgb(130, 170, 255),
            text_primary: Color::Rgb(200, 211, 245),
            text_muted: Color::Rgb(84, 92, 126),
            directory_fg: Color::Rgb(130, 170, 255),
            symlink_fg: Color::Rgb(195, 232, 141),
            file_fg: Color::Rgb(169, 184, 232),
            status_bg: Color::Rgb(45, 48, 96),
            status_fg: Color::Rgb(200, 211, 245),
            logo_accent: Color::Rgb(255, 158, 100),
            key_hint_fg: Color::Rgb(130, 170, 255),
            syntect_theme: "base16-ocean.dark",
            search_match_bg: Color::Rgb(80, 64, 0),
            search_match_active_bg: Color::Rgb(185, 140, 10),
            text_sel_bg: Color::Rgb(35, 85, 145),
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
        }
    }
}

impl ThemePreset {
    fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "fjord" => Some(Self::Fjord),
            "sandbar" => Some(Self::Sandbar),
            "oxide" => Some(Self::Oxide),
            "matrix" => Some(Self::Matrix),
            "norton" => Some(Self::Norton),
            "neon" => Some(Self::Neon),
            "monochrome" => Some(Self::Monochrome),
            "dracula" => Some(Self::Dracula),
            "zeta" => Some(Self::Zeta),
            "catppuccin_mocha" => Some(Self::CatppuccinMocha),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct KeymapConfig {
    pub quit: String,
    pub switch_pane: String,
    pub refresh: String,
    #[serde(default = "default_workspace_1")]
    pub workspace_1: String,
    #[serde(default = "default_workspace_2")]
    pub workspace_2: String,
    #[serde(default = "default_workspace_3")]
    pub workspace_3: String,
    #[serde(default = "default_workspace_4")]
    pub workspace_4: String,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            quit: String::from("q"),
            switch_pane: String::from("tab"),
            refresh: String::from("r"),
            workspace_1: String::from("alt+1"),
            workspace_2: String::from("alt+2"),
            workspace_3: String::from("alt+3"),
            workspace_4: String::from("alt+4"),
        }
    }
}

impl KeymapConfig {
    pub fn compile(&self) -> Result<RuntimeKeymap, ConfigError> {
        Ok(RuntimeKeymap {
            quit: parse_key_binding("quit", &self.quit)?,
            switch_pane: parse_key_binding("switch_pane", &self.switch_pane)?,
            refresh: parse_key_binding("refresh", &self.refresh)?,
            workspace: [
                parse_key_binding("workspace_1", &self.workspace_1)?,
                parse_key_binding("workspace_2", &self.workspace_2)?,
                parse_key_binding("workspace_3", &self.workspace_3)?,
                parse_key_binding("workspace_4", &self.workspace_4)?,
            ],
        })
    }
}

fn parse_key_binding(field: &'static str, raw: &str) -> Result<KeyBinding, ConfigError> {
    let normalized = raw.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(ConfigError::InvalidKeyBinding {
            field,
            value: raw.to_string(),
        });
    }

    let mut modifiers = KeyModifiers::NONE;
    let mut key_token = None;

    for part in normalized.split('+') {
        match part.trim() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "alt" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            token if !token.is_empty() => {
                if key_token.replace(token).is_some() {
                    return Err(ConfigError::InvalidKeyBinding {
                        field,
                        value: raw.to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    let code = match key_token {
        Some("tab") => KeyCode::Tab,
        Some("esc") | Some("escape") => KeyCode::Esc,
        Some("up") => KeyCode::Up,
        Some("down") => KeyCode::Down,
        Some("left") => KeyCode::Left,
        Some("right") => KeyCode::Right,
        Some(token) if token.chars().count() == 1 => {
            KeyCode::Char(token.chars().next().unwrap_or_default())
        }
        _ => {
            return Err(ConfigError::InvalidKeyBinding {
                field,
                value: raw.to_string(),
            });
        }
    };

    Ok(KeyBinding { code, modifiers })
}

fn default_workspace_1() -> String {
    String::from("alt+1")
}
fn default_workspace_2() -> String {
    String::from("alt+2")
}
fn default_workspace_3() -> String {
    String::from("alt+3")
}
fn default_workspace_4() -> String {
    String::from("alt+4")
}

/// Serialize a raw key event back into the string format accepted by `parse_key_binding`.
/// Used to save a captured rebind to the TOML config.
pub fn key_event_to_string(key: KeyEvent) -> Option<String> {
    let key_str = match key.code {
        KeyCode::Char(c) => c.to_ascii_lowercase().to_string(),
        KeyCode::Tab => String::from("tab"),
        KeyCode::Esc => String::from("esc"),
        KeyCode::Enter => String::from("enter"),
        KeyCode::Backspace => String::from("backspace"),
        KeyCode::Up => String::from("up"),
        KeyCode::Down => String::from("down"),
        KeyCode::Left => String::from("left"),
        KeyCode::Right => String::from("right"),
        KeyCode::F(n) => format!("f{n}"),
        // Modifier-only keys and other special keys cannot be used as bindings.
        _ => return None,
    };
    let mut parts = Vec::new();
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("ctrl");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        parts.push("alt");
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("shift");
    }
    parts.push(&key_str);
    Some(parts.join("+"))
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not determine config directory from ZETA_CONFIG, XDG_CONFIG_HOME, APPDATA, HOME, or USERPROFILE")]
    NoConfigHome,
    #[error("invalid key binding for {field}: {value}")]
    InvalidKeyBinding { field: &'static str, value: String },
    #[error("failed to create config directory {path}: {source}")]
    CreateDir {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to read config file {path}: {source}")]
    ReadFile {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to write config file {path}: {source}")]
    WriteFile {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse config file: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to serialize config file: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crossterm::event::{KeyCode, KeyModifiers};

    use super::{
        resolve_config_path_from_env, AppConfig, ConfigSource, IconMode, KeymapConfig, ThemeConfig,
        ThemePalette,
    };
    use crate::state::PaneLayout;

    #[test]
    fn parses_partial_config() {
        let raw = r#"
            [theme]
            preset = "sandbar"
            status_bar_label = "Test"

            [keymap]
            quit = "x"
            switch_pane = "tab"
            refresh = "r"
        "#;

        let config: AppConfig = toml::from_str(raw).expect("config should parse");
        assert_eq!(config.theme.preset, "sandbar");
        assert_eq!(config.keymap.quit, "x");
        assert_eq!(config.icon_mode, IconMode::Unicode);
        assert_eq!(config.pane_layout, PaneLayout::SideBySide);
        assert!(config.preview_on_selection);
        assert!(!config.preview_panel_open);
        assert!(config.bookmarks.is_empty());
    }

    #[test]
    fn parses_bookmarks() {
        let raw = r#"
            bookmarks = ["/tmp/projects", "/tmp/downloads"]

            [theme]
            preset = "fjord"
            status_bar_label = "Zeta"

            [keymap]
            quit = "q"
            switch_pane = "tab"
            refresh = "r"
        "#;

        let config: AppConfig = toml::from_str(raw).expect("config should parse");

        assert_eq!(
            config.bookmarks,
            vec![
                PathBuf::from("/tmp/projects"),
                PathBuf::from("/tmp/downloads")
            ]
        );
    }

    #[test]
    fn parses_ascii_icon_mode() {
        let raw = r#"
            icon_mode = "ascii"

            [theme]
            preset = "fjord"
            status_bar_label = "Zeta"

            [keymap]
            quit = "q"
            switch_pane = "tab"
            refresh = "r"
        "#;

        let config: AppConfig = toml::from_str(raw).expect("config should parse");

        assert_eq!(config.icon_mode, IconMode::Ascii);
    }

    #[test]
    fn parses_custom_icon_mode() {
        let raw = r#"
            icon_mode = "custom"

            [theme]
            preset = "fjord"
            status_bar_label = "Zeta"

            [keymap]
            quit = "q"
            switch_pane = "tab"
            refresh = "r"
        "#;

        let config: AppConfig = toml::from_str(raw).expect("config should parse");

        assert_eq!(config.icon_mode, IconMode::Custom);
    }

    #[test]
    fn prefers_env_override_path() {
        let resolved = resolve_config_path_from_env(
            Some(PathBuf::from("/tmp/custom.toml")),
            Some(PathBuf::from("/tmp/xdg")),
            Some(PathBuf::from("/tmp/appdata")),
            Some(PathBuf::from("/tmp/home")),
            Some(PathBuf::from("/tmp/userprofile")),
        )
        .expect("config path should resolve");

        assert_eq!(resolved, PathBuf::from("/tmp/custom.toml"));
    }

    #[test]
    fn falls_back_to_xdg_location() {
        let resolved = resolve_config_path_from_env(
            None,
            Some(PathBuf::from("/tmp/xdg")),
            Some(PathBuf::from("/tmp/appdata")),
            Some(PathBuf::from("/tmp/home")),
            Some(PathBuf::from("/tmp/userprofile")),
        )
        .expect("config path should resolve");

        assert_eq!(resolved, PathBuf::from("/tmp/xdg/zeta/config.toml"));
    }

    #[test]
    fn falls_back_to_appdata_location() {
        let resolved = resolve_config_path_from_env(
            None,
            None,
            Some(PathBuf::from(r"C:\Users\Test\AppData\Roaming")),
            Some(PathBuf::from("/tmp/home")),
            Some(PathBuf::from(r"C:\Users\Test")),
        )
        .expect("config path should resolve");

        assert_eq!(
            resolved,
            PathBuf::from(r"C:\Users\Test\AppData\Roaming")
                .join("zeta")
                .join("config.toml")
        );
    }

    #[test]
    fn falls_back_to_user_profile_location_when_appdata_missing() {
        let resolved = resolve_config_path_from_env(
            None,
            None,
            None,
            None,
            Some(PathBuf::from(r"C:\Users\Test")),
        )
        .expect("config path should resolve");

        assert_eq!(
            resolved,
            PathBuf::from(r"C:\Users\Test")
                .join("AppData")
                .join("Roaming")
                .join("zeta")
                .join("config.toml")
        );
    }

    #[test]
    fn load_missing_file_uses_defaults() {
        let path = PathBuf::from("/tmp/zeta-missing-config.toml");
        let loaded = AppConfig::load(&path).expect("missing config should return defaults");

        assert_eq!(loaded, AppConfig::default());
    }

    #[test]
    fn source_labels_are_stable() {
        assert_eq!(ConfigSource::Default.label(), "default");
        assert_eq!(ConfigSource::File.label(), "file");
    }

    #[test]
    fn compiles_ctrl_key_binding() {
        let keymap = KeymapConfig {
            quit: String::from("ctrl+c"),
            switch_pane: String::from("tab"),
            refresh: String::from("r"),
            workspace_1: String::from("alt+1"),
            workspace_2: String::from("alt+2"),
            workspace_3: String::from("alt+3"),
            workspace_4: String::from("alt+4"),
        };

        let compiled = keymap.compile().expect("keymap should compile");

        assert_eq!(compiled.quit.code, KeyCode::Char('c'));
        assert_eq!(compiled.quit.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn rejects_invalid_key_binding() {
        let keymap = KeymapConfig {
            quit: String::from("ctrl+alt+tab+q"),
            switch_pane: String::from("tab"),
            refresh: String::from("r"),
            workspace_1: String::from("alt+1"),
            workspace_2: String::from("alt+2"),
            workspace_3: String::from("alt+3"),
            workspace_4: String::from("alt+4"),
        };

        assert!(keymap.compile().is_err());
    }

    fn assert_palette_ladder(palette: ThemePalette) {
        assert_ne!(palette.surface_bg, palette.tools_bg);
        assert_ne!(palette.surface_bg, palette.prompt_bg);
        assert_ne!(palette.surface_bg, palette.status_bg);
        assert_ne!(palette.tools_bg, palette.prompt_bg);
        assert_ne!(palette.tools_bg, palette.status_bg);
        assert_ne!(palette.prompt_bg, palette.status_bg);

        assert_ne!(palette.logo_accent, palette.selection_bg);
        assert_ne!(palette.logo_accent, palette.selection_fg);
        assert_ne!(palette.selection_bg, palette.selection_fg);

        assert_ne!(palette.text_primary, palette.text_muted);
    }

    #[test]
    fn fjord_palette_exposes_distinct_surface_roles() {
        let palette = ThemePalette::resolve(&ThemeConfig {
            preset: String::from("fjord"),
            status_bar_label: String::from("Zeta"),
        })
        .palette;

        assert_palette_ladder(palette);
    }

    #[test]
    fn sandbar_palette_exposes_distinct_surface_roles() {
        let palette = ThemePalette::resolve(&ThemeConfig {
            preset: String::from("sandbar"),
            status_bar_label: String::from("Zeta"),
        })
        .palette;

        assert_palette_ladder(palette);
    }

    #[test]
    fn oxide_palette_exposes_distinct_surface_roles() {
        let palette = ThemePalette::resolve(&ThemeConfig {
            preset: String::from("oxide"),
            status_bar_label: String::from("Zeta"),
        })
        .palette;

        assert_palette_ladder(palette);
    }
}
