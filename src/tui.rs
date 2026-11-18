use anyhow::{Context, Result};
use crossterm::{
    cursor,
    event::{self, Event},
    execute,
    style::ResetColor,
    terminal::{self, Clear, ClearType},
};
use std::io::{Write, stdout};
use std::time::Duration;

use crate::adb::{DeviceStatus, detect_device};
use crate::config::AppConfig;
use crate::pipeline::StreamManager;

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let mut out = stdout();
        let _ = execute!(out, cursor::Show, ResetColor);
    }
}

pub fn run_tui(config: AppConfig) -> Result<()> {
    println!("\nScanning for connected smartphone via ADB...");

    // 1. Device validation loop
    let device = loop {
        match detect_device() {
            Ok(dev) => match dev.status {
                DeviceStatus::Connected => {
                    break dev;
                }
                DeviceStatus::Unauthorized => {
                    eprintln!("\n[WARN] Device unauthorized: {}", dev.serial);
                    eprintln!(
                        "   Please check your phone's screen and tap: 'Allow USB debugging'."
                    );
                    eprintln!("   Retrying in 2 seconds...");
                    std::thread::sleep(Duration::from_secs(2));
                }
                DeviceStatus::NotFound => {
                    eprintln!("\n[INFO] No device detected");
                    eprintln!("   1. Connect your phone using a USB-C data cable.");
                    eprintln!("   2. Enable USB Debugging in Settings > Developer Options.");
                    eprintln!("   Retrying in 3 seconds (Press Ctrl+C to cancel)...");
                    std::thread::sleep(Duration::from_secs(3));
                }
                DeviceStatus::Offline => {
                    eprintln!("\n[WARN] Device offline: {}", dev.serial);
                    eprintln!("   Reconnecting ADB server...");
                    let _ = std::process::Command::new("adb")
                        .args(["reconnect"])
                        .output();
                    std::thread::sleep(Duration::from_secs(2));
                }
            },
            Err(e) => {
                eprintln!("\n[ERROR] ADB Error: {}", e);
                std::thread::sleep(Duration::from_secs(3));
            }
        }
    };

    println!(
        "\n[OK] Connected: {} {} ({})",
        device.brand.to_uppercase(),
        device.model,
        device.serial
    );
    if let Some(pct) = device.battery_percent {
        println!(
            "Battery: {}%{}",
            pct,
            if device.is_charging.unwrap_or(false) {
                " (Charging)"
            } else {
                ""
            }
        );
    }

    // 2. Setup Stream Manager
    let mut mgr = StreamManager::new(config.clone());

    println!(
        "Starting virtual webcam stream to {}...",
        config.video_device_path()
    );
    if let Err(e) = mgr.start(config.clone()) {
        eprintln!("\n[ERROR] Failed to start camera stream: {}", e);
        return Err(e);
    }

    // 3. Render clean interactive dashboard
    render_dashboard(&device, &config);

    // 4. Raw terminal listener for ANY key press
    terminal::enable_raw_mode().context("Failed to enable raw terminal mode")?;
    let _terminal_guard = RawModeGuard;

    let any_key_pressed = loop {
        if event::poll(Duration::from_millis(500))?
            && let Event::Key(_) = event::read()?
        {
            break true;
        }

        // Check if stream crashed unexpectedly
        if !mgr.is_running() {
            break false;
        }
    };

    // Restore terminal
    let _ = terminal::disable_raw_mode();
    let mut out = stdout();
    let _ = execute!(out, cursor::Show, ResetColor);
    println!();

    if any_key_pressed {
        println!("\nKey pressed: Stopping stream and detaching virtual device...");
    } else {
        println!("\nStream process terminated.");
    }

    // 5. Clean teardown and auto-detach
    let _ = mgr.stop();
    println!(
        "Virtual webcam {} detached cleanly. Done.\n",
        config.video_device_path()
    );

    Ok(())
}

fn truncate_or_pad(s: &str, max_len: usize) -> String {
    let count = s.chars().count();
    if count > max_len {
        let keep = max_len.saturating_sub(3);
        let mut truncated: String = s.chars().take(keep).collect();
        truncated.push_str("...");
        truncated
    } else {
        format!("{:<width$}", s, width = max_len)
    }
}

fn box_line(content: &str, width: usize) -> String {
    let char_len = content.chars().count();
    let pad = width.saturating_sub(char_len);
    format!("║  {}{:pad$}║", content, "")
}

