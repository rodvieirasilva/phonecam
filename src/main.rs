use anyhow::{Context, Result};
use clap::Parser;
use phonecam::{
    adb,
    cli::Cli,
    config::AppConfig,
    pipeline::StreamManager,
    server::{AppState, create_router},
    tui, v4l2,
};
use std::process::Command;
use std::sync::{Arc, Mutex};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    // 0. Wi-Fi ADB Connect & TCP/IP Actions
    if let Some(ref addr) = cli.wifi {
        println!("Connecting to phone via ADB Wi-Fi at {}...", addr);
        match adb::connect_wifi(addr) {
            Ok(msg) => println!("✓ {}", msg),
            Err(e) => eprintln!("❌ Failed to connect: {}", e),
        }
        return Ok(());
    }

    if let Some(port_opt) = cli.tcpip {
        let port = port_opt.unwrap_or(5555);
        println!(
            "Switching connected device to ADB TCP/IP mode on port {}...",
            port
        );
        match adb::enable_tcpip(port) {
            Ok(msg) => {
                println!("✓ {}", msg);
                println!("You can now unplug the USB cable and run:");
                println!("  phonecam --wifi <PHONE_IP>:{}", port);
            }
            Err(e) => eprintln!("❌ Failed to enable TCP/IP mode: {}", e),
        }
        return Ok(());
    }

    // 1. Status action
    if cli.status {
        println!("Checking device connection...");
        match adb::detect_device() {
            Ok(dev) => {
                println!("Device: {} {} ({:?})", dev.brand, dev.model, dev.status);
                println!("Serial: {}", dev.serial);
                println!("Android: {}", dev.android_version);
                if let Some(b) = dev.battery_percent {
                    println!("Battery: {}%", b);
                }
            }
            Err(e) => eprintln!("Error detecting device: {}", e),
        }
        println!(
            "Virtual device /dev/video{}: {}",
            cli.video_nr,
            if v4l2::is_device_present(cli.video_nr) {
                "Present"
            } else {
                "Not created"
            }
        );
        return Ok(());
    }

    // 2. List cameras action
    if cli.list_cameras {
        println!("Querying phone cameras via scrcpy...");
        match adb::list_cameras(None) {
            Ok(cams) => {
                if cams.is_empty() {
                    println!("No cameras reported by phone.");
                } else {
                    println!("\nAvailable phone cameras:");
                    for cam in cams {
                        println!(
                            "  Camera ID {}: facing={:<5} res={:<12} zoom={:<8} fps={:?}",
                            cam.id, cam.facing, cam.resolution, cam.zoom_range, cam.fps_list
                        );
                    }
                }
            }
            Err(e) => eprintln!("Error querying cameras: {}", e),
        }
        return Ok(());
    }

    // 3. Detach action
    if cli.detach {
        println!("Detaching virtual webcam and terminating active streams...");
        let _ = v4l2::detach_loopback_device(cli.video_nr);
        let _ = Command::new("pkill")
            .args(["-f", "scrcpy.*--video-source=camera"])
            .output();
        println!("✓ Done.");
        return Ok(());
    }

    // Build configuration
    let config = AppConfig {
        video_nr: cli.video_nr,
        card_label: "PhoneCam Virtual Camera".to_string(),
        facing: cli.facing,
        camera_id: cli.camera_id,
        resolution: cli.res,
        camera_ar: cli.camera_ar,
        crop: cli.crop,
        fps: cli.fps,
        flip_h: cli.mirror,
        flip_v: cli.flip_v,
        invert: cli.invert,
        preview: cli.preview,
        torch: cli.torch,
        port: cli.port,
    };

    // 4. Interactive TUI mode (auto-detach on any key)
    if cli.tui {
        return tui::run_tui(config);
    }

    // 5. Headless Start mode
    if cli.start {
        println!(
            "Starting virtual webcam stream headless to {}...",
            config.video_device_path()
        );
        let mut mgr = StreamManager::new(config.clone());
        mgr.start(config)?;
        println!("Streaming started. Press Ctrl+C to stop.");
        tokio::signal::ctrl_c().await?;
        mgr.stop()?;
        return Ok(());
    }

    // 6. GUI mode (Default desktop behavior)
    run_gui(config).await
}

async fn run_gui(config: AppConfig) -> Result<()> {
    info!("Starting PhoneCam native GTK4 GUI on port {}", config.port);

    let port = config.port;
    let stream_mgr = Mutex::new(StreamManager::new(config.clone()));
    let app_state = Arc::new(AppState { stream_mgr });

    let router = create_router(app_state.clone());
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .context(format!("Failed to bind to port {}", port))?;

    // Spawn the local API server in the background
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    #[cfg(feature = "gui")]
    {
        info!("Launching native GTK4/Adwaita desktop window...");
        let _ = phonecam::ui::run_desktop_ui(port);
    }
    #[cfg(not(feature = "gui"))]
    {
        eprintln!(
            "Notice: PhoneCam was compiled without native GTK4 GUI support. Run with --tui instead."
        );
    }

    // Clean teardown when GUI closes
    let mut mgr = app_state.stream_mgr.lock().unwrap();
    let _ = mgr.stop();
    info!("PhoneCam shutdown completed.");

    Ok(())
}
