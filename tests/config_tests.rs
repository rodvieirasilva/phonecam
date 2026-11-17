use phonecam::config::AppConfig;

#[test]
fn test_default_config() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.video_nr, 10);
    assert_eq!(cfg.video_device_path(), "/dev/video10");
    assert_eq!(cfg.facing, "back");
    assert_eq!(cfg.resolution, "1920x1080");
    assert_eq!(cfg.fps, 30);
    assert!(!cfg.flip_h);
    assert!(!cfg.flip_v);
    assert!(!cfg.invert);
    assert!(!cfg.preview);
    assert!(!cfg.torch);
    assert_eq!(cfg.port, 41789);
}

#[test]
fn test_custom_device_path() {
    let cfg = AppConfig {
        video_nr: 42,
        ..Default::default()
    };
    assert_eq!(cfg.video_device_path(), "/dev/video42");
}

#[test]
fn test_orientation_flags() {
    let mut cfg = AppConfig::default();

    // Default
    assert_eq!(cfg.orientation_flag(), "0");

    // Mirror (Flip Horizontal)
    cfg.flip_h = true;
    cfg.flip_v = false;
    assert_eq!(cfg.orientation_flag(), "flip0");

    // Flip Vertical
    cfg.flip_h = false;
    cfg.flip_v = true;
    assert_eq!(cfg.orientation_flag(), "180");

    // Both
    cfg.flip_h = true;
    cfg.flip_v = true;
    assert_eq!(cfg.orientation_flag(), "flip180");
}

#[test]
fn test_config_json_serialization() {
    let cfg = AppConfig {
        video_nr: 20,
        card_label: "Custom Cam".to_string(),
        facing: "front".to_string(),
        camera_id: Some(1),
        resolution: "1280x720".to_string(),
        camera_ar: Some("16:9".to_string()),
        crop: Some("1280:720:0:0".to_string()),
        fps: 60,
        flip_h: true,
        flip_v: false,
        rotation: 0,
        invert: true,
        preview: true,
        torch: false,
        port: 41800,
    };

    let serialized = serde_json::to_string(&cfg).expect("Serialization failed");
    let deserialized: AppConfig =
        serde_json::from_str(&serialized).expect("Deserialization failed");

    assert_eq!(deserialized.video_nr, 20);
    assert_eq!(deserialized.card_label, "Custom Cam");
    assert_eq!(deserialized.facing, "front");
    assert_eq!(deserialized.camera_id, Some(1));
    assert_eq!(deserialized.resolution, "1280x720");
    assert_eq!(deserialized.camera_ar, Some("16:9".to_string()));
    assert_eq!(deserialized.crop, Some("1280:720:0:0".to_string()));
    assert_eq!(deserialized.fps, 60);
    assert!(deserialized.flip_h);
    assert!(!deserialized.flip_v);
    assert!(deserialized.invert);
    assert!(deserialized.preview);
    assert_eq!(deserialized.port, 41800);
}

#[test]
fn test_user_state_config_defaults() {
    let cfg = phonecam::config::UserStateConfig::default();
    assert_eq!(cfg.last_ip, "192.168.1.50:5555");
    assert_eq!(cfg.facing, "back");
    assert_eq!(cfg.resolution, "1920x1080");
    assert!(!cfg.auto_start);
    assert!(!cfg.flip_h);
}

#[test]
fn test_user_state_config_remember_ip() {
    let mut cfg = phonecam::config::UserStateConfig::default();
    cfg.remember_ip("192.168.1.100:5555");
    assert_eq!(cfg.last_ip, "192.168.1.100:5555");
    assert!(cfg.saved_ips.contains(&"192.168.1.100:5555".to_string()));

    // Idempotent: adding again doesn't duplicate
    let count_before = cfg.saved_ips.len();
    cfg.remember_ip("192.168.1.100:5555");
    assert_eq!(cfg.saved_ips.len(), count_before);
}

#[test]
fn test_user_state_config_toml_save_load() {
    let cfg = phonecam::config::UserStateConfig {
        facing: "front".to_string(),
        resolution: "1280x720".to_string(),
        auto_start: true,
        flip_h: true,
        rotation: 90,
        ..Default::default()
    };

    let toml_str = toml::to_string_pretty(&cfg).expect("TOML serialization failed");
    assert!(toml_str.contains("facing = \"front\""));
    assert!(toml_str.contains("auto_start = true"));
    assert!(toml_str.contains("rotation = 90"));

    let loaded: phonecam::config::UserStateConfig =
        toml::from_str(&toml_str).expect("TOML deserialization failed");
    assert_eq!(loaded.facing, "front");
    assert_eq!(loaded.resolution, "1280x720");
    assert!(loaded.auto_start);
    assert!(loaded.flip_h);
    assert_eq!(loaded.rotation, 90);
}

#[test]
fn test_config_template_parsing() {
    let template = phonecam::config::UserStateConfig::template_str();
    let loaded: phonecam::config::UserStateConfig =
        toml::from_str(template).expect("Failed to parse config.example.toml template");
    assert_eq!(loaded.last_ip, "192.168.1.50:5555");
    assert_eq!(loaded.facing, "back");
    assert_eq!(loaded.resolution, "1920x1080");
    assert_eq!(loaded.rotation, 0);
    assert!(!loaded.auto_start);
}
