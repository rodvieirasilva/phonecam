use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;

use crate::adb::{CameraInfo, DeviceInfo, detect_device, list_cameras};
use crate::config::AppConfig;
use crate::pipeline::StreamManager;
use crate::util::get_omarchy_accent;
use crate::v4l2::is_device_present;

pub struct AppState {
    pub stream_mgr: Mutex<StreamManager>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub device: DeviceInfo,
    pub cameras: Vec<CameraInfo>,
    pub config: AppConfig,
    pub is_streaming: bool,
    pub v4l2_device_present: bool,
    pub accent_color: String,
}

#[derive(Deserialize)]
pub struct FacingPayload {
    pub facing: String,
}

#[derive(Deserialize)]
pub struct CameraSelectPayload {
    pub id: u32,
    pub facing: String,
}

#[derive(Deserialize)]
pub struct WifiConnectPayload {
    pub address: String,
}

#[derive(Deserialize, Default)]
pub struct WifiDisconnectPayload {
    pub address: Option<String>,
}

#[derive(Deserialize)]
pub struct RotationPayload {
    pub rotation: u32,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/status", get(get_status))
        .route("/api/start", post(start_stream))
        .route("/api/stop", post(stop_stream))
        .route("/api/toggle-flip-h", post(toggle_flip_h))
        .route("/api/toggle-flip-v", post(toggle_flip_v))
        .route("/api/set-rotation", post(set_rotation))
        .route("/api/toggle-invert", post(toggle_invert))
        .route("/api/set-facing", post(set_facing))
        .route("/api/select-camera", post(select_camera))
        .route("/api/connect-wifi", post(connect_wifi_endpoint))
        .route("/api/disconnect-wifi", post(disconnect_wifi_endpoint))
        .route("/api/enable-tcpip", post(enable_tcpip_endpoint))
        .route("/api/logs", get(get_logs))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn get_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let mut mgr = state.stream_mgr.lock().unwrap();
    let _ = mgr.check_and_recover();
    let is_streaming =
        mgr.is_running() || mgr.state() == &crate::pipeline::StreamState::StandbyWaitingReconnect;
    let config = mgr.current_config().clone();
    drop(mgr);

    let device = detect_device().unwrap_or_else(|_| DeviceInfo {
        serial: String::new(),
        status: crate::adb::DeviceStatus::NotFound,
        model: "Unknown".to_string(),
        brand: "Unknown".to_string(),
        android_version: "Unknown".to_string(),
        battery_percent: None,
        is_charging: None,
    });

    let cameras = if !device.serial.is_empty() {
        list_cameras(Some(&device.serial)).unwrap_or_default()
    } else {
        Vec::new()
    };

    let v4l2_device_present = is_device_present(config.video_nr);
    let accent_color = get_omarchy_accent();

    Json(StatusResponse {
        device,
        cameras,
        config,
        is_streaming,
        v4l2_device_present,
        accent_color,
    })
}

async fn start_stream(
    State(state): State<Arc<AppState>>,
    Json(config): Json<AppConfig>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // If no device is currently connected, attempt auto-connecting via saved Wi-Fi IP
    if let Ok(dev) = crate::adb::detect_device()
        && dev.status == crate::adb::DeviceStatus::NotFound
    {
        let ucfg = crate::config::UserStateConfig::load();
        let ip = ucfg.last_ip.trim();
        if !ip.is_empty() {
            let _ = crate::adb::connect_wifi(ip);
            tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
        }
    }

    let mut mgr = state.stream_mgr.lock().unwrap();
    match mgr.start(config) {
        Ok(_) => Ok(Json(
            serde_json::json!({ "success": true, "message": "Stream started" }),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to start stream: {}", e),
        )),
    }
}

async fn stop_stream(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut mgr = state.stream_mgr.lock().unwrap();
    match mgr.stop() {
        Ok(_) => Ok(Json(
            serde_json::json!({ "success": true, "message": "Stream stopped and detached" }),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to stop stream: {}", e),
        )),
    }
}

async fn toggle_flip_h(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut mgr = state.stream_mgr.lock().unwrap();
    let mut new_val = false;
    mgr.update_config(|cfg| {
        cfg.flip_h = !cfg.flip_h;
        new_val = cfg.flip_h;
    })
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(
        serde_json::json!({ "success": true, "flip_h": new_val }),
    ))
}

async fn toggle_flip_v(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut mgr = state.stream_mgr.lock().unwrap();
    let mut new_val = false;
    mgr.update_config(|cfg| {
        cfg.flip_v = !cfg.flip_v;
        new_val = cfg.flip_v;
    })
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(
        serde_json::json!({ "success": true, "flip_v": new_val }),
    ))
}

async fn set_rotation(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RotationPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut mgr = state.stream_mgr.lock().unwrap();
    let rot = payload.rotation;
    mgr.update_config(|cfg| {
        cfg.rotation = rot;
        cfg.flip_v = rot == 180;
    })
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(
        serde_json::json!({ "success": true, "rotation": rot }),
    ))
}

async fn toggle_invert(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut mgr = state.stream_mgr.lock().unwrap();
    let mut new_val = false;
    mgr.update_config(|cfg| {
        cfg.invert = !cfg.invert;
        new_val = cfg.invert;
    })
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(
        serde_json::json!({ "success": true, "invert": new_val }),
    ))
}

async fn set_facing(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FacingPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut mgr = state.stream_mgr.lock().unwrap();
    let facing_clone = payload.facing.clone();
    mgr.update_config(|cfg| {
        cfg.facing = payload.facing;
        cfg.camera_id = None;
    })
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(
        serde_json::json!({ "success": true, "facing": facing_clone }),
    ))
}

async fn select_camera(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CameraSelectPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut mgr = state.stream_mgr.lock().unwrap();
    let id = payload.id;
    let facing = payload.facing.clone();
    mgr.update_config(|cfg| {
        cfg.camera_id = Some(payload.id);
        cfg.facing = payload.facing;
    })
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(
        serde_json::json!({ "success": true, "camera_id": id, "facing": facing }),
    ))
}

async fn get_logs(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    let mgr = state.stream_mgr.lock().unwrap();
    let logs = mgr.get_recent_logs(60);
    Json(logs)
}

async fn connect_wifi_endpoint(
    Json(payload): Json<WifiConnectPayload>,
) -> (StatusCode, Json<serde_json::Value>) {
    match crate::adb::connect_wifi(&payload.address) {
        Ok(msg) => (
            StatusCode::OK,
            Json(serde_json::json!({ "success": true, "message": msg })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": e.to_string() })),
        ),
    }
}

async fn disconnect_wifi_endpoint(
    payload: Option<Json<WifiDisconnectPayload>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let addr = payload.and_then(|p| p.address.clone());
    match crate::adb::disconnect_wifi(addr.as_deref()) {
        Ok(msg) => (
            StatusCode::OK,
            Json(serde_json::json!({ "success": true, "message": msg })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": e.to_string() })),
        ),
    }
}

async fn enable_tcpip_endpoint() -> (StatusCode, Json<serde_json::Value>) {
    match crate::adb::enable_tcpip(5555) {
        Ok(msg) => (
            StatusCode::OK,
            Json(serde_json::json!({ "success": true, "message": msg })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": e.to_string() })),
        ),
    }
}
