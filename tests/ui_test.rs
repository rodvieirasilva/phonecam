use adw::prelude::*;
use gtk4::glib;
use phonecam::ui::gtk_app::*;

#[test]
fn test_build_theme_css_dark() {
    let palette = ThemePalette::default();
    let css = build_theme_css(&palette);
    assert!(css.contains("@define-color window_bg_color"));
    assert!(css.contains(&palette.background));
    assert!(css.contains(&palette.bright_foreground));
    assert!(css.contains(&palette.accent));
}

#[test]
fn test_build_theme_css_light() {
    let palette = ThemePalette {
        mode: "light".to_string(),
        accent: "#10b981".to_string(),
        ..Default::default()
    };
    let css = build_theme_css(&palette);
    assert!(css.contains("@define-color accent_fg_color #ffffff;"));
    assert!(css.contains("#10b981"));
}

#[test]
fn test_theme_palette_load_current() {
    let palette = ThemePalette::load_current();
    assert!(!palette.accent.is_empty());
    assert!(!palette.background.is_empty());
}

#[test]
fn test_http_get_offline() {
    let res = http_get(59997, "/api/status");
    assert!(res.is_err());
}

#[test]
fn test_http_post_offline() {
    let res = http_post(59997, "/api/stop", None);
    assert!(res.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_gtk_ui_widget_tree_and_interactions() {
    // Isolate HOME so tests run with fresh default config
    let orig_home = std::env::var("HOME").ok();
    let test_dir = std::env::temp_dir().join(format!("phonecam_ui_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&test_dir);
    let _ = std::fs::create_dir_all(&test_dir);
    unsafe { std::env::set_var("HOME", &test_dir) };

    // If running in a strictly displayless environment where GTK cannot initialize, return gracefully
    if gtk4::init().is_err() || adw::init().is_err() {
        if let Some(h) = orig_home {
            unsafe { std::env::set_var("HOME", h) };
        }
        let _ = std::fs::remove_dir_all(&test_dir);
        eprintln!("GTK4 / Libadwaita could not initialize display. Skipping UI test.");
        return;
    }

    use axum::routing::{get, post};
    let app = axum::Router::new()
        .route(
            "/api/status",
            get(|| async {
                axum::Json(serde_json::json!({
                    "is_streaming": false,
                    "device": {
                        "status": "Connected",
                        "brand": "Google",
                        "model": "Pixel 8",
                        "serial": "RQCWC0370LW",
                        "battery_percent": 88,
                        "is_charging": true
                    },
                    "cameras": [
                        { "id": 0, "facing": "back", "resolution": "1920x1080", "zoom_range": "1.0x" },
                        { "id": 1, "facing": "front", "resolution": "1920x1080", "zoom_range": "1.0x" }
                    ],
                    "config": {
                        "facing": "back",
                        "resolution": "1920x1080",
                        "fps": 30,
                        "flip_h": false,
                        "flip_v": false,
                        "invert": false,
                        "torch": false,
                        "preview": false
                    }
                }))
            }),
        )
        .route(
            "/api/start",
            post(|| async { axum::Json(serde_json::json!({ "success": true })) }),
        )
        .route(
            "/api/stop",
            post(|| async { axum::Json(serde_json::json!({ "success": true })) }),
        )
        .route(
            "/api/toggle-flip-h",
            post(|| async { axum::Json(serde_json::json!({ "success": true })) }),
        )
        .route(
            "/api/toggle-flip-v",
            post(|| async { axum::Json(serde_json::json!({ "success": true })) }),
        )
        .route(
            "/api/toggle-invert",
            post(|| async { axum::Json(serde_json::json!({ "success": true })) }),
        )
        .route(
            "/api/set-facing",
            post(|| async { axum::Json(serde_json::json!({ "success": true })) }),
        )
        .route(
            "/api/connect-wifi",
            post(|| async { axum::Json(serde_json::json!({ "success": true, "message": "connected" })) }),
        )
        .route(
            "/api/enable-tcpip",
            post(|| async { axum::Json(serde_json::json!({ "success": true, "message": "tcpip enabled" })) }),
        )
        .route(
            "/api/logs",
            get(|| async { axum::Json(vec!["[PhoneCam] virtual camera ready".to_string()]) }),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Verify online HTTP communication
    let status_res = http_get(port, "/api/status");
    assert!(status_res.is_ok());

    // Build GTK4 UI Window
    let window = build_ui_window(None, port);
    let root = window.upcast_ref::<gtk4::Widget>();

    // 1. Verify existence of all testIDs / widget names
    let btn_refresh: gtk4::Button =
        find_widget_by_name(root, "btn-refresh").expect("btn-refresh must exist");
    let dev_row: adw::ActionRow =
        find_widget_by_name(root, "text-device-name").expect("text-device-name must exist");
    let bat_label: gtk4::Label =
        find_widget_by_name(root, "text-battery").expect("text-battery must exist");
    let wifi_input: gtk4::Entry =
        find_widget_by_name(root, "input-wifi-address").expect("input-wifi-address must exist");
    let btn_wifi_connect: gtk4::Button =
        find_widget_by_name(root, "btn-wifi-connect").expect("btn-wifi-connect must exist");
    let btn_wifi_tcpip: gtk4::Button =
        find_widget_by_name(root, "btn-wifi-tcpip").expect("btn-wifi-tcpip must exist");
    let wifi_status: adw::ActionRow =
        find_widget_by_name(root, "text-wifi-status").expect("text-wifi-status must exist");
    let btn_camera_rear: gtk4::Button =
        find_widget_by_name(root, "btn-camera-rear").expect("btn-camera-rear must exist");
    let btn_camera_front: gtk4::Button =
        find_widget_by_name(root, "btn-camera-front").expect("btn-camera-front must exist");
    let combo_lens: adw::ComboRow =
        find_widget_by_name(root, "combo-camera-lens").expect("combo-camera-lens must exist");
    let combo_res: adw::ComboRow =
        find_widget_by_name(root, "combo-resolution").expect("combo-resolution must exist");
    let combo_crop: adw::ComboRow =
        find_widget_by_name(root, "combo-crop").expect("combo-crop must exist");
    let combo_rot: adw::ComboRow =
        find_widget_by_name(root, "combo-rotation").expect("combo-rotation must exist");
    let switch_fliph: gtk4::Switch =
        find_widget_by_name(root, "switch-flip-h").expect("switch-flip-h must exist");
    let switch_invert: gtk4::Switch =
        find_widget_by_name(root, "switch-invert").expect("switch-invert must exist");
    let switch_torch: gtk4::Switch =
        find_widget_by_name(root, "switch-torch").expect("switch-torch must exist");
    let switch_autostart: gtk4::Switch =
        find_widget_by_name(root, "switch-auto-start").expect("switch-auto-start must exist");
    let doc_link: gtk4::LinkButton =
        find_widget_by_name(root, "link-adb-docs").expect("link-adb-docs must exist");
    let btn_launch_camera: gtk4::Button =
        find_widget_by_name(root, "btn-launch-camera").expect("btn-launch-camera must exist");
    let btn_preview: gtk4::Button =
        find_widget_by_name(root, "btn-preview").expect("btn-preview must exist");
    let btn_stream: gtk4::Button =
        find_widget_by_name(root, "btn-stream-action").expect("btn-stream-action must exist");
    let stream_status: adw::ActionRow =
        find_widget_by_name(root, "text-stream-status").expect("text-stream-status must exist");
    let logs_row: adw::ActionRow =
        find_widget_by_name(root, "row-logs").expect("row-logs must exist");

    // 2. Initial state verification
    assert_eq!(dev_row.title(), "Detecting Smartphone...");
    assert_eq!(bat_label.label(), "");
    assert_eq!(wifi_status.title(), "Wireless Status");
    assert_eq!(combo_lens.title(), "Specific Lens Sensor");
    assert_eq!(combo_rot.title(), "Rotation Angle");
    assert_eq!(logs_row.title(), "Recent Activity");
    assert_eq!(btn_stream.label().as_deref(), Some("Start Webcam"));
    assert_eq!(stream_status.title(), "VIRTUAL CAMERA OFFLINE");
    assert_eq!(doc_link.label().as_deref(), Some("View Guide ↗"));
    assert_eq!(btn_launch_camera.label().as_deref(), Some("Camera App"));
    assert!(!switch_fliph.is_active());
    assert!(!switch_invert.is_active());
    assert_eq!(combo_res.selected(), 0);
    assert_eq!(combo_crop.selected(), 0);
    assert_eq!(combo_rot.selected(), 0);

    // 3. Simulate camera facing button clicks
    btn_camera_front.emit_clicked();
    btn_camera_rear.emit_clicked();

    // 4. Simulate switch toggles
    switch_fliph.set_active(true);
    assert!(switch_fliph.is_active());
    switch_invert.set_active(true);
    assert!(switch_invert.is_active());
    switch_torch.set_active(true);
    assert!(switch_torch.is_active());
    switch_autostart.set_active(true);
    assert!(switch_autostart.is_active());

    // 5. Simulate combobox selection changes (resolution, crop, 90° rotation)
    combo_res.set_selected(1);
    assert_eq!(combo_res.selected(), 1);
    combo_crop.set_selected(2);
    assert_eq!(combo_crop.selected(), 2);
    combo_rot.set_selected(1); // 90° clockwise
    assert_eq!(combo_rot.selected(), 1);

    // 6. Simulate Wi-Fi ADB actions
    wifi_input.set_text("192.168.1.150:5555");
    btn_wifi_connect.emit_clicked();
    btn_wifi_tcpip.emit_clicked();

    // 7. Simulate Stream start & stop (checking sensitivity state transitions)
    btn_stream.emit_clicked();
    assert_eq!(btn_stream.label().as_deref(), Some("Stop & Detach"));
    assert_eq!(stream_status.title(), "● VIRTUAL CAMERA ACTIVE");
    assert!(!combo_res.is_sensitive());
    assert!(!combo_crop.is_sensitive());
    assert!(!combo_rot.is_sensitive());
    assert!(!btn_camera_rear.is_sensitive());
    assert!(!btn_camera_front.is_sensitive());
    assert!(!btn_wifi_connect.is_sensitive());
    assert!(!btn_wifi_tcpip.is_sensitive());

    btn_stream.emit_clicked();
    assert_eq!(btn_stream.label().as_deref(), Some("Start Webcam"));
    assert_eq!(stream_status.title(), "VIRTUAL CAMERA OFFLINE");
    assert!(combo_res.is_sensitive());
    assert!(combo_crop.is_sensitive());
    assert!(combo_rot.is_sensitive());
    assert!(btn_camera_rear.is_sensitive());
    assert!(btn_camera_front.is_sensitive());

    // 8. Simulate Preview button & Camera App button clicks
    btn_preview.emit_clicked();
    btn_launch_camera.emit_clicked();

    // 9. Simulate Refresh button click
    btn_refresh.emit_clicked();

    // Run pending GLib events
    let ctx = glib::MainContext::default();
    while ctx.iteration(false) {}

    // 10. Verify TOML config persistence on disk
    let config_path = test_dir.join(".config/phonecam/config.toml");
    assert!(config_path.exists(), "config.toml must be written to disk");
    let toml_data = std::fs::read_to_string(&config_path).expect("Read config.toml");
    assert!(toml_data.contains("192.168.1.150:5555"));
    assert!(toml_data.contains("flip_h = true"));
    assert!(toml_data.contains("rotation = 90"));

    // Verify window is ready
    assert_eq!(window.widget_name(), "window-main");

    if let Some(h) = orig_home {
        unsafe { std::env::set_var("HOME", h) };
    }
    let _ = std::fs::remove_dir_all(&test_dir);
}
