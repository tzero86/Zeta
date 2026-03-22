use zeta::config::AppConfig;

#[test]
fn default_config_has_expected_keys() {
    let config = AppConfig::default();

    assert_eq!(config.keymap.quit, "q");
    assert_eq!(config.keymap.switch_pane, "tab");
}
