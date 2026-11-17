use anyhow::{Context, Result};
use std::fs::File;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tracing::{info, warn};

use crate::config::AppConfig;
use crate::v4l2::{detach_loopback_device, setup_loopback_device};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamState {
    Stopped,
    Starting,
    Running,
    StandbyWaitingReconnect,
    Error(String),
}

/// Pure helper function to build command arguments for scrcpy
pub fn build_scrcpy_args(config: &AppConfig, sink_or_record: &str, is_pipe: bool) -> Vec<String> {
    let mut args = vec![
        "--video-source=camera".to_string(),
        format!("--camera-size={}", config.resolution),
        format!("--camera-fps={}", config.fps),
        format!("--capture-orientation={}", config.orientation_flag()),
        "--no-audio".to_string(),
    ];

    // scrcpy strictly forbids specifying BOTH --camera-id and --camera-facing
    if let Some(id) = config.camera_id {
        args.push(format!("--camera-id={}", id));
    } else {
        args.push(format!("--camera-facing={}", config.facing));
    }

    if let Some(ref ar) = config.camera_ar {
        args.push(format!("--camera-ar={}", ar));
    }
    if let Some(ref crop) = config.crop {
        args.push(format!("--crop={}", crop));
    }
    if config.torch {
        args.push("--camera-torch".to_string());
    }
    args.push("--no-window".to_string());
    args.push("--no-control".to_string());
    args.push("--no-playback".to_string());

    if is_pipe {
        args.push("-r".to_string());
        args.push("-".to_string());
        args.push("--record-format=mkv".to_string());
    } else {
        args.push(format!("--v4l2-sink={}", sink_or_record));
    }

    args
}

/// Pure helper function to build ffmpeg negate args
pub fn build_ffmpeg_invert_args(sink: &str) -> Vec<String> {
    vec![
        "-y".to_string(),
        "-f".to_string(),
        "matroska".to_string(),
        "-i".to_string(),
        "-".to_string(),
        "-vf".to_string(),
        "negate".to_string(),
        "-f".to_string(),
        "v4l2".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        sink.to_string(),
    ]
}

pub struct StreamManager {
    config: AppConfig,
    active_child: Option<Child>,
    ffmpeg_child: Option<Child>,
    standby_child: Option<Child>,
    state: StreamState,
    log_file_path: String,
}

