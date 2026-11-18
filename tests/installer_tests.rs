use phonecam::installer::Installer;

#[test]
fn test_installer_validate_templates() {
    assert!(Installer::validate_templates().is_ok());
}

#[test]
fn test_installer_helpers() {
    unsafe {
        std::env::set_var("PHONECAM_TEST", "1");
    }
    let home = Installer::home_dir();
    assert!(!home.as_os_str().is_empty());

    let bin = Installer::resolve_bin_path();
    assert!(!bin.as_os_str().is_empty());
    assert!(Installer::is_test_or_ci());
}

#[test]
fn test_installer_install_and_uninstall_lifecycle() {
    let temp_dir =
        std::env::temp_dir().join(format!("phonecam_install_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::create_dir_all(&temp_dir);

    // Create fake binary
    let fake_bin_dir = temp_dir.join("build");
    let _ = std::fs::create_dir_all(&fake_bin_dir);
    let fake_bin = fake_bin_dir.join("phonecam");
    std::fs::write(&fake_bin, b"fake-phonecam-elf-binary").unwrap();

    // 1. Test install_to in user mode (manage_system = false)
    assert!(Installer::install_to(&temp_dir, &fake_bin, false).is_ok());

    // Verify user binary
    let installed_bin = temp_dir.join(".local/bin/phonecam");
    assert!(installed_bin.exists());
    let bin_content = std::fs::read(&installed_bin).unwrap();
    assert_eq!(bin_content, b"fake-phonecam-elf-binary");

    // Verify desktop file
    let desktop_file = temp_dir.join(".local/share/applications/phonecam.desktop");
    assert!(desktop_file.exists());
    let desktop_content = std::fs::read_to_string(&desktop_file).unwrap();
    assert!(desktop_content.contains("[Desktop Entry]"));
    assert!(desktop_content.contains("Name=PhoneCam"));

    // Verify icons
    let icon_svg = temp_dir.join(".local/share/icons/hicolor/scalable/apps/phonecam.svg");
    assert!(icon_svg.exists());
    let svg_content = std::fs::read_to_string(&icon_svg).unwrap();
    assert!(svg_content.contains("<svg"));

    let icon_png = temp_dir.join(".local/share/icons/hicolor/512x512/apps/phonecam.png");
    assert!(icon_png.exists());
    let png_bytes = std::fs::read(&icon_png).unwrap();
    assert_eq!(&png_bytes[1..4], b"PNG");

    // Verify config templates
    let example_config = temp_dir.join(".config/phonecam/config.example.toml");
    assert!(example_config.exists());
    let user_config = temp_dir.join(".config/phonecam/config.toml");
    assert!(user_config.exists());

    // Verify resources
    let qml_file = temp_dir.join(".local/share/phonecam/ui/Main.qml");
    assert!(qml_file.exists());

    // 2. Test idempotency (installing again succeeds)
    assert!(Installer::install_to(&temp_dir, &fake_bin, false).is_ok());

    // 3. Test uninstall_from
    assert!(Installer::uninstall_from(&temp_dir, false).is_ok());
    assert!(!installed_bin.exists());
    assert!(!desktop_file.exists());
    assert!(!icon_svg.exists());
    assert!(!icon_png.exists());
    assert!(!temp_dir.join(".local/share/phonecam").exists());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_installer_uninstall_empty_directory() {
    let temp_dir =
        std::env::temp_dir().join(format!("phonecam_install_empty_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::create_dir_all(&temp_dir);

    // Uninstall on empty directory should succeed without errors
    assert!(Installer::uninstall_from(&temp_dir, false).is_ok());

    let _ = std::fs::remove_dir_all(&temp_dir);
}
