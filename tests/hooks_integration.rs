//! Integration tests for shell hook trigger points.

use std::path::PathBuf;
use std::time::Instant;

use zeta::config::{AppConfig, ConfigSource, HookConfig, HookEvent, LoadedConfig};
use zeta::state::AppState;

fn make_state(config: AppConfig) -> AppState {
    let loaded = LoadedConfig {
        config,
        path: PathBuf::from(""),
        source: ConfigSource::File,
    };
    AppState::bootstrap(loaded, Instant::now()).expect("bootstrap failed")
}

fn config_with_hook(event: HookEvent, command: &str) -> AppConfig {
    let mut cfg = AppConfig::default();
    cfg.hooks = vec![HookConfig { event, command: command.into() }];
    cfg
}

#[test]
fn on_start_hook_fires_in_initial_commands() {
    let cfg = config_with_hook(HookEvent::OnStart, "echo start");
    let mut state = make_state(cfg);
    let cmds = state.initial_commands();
    let hook_cmds: Vec<_> = cmds
        .iter()
        .filter(|c| matches!(c, zeta::action::Command::RunHook { .. }))
        .collect();
    assert_eq!(hook_cmds.len(), 1, "expected 1 on_start RunHook command");
}

#[test]
fn no_hooks_initial_commands_has_no_run_hook() {
    let mut state = make_state(AppConfig::default());
    let cmds = state.initial_commands();
    assert!(
        cmds.iter().all(|c| !matches!(c, zeta::action::Command::RunHook { .. })),
        "expected no RunHook commands with no hooks configured"
    );
}
