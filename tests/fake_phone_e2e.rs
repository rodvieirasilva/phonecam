use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

use phonecam::config::AppConfig;
use phonecam::pipeline::StreamManager;
use phonecam::server::{AppState, create_router};

/// Setup a temporary mock environment with a fake phone
struct FakePhoneEnv {
    mock_bin_dir: PathBuf,
    original_path: String,
}

impl FakePhoneEnv {
    fn setup() -> Self {
        let mock_bin_dir =
            std::env::temp_dir().join(format!("phonecam_mock_adb_{}", std::process::id()));
        fs::create_dir_all(&mock_bin_dir).unwrap();

        // 1. Mock adb
        let mock_adb_path = mock_bin_dir.join("adb");
        let mock_adb_content = r#"#!/usr/bin/env bash
if [[ "$*" == *"devices -l"* ]]; then
    echo "List of devices attached"
    echo "FAKESAM001            device usb:1-1 product:dm3q model:Galaxy_S24_Ultra device:dm3q"
    exit 0
elif [[ "$*" == *"getprop ro.product.model"* ]]; then
    echo "Galaxy S24 Ultra"
    exit 0
elif [[ "$*" == *"getprop ro.product.brand"* ]]; then
    echo "Samsung"
    exit 0
elif [[ "$*" == *"getprop ro.build.version.release"* ]]; then
    echo "15"
    exit 0
elif [[ "$*" == *"dumpsys battery"* ]]; then
    echo "status: 2"
    echo "level: 95"
    exit 0
fi
exit 0
"#;
        fs::write(&mock_adb_path, mock_adb_content).unwrap();
        let mut perms = fs::metadata(&mock_adb_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&mock_adb_path, perms).unwrap();

        // 2. Mock scrcpy
        let mock_scrcpy_path = mock_bin_dir.join("scrcpy");
        let mock_scrcpy_content = r#"#!/usr/bin/env bash
if [[ "$*" == *"--list-cameras"* ]]; then
    cat << 'EOF'
[server] INFO: List of cameras:
    --camera-id=0    (back, 4080x3060, fps={15, 20, 24, 30}, zoom-range=[0.55, 8])
    --camera-id=1    (front, 3648x2736, fps={15, 20, 24, 30}, zoom-range=[1, 4])
    --camera-id=2    (back, 4000x3000, fps={15, 20, 24, 30}, zoom-range=[1, 8])
EOF
    exit 0
fi
exit 0
"#;
        fs::write(&mock_scrcpy_path, mock_scrcpy_content).unwrap();
        let mut perms_sc = fs::metadata(&mock_scrcpy_path).unwrap().permissions();
        perms_sc.set_mode(0o755);
        fs::set_permissions(&mock_scrcpy_path, perms_sc).unwrap();

        let original_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", mock_bin_dir.display(), original_path);
        // SAFETY: Test runner is single-threaded or env is scoped
        unsafe {
            std::env::set_var("PATH", new_path);
        }

        Self {
            mock_bin_dir,
            original_path,
        }
    }
}

impl Drop for FakePhoneEnv {
    fn drop(&mut self) {
        unsafe {
            std::env::set_var("PATH", &self.original_path);
        }
        let _ = fs::remove_dir_all(&self.mock_bin_dir);
    }
}

#[tokio::test]
async fn test_fake_phone_detection_and_api() {
    let _env = FakePhoneEnv::setup();

    // 1. Direct device detection with mock adb
    let dev = phonecam::adb::detect_device().expect("Failed detecting mock device");
    assert_eq!(dev.serial, "FAKESAM001");
    assert_eq!(dev.status, phonecam::adb::DeviceStatus::Connected);
    assert_eq!(dev.model, "Galaxy S24 Ultra");
    assert_eq!(dev.brand, "Samsung");
    assert_eq!(dev.android_version, "15");
    assert_eq!(dev.battery_percent, Some(95));
    assert_eq!(dev.is_charging, Some(true));

    // 2. Camera listing with mock scrcpy
    let cams =
        phonecam::adb::list_cameras(Some("FAKESAM001")).expect("Failed listing mock cameras");
    assert_eq!(cams.len(), 3);
    assert_eq!(cams[0].id, 0);
    assert_eq!(cams[0].facing, "back");
    assert_eq!(cams[1].id, 1);
    assert_eq!(cams[1].facing, "front");

    // 3. API Status endpoint with fake phone
    let cfg = AppConfig::default();
    let stream_mgr = Mutex::new(StreamManager::new(cfg));
    let state = Arc::new(AppState { stream_mgr });
    let router = create_router(state.clone());

    let req = Request::builder()
        .uri("/api/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["device"]["serial"], "FAKESAM001");
    assert_eq!(json["device"]["model"], "Galaxy S24 Ultra");
    assert_eq!(json["device"]["battery_percent"], 95);
    assert_eq!(json["cameras"].as_array().unwrap().len(), 3);

    // 4. Test camera selection via API (bidirectional switch)
    let payload = serde_json::json!({ "id": 1, "facing": "front" }).to_string();
    let select_req = Request::builder()
        .uri("/api/select-camera")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(payload))
        .unwrap();

    let select_res = router.clone().oneshot(select_req).await.unwrap();
    assert_eq!(select_res.status(), StatusCode::OK);

    let mgr = state.stream_mgr.lock().unwrap();
    assert_eq!(mgr.current_config().camera_id, Some(1));
    assert_eq!(mgr.current_config().facing, "front");
}
