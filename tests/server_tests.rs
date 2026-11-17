use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

use phonecam::config::AppConfig;
use phonecam::pipeline::StreamManager;
use phonecam::server::{AppState, create_router};

fn setup_test_app() -> (axum::Router, Arc<AppState>) {
    let cfg = AppConfig::default();
    let stream_mgr = Mutex::new(StreamManager::new(cfg));
    let state = Arc::new(AppState { stream_mgr });
    let router = create_router(state.clone());
    (router, state)
}

#[tokio::test]
async fn test_get_status_endpoint() {
    let (app, _) = setup_test_app();

    let request = Request::builder()
        .uri("/api/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json.get("device").is_some());
    assert!(json.get("config").is_some());
    assert!(json.get("accent_color").is_some());
    assert_eq!(json["config"]["facing"], "back");
}

#[tokio::test]
async fn test_toggle_flip_h_endpoint() {
    let (app, state) = setup_test_app();

    let request = Request::builder()
        .uri("/api/toggle-flip-h")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["flip_h"], true);

    let mgr = state.stream_mgr.lock().unwrap();
    assert!(mgr.current_config().flip_h);
}

#[tokio::test]
async fn test_toggle_flip_v_endpoint() {
    let (app, state) = setup_test_app();

    let request = Request::builder()
        .uri("/api/toggle-flip-v")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["flip_v"], true);

    let mgr = state.stream_mgr.lock().unwrap();
    assert!(mgr.current_config().flip_v);
}

#[tokio::test]
async fn test_toggle_invert_endpoint() {
    let (app, state) = setup_test_app();

    let request = Request::builder()
        .uri("/api/toggle-invert")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["invert"], true);

    let mgr = state.stream_mgr.lock().unwrap();
    assert!(mgr.current_config().invert);
}

#[tokio::test]
async fn test_set_facing_endpoint() {
    let (app, state) = setup_test_app();

    let payload = serde_json::json!({ "facing": "front" }).to_string();
    let request = Request::builder()
        .uri("/api/set-facing")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(payload))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let mgr = state.stream_mgr.lock().unwrap();
    assert_eq!(mgr.current_config().facing, "front");
}

#[tokio::test]
async fn test_select_camera_endpoint() {
    let (app, state) = setup_test_app();

    let payload = serde_json::json!({ "id": 2, "facing": "back" }).to_string();
    let request = Request::builder()
        .uri("/api/select-camera")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(payload))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let mgr = state.stream_mgr.lock().unwrap();
    assert_eq!(mgr.current_config().camera_id, Some(2));
    assert_eq!(mgr.current_config().facing, "back");
}

#[tokio::test]
async fn test_get_logs_endpoint() {
    let (app, _) = setup_test_app();

    let request = Request::builder()
        .uri("/api/logs")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
}

#[tokio::test]
async fn test_connect_wifi_endpoint_invalid_ip() {
    let (app, _) = setup_test_app();

    let request = Request::builder()
        .uri("/api/connect-wifi")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"address": "127.0.0.1:9999"}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Since 9999 is unlikely to be listening ADB, it either returns 400 or handled gracefully
    assert!(response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::OK);
}

#[tokio::test]
async fn test_enable_tcpip_endpoint() {
    let (app, _) = setup_test_app();

    let request = Request::builder()
        .uri("/api/enable-tcpip")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::OK);
}

#[tokio::test]
async fn test_disconnect_wifi_endpoint() {
    let (app, _) = setup_test_app();

    let request = Request::builder()
        .uri("/api/disconnect-wifi")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"address": "127.0.0.1:5555"}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert!(response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::OK);
}
