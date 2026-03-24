use std::path::Path;

#[test]
fn bundled_icon_font_is_present() {
    assert!(Path::new("assets/fonts/zeta-icons.ttf").exists());
}
