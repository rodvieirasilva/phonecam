use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone, PartialEq, Eq)]
#[command(
    name = "phonecam",
    author = "Rodrigo Vieira",
    version = env!("CARGO_PKG_VERSION"),
    about = "High-performance smartphone webcam virtual device manager for Omarchy"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Launch the GTK4 / Libadwaita desktop user interface (default)
    #[arg(long)]
    pub gui: bool,

    /// Launch interactive terminal dashboard with auto-detach on any key
    #[arg(long, short = 't')]
    pub tui: bool,

    /// Start streaming immediately in headless mode
    #[arg(long)]
    pub start: bool,

    /// Stop streaming and detach virtual video device immediately
    #[arg(long, short = 'd')]
    pub detach: bool,

    /// Check connection and print current status
    #[arg(long, short = 's')]
    pub status: bool,

    /// List available camera IDs from connected smartphone
    #[arg(long)]
    pub list_cameras: bool,

    /// Camera facing direction: 'back' or 'front'
    #[arg(long, short = 'f', default_value = "back")]
    pub facing: String,

    /// Specific device camera id to mirror (e.g. 0, 1, 2)
    #[arg(long)]
    pub camera_id: Option<u32>,

    /// Video capture resolution (e.g. 1920x1080, 1280x720, 3840x2160)
    #[arg(long, short = 'r', default_value = "1920x1080")]
    pub res: String,

    /// Camera capture framerate
    #[arg(long, default_value_t = 30)]
    pub fps: u32,

    /// Camera capture aspect ratio crop (e.g. 16:9, 4:3, 1:1, sensor)
    #[arg(long)]
    pub camera_ar: Option<String>,

    /// Crop area on camera sensor: width:height:x:y
    #[arg(long)]
    pub crop: Option<String>,

    /// Mirror / flip image horizontally (great for selfie webcams)
    #[arg(long, short = 'm')]
    pub mirror: bool,

    /// Flip image vertically
    #[arg(long)]
    pub flip_v: bool,

    /// Clockwise capture rotation in degrees (0, 90, 180, 270)
    #[arg(long, default_value_t = 0)]
    pub rotation: u32,

    /// Invert video colors (negative effect)
    #[arg(long, short = 'i')]
    pub invert: bool,

    /// Show local on-screen preview window
    #[arg(long, short = 'p')]
    pub preview: bool,

    /// Turn on camera flashlight / torch
    #[arg(long)]
    pub torch: bool,

    /// Automatically start streaming when app is opened
    #[arg(long)]
    pub auto_start: bool,

    /// Connect to smartphone over Wi-Fi via ADB (e.g. 192.168.1.50:5555)
    #[arg(long)]
    pub wifi: Option<String>,

    /// Enable ADB TCP/IP mode on USB device (default port: 5555)
    #[arg(long)]
    pub tcpip: Option<Option<u16>>,

    /// Virtual video device number (e.g. 10 for /dev/video10)
    #[arg(long, default_value_t = 10)]
    pub video_nr: u32,

    /// Internal HTTP API server port
    #[arg(long, default_value_t = 41789)]
    pub port: u16,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum Commands {
    /// Install PhoneCam binary, desktop launcher, icons, and configuration
    Install {
        /// Install only for the current user without system root configuration
        #[arg(long)]
        user: bool,
    },
    /// Uninstall PhoneCam and clean up system and user files
    Uninstall {
        /// Uninstall only user files without modifying system directories
        #[arg(long)]
        user: bool,
    },
}