/// Format the dashboard into an array of lines, guaranteed to be exactly the same width
pub fn format_dashboard(dev: &crate::adb::DeviceInfo, cfg: &AppConfig) -> Vec<String> {
    const CONTENT_WIDTH: usize = 64;
    let border_top = format!("╔{}╗", "═".repeat(CONTENT_WIDTH + 2));
    let border_mid = format!("╠{}╣", "═".repeat(CONTENT_WIDTH + 2));
    let border_bot = format!("╚{}╝", "═".repeat(CONTENT_WIDTH + 2));

    let version = env!("CARGO_PKG_VERSION");
    let title = format!("PHONECAM VIRTUAL WEBCAM v{version}");
    let title_pad = CONTENT_WIDTH.saturating_sub(title.len()) / 2;
    let centered_title = format!(
        "{:left_pad$}{}{:right_pad$}",
        "",
        title,
        "",
        left_pad = title_pad,
        right_pad = CONTENT_WIDTH - title_pad - title.len()
    );

    let dev_model = truncate_or_pad(&dev.model, 20);
    let dev_brand = truncate_or_pad(&dev.brand.to_uppercase(), 20);
    let serial = truncate_or_pad(&dev.serial, 20);
    let android = truncate_or_pad(&dev.android_version, 20);

    let bat_pct = dev.battery_percent.unwrap_or(0);
    let charging_str = if dev.is_charging.unwrap_or(false) {
        " (Charging)"
    } else {
        ""
    };
    let battery = truncate_or_pad(&format!("{bat_pct}%{charging_str}"), 20);
    let output = truncate_or_pad(&cfg.video_device_path(), 20);

    let camera = truncate_or_pad(&cfg.facing.to_uppercase(), 20);
    let res = truncate_or_pad(&cfg.resolution, 20);
    let fps = truncate_or_pad(&format!("{} FPS", cfg.fps), 20);
    let crop = truncate_or_pad(cfg.crop.as_deref().unwrap_or("None (Full)"), 20);

    let mirror_val = if cfg.flip_h { "ON" } else { "OFF" };
    let invert_val = if cfg.invert { "ON" } else { "OFF" };
    let flipv_val = if cfg.flip_v { "ON" } else { "OFF" };
    let effects_line = format!(
        "{:<10} Mirror: {:<6}  Invert: {:<6}  Flip-V: {:<6}",
        "Effects:", mirror_val, invert_val, flipv_val
    );

    vec![
        border_top,
        box_line(&centered_title, CONTENT_WIDTH),
        border_mid.clone(),
        box_line(
            &format!(
                "{:<10} {}  {:<10} {}",
                "Device:", dev_model, "Brand:", dev_brand
            ),
            CONTENT_WIDTH,
        ),
        box_line(
            &format!(
                "{:<10} {}  {:<10} {}",
                "Serial:", serial, "Android:", android
            ),
            CONTENT_WIDTH,
        ),
        box_line(
            &format!(
                "{:<10} {}  {:<10} {}",
                "Battery:", battery, "Output:", output
            ),
            CONTENT_WIDTH,
        ),
        border_mid.clone(),
        box_line(
            &format!("{:<10} {}  {:<10} {}", "Camera:", camera, "Size:", res),
            CONTENT_WIDTH,
        ),
        box_line(
            &format!("{:<10} {}  {:<10} {}", "FPS:", fps, "Crop:", crop),
            CONTENT_WIDTH,
        ),
        box_line(&effects_line, CONTENT_WIDTH),
        border_mid.clone(),
        box_line(
            "Status:    ● STREAMING LIVE (UVC Virtual Webcam Ready)",
            CONTENT_WIDTH,
        ),
        box_line(
            "Compatible apps: Discord, Chromium, Firefox, Zoom, OBS, etc.",
            CONTENT_WIDTH,
        ),
        border_mid,
        box_line(
            "Press ANY KEY (or 'q' / Ctrl+C) to stop and auto-detach",
            CONTENT_WIDTH,
        ),
        border_bot,
    ]
}

pub fn render_dashboard(dev: &crate::adb::DeviceInfo, cfg: &AppConfig) {
    let mut out = stdout();
    let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));

    let lines = format_dashboard(dev, cfg);
    for line in lines {
        println!("{}", line);
    }

    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adb::{DeviceInfo, DeviceStatus};

    #[test]
    fn test_format_dashboard_alignment_standard() {
        let dev = DeviceInfo {
            serial: "RFCH000000D".to_string(),
            status: DeviceStatus::Connected,
            model: "Pixel 8 Pro".to_string(),
            brand: "Google".to_string(),
            android_version: "15".to_string(),
            battery_percent: Some(95),
            is_charging: Some(true),
        };
        let cfg = AppConfig::default();

        let lines = format_dashboard(&dev, &cfg);
        assert_eq!(lines.len(), 16);

        for (i, line) in lines.iter().enumerate() {
            let count = line.chars().count();
            assert_eq!(
                count, 68,
                "Line {} does not have 68 chars (got {}): '{}'",
                i, count, line
            );
        }
    }

    #[test]
    fn test_format_dashboard_alignment_long_strings_overflow() {
        let dev = DeviceInfo {
            serial: "SUPER_LONG_SERIAL_NUMBER_THAT_WOULD_OVERFLOW_EVERYTHING".to_string(),
            status: DeviceStatus::Connected,
            model: "Samsung Galaxy S24 Ultra Titanium 512GB Enterprise Edition".to_string(),
            brand: "Samsung Electronics Worldwide Mobile Division".to_string(),
            android_version: "15.0.0_r35_preview_developer_build".to_string(),
            battery_percent: Some(100),
            is_charging: Some(false),
        };
        let cfg = AppConfig {
            resolution: "3840x2160 (4K UHD Ultra High Definition)".to_string(),
            crop: Some("Custom Ultra Widescreen 32:9 Center Cut".to_string()),
            ..Default::default()
        };

        let lines = format_dashboard(&dev, &cfg);
        for (i, line) in lines.iter().enumerate() {
            let count = line.chars().count();
            assert_eq!(
                count, 68,
                "Overflow test: Line {} does not have 68 chars (got {}): '{}'",
                i, count, line
            );
        }
    }

    #[test]
    fn test_format_dashboard_alignment_empty_strings() {
        let dev = DeviceInfo {
            serial: "".to_string(),
            status: DeviceStatus::Connected,
            model: "".to_string(),
            brand: "".to_string(),
            android_version: "".to_string(),
            battery_percent: None,
            is_charging: None,
        };
        let cfg = AppConfig {
            crop: None,
            ..Default::default()
        };

        let lines = format_dashboard(&dev, &cfg);
        for (i, line) in lines.iter().enumerate() {
            let count = line.chars().count();
            assert_eq!(
                count, 68,
                "Empty test: Line {} does not have 68 chars (got {}): '{}'",
                i, count, line
            );
        }
    }
}
