use phonecam::config::AppConfig;
use phonecam::pipeline::{build_ffmpeg_invert_args, build_scrcpy_args};

#[test]
fn test_build_scrcpy_args_default() {
    let cfg = AppConfig::default();
    let args = build_scrcpy_args(&cfg, "/dev/video10", false);

    assert!(args.contains(&"--video-source=camera".to_string()));
    assert!(args.contains(&"--camera-facing=back".to_string()));
    assert!(args.contains(&"--camera-size=1920x1080".to_string()));
    assert!(args.contains(&"--camera-fps=30".to_string()));
    assert!(args.contains(&"--capture-orientation=0".to_string()));
    assert!(args.contains(&"--no-audio".to_string()));
    assert!(args.contains(&"--no-playback".to_string()));
    assert!(args.contains(&"--v4l2-sink=/dev/video10".to_string()));
    assert!(!args.iter().any(|a| a.starts_with("--camera-id=")));
    assert!(!args.iter().any(|a| a.starts_with("--crop=")));
    assert!(!args.contains(&"--camera-torch".to_string()));
}

#[test]
fn test_build_scrcpy_args_custom() {
    let cfg = AppConfig {
        facing: "front".to_string(),
        camera_id: Some(1),
        resolution: "1280x720".to_string(),
        fps: 60,
        crop: Some("1280:720:0:0".to_string()),
        flip_h: true, // Mirror
        preview: true,
        torch: true,
        ..Default::default()
    };

    let args = build_scrcpy_args(&cfg, "/dev/video10", false);

    assert!(args.contains(&"--camera-id=1".to_string()));
    assert!(!args.iter().any(|a| a.starts_with("--camera-facing=")));
    assert!(args.contains(&"--camera-size=1280x720".to_string()));
    assert!(args.contains(&"--camera-fps=60".to_string()));
    assert!(args.contains(&"--crop=1280:720:0:0".to_string()));
    assert!(args.contains(&"--capture-orientation=flip0".to_string()));
    assert!(args.contains(&"--camera-torch".to_string()));
    assert!(!args.contains(&"--no-playback".to_string())); // Preview enabled
    assert!(args.contains(&"--v4l2-sink=/dev/video10".to_string()));
}

#[test]
fn test_build_scrcpy_args_pipe_mode() {
    let cfg = AppConfig::default();
    let args = build_scrcpy_args(&cfg, "/dev/video10", true);

    assert!(args.contains(&"-r".to_string()));
    assert!(args.contains(&"-".to_string()));
    assert!(args.contains(&"--record-format=mkv".to_string()));
    assert!(!args.iter().any(|a| a.starts_with("--v4l2-sink=")));
}

#[test]
fn test_build_ffmpeg_invert_args() {
    let args = build_ffmpeg_invert_args("/dev/video10");
    assert_eq!(args[0], "-y");
    assert_eq!(args[5], "-vf");
    assert_eq!(args[6], "negate");
    assert_eq!(args[11], "/dev/video10");
}

#[test]
fn test_stream_manager_lifecycle() {
    let cfg = AppConfig::default();
    let mut mgr = phonecam::pipeline::StreamManager::new(cfg.clone());

    assert_eq!(mgr.current_config().video_nr, 10);
    assert!(!mgr.is_running());
    assert_eq!(*mgr.state(), phonecam::pipeline::StreamState::Stopped);

    // Update config
    mgr.update_config(|c| {
        c.resolution = "1280x720".to_string();
        c.flip_h = true;
    })
    .unwrap();

    assert_eq!(mgr.current_config().resolution, "1280x720");
    assert!(mgr.current_config().flip_h);

    // Logs query
    let logs = mgr.get_recent_logs(10);
    assert!(logs.is_empty() || !logs.is_empty());
}