impl StreamManager {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            active_child: None,
            ffmpeg_child: None,
            standby_child: None,
            state: StreamState::Stopped,
            log_file_path: "/tmp/phonecam.log".to_string(),
        }
    }

    pub fn current_config(&self) -> &AppConfig {
        &self.config
    }

    /// Update configuration and restart stream if it is currently active
    pub fn update_config<F: FnOnce(&mut AppConfig)>(&mut self, f: F) -> Result<()> {
        f(&mut self.config);
        if self.is_running() {
            let new_cfg = self.config.clone();
            self.start(new_cfg)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn state(&self) -> &StreamState {
        &self.state
    }

    pub fn is_running(&mut self) -> bool {
        if let Some(child) = &mut self.active_child {
            match child.try_wait() {
                Ok(None) => true,
                Ok(Some(status)) => {
                    info!("Stream process exited with status: {}", status);
                    self.state = StreamState::Stopped;
                    false
                }
                Err(e) => {
                    warn!("Error checking stream process: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    /// Start or restart streaming with updated configuration
    pub fn start(&mut self, config: AppConfig) -> Result<()> {
        let _ = self.stop();

        self.config = config;
        self.state = StreamState::Starting;

        // Ensure v4l2loopback is loaded and device node is ready
        let dev_path = setup_loopback_device(self.config.video_nr, &self.config.card_label)
            .context("Failed setting up v4l2 virtual device")?;

        let dev_str = dev_path.to_string_lossy().to_string();

        let log_file = File::create(&self.log_file_path).context("Failed to create log file")?;
        let log_err = log_file
            .try_clone()
            .context("Failed cloning log file handle")?;

        info!("Starting scrcpy camera stream with sink: {}", dev_str);

        if self.config.invert {
            info!("Inversion active: routing scrcpy through ffmpeg negate filter");

            let scrcpy_args = build_scrcpy_args(&self.config, &dev_str, true);
            let mut scrcpy_cmd = Command::new("scrcpy");
            scrcpy_cmd.args(&scrcpy_args);
            scrcpy_cmd
                .stdout(Stdio::piped())
                .stderr(Stdio::from(log_err));

            let mut scrcpy_child = scrcpy_cmd
                .spawn()
                .context("Failed to spawn scrcpy for invert pipeline")?;

            let scrcpy_stdout = scrcpy_child
                .stdout
                .take()
                .context("Failed to capture scrcpy stdout")?;

            let ffmpeg_args = build_ffmpeg_invert_args(&dev_str);
            let mut ffmpeg_cmd = Command::new("ffmpeg");
            ffmpeg_cmd
                .args(&ffmpeg_args)
                .stdin(Stdio::from(scrcpy_stdout))
                .stdout(Stdio::null())
                .stderr(Stdio::from(log_file));

            let ffmpeg_child = ffmpeg_cmd
                .spawn()
                .context("Failed to spawn ffmpeg negate filter")?;

            self.active_child = Some(scrcpy_child);
            self.ffmpeg_child = Some(ffmpeg_child);
        } else {
            let scrcpy_args = build_scrcpy_args(&self.config, &dev_str, false);
            let mut scrcpy_cmd = Command::new("scrcpy");
            scrcpy_cmd.args(&scrcpy_args);
            scrcpy_cmd
                .stdout(Stdio::from(log_file))
                .stderr(Stdio::from(log_err));

            let child = scrcpy_cmd
                .spawn()
                .context("Failed to spawn scrcpy camera stream")?;

            self.active_child = Some(child);
        }

        std::thread::sleep(Duration::from_millis(800));

        if !self.is_running() {
            let log_content = std::fs::read_to_string(&self.log_file_path).unwrap_or_default();
            let err_msg = format!("scrcpy failed to start. Log: {}", log_content);
            self.state = StreamState::Error(err_msg.clone());
            anyhow::bail!(err_msg);
        }

        self.state = StreamState::Running;
        info!("Camera stream is running cleanly on {}", dev_str);
        Ok(())
    }

    /// Stop streaming and detach virtual device
    pub fn stop(&mut self) -> Result<()> {
        info!("Stopping camera stream...");

        self.exit_standby();

        if let Some(mut child) = self.active_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        if let Some(mut child) = self.ffmpeg_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        let _ = Command::new("pkill")
            .args(["-f", "scrcpy.*--video-source=camera"])
            .output();

        self.state = StreamState::Stopped;

        let _ = detach_loopback_device(self.config.video_nr);

        info!("Camera stream stopped and virtual device detached.");
        Ok(())
    }

    /// Enter standby mode (e.g. when USB cable is unplugged).
    /// Keeps /dev/video10 active with a continuous blank video feed
    /// so client applications (OBS, Discord, Zoom) don't crash or lose the device.
    pub fn enter_standby(&mut self) -> Result<()> {
        if self.standby_child.is_some() {
            return Ok(());
        }

        info!(
            "Phone disconnected: entering standby mode with blank frame loop on {}",
            self.config.video_device_path()
        );
        let dev_str = self.config.video_device_path();

        let standby = Command::new("ffmpeg")
            .args([
                "-re",
                "-f",
                "lavfi",
                "-i",
                &format!(
                    "color=c=black:s={}:r={}",
                    self.config.resolution, self.config.fps
                ),
                "-f",
                "v4l2",
                "-pix_fmt",
                "yuv420p",
                &dev_str,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();

        self.standby_child = standby;
        self.state = StreamState::StandbyWaitingReconnect;
        Ok(())
    }

    /// Exit standby mode and terminate the blank frame generator
    pub fn exit_standby(&mut self) {
        if let Some(mut child) = self.standby_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Check health and perform automatic recovery when cable is reconnected
    pub fn check_and_recover(&mut self) -> Result<()> {
        match self.state {
            StreamState::Running => {
                if !self.is_running() {
                    // scrcpy exited unexpectedly (e.g. cable unplugged)
                    let _ = self.enter_standby();
                }
            }
            StreamState::StandbyWaitingReconnect => {
                // Poll ADB to check if smartphone has reconnected
                if let Ok(dev) = crate::adb::detect_device()
                    && dev.status == crate::adb::DeviceStatus::Connected
                {
                    info!("Phone reconnected! Stopping standby and resuming live stream...");
                    self.exit_standby();
                    let cfg = self.config.clone();
                    let _ = self.start(cfg);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Read the latest lines of logs
    pub fn get_recent_logs(&self, max_lines: usize) -> Vec<String> {
        if let Ok(content) = std::fs::read_to_string(&self.log_file_path) {
            let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            let start = if lines.len() > max_lines {
                lines.len() - max_lines
            } else {
                0
            };
            lines[start..].to_vec()
        } else {
            Vec::new()
        }
    }
}

impl Drop for StreamManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
