use phonecam::util::{get_omarchy_accent, get_sudo_password, home_dir};

#[test]
fn test_home_dir() {
    let home = home_dir();
    assert!(home.exists());
}

#[test]
fn test_omarchy_accent() {
    let accent = get_omarchy_accent();
    assert!(accent.starts_with('#'));
    assert_eq!(accent.len(), 7); // e.g. #FFD60A
}

#[test]
fn test_sudo_password() {
    // Should run safely without panic whether credentials exist or not
    let _ = get_sudo_password();
}
