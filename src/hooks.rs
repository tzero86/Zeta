use crate::action::Command;
use crate::config::{HookConfig, HookEvent};

#[derive(Clone, Debug, Default)]
/// Runtime context passed to hook builders.
pub struct HookEnv {
    /// The current working directory (all events).
    pub path: String,
    /// The previous working directory (`on_cd` only).
    pub old_path: Option<String>,
    /// Which pane triggered the event (`on_cd`, `on_open`).
    pub pane: String,
    /// Zeta version string (`on_start`).
    pub version: String,
}

/// Return one `Command::RunHook` for every hook that matches `event`.
///
/// The returned commands are in config order. Env vars are built per hook:
/// - `ZETA_PATH` — always included
/// - `ZETA_OLD_PATH` — only when `old_path` is `Some`
/// - `ZETA_PANE` — for `on_cd` and `on_open`
/// - `ZETA_VERSION` — for `on_start`
pub fn commands_for_event(
    hooks: &[HookConfig],
    event: HookEvent,
    env: &HookEnv,
) -> Vec<Command> {
    hooks
        .iter()
        .filter(|h| h.event == event)
        .map(|h| {
            let mut vars: Vec<(String, String)> = Vec::new();
            vars.push(("ZETA_PATH".into(), env.path.clone()));
            if let Some(old) = &env.old_path {
                vars.push(("ZETA_OLD_PATH".into(), old.clone()));
            }
            match event {
                HookEvent::OnCd | HookEvent::OnOpen => {
                    vars.push(("ZETA_PANE".into(), env.pane.clone()));
                }
                HookEvent::OnStart => {
                    vars.push(("ZETA_VERSION".into(), env.version.clone()));
                }
                HookEvent::OnExit => {}
            }
            Command::RunHook {
                command: h.command.clone(),
                env: vars,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Command;
    use crate::config::{HookConfig, HookEvent};

    #[test]
    fn no_hooks_returns_empty() {
        let env = HookEnv {
            path: "/home/user".into(),
            old_path: None,
            pane: "left".into(),
            version: "0.5.0".into(),
        };
        let cmds = commands_for_event(&[], HookEvent::OnCd, &env);
        assert!(cmds.is_empty());
    }

    #[test]
    fn matching_hook_returns_command() {
        let hooks = vec![HookConfig { event: HookEvent::OnCd, command: "echo cd".into() }];
        let env = HookEnv {
            path: "/home/user".into(),
            old_path: Some("/tmp".into()),
            pane: "left".into(),
            version: "0.5.0".into(),
        };
        let cmds = commands_for_event(&hooks, HookEvent::OnCd, &env);
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            Command::RunHook { command, env: e } => {
                assert_eq!(command, "echo cd");
                assert!(e.iter().any(|(k, v)| k == "ZETA_PATH" && v == "/home/user"));
                assert!(e.iter().any(|(k, v)| k == "ZETA_OLD_PATH" && v == "/tmp"));
            }
            _ => panic!("wrong command variant"),
        }
    }

    #[test]
    fn non_matching_hook_skipped() {
        let hooks = vec![HookConfig { event: HookEvent::OnOpen, command: "echo open".into() }];
        let env = HookEnv {
            path: "/home/user".into(),
            old_path: None,
            pane: "left".into(),
            version: "0.5.0".into(),
        };
        let cmds = commands_for_event(&hooks, HookEvent::OnCd, &env);
        assert!(cmds.is_empty());
    }

    #[test]
    fn multiple_hooks_same_event_all_returned() {
        let hooks = vec![
            HookConfig { event: HookEvent::OnCd, command: "echo first".into() },
            HookConfig { event: HookEvent::OnCd, command: "echo second".into() },
        ];
        let env = HookEnv {
            path: "/tmp".into(),
            old_path: None,
            pane: "right".into(),
            version: "0.5.0".into(),
        };
        let cmds = commands_for_event(&hooks, HookEvent::OnCd, &env);
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn on_start_env_includes_version() {
        let hooks = vec![HookConfig { event: HookEvent::OnStart, command: "echo start".into() }];
        let env = HookEnv {
            path: "/home".into(),
            old_path: None,
            pane: "left".into(),
            version: "0.5.0".into(),
        };
        let cmds = commands_for_event(&hooks, HookEvent::OnStart, &env);
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            Command::RunHook { env: e, .. } => {
                assert!(e.iter().any(|(k, v)| k == "ZETA_VERSION" && v == "0.5.0"));
                assert!(!e.iter().any(|(k, _)| k == "ZETA_PANE"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn old_path_none_omitted_from_env() {
        let hooks = vec![HookConfig { event: HookEvent::OnCd, command: "echo cd".into() }];
        let env = HookEnv {
            path: "/tmp".into(),
            old_path: None,
            pane: "left".into(),
            version: "0.5.0".into(),
        };
        let cmds = commands_for_event(&hooks, HookEvent::OnCd, &env);
        match &cmds[0] {
            Command::RunHook { env: e, .. } => {
                assert!(!e.iter().any(|(k, _)| k == "ZETA_OLD_PATH"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn commands_are_in_declaration_order() {
        let hooks = vec![
            HookConfig { event: HookEvent::OnCd, command: "echo first".into() },
            HookConfig { event: HookEvent::OnCd, command: "echo second".into() },
        ];
        let env = HookEnv {
            path: "/tmp".into(),
            old_path: None,
            pane: "left".into(),
            version: String::new(),
        };
        let cmds = commands_for_event(&hooks, HookEvent::OnCd, &env);
        assert_eq!(cmds.len(), 2);
        match (&cmds[0], &cmds[1]) {
            (Command::RunHook { command: c0, .. }, Command::RunHook { command: c1, .. }) => {
                assert_eq!(c0, "echo first");
                assert_eq!(c1, "echo second");
            }
            _ => panic!("expected RunHook variants"),
        }
    }

    #[test]
    fn on_cd_env_vars_correct() {
        let hooks = vec![HookConfig { event: HookEvent::OnCd, command: "echo cd".into() }];
        let env = HookEnv {
            path: "/new".into(),
            old_path: Some("/old".into()),
            pane: "right".into(),
            version: String::new(),
        };
        let cmds = commands_for_event(&hooks, HookEvent::OnCd, &env);
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            Command::RunHook { env: e, .. } => {
                assert!(e.iter().any(|(k, v)| k == "ZETA_PATH" && v == "/new"));
                assert!(e.iter().any(|(k, v)| k == "ZETA_OLD_PATH" && v == "/old"));
                assert!(e.iter().any(|(k, v)| k == "ZETA_PANE" && v == "right"));
                assert!(!e.iter().any(|(k, _)| k == "ZETA_VERSION"));
            }
            _ => panic!("expected RunHook"),
        }
    }

    #[test]
    fn on_exit_env_has_no_extras() {
        let hooks = vec![HookConfig { event: HookEvent::OnExit, command: "echo bye".into() }];
        let env = HookEnv {
            path: "/home".into(),
            old_path: None,
            pane: String::new(),
            version: String::new(),
        };
        let cmds = commands_for_event(&hooks, HookEvent::OnExit, &env);
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            Command::RunHook { env: e, .. } => {
                assert!(e.iter().any(|(k, _)| k == "ZETA_PATH"));
                assert!(!e.iter().any(|(k, _)| k == "ZETA_OLD_PATH"));
                assert!(!e.iter().any(|(k, _)| k == "ZETA_PANE"));
                assert!(!e.iter().any(|(k, _)| k == "ZETA_VERSION"));
            }
            _ => panic!("expected RunHook"),
        }
    }
}
