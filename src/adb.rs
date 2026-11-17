use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceInfo {
    pub serial: String,
    pub status: DeviceStatus,
    pub model: String,
    pub brand: String,
    pub android_version: String,
    pub battery_percent: Option<u32>,
    pub is_charging: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Connected,
    Unauthorized,
    Offline,
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CameraInfo {
    pub id: u32,
    pub facing: String,
    pub resolution: String,
    pub fps_list: Vec<u32>,
    pub zoom_range: String,
}

/// Parse raw output of `adb devices -l` into serial and DeviceStatus
pub fn parse_adb_devices(stdout: &str) -> (Option<String>, DeviceStatus) {
    let mut found_serial = None;
    let mut status = DeviceStatus::NotFound;

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("List of devices") || line.starts_with('*') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let serial = parts[0];
            let state = parts[1];
            found_serial = Some(serial.to_string());

            match state {
                "device" => {
                    status = DeviceStatus::Connected;
                    break;
                }
                "unauthorized" => {
                    status = DeviceStatus::Unauthorized;
                    break;
                }
                "offline" => {
                    status = DeviceStatus::Offline;
                    break;
                }
                _ => {}
            }
        }
    }

    (found_serial, status)
}

/// Parse raw output of `dumpsys battery` into level percentage and charging state
pub fn parse_battery_info(output: &str) -> (Option<u32>, Option<bool>) {
    let mut level = None;
    let mut charging = None;

    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("level:") {
            if let Some(val_str) = line.split(':').nth(1) {
                level = val_str.trim().parse::<u32>().ok();
            }
        } else if line.starts_with("status:") {
            // 2 = Charging, 5 = Full
            if let Some(val_str) = line.split(':').nth(1)
                && let Ok(st) = val_str.trim().parse::<u32>()
            {
                charging = Some(st == 2 || st == 5);
            }
        }
    }
    (level, charging)
}

/// Parse camera enumeration from `scrcpy --list-cameras`
pub fn parse_scrcpy_cameras(combined_output: &str) -> Vec<CameraInfo> {
    let mut cameras = Vec::new();

    // Look for lines like:
    // --camera-id=0    (back, 4080x3060, fps={15, 20, 24, 30}, zoom-range=[0.55, 8])
    for line in combined_output.lines() {
        let line = line.trim();
        if line.contains("--camera-id=")
            && let Some(start) = line.find("--camera-id=")
        {
            let rest = &line[start + "--camera-id=".len()..];
            let id_str = rest.split_whitespace().next().unwrap_or("0");
            let id = id_str.parse::<u32>().unwrap_or(0);

            let facing = if line.contains("(front") {
                "front".to_string()
            } else if line.contains("(back") {
                "back".to_string()
            } else {
                "external".to_string()
            };

            // Resolution
            let mut resolution = "Unknown".to_string();
            if let Some(paren_start) = line.find('(')
                && let Some(paren_end) = line.find(')')
            {
                let inner = &line[paren_start + 1..paren_end];
                for part in inner.split(',') {
                    let part = part.trim();
                    if part.contains('x') && part.chars().all(|c| c.is_ascii_digit() || c == 'x') {
                        resolution = part.to_string();
                        break;
                    }
                }
            }

            // FPS list
            let mut fps_list = Vec::new();
            if let Some(fps_start) = line.find("fps={")
                && let Some(fps_end) = line[fps_start..].find('}')
            {
                let fps_str = &line[fps_start + 5..fps_start + fps_end];
                for fps_val in fps_str.split(',') {
                    if let Ok(v) = fps_val.trim().parse::<u32>() {
                        fps_list.push(v);
                    }
                }
            }

            // Zoom range
            let mut zoom_range = "1.0x".to_string();
            if let Some(zoom_start) = line.find("zoom-range=[")
                && let Some(zoom_end) = line[zoom_start..].find(']')
            {
                zoom_range = line[zoom_start + 12..zoom_start + zoom_end]
                    .trim()
                    .to_string();
            }

            cameras.push(CameraInfo {
                id,
                facing,
                resolution,
                fps_list,
                zoom_range,
            });
        }
    }

    cameras
}

/// Detect attached smartphone using ADB
pub fn detect_device() -> Result<DeviceInfo> {
    if which::which("adb").is_err() {
        anyhow::bail!("adb (Android Debug Bridge) is not installed. Please install android-tools.");
    }

    let output = Command::new("adb")
        .args(["devices", "-l"])
        .output()
        .context("Failed to run 'adb devices -l'")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let (found_serial, status) = parse_adb_devices(&stdout);

    let serial = match found_serial {
        Some(s) => s,
        None => {
            return Ok(DeviceInfo {
                serial: String::new(),
                status: DeviceStatus::NotFound,
                model: "No device detected".to_string(),
                brand: "Unknown".to_string(),
                android_version: "Unknown".to_string(),
                battery_percent: None,
                is_charging: None,
            });
        }
    };

    if status != DeviceStatus::Connected {
        return Ok(DeviceInfo {
            serial,
            status,
            model: "Device waiting for authorization".to_string(),
            brand: "Android".to_string(),
            android_version: "Unknown".to_string(),
            battery_percent: None,
            is_charging: None,
        });
    }

    // Retrieve detailed device properties
    let model =
        get_prop(&serial, "ro.product.model").unwrap_or_else(|_| "Android Device".to_string());
    let brand = get_prop(&serial, "ro.product.brand").unwrap_or_else(|_| "Generic".to_string());
    let android_ver =
        get_prop(&serial, "ro.build.version.release").unwrap_or_else(|_| "Unknown".to_string());

    // Retrieve battery level
    let (battery, charging) = get_battery(&serial);

    Ok(DeviceInfo {
        serial,
        status: DeviceStatus::Connected,
        model,
        brand,
        android_version: android_ver,
        battery_percent: battery,
        is_charging: charging,
    })
}

fn get_prop(serial: &str, prop: &str) -> Result<String> {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "getprop", prop])
        .output()?;
    let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if val.is_empty() {
        anyhow::bail!("Property {} empty", prop);
    }
    Ok(val)
}

fn get_battery(serial: &str) -> (Option<u32>, Option<bool>) {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "dumpsys", "battery"])
        .output();

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        parse_battery_info(&text)
    } else {
        (None, None)
    }
}

/// Enumerate available phone cameras via scrcpy
pub fn list_cameras(serial: Option<&str>) -> Result<Vec<CameraInfo>> {
    let mut cmd = Command::new("scrcpy");
    if let Some(s) = serial {
        cmd.args(["-s", s]);
    }
    cmd.arg("--list-cameras");

    let output = cmd
        .output()
        .context("Failed to execute 'scrcpy --list-cameras'")?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}\n{}", stdout, stderr);

    Ok(parse_scrcpy_cameras(&combined))
}

/// Connect to Android device over Wi-Fi via ADB TCP/IP (e.g. "192.168.1.50:5555")
pub fn connect_wifi(ip_port: &str) -> Result<String> {
    let output = Command::new("adb")
        .args(["connect", ip_port])
        .output()
        .context("Failed to run 'adb connect'")?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        anyhow::bail!(
            "adb connect failed: {}",
            if stderr.is_empty() { stdout } else { stderr }
        );
    }
}

/// Enable TCP/IP listening mode on currently connected USB device
pub fn enable_tcpip(port: u16) -> Result<String> {
    let output = Command::new("adb")
        .args(["tcpip", &port.to_string()])
        .output()
        .context("Failed to run 'adb tcpip'")?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        anyhow::bail!(
            "adb tcpip failed: {}",
            if stderr.is_empty() { stdout } else { stderr }
        );
    }
}
