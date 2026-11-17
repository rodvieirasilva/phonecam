use std::fs::File;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::config::AppConfig;
use crate::v4l2::{setup_loopback_device, detach_loopback_device};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamState {
    Stopped,
    Starting,
    Running,
    Error(String),
}

pub struct StreamManager {
    config: AppConfig,
    active_child: Option<Child>,
    ffmpeg_child: Option<Child>,
    state: StreamState,
    log_file_path: String,
}

impl StreamManager {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            active_child: None,
            ffmpeg_child: None,
            state: StreamState::Stopped,
            log_file_path: "/tmp/phonecam.log".to_string(),
        }
    }

    pub fn current_config(&self) -> &AppConfig {
        &self.config
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
        // Stop any running stream first
        let _ = self.stop();

        self.config = config;
        self.state = StreamState::Starting;

        // Ensure v4l2loopback is loaded and device node is ready
        let dev_path = setup_loopback_device(self.config.video_nr, &self.config.card_label)
            .context("Failed setting up v4l2 virtual device")?;

        let dev_str = dev_path.to_string_lossy().to_string();

        let log_file = File::create(&self.log_file_path)
            .context("Failed to create log file")?;
        let log_err = log_file.try_clone().context("Failed cloning log file handle")?;

        info!("Starting scrcpy camera stream with sink: {}", dev_str);

        if self.config.invert {
            // Pipeline mode: scrcpy -> stdout -> ffmpeg negate -> v4l2sink
            info!("Inversion active: routing scrcpy through ffmpeg negate filter");

            let mut scrcpy_cmd = Command::new("scrcpy");
            scrcpy_cmd
                .arg("--video-source=camera")
                .arg(format!("--camera-facing={}", self.config.facing))
                .arg(format!("--camera-size={}", self.config.resolution))
                .arg(format!("--camera-fps={}", self.config.fps))
                .arg(format!("--capture-orientation={}", self.config.orientation_flag()))
                .arg("--no-audio")
                .arg("-r")
                .arg("-")
                .arg("--record-format=mkv");

            if let Some(id) = self.config.camera_id {
                scrcpy_cmd.arg(format!("--camera-id={}", id));
            }
            if let Some(ref crop) = self.config.crop {
                scrcpy_cmd.arg(format!("--crop={}", crop));
            }
            if self.config.torch {
                scrcpy_cmd.arg("--camera-torch");
            }
            if !self.config.preview {
                scrcpy_cmd.arg("--no-playback");
            }

            scrcpy_cmd.stdout(Stdio::piped()).stderr(Stdio::from(log_err));

            let mut scrcpy_child = scrcpy_cmd.spawn()
                .context("Failed to spawn scrcpy for invert pipeline")?;

            let scrcpy_stdout = scrcpy_child.stdout.take()
                .context("Failed to capture scrcpy stdout")?;

            // ffmpeg -i - -vf "negate" -f v4l2 /dev/video10
            let mut ffmpeg_cmd = Command::new("ffmpeg");
            ffmpeg_cmd
                .args(["-y", "-f", "matroska", "-i", "-"])
                .args(["-vf", "negate"])
                .args(["-f", "v4l2", "-pix_fmt", "yuv420p", &dev_str])
                .stdin(Stdio::from(scrcpy_stdout))
                .stdout(Stdio::null())
                .stderr(Stdio::from(log_file));

            let ffmpeg_child = ffmpeg_cmd.spawn()
                .context("Failed to spawn ffmpeg negate filter")?;

            self.active_child = Some(scrcpy_child);
            self.ffmpeg_child = Some(ffmpeg_child);
        } else {
            // Direct zero-latency mode: scrcpy directly into v4l2sink
            let mut scrcpy_cmd = Command::new("scrcpy");
            scrcpy_cmd
                .arg("--video-source=camera")
                .arg(format!("--camera-facing={}", self.config.facing))
                .arg(format!("--camera-size={}", self.config.resolution))
                .arg(format!("--camera-fps={}", self.config.fps))
                .arg(format!("--capture-orientation={}", self.config.orientation_flag()))
                .arg("--no-audio")
                .arg(format!("--v4l2-sink={}", dev_str));

            if let Some(id) = self.config.camera_id {
                scrcpy_cmd.arg(format!("--camera-id={}", id));
            }
            if let Some(ref crop) = self.config.crop {
                scrcpy_cmd.arg(format!("--crop={}", crop));
            }
            if self.config.torch {
                scrcpy_cmd.arg("--camera-torch");
            }
            if !self.config.preview {
                scrcpy_cmd.arg("--no-playback");
            }

            scrcpy_cmd.stdout(Stdio::from(log_file)).stderr(Stdio::from(log_err));

            let child = scrcpy_cmd.spawn()
                .context("Failed to spawn scrcpy camera stream")?;

            self.active_child = Some(child);
        }

        // Give it 800ms to verify it didn't exit immediately with an error
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

        if let Some(mut child) = self.active_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        if let Some(mut child) = self.ffmpeg_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        // Kill any dangling scrcpy camera instances
        let _ = Command::new("pkill").args(["-f", "scrcpy.*--video-source=camera"]).output();

        self.state = StreamState::Stopped;

        // Detach loopback device
        let _ = detach_loopback_device(self.config.video_nr);

        info!("Camera stream stopped and virtual device detached.");
        Ok(())
    }

    /// Read the latest lines of logs
    pub fn get_recent_logs(&self, max_lines: usize) -> Vec<String> {
        if let Ok(content) = std::fs::read_to_string(&self.log_file_path) {
            let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            let start = if lines.len() > max_lines { lines.len() - max_lines } else { 0 };
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
