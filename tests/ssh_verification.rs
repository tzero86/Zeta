// Integration tests for SSH host key verification and fingerprints

#[cfg(test)]
mod ssh_verification_tests {
    use zeta::state::ssh::{HostKeyFingerprints, SshAuthMethod, SshConnectionState, SshErrorKind};

    #[test]
    fn host_key_fingerprints_can_be_created() {
        let fingerprints = HostKeyFingerprints {
            md5: "aa:bb:cc:dd:ee:ff:00:11:22:33:44:55:66:77:88:99".to_string(),
            sha256: "SHA256:aAbBcCdDeEfF0011223344556677889900112233445".to_string(),
        };

        assert_eq!(
            fingerprints.md5,
            "aa:bb:cc:dd:ee:ff:00:11:22:33:44:55:66:77:88:99"
        );
        assert!(fingerprints.sha256.starts_with("SHA256:"));
    }

    #[test]
    fn ssh_connection_state_default_has_strict_mode_disabled() {
        let state = SshConnectionState::default();
        assert!(
            !state.known_hosts_strict,
            "strict mode should be disabled by default"
        );
    }

    #[test]
    fn ssh_connection_state_can_enable_strict_mode() {
        let state = SshConnectionState {
            known_hosts_strict: true,
            ..Default::default()
        };
        assert!(state.known_hosts_strict);
    }

    #[test]
    fn ssh_error_kind_host_key_mismatch_message() {
        let error = SshErrorKind::HostKeyMismatch(
            "Possible MITM attack — host key mismatch with ~/.ssh/known_hosts".to_string(),
        );
        let msg = error.message();
        assert!(
            msg.contains("mismatch"),
            "message should contain 'mismatch'"
        );
    }

    #[test]
    fn ssh_error_kind_host_key_mismatch_color_is_red() {
        let error = SshErrorKind::HostKeyMismatch("test".to_string());
        assert_eq!(error.color_code(), "red", "host key mismatch should be red");
    }

    #[test]
    fn ssh_error_kind_unknown_host_color_is_yellow() {
        let error = SshErrorKind::HostKeyUnknown("test".to_string());
        assert_eq!(
            error.color_code(),
            "yellow",
            "unknown host should be yellow"
        );
    }

    #[test]
    fn ssh_auth_method_variations() {
        assert_eq!(SshAuthMethod::default(), SshAuthMethod::Password);
        let _password = SshAuthMethod::Password;
        let _key_file = SshAuthMethod::KeyFile;
        let _agent = SshAuthMethod::Agent;
    }

    #[test]
    fn ssh_connection_state_preserves_all_fields() {
        let state = SshConnectionState {
            address: "user@example.com:22".to_string(),
            auth_method: SshAuthMethod::KeyFile,
            credential: "/home/user/.ssh/id_rsa".to_string(),
            known_hosts_strict: true,
            ..Default::default()
        };

        assert_eq!(state.address, "user@example.com:22");
        assert_eq!(state.auth_method, SshAuthMethod::KeyFile);
        assert_eq!(state.credential, "/home/user/.ssh/id_rsa");
        assert!(state.known_hosts_strict);
    }
}
