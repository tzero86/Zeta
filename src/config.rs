use std::env;
use std::fs as std_fs;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyModifiers};
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
}

impl ThemePreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fjord => "fjord",
            Self::Sandbar => "sandbar",
            Self::Oxide => "oxide",
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
            preset: String::from("fjord"),
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
                palette: Self::fjord(),
                preset: String::from("fjord"),
                warning: Some(format!(
                    "unknown theme preset '{}', using fjord",
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
            syntect_theme: "base16-solarized.light",
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
        }
    }
}

impl ThemePreset {
    fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "fjord" => Some(Self::Fjord),
            "sandbar" => Some(Self::Sandbar),
            "oxide" => Some(Self::Oxide),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KeymapConfig {
    pub quit: String,
    pub switch_pane: String,
    pub refresh: String,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            quit: String::from("q"),
            switch_pane: String::from("tab"),
            refresh: String::from("r"),
        }
    }
}

impl KeymapConfig {
    pub fn compile(&self) -> Result<RuntimeKeymap, ConfigError> {
        Ok(RuntimeKeymap {
            quit: parse_key_binding("quit", &self.quit)?,
            switch_pane: parse_key_binding("switch_pane", &self.switch_pane)?,
            refresh: parse_key_binding("refresh", &self.refresh)?,
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
