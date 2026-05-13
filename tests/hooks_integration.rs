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
    AppConfig {
        hooks: vec![HookConfig {
            event,
            command: command.into(),
        }],
        ..AppConfig::default()
    }
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
fn on_open_hook_command_built_correctly() {
    use zeta::action::Command;
    use zeta::config::{HookConfig, HookEvent};
    use zeta::hooks::{commands_for_event, HookEnv};

    let hooks = vec![HookConfig {
        event: HookEvent::OnOpen,
        command: "echo open".into(),
    }];
    let env = HookEnv {
        path: "/home/user/file.txt".into(),
        old_path: None,
        pane: "left".into(),
        version: String::new(),
    };
    let cmds = commands_for_event(&hooks, HookEvent::OnOpen, &env, 0);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        Command::RunHook {
            command, env: e, ..
        } => {
            assert_eq!(command, "echo open");
            assert!(e
                .iter()
                .any(|(k, v)| k == "ZETA_PATH" && v == "/home/user/file.txt"));
            assert!(e.iter().any(|(k, v)| k == "ZETA_PANE" && v == "left"));
            assert!(!e.iter().any(|(k, _)| k == "ZETA_OLD_PATH"));
        }
        _ => panic!("expected RunHook"),
    }
}

#[test]
fn on_open_hook_fires_via_apply_for_file_not_dir() {
    use zeta::action::{Action, Command};
    use zeta::config::HookEvent;
    use zeta::fs::{EntryInfo, EntryKind};
    use zeta::jobs::JobResult;
    use zeta::pane::PaneId;

    let cfg = config_with_hook(HookEvent::OnOpen, "echo open");
    let mut state = make_state(cfg);

    let base = PathBuf::from("/test");

    // Populate left pane with a file entry so OpenSelectedInEditor has something to open.
    let file_entry = EntryInfo {
        lower_name: "readme.md".into(),
        name: "readme.md".into(),
        path: base.join("readme.md"),
        kind: EntryKind::File,
        size_bytes: Some(42),
        modified: None,
        link_target: None,
    };
    state.apply_job_result_commands(JobResult::DirectoryScanned {
        workspace_id: 0,
        pane: PaneId::Left,
        path: base.clone(),
        entries: vec![file_entry],
        elapsed_ms: 0,
    });

    // After a DirectoryScanned, selection is at index 0 which is the ".." parent entry.
    // Move down once to select "readme.md".
    state
        .apply(Action::MoveSelectionDown)
        .expect("MoveSelectionDown should succeed");

    // Positive case: file selected → on_open hook fires.
    let cmds = state
        .apply(Action::OpenSelectedInEditor)
        .expect("apply should succeed");
    let hook_cmds: Vec<_> = cmds
        .iter()
        .filter(|c| matches!(c, Command::RunHook { .. }))
        .collect();
    assert_eq!(
        hook_cmds.len(),
        1,
        "expected 1 on_open RunHook for file entry"
    );

    // Negative case: directory selected → hook must NOT fire.
    let dir_entry = EntryInfo {
        lower_name: "subdir".into(),
        name: "subdir".into(),
        path: base.join("subdir"),
        kind: EntryKind::Directory,
        size_bytes: None,
        modified: None,
        link_target: None,
    };
    state.apply_job_result_commands(JobResult::DirectoryScanned {
        workspace_id: 0,
        pane: PaneId::Left,
        path: base.clone(),
        entries: vec![dir_entry],
        elapsed_ms: 0,
    });

    // Move to select "subdir" (index 0 is "..", index 1 is "subdir").
    state
        .apply(Action::MoveSelectionDown)
        .expect("MoveSelectionDown should succeed");

    let dir_cmds = state
        .apply(Action::OpenSelectedInEditor)
        .expect("apply should succeed");
    assert!(
        dir_cmds
            .iter()
            .all(|c| !matches!(c, Command::RunHook { .. })),
        "on_open hook must not fire for directory entry"
    );
}

#[test]
fn no_hooks_initial_commands_has_no_run_hook() {
    let mut state = make_state(AppConfig::default());
    let cmds = state.initial_commands();
    assert!(
        cmds.iter()
            .all(|c| !matches!(c, zeta::action::Command::RunHook { .. })),
        "expected no RunHook commands with no hooks configured"
    );
}

#[test]
fn on_cd_hook_fires_on_directory_change_not_refresh() {
    use zeta::action::Command;
    use zeta::jobs::JobResult;
    use zeta::pane::PaneId;

    let cfg = config_with_hook(HookEvent::OnCd, "echo cd");
    let mut state = make_state(cfg);

    let new_path = PathBuf::from("/some/new/path");

    // Positive case: navigate to a new directory → hook fires.
    // Initial pane entries are empty, so is_refresh is always false on first scan.
    let cmds = state.apply_job_result_commands(JobResult::DirectoryScanned {
        workspace_id: 0,
        pane: PaneId::Left,
        path: new_path.clone(),
        entries: vec![],
        elapsed_ms: 0,
    });
    let hook_cmds: Vec<_> = cmds
        .iter()
        .filter(|c| matches!(c, Command::RunHook { .. }))
        .collect();
    assert_eq!(hook_cmds.len(), 1, "expected on_cd RunHook for navigation");

    // Populate entries so the pane has non-empty state, enabling refresh detection.
    let dummy_entry = zeta::fs::EntryInfo {
        lower_name: "file.txt".into(),
        name: "file.txt".into(),
        path: new_path.join("file.txt"),
        kind: zeta::fs::EntryKind::File,
        size_bytes: None,
        modified: None,
        link_target: None,
    };
    state.apply_job_result_commands(JobResult::DirectoryScanned {
        workspace_id: 0,
        pane: PaneId::Left,
        path: new_path.clone(),
        entries: vec![dummy_entry.clone()],
        elapsed_ms: 0,
    });

    // Negative case: same path with existing entries → refresh, no hook.
    let refresh_cmds = state.apply_job_result_commands(JobResult::DirectoryScanned {
        workspace_id: 0,
        pane: PaneId::Left,
        path: new_path.clone(),
        entries: vec![dummy_entry],
        elapsed_ms: 0,
    });
    assert!(
        refresh_cmds
            .iter()
            .all(|c| !matches!(c, Command::RunHook { .. })),
        "expected no RunHook for directory refresh"
    );
}
