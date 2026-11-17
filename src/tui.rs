use std::io::{stdout, Write};
use std::time::Duration;
use anyhow::{Context, Result};
use crossterm::{
    cursor,
    event::{self, Event},
    execute,
    style::ResetColor,
    terminal::{self, Clear, ClearType},
};

use crate::config::AppConfig;
use crate::adb::{detect_device, DeviceStatus};
use crate::pipeline::StreamManager;

pub fn run_tui(config: AppConfig) -> Result<()> {
    println!("\n🔍 Scanning for connected smartphone via ADB...");

    // 1. Device validation loop
    let device = loop {
        match detect_device() {
            Ok(dev) => match dev.status {
                DeviceStatus::Connected => {
                    break dev;
                }
                DeviceStatus::Unauthorized => {
                    eprintln!("\n⚠️  DEVICE UNAUTHORIZED: {}", dev.serial);
                    eprintln!("   Please check your phone's screen and tap: 'Allow USB debugging'.");
                    eprintln!("   Retrying in 2 seconds...");
                    std::thread::sleep(Duration::from_secs(2));
                }
                DeviceStatus::NotFound => {
                    eprintln!("\n❌ NO DEVICE DETECTED");
                    eprintln!("   1. Connect your phone using a USB-C data cable.");
                    eprintln!("   2. Enable USB Debugging in Settings > Developer Options.");
                    eprintln!("   Retrying in 3 seconds (Press Ctrl+C to cancel)...");
                    std::thread::sleep(Duration::from_secs(3));
                }
                DeviceStatus::Offline => {
                    eprintln!("\n⚠️  DEVICE OFFLINE: {}", dev.serial);
                    eprintln!("   Reconnecting ADB server...");
                    let _ = std::process::Command::new("adb").args(["reconnect"]).output();
                    std::thread::sleep(Duration::from_secs(2));
                }
            },
            Err(e) => {
                eprintln!("\n❌ ADB Error: {}", e);
                std::thread::sleep(Duration::from_secs(3));
            }
        }
    };

    println!("\n✅ Connected: {} {} ({})", device.brand.to_uppercase(), device.model, device.serial);
    if let Some(pct) = device.battery_percent {
        println!("🔋 Battery: {}%{}", pct, if device.is_charging.unwrap_or(false) { " (Charging)" } else { "" });
    }

    // 2. Setup Stream Manager
    let mut mgr = StreamManager::new(config.clone());

    println!("🚀 Starting virtual webcam stream to {}...", config.video_device_path());
    if let Err(e) = mgr.start(config.clone()) {
        eprintln!("\n❌ Failed to start camera stream: {}", e);
        return Err(e);
    }

    // 3. Render clean interactive dashboard
    render_dashboard(&device, &config);

    // 4. Raw terminal listener for ANY key press
    terminal::enable_raw_mode().context("Failed to enable raw terminal mode")?;

    let any_key_pressed = loop {
        if event::poll(Duration::from_millis(500))? {
            if let Event::Key(_) = event::read()? {
                break true;
            }
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
        println!("\n🛑 Key pressed: Stopping stream and detaching virtual device...");
    } else {
        println!("\n⚠️ Stream process terminated.");
    }

    // 5. Clean teardown and auto-detach
    let _ = mgr.stop();
    println!("✨ Virtual webcam {} detached cleanly. Done.\n", config.video_device_path());

    Ok(())
}

fn render_dashboard(dev: &crate::adb::DeviceInfo, cfg: &AppConfig) {
    let mut out = stdout();
    let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));

    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                    📱 PHONECAM VIRTUAL WEBCAM                    ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  Device:    {:<20} Brand:   {:<20}║", dev.model, dev.brand.to_uppercase());
    println!("║  Serial:    {:<20} Android: {:<20}║", dev.serial, dev.android_version);
    println!(
        "║  Battery:   {:<20} Output:  {:<20}║",
        format!("{}%{}", dev.battery_percent.unwrap_or(0), if dev.is_charging.unwrap_or(false) { " ⚡" } else { "" }),
        cfg.video_device_path()
    );
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  Camera:    {:<20} Size:    {:<20}║", cfg.facing.to_uppercase(), cfg.resolution);
    println!("║  Framerate: {:<20} Crop:    {:<20}║", format!("{} FPS", cfg.fps), cfg.crop.as_deref().unwrap_or("None (Full)"));
    println!(
        "║  Effects:   Mirror: {:<12} Invert:  {:<12} Flip-V: {:<6}║",
        if cfg.flip_h { "ON" } else { "OFF" },
        if cfg.invert { "ON" } else { "OFF" },
        if cfg.flip_v { "ON" } else { "OFF" }
    );
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  Status:    ● STREAMING LIVE (UVC Virtual Webcam Ready)         ║");
    println!("║  Compatible apps: Discord, Chromium, Firefox, Zoom, OBS, etc.   ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  👉 Press ANY KEY (or 'q' / Ctrl+C) to stop and auto-detach     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    let _ = out.flush();
}
