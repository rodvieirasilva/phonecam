use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub video_nr: u32,
    pub card_label: String,
    pub facing: String,
    pub camera_id: Option<u32>,
    pub resolution: String,
    pub crop: Option<String>,
    pub fps: u32,
    pub flip_h: bool,
    pub flip_v: bool,
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
            crop: None,
            fps: 30,
            flip_h: false,
            flip_v: false,
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
        match (self.flip_h, self.flip_v) {
            (false, false) => "0",
            (true, false) => "flip0",
            (false, true) => "180",
            (true, true) => "flip180",
        }
    }
}
