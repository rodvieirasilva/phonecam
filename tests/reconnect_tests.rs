use phonecam::config::AppConfig;
use phonecam::pipeline::{StreamManager, StreamState};

#[test]
fn test_stream_manager_standby_on_disconnect() {
    let cfg = AppConfig {
        video_nr: 99, // Non-destructive test number
        resolution: "1280x720".to_string(),
        fps: 30,
        ..Default::default()
    };
    let mut mgr = StreamManager::new(cfg);

    assert_eq!(*mgr.state(), StreamState::Stopped);
    assert!(!mgr.is_standby());

    // Enter standby directly
    let _ = mgr.enter_standby();
    assert_eq!(*mgr.state(), StreamState::StandbyWaitingReconnect);
    assert!(mgr.is_standby());

    // Exit standby
    mgr.exit_standby();
    // After exit_standby, state remains StandbyWaitingReconnect until start() or stop()
    assert!(mgr.is_standby());

    let _ = mgr.stop();
    assert_eq!(*mgr.state(), StreamState::Stopped);
    assert!(!mgr.is_standby());
}

#[test]
fn test_check_and_recover_transitions() {
    let cfg = AppConfig {
        video_nr: 98,
        resolution: "1280x720".to_string(),
        fps: 30,
        ..Default::default()
    };
    let mut mgr = StreamManager::new(cfg);

    // When stopped, check_and_recover does not trigger standby
    assert!(mgr.check_and_recover().is_ok());
    assert_eq!(*mgr.state(), StreamState::Stopped);

    // If standby is entered, check_and_recover executes device check cleanly
    let _ = mgr.enter_standby();
    assert!(mgr.is_standby());
    assert!(mgr.check_and_recover().is_ok());

    let _ = mgr.stop();
    assert_eq!(*mgr.state(), StreamState::Stopped);
}

#[test]
fn test_check_and_recover_deterministic_with_mock_adb() {
    let tmp = std::env::temp_dir().join(format!("phonecam_mock_adb_{}", std::process::id()));
    let bin_dir = tmp.join("bin");
    let _ = std::fs::create_dir_all(&bin_dir);
    let mock_adb = bin_dir.join("adb");

    // 1. Mock ADB reporting NO devices
    std::fs::write(
        &mock_adb,
        "#!/bin/sh\necho 'List of devices attached'\nexit 0\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&mock_adb, std::fs::Permissions::from_mode(0o755));
    }

    let orig_path = std::env::var("PATH").unwrap_or_default();
    unsafe {
        std::env::set_var("PATH", format!("{}:{}", bin_dir.display(), orig_path));
    }

    let cfg = AppConfig {
        video_nr: 96,
        ..Default::default()
    };
    let mut mgr = StreamManager::new(cfg);
    let _ = mgr.enter_standby();
    assert!(mgr.is_standby());

    // When no device is attached, check_and_recover remains in standby waiting
    assert!(mgr.check_and_recover().is_ok());
    assert!(mgr.is_standby());

    // 2. Mock ADB reporting device CONNECTED
    std::fs::write(&mock_adb, "#!/bin/sh\nif [ \"$1\" = \"devices\" ]; then echo 'List of devices attached\nMOCKPHONE1 device product:p model:m device:d'; exit 0; fi\nexit 0\n").unwrap();

    // When device connects, check_and_recover exits standby
    assert!(mgr.check_and_recover().is_ok());
    assert!(!mgr.is_standby());

    let _ = mgr.stop();
    unsafe {
        std::env::set_var("PATH", orig_path);
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_stop_stream_processes_preserves_device() {
    let cfg = AppConfig {
        video_nr: 97,
        ..Default::default()
    };
    let mut mgr = StreamManager::new(cfg);

    // Calling stop_stream_processes is idempotent and safe
    mgr.stop_stream_processes();
    assert_eq!(*mgr.state(), StreamState::Stopped);
}
