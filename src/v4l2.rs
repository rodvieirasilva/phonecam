use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::util::run_elevated;

/// Check if v4l2loopback module is currently loaded in the kernel
pub fn is_module_loaded() -> bool {
    if let Ok(content) = fs::read_to_string("/proc/modules") {
        for line in content.lines() {
            if line.starts_with("v4l2loopback ") {
                return true;
            }
        }
    }
    false
}

/// Check if the target video device exists (e.g., /dev/video10)
pub fn is_device_present(video_nr: u32) -> bool {
    let path = format!("/dev/video{}", video_nr);
    Path::new(&path).exists()
}

/// Ensure that v4l2loopback is loaded and the video device is ready
pub fn setup_loopback_device(video_nr: u32, card_label: &str) -> Result<PathBuf> {
    let dev_path = format!("/dev/video{}", video_nr);
    let dev_buf = PathBuf::from(&dev_path);

    if is_device_present(video_nr) {
        info!("Virtual video device {} already exists", dev_path);
        // Ensure read/write permissions
        let _ = run_elevated("chmod", &["666", &dev_path]);
        return Ok(dev_buf);
    }

    info!("Loading v4l2loopback kernel module for device {} ('{}')...", dev_path, card_label);

    let nr_arg = format!("video_nr={}", video_nr);
    let label_arg = format!("card_label={}", card_label);

    let output = run_elevated(
        "modprobe",
        &[
            "v4l2loopback",
            "devices=1",
            &nr_arg,
            &label_arg,
            "exclusive_caps=1",
        ],
    ).context("Failed to modprobe v4l2loopback. Is v4l2loopback-dkms installed?")?;

    info!("Module loaded: {}", output.trim());

    // Wait up to 2 seconds for device node creation
    for _ in 0..20 {
        if dev_buf.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    if !dev_buf.exists() {
        anyhow::bail!("Virtual device {} was not created after modprobe", dev_path);
    }

    // Set permissions so regular users can write video frames
    let _ = run_elevated("chmod", &["666", &dev_path]);

    info!("Virtual device {} initialized successfully", dev_path);
    Ok(dev_buf)
}

/// Safely unload the v4l2loopback module and detach the virtual device
pub fn detach_loopback_device(video_nr: u32) -> Result<()> {
    let dev_path = format!("/dev/video{}", video_nr);
    info!("Detaching virtual video device {}...", dev_path);

    if !is_module_loaded() && !is_device_present(video_nr) {
        info!("Device {} already detached", dev_path);
        return Ok(());
    }

    // Check if any process is actively holding the device open
    let lsof = Command::new("fuser").arg(&dev_path).output();
    if let Ok(out) = lsof {
        let pids = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !pids.is_empty() {
            warn!("Active PIDs using {}: {}", dev_path, pids);
        }
    }

    // Try rmmod
    let res = run_elevated("rmmod", &["v4l2loopback"]);
    match res {
        Ok(_) => {
            info!("Successfully unloaded v4l2loopback kernel module");
            Ok(())
        }
        Err(e) => {
            warn!("rmmod v4l2loopback returned: {}", e);
            // Non-fatal if module is in use by another sink
            Ok(())
        }
    }
}
