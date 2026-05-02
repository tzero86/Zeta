use std::path::PathBuf;
use std::time::Instant;

use zeta::config::{AppConfig, ConfigSource, LoadedConfig, generate_annotated_config};
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
    let cfg = AppConfig::default();
    let text = generate_annotated_config(&cfg);
    let parsed: AppConfig = basic_toml::from_str(&text)
        .expect("generated annotated config must be valid TOML");
    assert_eq!(cfg.theme.preset, parsed.theme.preset);
    assert_eq!(cfg.editor.tab_width, parsed.editor.tab_width);
}
