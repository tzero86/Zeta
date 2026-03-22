use std::env;
use std::fs as std_fs;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::action::KeyBinding;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppConfig {
    pub theme: ThemeConfig,
    pub keymap: KeymapConfig,
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
    let home = env::var_os("HOME").map(PathBuf::from);

    resolve_config_path_from_env(env_override, xdg_home, home)
}

fn resolve_config_path_from_env(
    env_override: Option<PathBuf>,
    xdg_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Result<PathBuf, ConfigError> {
    if let Some(path) = env_override {
        return Ok(path);
    }

    if let Some(path) = xdg_home {
        return Ok(path.join("zeta").join("config.toml"));
    }

    if let Some(path) = home {
        return Ok(path.join(".config").join("zeta").join("config.toml"));
    }

    Err(ConfigError::NoConfigHome)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ThemeConfig {
    pub accent: String,
    pub status_bar_label: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            accent: String::from("cyan"),
            status_bar_label: String::from("Zeta"),
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
    #[error("could not determine config directory from ZETA_CONFIG, XDG_CONFIG_HOME, or HOME")]
    NoConfigHome,
    #[error("invalid key binding for {field}: {value}")]
    InvalidKeyBinding { field: &'static str, value: String },
    #[error("failed to read config file {path}: {source}")]
    ReadFile {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse config file: {0}")]
    Parse(#[from] toml::de::Error),
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crossterm::event::{KeyCode, KeyModifiers};

    use super::{resolve_config_path_from_env, AppConfig, ConfigSource, KeymapConfig};

    #[test]
    fn parses_partial_config() {
        let raw = r#"
            [theme]
            accent = "amber"
            status_bar_label = "Test"

            [keymap]
            quit = "x"
            switch_pane = "tab"
            refresh = "r"
        "#;

        let config: AppConfig = toml::from_str(raw).expect("config should parse");
        assert_eq!(config.theme.accent, "amber");
        assert_eq!(config.keymap.quit, "x");
    }

    #[test]
    fn prefers_env_override_path() {
        let resolved = resolve_config_path_from_env(
            Some(PathBuf::from("/tmp/custom.toml")),
            Some(PathBuf::from("/tmp/xdg")),
            Some(PathBuf::from("/tmp/home")),
        )
        .expect("config path should resolve");

        assert_eq!(resolved, PathBuf::from("/tmp/custom.toml"));
    }

    #[test]
    fn falls_back_to_xdg_location() {
        let resolved = resolve_config_path_from_env(
            None,
            Some(PathBuf::from("/tmp/xdg")),
            Some(PathBuf::from("/tmp/home")),
        )
        .expect("config path should resolve");

        assert_eq!(resolved, PathBuf::from("/tmp/xdg/zeta/config.toml"));
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
}
