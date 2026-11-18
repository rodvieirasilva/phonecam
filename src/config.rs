use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub video_nr: u32,
    pub card_label: String,
    pub facing: String,
    pub camera_id: Option<u32>,
    pub resolution: String,
    pub camera_ar: Option<String>,
    pub crop: Option<String>,
    pub fps: u32,
    pub flip_h: bool,
    pub flip_v: bool,
    #[serde(default)]
    pub rotation: u32,
    pub invert: bool,
    pub preview: bool,
    pub torch: bool,
    pub port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            video_nr: 10,
            card_label: "PhoneCam Virtual Camera".to_string(),
            facing: "back".to_string(),
            camera_id: None,
            resolution: "1920x1080".to_string(),
            camera_ar: None,
            crop: None,
            fps: 30,
            flip_h: false,
            flip_v: false,
            rotation: 0,
            invert: false,
            preview: false,
            torch: false,
            port: 41789,
        }
    }
}

impl AppConfig {
    pub fn video_device_path(&self) -> String {
        format!("/dev/video{}", self.video_nr)
    }

    /// Calculate capture orientation flag for scrcpy
    pub fn orientation_flag(&self) -> &'static str {
        let rot = if self.rotation != 0 {
            self.rotation
        } else if self.flip_v {
            180
        } else {
            0
        };

        match (self.flip_h, rot) {
            (false, 0) => "0",
            (false, 90) => "90",
            (false, 180) => "180",
            (false, 270) => "270",
            (true, 0) => "flip0",
            (true, 90) => "flip90",
            (true, 180) => "flip180",
            (true, 270) => "flip270",
            _ => "0",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserStateConfig {
    #[serde(default = "default_last_ip")]
    pub last_ip: String,
    #[serde(default)]
    pub saved_ips: Vec<String>,
    #[serde(default = "default_facing")]
    pub facing: String,
    #[serde(default)]
    pub camera_id: Option<String>,
    #[serde(default = "default_resolution")]
    pub resolution: String,
    #[serde(default)]
    pub crop: Option<String>,
    #[serde(default)]
    pub camera_ar: Option<String>,
    #[serde(default)]
    pub flip_h: bool,
    #[serde(default)]
    pub flip_v: bool,
    #[serde(default)]
    pub rotation: u32,
    #[serde(default)]
    pub invert: bool,
    #[serde(default)]
    pub torch: bool,
    #[serde(default)]
    pub preview: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub last_device_serial: Option<String>,
    #[serde(default)]
    pub use_credentials_file: bool,
}

fn default_last_ip() -> String {
    "192.168.1.50:5555".to_string()
}
fn default_facing() -> String {
    "back".to_string()
}
fn default_resolution() -> String {
    "1920x1080".to_string()
}

impl Default for UserStateConfig {
    fn default() -> Self {
        Self {
            last_ip: default_last_ip(),
            saved_ips: vec![default_last_ip()],
            facing: default_facing(),
            camera_id: None,
            resolution: default_resolution(),
            crop: None,
            camera_ar: None,
            flip_h: false,
            flip_v: false,
            rotation: 0,
            invert: false,
            torch: false,
            preview: false,
            auto_start: false,
            last_device_serial: None,
            use_credentials_file: false,
        }
    }
}

impl UserStateConfig {
    pub fn config_file_path() -> PathBuf {
        crate::util::home_dir().join(".config/phonecam/config.toml")
    }

    pub fn template_str() -> &'static str {
        include_str!("../config.example.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_file_path();
        if let Ok(data) = std::fs::read_to_string(&path)
            && let Ok(cfg) = toml::from_str::<Self>(&data)
        {
            return cfg;
        }

        // Migrate from old config.json if present
        let json_path = crate::util::home_dir().join(".config/phonecam/config.json");
        if let Ok(data) = std::fs::read_to_string(&json_path)
            && let Ok(cfg) = serde_json::from_str::<Self>(&data)
        {
            let _ = cfg.save();
            let _ = std::fs::remove_file(&json_path);
            return cfg;
        }

        // If file doesn't exist, initialize from template
        let default_cfg: Self = toml::from_str(Self::template_str()).unwrap_or_default();
        let _ = default_cfg.save();
        default_cfg
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_file_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn remember_ip(&mut self, ip: &str) {
        let trimmed = ip.trim();
        if !trimmed.is_empty() {
            self.last_ip = trimmed.to_string();
            if !self.saved_ips.contains(&self.last_ip) {
                self.saved_ips.push(self.last_ip.clone());
            }
        }
    }
}
