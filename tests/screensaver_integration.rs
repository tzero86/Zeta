use zeta::screensaver::ScreensaverState;

#[test]
fn screensaver_activation_cycle() {
    let mut ss = ScreensaverState::new(1, true);
    assert!(!ss.active);
    // Manual activation
    ss.active = true;
    assert!(ss.active);
    // Dismiss
    ss.active = false;
    assert!(!ss.active);
}

#[test]
fn screensaver_disabled_wont_activate() {
    let ss = ScreensaverState::new(300, false);
    assert!(!ss.enabled);
    assert!(!ss.active);
}

#[test]
fn timeout_zero_never_activates_via_timer() {
    let ss = ScreensaverState::new(0, true);
    assert!(!ss.active);
    assert_eq!(ss.timeout_secs, 0);
}
