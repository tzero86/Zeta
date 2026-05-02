use std::path::PathBuf;
use std::time::Instant;

use zeta::action::Action;
use zeta::config::{generate_annotated_config, AppConfig, ConfigSource, LoadedConfig};
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
    let parsed: AppConfig =
        basic_toml::from_str(&text).expect("generated annotated config must be valid TOML");
    assert_eq!(cfg.theme.preset, parsed.theme.preset);
    assert_eq!(cfg.editor.tab_width, parsed.editor.tab_width);
}

#[test]
fn cancel_wizard_does_not_write_config_and_restores_theme() {
    let mut state = make_state(ConfigSource::Default);
    let _cmds = state.initial_commands();
    assert_eq!(state.modal_kind(), Some(ModalKind::FirstRunWizard));

    let original_preset = state.config().theme.preset.clone();

    // Navigate to a different theme via live preview, then cancel.
    state.apply(Action::WizardMoveDown).expect("move down");
    state.apply(Action::WizardMoveDown).expect("move down");
    state.apply(Action::WizardClose).expect("close");

    // Wizard should be gone.
    assert_eq!(
        state.modal_kind(),
        None,
        "modal should be dismissed on cancel"
    );

    // Theme preset in config must be unchanged (no write happened).
    assert_eq!(
        state.config().theme.preset,
        original_preset,
        "cancel must not mutate config.theme.preset"
    );

    // Resolved theme must match the original config preset (live preview undone).
    assert_eq!(
        state.theme().preset,
        original_preset,
        "cancel must restore the resolved theme to the original preset"
    );
}
