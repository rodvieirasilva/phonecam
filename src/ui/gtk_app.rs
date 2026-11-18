use adw::prelude::*;
use gtk4::gio;
use gtk4::glib;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;

use crate::config::UserStateConfig;

pub fn run_gtk_app(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting PhoneCam native GTK4/Adwaita application on port {port}");

    let app = adw::Application::builder()
        .application_id("com.omarchy.phonecam")
        .build();

    let port_rc = Rc::new(port);

    app.connect_activate(move |app| {
        if let Some(existing_window) = app.active_window() {
            existing_window.present();
            return;
        }

        let window = build_ui_window(Some(app), *port_rc);
        window.present();
    });

    let empty_args: [&str; 0] = [];
    let _ = app.run_with_args(&empty_args);

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemePalette {
    pub mode: String,
    pub accent: String,
    pub background: String,
    pub dark_background: String,
    pub darker_background: String,
    pub lighter_background: String,
    pub foreground: String,
    pub bright_foreground: String,
    pub green: String,
    pub red: String,
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self {
            mode: "dark".to_string(),
            accent: crate::util::get_omarchy_accent(),
            background: "#1a1b26".to_string(),
            dark_background: "#13141c".to_string(),
            darker_background: "#0e0e14".to_string(),
            lighter_background: "#24283b".to_string(),
            foreground: "#a9b1d6".to_string(),
            bright_foreground: "#c0caf5".to_string(),
            green: "#9ece6a".to_string(),
            red: "#f7768e".to_string(),
        }
    }
}

impl ThemePalette {
    pub fn load_current() -> Self {
        let colors_path =
            crate::util::home_dir().join(".local/state/omarchy/current/theme/colors.toml");
        if let Ok(content) = std::fs::read_to_string(&colors_path) {
            let mut pal = Self::default();
            for line in content.lines() {
                let line = line.trim();
                if let Some((k, v)) = line.split_once('=') {
                    let k = k.trim();
                    let v = v.trim().trim_matches('"').trim_matches('\'');
                    match k {
                        "mode" => pal.mode = v.to_string(),
                        "accent" => pal.accent = v.to_string(),
                        "background" => pal.background = v.to_string(),
                        "dark_background" => pal.dark_background = v.to_string(),
                        "darker_background" => pal.darker_background = v.to_string(),
                        "lighter_background" => pal.lighter_background = v.to_string(),
                        "foreground" => pal.foreground = v.to_string(),
                        "bright_foreground" => pal.bright_foreground = v.to_string(),
                        "green" => pal.green = v.to_string(),
                        "red" => pal.red = v.to_string(),
                        _ => {}
                    }
                }
            }
            return pal;
        }
        Self::default()
    }
}

pub fn build_theme_css(palette: &ThemePalette) -> String {
    let accent_fg = if palette.mode == "light" {
        "#ffffff"
    } else {
        "#18181b"
    };

    let template = include_str!("theme.css");
    template
        .replace("{bg}", &palette.background)
        .replace("{fg}", &palette.bright_foreground)
        .replace("{dark_bg}", &palette.dark_background)
        .replace("{card_bg}", &palette.lighter_background)
        .replace("{accent}", &palette.accent)
        .replace("{accent_fg}", accent_fg)
}

pub fn apply_theme(provider: &gtk4::CssProvider, palette: &ThemePalette) {
    let style_manager = adw::StyleManager::default();
    if palette.mode == "light" {
        style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
    } else {
        style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
    }
    let css = build_theme_css(palette);
    provider.load_from_data(&css);
}

pub fn setup_theme_monitor(
    css_provider: &gtk4::CssProvider,
    current_palette: &Rc<RefCell<ThemePalette>>,
) -> Vec<gio::FileMonitor> {
    let mut monitors = Vec::new();
    let theme_dir = crate::util::home_dir().join(".local/state/omarchy/current/theme");
    let current_dir = crate::util::home_dir().join(".local/state/omarchy/current");

    let css_provider = css_provider.clone();
    let current_palette = current_palette.clone();

    let reload_theme = Rc::new(move || {
        let new_palette = ThemePalette::load_current();
        if new_palette != *current_palette.borrow() {
            *current_palette.borrow_mut() = new_palette.clone();
            apply_theme(&css_provider, &new_palette);
        }
    });

    for path in [&current_dir, &theme_dir] {
        if path.exists() {
            let file = gio::File::for_path(path);
            if let Ok(monitor) =
                file.monitor_directory(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
            {
                let cb = reload_theme.clone();
                let debounce_id = Rc::new(RefCell::new(None::<glib::SourceId>));
                let debounce_id_clone = debounce_id.clone();

                monitor.connect_changed(move |_, _, _, _| {
                    if let Some(id) = debounce_id_clone.borrow_mut().take() {
                        id.remove();
                    }
                    let cb_inner = cb.clone();
                    let id = glib::timeout_add_local_once(
                        std::time::Duration::from_millis(80),
                        move || {
                            cb_inner();
                        },
                    );
                    *debounce_id_clone.borrow_mut() = Some(id);
                });
                monitors.push(monitor);
            }
        }
    }

    monitors
}

pub fn build_ui_window(app: Option<&adw::Application>, port: u16) -> adw::ApplicationWindow {
    let user_cfg = Rc::new(RefCell::new(UserStateConfig::load()));
    let initial_cfg = user_cfg.borrow().clone();

    let version = env!("CARGO_PKG_VERSION");
    let window_title = format!("PhoneCam v{} — Smartphone Virtual Webcam", version);
    let mut builder = adw::ApplicationWindow::builder()
        .title(&window_title)
        .icon_name("phonecam")
        .default_width(560)
        .default_height(780);
    if let Some(a) = app {
        builder = builder.application(a);
    }
    let window = builder.build();
    window.set_widget_name("window-main");

    let header = adw::HeaderBar::new();
    let title = adw::WindowTitle::new(
        &format!("PhoneCam v{}", version),
        "Studio Smartphone Webcam for Omarchy",
    );
    header.set_title_widget(Some(&title));

    let refresh_btn = gtk4::Button::builder()
        .name("btn-refresh")
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Refresh device & logs")
        .valign(gtk4::Align::Center)
        .build();

    let page = adw::PreferencesPage::new();

    // 1. Virtual Webcam Device Group (Start Webcam, Preview, Auto-start at the top)
    let stream_group = adw::PreferencesGroup::builder()
        .title("Virtual Device /dev/video10")
        .name("group-stream")
        .build();

    let stream_row = adw::ActionRow::builder()
        .name("text-stream-status")
        .title("VIRTUAL CAMERA OFFLINE")
        .subtitle("Device node: /dev/video10 (v4l2loopback)")
        .build();

    let btn_stream_action = gtk4::Button::builder()
        .name("btn-stream-action")
        .label("Start Webcam")
        .valign(gtk4::Align::Center)
        .css_classes(["suggested-action"])
        .tooltip_text("Start virtual camera stream (auto-connects to saved Wi-Fi if needed)")
        .build();

    let btn_preview = gtk4::Button::builder()
        .name("btn-preview")
        .label("Preview")
        .valign(gtk4::Align::Center)
        .tooltip_text("Open real-time desktop preview of /dev/video10")
        .build();

    let stream_btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    stream_btn_box.append(&btn_stream_action);
    stream_btn_box.append(&btn_preview);
    stream_row.add_suffix(&stream_btn_box);
    stream_group.add(&stream_row);

    let switch_auto_start = gtk4::Switch::builder()
        .name("switch-auto-start")
        .active(initial_cfg.auto_start)
        .valign(gtk4::Align::Center)
        .build();

    let row_auto_start = adw::ActionRow::builder()
        .name("row-auto-start")
        .title("Auto-start Webcam on Launch")
        .subtitle("Automatically activate camera stream when PhoneCam opens")
        .activatable_widget(&switch_auto_start)
        .build();
    row_auto_start.add_suffix(&switch_auto_start);
    stream_group.add(&row_auto_start);
    page.add(&stream_group);

    // 2. Smartphone Device & Wireless Connection Group
    let device_group = adw::PreferencesGroup::builder()
        .title("Smartphone Device")
        .description("USB-C or Wireless ADB Connection")
        .name("group-device")
        .build();

    let device_row = adw::ActionRow::builder()
        .name("text-device-name")
        .title("Detecting Smartphone...")
        .subtitle("Connect phone via USB-C or Wi-Fi ADB")
        .build();

    let battery_label = gtk4::Label::builder()
        .name("text-battery")
        .label("")
        .css_classes(["dim-label"])
        .valign(gtk4::Align::Center)
        .build();

    let dev_suffix_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    dev_suffix_box.append(&battery_label);
    dev_suffix_box.append(&refresh_btn);
    device_row.add_suffix(&dev_suffix_box);
    device_group.add(&device_row);

    let wifi_entry = gtk4::Entry::builder()
        .name("input-wifi-address")
        .placeholder_text("192.168.1.50:5555")
        .text(&initial_cfg.last_ip)
        .valign(gtk4::Align::Center)
        .hexpand(true)
        .build();

    let wifi_entry_row = adw::ActionRow::builder()
        .title("Wireless IP:Port")
        .activatable_widget(&wifi_entry)
        .build();
    wifi_entry_row.add_suffix(&wifi_entry);
    device_group.add(&wifi_entry_row);

    let wifi_actions_row = adw::ActionRow::builder()
        .name("row-wifi-actions")
        .title("Wireless Pairing Actions")
        .build();

    let btn_wifi_connect = gtk4::Button::builder()
        .name("btn-wifi-connect")
        .label("Connect Wi-Fi")
        .valign(gtk4::Align::Center)
        .css_classes(["suggested-action"])
        .build();

    let btn_wifi_tcpip = gtk4::Button::builder()
        .name("btn-wifi-tcpip")
        .label("Enable USB TCP/IP")
        .valign(gtk4::Align::Center)
        .tooltip_text("Switches USB-connected phone to port 5555 for wireless disconnection")
        .build();

    let wifi_btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    wifi_btn_box.append(&btn_wifi_tcpip);
    wifi_btn_box.append(&btn_wifi_connect);
    wifi_actions_row.add_suffix(&wifi_btn_box);
    device_group.add(&wifi_actions_row);

    let doc_link_row = adw::ActionRow::builder()
        .name("row-adb-docs")
        .title("Setup Guide")
        .subtitle("Learn how to enable USB Debugging &amp; Wireless ADB on Android")
        .build();

    let doc_link_btn = gtk4::LinkButton::builder()
        .name("link-adb-docs")
        .uri("https://github.com/rom1v/scrcpy/blob/master/doc/connection.md")
        .label("View Guide ↗")
        .valign(gtk4::Align::Center)
        .build();
    doc_link_row.add_suffix(&doc_link_btn);
    device_group.add(&doc_link_row);

    let wifi_status_row = adw::ActionRow::builder()
        .name("text-wifi-status")
        .title("WI-FI DISCONNECTED")
        .subtitle("Connect phone via USB once and click 'Enable USB TCP/IP', then unplug cable and click 'Connect Wi-Fi'.")
        .build();
    device_group.add(&wifi_status_row);
    page.add(&device_group);

    // 3. Camera Selection & Sensor Group
    let camera_group = adw::PreferencesGroup::builder()
        .title("Camera Selection &amp; Sensor")
        .name("group-camera")
        .build();

    let facing_row = adw::ActionRow::builder()
        .title("Camera Facing")
        .subtitle("Select rear primary or front selfie camera")
        .build();

    let is_facing_front = initial_cfg.facing == "front";
    let btn_camera_rear = gtk4::Button::builder()
        .name("btn-camera-rear")
        .label("Rear (Back)")
        .valign(gtk4::Align::Center)
        .css_classes(if !is_facing_front {
            vec!["suggested-action"]
        } else {
            vec![]
        })
        .build();

    let btn_camera_front = gtk4::Button::builder()
        .name("btn-camera-front")
        .label("Selfie (Front)")
        .valign(gtk4::Align::Center)
        .css_classes(if is_facing_front {
            vec!["suggested-action"]
        } else {
            vec![]
        })
        .build();

    let facing_btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    facing_btn_box.append(&btn_camera_rear);
    facing_btn_box.append(&btn_camera_front);
    facing_row.add_suffix(&facing_btn_box);
    camera_group.add(&facing_row);

    let lens_strings = gtk4::StringList::new(&["Default Camera"]);
    let combo_camera_lens = adw::ComboRow::builder()
        .name("combo-camera-lens")
        .title("Specific Lens Sensor")
        .model(&lens_strings)
        .build();
    camera_group.add(&combo_camera_lens);
    page.add(&camera_group);

    // 4. Geometry, Cut & Resize Group
    let geom_group = adw::PreferencesGroup::builder()
        .title("Cut (Crop) &amp; Resize")
        .name("group-geometry")
        .build();

    let res_strings = gtk4::StringList::new(&[
        "1920x1080 (1080p FHD)",
        "1280x720 (720p HD)",
        "3840x2160 (4K UHD)",
        "2560x1440 (2K QHD)",
    ]);
    let initial_res_idx = match initial_cfg.resolution.as_str() {
        "1280x720" => 1,
        "3840x2160" => 2,
        "2560x1440" => 3,
        _ => 0,
    };
    let combo_resolution = adw::ComboRow::builder()
        .name("combo-resolution")
        .title("Resolution (Resize)")
        .model(&res_strings)
        .selected(initial_res_idx)
        .build();
    geom_group.add(&combo_resolution);

    let crop_strings = gtk4::StringList::new(&[
        "Full Sensor (Native)",
        "16:9 Widescreen Crop",
        "4:3 Standard Crop",
        "1:1 Square (Avatar)",
        "Custom 1080p Center Cut",
    ]);
    let initial_crop_idx = match initial_cfg
        .crop
        .as_deref()
        .or(initial_cfg.camera_ar.as_deref())
    {
        Some("16:9") => 1,
        Some("4:3") => 2,
        Some("1:1") => 3,
        Some("custom") | Some("1920:1080:0:0") => 4,
        _ => 0,
    };
    let combo_crop = adw::ComboRow::builder()
        .name("combo-crop")
        .title("Framing / Cut (Crop)")
        .model(&crop_strings)
        .selected(initial_crop_idx)
        .build();
    geom_group.add(&combo_crop);
    page.add(&geom_group);

    // 5. Transformations & Filters Group
    let fx_group = adw::PreferencesGroup::builder()
        .title("Transform, Invert &amp; Filters")
        .name("group-transforms")
        .build();

    let rotation_strings = gtk4::StringList::new(&[
        "0° (Normal)",
        "90° (Clockwise)",
        "180° (Inverted)",
        "270° (Counter-Clockwise)",
    ]);
    let initial_rot_idx = match initial_cfg.rotation {
        90 => 1,
        180 => 2,
        270 => 3,
        _ => {
            if initial_cfg.flip_v {
                2
            } else {
                0
            }
        }
    };
    let combo_rotation = adw::ComboRow::builder()
        .name("combo-rotation")
        .title("Rotation Angle")
        .subtitle("Clockwise camera sensor rotation (0°, 90°, 180°, 270°)")
        .model(&rotation_strings)
        .selected(initial_rot_idx)
        .build();
    fx_group.add(&combo_rotation);

    let switch_flip_h = gtk4::Switch::builder()
        .name("switch-flip-h")
        .active(initial_cfg.flip_h)
        .valign(gtk4::Align::Center)
        .build();
    let row_flip_h = adw::ActionRow::builder()
        .title("Mirror (Flip Horizontal)")
        .subtitle("Natural webcam orientation")
        .activatable_widget(&switch_flip_h)
        .build();
    row_flip_h.add_suffix(&switch_flip_h);
    fx_group.add(&row_flip_h);

    let switch_invert = gtk4::Switch::builder()
        .name("switch-invert")
        .active(initial_cfg.invert)
        .valign(gtk4::Align::Center)
        .build();
    let row_invert = adw::ActionRow::builder()
        .title("Invert Colors (Negative)")
        .subtitle("Hardware ffmpeg color inversion filter")
        .activatable_widget(&switch_invert)
        .build();
    row_invert.add_suffix(&switch_invert);
    fx_group.add(&row_invert);

    let switch_torch = gtk4::Switch::builder()
        .name("switch-torch")
        .active(initial_cfg.torch)
        .valign(gtk4::Align::Center)
        .build();
    let row_torch = adw::ActionRow::builder()
        .title("Camera Torch (Flashlight)")
        .subtitle("Rear lens studio lighting")
        .activatable_widget(&switch_torch)
        .build();
    row_torch.add_suffix(&switch_torch);
    fx_group.add(&row_torch);
    page.add(&fx_group);

    // 7. Live Console & Diagnostics
    let logs_group = adw::PreferencesGroup::builder()
        .title("Logs &amp; Diagnostics")
        .description("Press ESC or 'q' to stop &amp; detach. Space to toggle stream.")
        .name("group-logs")
        .build();

    let logs_row = adw::ActionRow::builder()
        .name("row-logs")
        .title("Recent Activity")
        .subtitle("Ready")
        .build();
    logs_group.add(&logs_row);
    page.add(&logs_group);

    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    content_box.append(&header);
    content_box.append(&page);
    window.set_content(Some(&content_box));

    // Omarchy dynamic CSS theming
    let css_provider = gtk4::CssProvider::new();
    let current_palette = Rc::new(RefCell::new(ThemePalette::load_current()));
    apply_theme(&css_provider, &current_palette.borrow());
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    let theme_monitors = setup_theme_monitor(&css_provider, &current_palette);

    // State guards & bindings
    let is_updating = Arc::new(AtomicBool::new(false));
    let is_streaming_state = Arc::new(AtomicBool::new(false));

    // Connect button click events
    let is_updating_clone = is_updating.clone();
    let is_streaming_clone = is_streaming_state.clone();
    let btn_action_weak = btn_stream_action.downgrade();
    let stream_row_weak = stream_row.downgrade();
    let combo_rot_weak = combo_rotation.downgrade();
    let switch_fliph_weak = switch_flip_h.downgrade();
    let switch_inv_weak = switch_invert.downgrade();
    let switch_torch_weak = switch_torch.downgrade();
    let switch_as_weak = switch_auto_start.downgrade();
    let btn_front_weak = btn_camera_front.downgrade();
    let btn_rear_weak = btn_camera_rear.downgrade();
    let combo_lens_weak = combo_camera_lens.downgrade();
    let combo_res_weak = combo_resolution.downgrade();
    let combo_crop_weak = combo_crop.downgrade();
    let btn_preview_weak = btn_preview.downgrade();
    let btn_wifi_conn_weak = btn_wifi_connect.downgrade();
    let btn_wifi_tcp_weak = btn_wifi_tcpip.downgrade();
    let wifi_entry_weak = wifi_entry.downgrade();
    let wifi_status_weak = wifi_status_row.downgrade();

    // Consolidated Dynamic UI Sensitivities & Connection States
    let update_dynamic_ui = {
        let btn_action_weak = btn_action_weak.clone();
        let btn_preview_weak = btn_preview_weak.clone();
        let stream_row_weak = stream_row_weak.clone();
        let rear_btn_weak = btn_rear_weak.clone();
        let front_btn_weak = btn_front_weak.clone();
        let combo_lens_weak = combo_lens_weak.clone();
        let combo_res_weak = combo_res_weak.clone();
        let combo_crop_weak = combo_crop_weak.clone();
        let combo_rot_weak = combo_rot_weak.clone();
        let switch_as_weak = switch_as_weak.clone();
        let btn_wifi_conn_weak = btn_wifi_conn_weak.clone();
        let btn_wifi_tcp_weak = btn_wifi_tcp_weak.clone();
        let wifi_entry_weak = wifi_entry_weak.clone();
        let wifi_status_weak = wifi_status_weak.clone();

        move |streaming: bool, dev_status: &str, serial: &str| {
            if let (Some(btn), Some(s_row), Some(btn_prev)) = (
                btn_action_weak.upgrade(),
                stream_row_weak.upgrade(),
                btn_preview_weak.upgrade(),
            ) {
                if streaming {
                    btn.set_label("Stop & Detach");
                    btn.remove_css_class("suggested-action");
                    btn.add_css_class("destructive-action");
                    s_row.set_title("● VIRTUAL CAMERA ACTIVE");
                    btn_prev.set_label("Open Preview");
                } else {
                    btn.set_label("Start Webcam");
                    btn.remove_css_class("destructive-action");
                    btn.add_css_class("suggested-action");
                    s_row.set_title("VIRTUAL CAMERA OFFLINE");
                    btn_prev.set_label("Preview");
                }
            }

            let enable_settings = !streaming;
            if let Some(w) = rear_btn_weak.upgrade() {
                w.set_sensitive(enable_settings);
            }
            if let Some(w) = front_btn_weak.upgrade() {
                w.set_sensitive(enable_settings);
            }
            if let Some(w) = combo_lens_weak.upgrade() {
                w.set_sensitive(enable_settings);
            }
            if let Some(w) = combo_res_weak.upgrade() {
                w.set_sensitive(enable_settings);
            }
            if let Some(w) = combo_crop_weak.upgrade() {
                w.set_sensitive(enable_settings);
            }
            if let Some(w) = combo_rot_weak.upgrade() {
                w.set_sensitive(enable_settings);
            }
            if let Some(w) = switch_as_weak.upgrade() {
                w.set_sensitive(enable_settings);
            }

            // Wi-Fi / USB buttons state logic
            let is_wifi =
                dev_status == "Connected" && (serial.contains(':') || serial.contains('.'));
            let is_usb = dev_status == "Connected" && !is_wifi;

            if let (Some(btn_conn), Some(btn_tcp), Some(entry), Some(status_row)) = (
                btn_wifi_conn_weak.upgrade(),
                btn_wifi_tcp_weak.upgrade(),
                wifi_entry_weak.upgrade(),
                wifi_status_weak.upgrade(),
            ) {
                if streaming {
                    btn_conn.set_sensitive(false);
                    btn_tcp.set_sensitive(false);
                    entry.set_sensitive(false);
                } else if is_wifi {
                    btn_conn.set_label("Disconnect Wi-Fi");
                    btn_conn.remove_css_class("suggested-action");
                    btn_conn.add_css_class("destructive-action");
                    btn_conn.set_sensitive(true);
                    btn_tcp.set_sensitive(false);
                    entry.set_sensitive(true);
                    status_row.set_title("● WI-FI CONNECTED");
                    status_row.set_subtitle(&format!("Connected wirelessly via ADB ({serial})"));
                } else if is_usb {
                    btn_conn.set_label("Connect Wi-Fi");
                    btn_conn.remove_css_class("destructive-action");
                    btn_conn.add_css_class("suggested-action");
                    btn_tcp.set_sensitive(true);
                    btn_conn.set_sensitive(true);
                    entry.set_sensitive(true);
                    status_row.set_title("USB LINK ACTIVE");
                    status_row.set_subtitle(&format!(
                        "USB connected ({serial}). Click 'Enable USB TCP/IP' to prepare wireless mode."
                    ));
                } else if dev_status == "Unauthorized" {
                    btn_conn.set_label("Connect Wi-Fi");
                    btn_conn.remove_css_class("destructive-action");
                    btn_conn.add_css_class("suggested-action");
                    btn_tcp.set_sensitive(false);
                    btn_conn.set_sensitive(false);
                    entry.set_sensitive(false);
                    status_row.set_title("DEVICE UNAUTHORIZED");
                    status_row.set_subtitle("Unlock phone & tap 'Allow' for USB debugging.");
                } else {
                    // NotFound or Offline
                    btn_conn.set_label("Connect Wi-Fi");
                    btn_conn.remove_css_class("destructive-action");
                    btn_conn.add_css_class("suggested-action");
                    btn_tcp.set_sensitive(false);
                    btn_conn.set_sensitive(true);
                    entry.set_sensitive(true);
                    status_row.set_title("WI-FI DISCONNECTED");
                    status_row.set_subtitle(
                        "Enter phone IP:5555 and click 'Connect Wi-Fi', or click 'Start Webcam' to auto-connect.",
                    );
                }
            }
        }
    };

    // Initial sensitivities setup
    update_dynamic_ui(false, "NotFound", "");

    // Start / Stop Stream Action
    let update_ui_for_start = update_dynamic_ui.clone();
    let user_cfg_for_start = user_cfg.clone();
    let start_stop_handler = {
        let _is_updating = is_updating_clone.clone();
        let is_streaming = is_streaming_clone.clone();
        let combo_rot_weak = combo_rot_weak.clone();
        let switch_fliph_weak = switch_fliph_weak.clone();
        let switch_inv_weak = switch_inv_weak.clone();
        let switch_torch_weak = switch_torch_weak.clone();
        let btn_front_weak = btn_front_weak.clone();
        let combo_res_weak = combo_res_weak.clone();
        let combo_crop_weak = combo_crop_weak.clone();
        let wifi_entry_weak = wifi_entry_weak.clone();
        let wifi_status_weak = wifi_status_weak.clone();
        let user_cfg = user_cfg_for_start.clone();
        let update_ui = update_ui_for_start.clone();

        move || {
            let currently_streaming = is_streaming.load(Ordering::SeqCst);
            if currently_streaming {
                let _ = http_post(port, "/api/stop", None);
                is_streaming.store(false, Ordering::SeqCst);
                update_ui(false, "Connected", "");
            } else {
                // If phone is not connected, attempt auto-connect via Wi-Fi if an IP is configured
                let status_res = http_get(port, "/api/status");
                let is_connected = status_res
                    .as_ref()
                    .ok()
                    .and_then(|st| st.get("device").and_then(|d| d["status"].as_str()))
                    .map(|s| s == "Connected")
                    .unwrap_or(false);

                if !is_connected {
                    let entered_ip = wifi_entry_weak
                        .upgrade()
                        .map(|e| e.text().trim().to_string())
                        .unwrap_or_default();
                    let target_ip = if !entered_ip.is_empty() {
                        entered_ip
                    } else {
                        user_cfg.borrow().last_ip.trim().to_string()
                    };

                    if !target_ip.is_empty() {
                        let full_addr = if target_ip.contains(':') {
                            target_ip.clone()
                        } else {
                            format!("{target_ip}:5555")
                        };
                        user_cfg.borrow_mut().remember_ip(&full_addr);
                        let _ = user_cfg.borrow().save();

                        if let Some(st_row) = wifi_status_weak.upgrade() {
                            st_row
                                .set_subtitle(&format!("Auto-connecting Wi-Fi to {full_addr}..."));
                        }

                        let connect_result = http_post(
                            port,
                            "/api/connect-wifi",
                            Some(&serde_json::json!({ "address": full_addr })),
                        );

                        if let Some(st_row) = wifi_status_weak.upgrade() {
                            match connect_result {
                                Ok(val) => {
                                    let msg = val["message"].as_str().unwrap_or("Connected");
                                    st_row.set_subtitle(msg);
                                }
                                Err(e) => {
                                    st_row.set_subtitle(&format!("Auto-connect failed: {e}"));
                                }
                            }
                        }

                        // Allow ADB wireless handshake to establish before starting scrcpy
                        std::thread::sleep(std::time::Duration::from_millis(600));
                    }
                }

                let res_idx = combo_res_weak.upgrade().map(|c| c.selected()).unwrap_or(0);
                let resolution = match res_idx {
                    1 => "1280x720",
                    2 => "3840x2160",
                    3 => "2560x1440",
                    _ => "1920x1080",
                };

                let crop_idx = combo_crop_weak.upgrade().map(|c| c.selected()).unwrap_or(0);
                let (camera_ar, crop) = match crop_idx {
                    1 => (Some("16:9"), None),
                    2 => (Some("4:3"), None),
                    3 => (Some("1:1"), None),
                    4 => (None, Some("1920:1080:0:0")),
                    _ => (None, None),
                };

                let rot_idx = combo_rot_weak.upgrade().map(|c| c.selected()).unwrap_or(0);
                let rotation = match rot_idx {
                    1 => 90,
                    2 => 180,
                    3 => 270,
                    _ => 0,
                };

                let flip_h = switch_fliph_weak
                    .upgrade()
                    .map(|s| s.is_active())
                    .unwrap_or(false);

                let invert = switch_inv_weak
                    .upgrade()
                    .map(|s| s.is_active())
                    .unwrap_or(false);

                let torch = switch_torch_weak
                    .upgrade()
                    .map(|s| s.is_active())
                    .unwrap_or(false);

                let is_front = btn_front_weak
                    .upgrade()
                    .map(|b| b.has_css_class("suggested-action"))
                    .unwrap_or(false);
                let facing = if is_front { "front" } else { "back" };

                let camera_id = user_cfg
                    .borrow()
                    .camera_id
                    .as_deref()
                    .and_then(|s| s.parse::<u32>().ok());

                let mut payload = serde_json::json!({
                    "video_nr": 10,
                    "card_label": "PhoneCam Virtual Camera",
                    "facing": facing,
                    "resolution": resolution,
                    "camera_ar": camera_ar,
                    "crop": crop,
                    "fps": 30,
                    "flip_h": flip_h,
                    "flip_v": rotation == 180,
                    "rotation": rotation,
                    "invert": invert,
                    "preview": false,
                    "torch": torch,
                    "port": port
                });

                if let Some(cid) = camera_id {
                    payload["camera_id"] = serde_json::json!(cid);
                }

                let _ = http_post(port, "/api/start", Some(&payload));
                is_streaming.store(true, Ordering::SeqCst);
                update_ui(true, "Connected", "");
            }
        }
    };

    let ssh_clone = start_stop_handler.clone();
    btn_stream_action.connect_clicked(move |_| {
        ssh_clone();
    });

    let ssh_preview = start_stop_handler.clone();
    let is_streaming_for_preview = is_streaming_state.clone();
    let preview_child: Rc<RefCell<Option<std::process::Child>>> = Rc::new(RefCell::new(None));
    let preview_child_clone = preview_child.clone();

    btn_preview.connect_clicked(move |_| {
        // Toggle off if already running
        if let Some(mut child) = preview_child_clone.borrow_mut().take()
            && child.try_wait().ok() == Some(None)
        {
            let _ = child.kill();
            let _ = child.wait();
            return;
        }

        let spawn_viewer = || -> Option<std::process::Child> {
            if which::which("mpv").is_ok() {
                std::process::Command::new("mpv")
                    .args([
                        "--title=PhoneCam Preview",
                        "--profile=low-latency",
                        "--untimed",
                        "av://v4l2:/dev/video10",
                    ])
                    .spawn()
                    .ok()
            } else if which::which("ffplay").is_ok() {
                std::process::Command::new("ffplay")
                    .args([
                        "-window_title",
                        "PhoneCam Preview",
                        "-fflags",
                        "nobuffer",
                        "-f",
                        "v4l2",
                        "/dev/video10",
                    ])
                    .spawn()
                    .ok()
            } else {
                None
            }
        };

        let is_running = is_streaming_for_preview.load(Ordering::SeqCst);
        if is_running {
            *preview_child_clone.borrow_mut() = spawn_viewer();
        } else {
            ssh_preview();
            let pc_ref = preview_child_clone.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(800), move || {
                *pc_ref.borrow_mut() = spawn_viewer();
            });
        }
    });

    // Switches & user config state persistence
    let ucfg_as = user_cfg.clone();
    switch_auto_start.connect_active_notify(move |sw| {
        ucfg_as.borrow_mut().auto_start = sw.is_active();
        let _ = ucfg_as.borrow().save();
    });

    let ucfg_rot = user_cfg.clone();
    combo_rotation.connect_selected_notify(move |combo| {
        let rot = match combo.selected() {
            1 => 90,
            2 => 180,
            3 => 270,
            _ => 0,
        };
        ucfg_rot.borrow_mut().rotation = rot;
        ucfg_rot.borrow_mut().flip_v = rot == 180;
        let _ = ucfg_rot.borrow().save();
        let _ = http_post(
            port,
            "/api/set-rotation",
            Some(&serde_json::json!({ "rotation": rot })),
        );
    });

    let is_up_fliph = is_updating.clone();
    let ucfg_fliph = user_cfg.clone();
    switch_flip_h.connect_active_notify(move |sw| {
        ucfg_fliph.borrow_mut().flip_h = sw.is_active();
        let _ = ucfg_fliph.borrow().save();
        if !is_up_fliph.load(Ordering::SeqCst) {
            let _ = http_post(port, "/api/toggle-flip-h", None);
        }
    });

    let is_up_inv = is_updating.clone();
    let ucfg_inv = user_cfg.clone();
    switch_invert.connect_active_notify(move |sw| {
        ucfg_inv.borrow_mut().invert = sw.is_active();
        let _ = ucfg_inv.borrow().save();
        if !is_up_inv.load(Ordering::SeqCst) {
            let _ = http_post(port, "/api/toggle-invert", None);
        }
    });

    let ucfg_torch = user_cfg.clone();
    switch_torch.connect_active_notify(move |sw| {
        ucfg_torch.borrow_mut().torch = sw.is_active();
        let _ = ucfg_torch.borrow().save();
    });

    let ucfg_res = user_cfg.clone();
    combo_resolution.connect_selected_notify(move |combo| {
        let res = match combo.selected() {
            1 => "1280x720",
            2 => "3840x2160",
            3 => "2560x1440",
            _ => "1920x1080",
        };
        ucfg_res.borrow_mut().resolution = res.to_string();
        let _ = ucfg_res.borrow().save();
    });

    let ucfg_crop = user_cfg.clone();
    combo_crop.connect_selected_notify(move |combo| {
        let (crop, ar) = match combo.selected() {
            1 => (None, Some("16:9")),
            2 => (None, Some("4:3")),
            3 => (None, Some("1:1")),
            4 => (Some("1920:1080:0:0"), None),
            _ => (None, None),
        };
        ucfg_crop.borrow_mut().crop = crop.map(|s| s.to_string());
        ucfg_crop.borrow_mut().camera_ar = ar.map(|s| s.to_string());
        let _ = ucfg_crop.borrow().save();
    });

    let ucfg_lens = user_cfg.clone();
    combo_camera_lens.connect_selected_notify(move |combo| {
        let sel = combo.selected();
        if sel == 0 {
            ucfg_lens.borrow_mut().camera_id = None;
        } else if let Some(item) = combo
            .model()
            .and_then(|m| m.downcast::<gtk4::StringList>().ok())
            && let Some(str_val) = item.string(sel)
            && let Some(id_str) = str_val
                .strip_prefix("Camera ")
                .and_then(|s| s.split_whitespace().next())
        {
            ucfg_lens.borrow_mut().camera_id = Some(id_str.to_string());
        }
        let _ = ucfg_lens.borrow().save();
    });

    // Camera facing buttons
    let rear_btn_weak = btn_camera_rear.downgrade();
    let front_btn_weak = btn_camera_front.downgrade();
    let ucfg_rear = user_cfg.clone();
    btn_camera_rear.connect_clicked(move |_| {
        if let (Some(rear), Some(front)) = (rear_btn_weak.upgrade(), front_btn_weak.upgrade()) {
            rear.add_css_class("suggested-action");
            front.remove_css_class("suggested-action");
            ucfg_rear.borrow_mut().facing = "back".to_string();
            let _ = ucfg_rear.borrow().save();
            let _ = http_post(
                port,
                "/api/set-facing",
                Some(&serde_json::json!({ "facing": "back" })),
            );
        }
    });

    let rear_btn_weak2 = btn_camera_rear.downgrade();
    let front_btn_weak2 = btn_camera_front.downgrade();
    let ucfg_front = user_cfg.clone();
    btn_camera_front.connect_clicked(move |_| {
        if let (Some(rear), Some(front)) = (rear_btn_weak2.upgrade(), front_btn_weak2.upgrade()) {
            front.add_css_class("suggested-action");
            rear.remove_css_class("suggested-action");
            ucfg_front.borrow_mut().facing = "front".to_string();
            let _ = ucfg_front.borrow().save();
            let _ = http_post(
                port,
                "/api/set-facing",
                Some(&serde_json::json!({ "facing": "front" })),
            );
        }
    });

    // Wi-Fi ADB actions
    let wifi_entry_weak = wifi_entry.downgrade();
    let wifi_status_weak = wifi_status_row.downgrade();
    let btn_wifi_conn_weak_click = btn_wifi_connect.downgrade();
    let ucfg_wifi = user_cfg.clone();
    btn_wifi_connect.connect_clicked(move |_| {
        if let (Some(entry), Some(status_row), Some(btn)) = (
            wifi_entry_weak.upgrade(),
            wifi_status_weak.upgrade(),
            btn_wifi_conn_weak_click.upgrade(),
        ) {
            let is_disconnect = btn.label().as_deref() == Some("Disconnect Wi-Fi");
            if is_disconnect {
                btn.set_sensitive(false);
                status_row.set_subtitle("Disconnecting wireless ADB...");
                let _ = http_post(port, "/api/disconnect-wifi", None);
                status_row.set_title("WI-FI DISCONNECTED");
                status_row.set_subtitle("Wireless link disconnected.");
                btn.set_label("Connect Wi-Fi");
                btn.remove_css_class("destructive-action");
                btn.add_css_class("suggested-action");
                btn.set_sensitive(true);
                return;
            }

            let addr = entry.text().trim().to_string();
            if addr.is_empty() {
                status_row.set_subtitle("Please enter phone IP (e.g. 192.168.1.50:5555)");
                return;
            }
            btn.set_sensitive(false);
            status_row.set_subtitle(&format!("Connecting to {addr}..."));
            let full_addr = if addr.contains(':') {
                addr.clone()
            } else {
                format!("{addr}:5555")
            };
            ucfg_wifi.borrow_mut().remember_ip(&full_addr);
            let _ = ucfg_wifi.borrow().save();
            match http_post(
                port,
                "/api/connect-wifi",
                Some(&serde_json::json!({ "address": full_addr })),
            ) {
                Ok(val) => {
                    let msg = val["message"].as_str().unwrap_or("Connected");
                    status_row.set_title("● WI-FI CONNECTED");
                    status_row.set_subtitle(msg);
                    btn.set_label("Disconnect Wi-Fi");
                    btn.remove_css_class("suggested-action");
                    btn.add_css_class("destructive-action");
                    btn.set_sensitive(true);
                }
                Err(e) => {
                    status_row.set_title("WI-FI DISCONNECTED");
                    status_row.set_subtitle(&format!("Connection failed: {e}"));
                    btn.set_label("Connect Wi-Fi");
                    btn.remove_css_class("destructive-action");
                    btn.add_css_class("suggested-action");
                    btn.set_sensitive(true);
                }
            }
        }
    });

    let wifi_status_tcpip = wifi_status_row.downgrade();
    btn_wifi_tcpip.connect_clicked(move |_| {
        if let Some(status_row) = wifi_status_tcpip.upgrade() {
            status_row.set_subtitle("Enabling TCP/IP mode on port 5555...");
            match http_post(port, "/api/enable-tcpip", None) {
                Ok(val) => {
                    let msg = val["message"].as_str().unwrap_or("TCP/IP enabled");
                    status_row.set_subtitle(&format!("{msg}. You can now unplug USB!"));
                }
                Err(e) => {
                    status_row.set_subtitle(&format!("Failed: {e}. Is phone plugged via USB?"));
                }
            }
        }
    });

    // Auto-save Wi-Fi address as user types or hits enter
    let ucfg_entry = user_cfg.clone();
    wifi_entry.connect_changed(move |entry| {
        let text = entry.text().trim().to_string();
        if !text.is_empty() && (text.contains('.') || text.contains(':')) {
            ucfg_entry.borrow_mut().remember_ip(&text);
            let _ = ucfg_entry.borrow().save();
        }
    });

    let btn_wifi_conn_for_enter = btn_wifi_connect.downgrade();
    wifi_entry.connect_activate(move |_| {
        if let Some(btn) = btn_wifi_conn_for_enter.upgrade() {
            btn.emit_clicked();
        }
    });

    // Refresh action
    let dev_row_rf = device_row.downgrade();
    let bat_lbl_rf = battery_label.downgrade();
    let logs_row_rf = logs_row.downgrade();
    let refresh_handler = move || {
        if let Ok(st) = http_get(port, "/api/status") {
            if let Some(row) = dev_row_rf.upgrade()
                && let Some(dev) = st.get("device")
            {
                let status = dev["status"].as_str().unwrap_or("NotFound");
                let brand = dev["brand"].as_str().unwrap_or("");
                let model = dev["model"].as_str().unwrap_or("");
                let serial = dev["serial"].as_str().unwrap_or("");

                if status == "Connected" {
                    row.set_title(&format!("{} {}", brand.to_uppercase(), model));
                    if serial.contains(':') || serial.contains('.') {
                        row.set_subtitle(&format!("● Connected via Wi-Fi ({serial})"));
                    } else {
                        row.set_subtitle(&format!("● Connected via USB ({serial})"));
                    }
                } else if status == "Unauthorized" {
                    row.set_title("Unauthorized Device");
                    row.set_subtitle("Tap 'Allow' on your phone to authorize USB debugging");
                } else {
                    row.set_title("No smartphone detected");
                    row.set_subtitle("Connect phone via USB-C &amp; Enable USB Debugging");
                }
            }
            if let Some(lbl) = bat_lbl_rf.upgrade() {
                if let Some(b) = st["device"]["battery_percent"].as_u64() {
                    let chg = st["device"]["is_charging"].as_bool().unwrap_or(false);
                    lbl.set_label(&format!("{b}%{}", if chg { " (Charging)" } else { "" }));
                } else {
                    lbl.set_label("");
                }
            }
        }
        if let Ok(logs) = http_get(port, "/api/logs")
            && let (Some(row), Some(arr)) = (logs_row_rf.upgrade(), logs.as_array())
            && let Some(last) = arr.last().and_then(|v| v.as_str())
        {
            row.set_subtitle(&glib::markup_escape_text(last));
        }
    };

    let rf_clone = refresh_handler.clone();
    refresh_btn.connect_clicked(move |_| {
        rf_clone();
    });

    // Periodic telemetry timer (every 1500ms)
    let dev_row_timer = device_row.downgrade();
    let bat_lbl_timer = battery_label.downgrade();
    let logs_row_timer = logs_row.downgrade();
    let combo_lens_timer = combo_camera_lens.downgrade();
    let is_streaming_timer = is_streaming_state.clone();
    let is_updating_timer = is_updating.clone();
    let ucfg_timer = user_cfg.clone();
    let update_ui_timer = update_dynamic_ui.clone();

    glib::timeout_add_local(std::time::Duration::from_millis(1500), move || {
        let Some(row) = dev_row_timer.upgrade() else {
            return glib::ControlFlow::Break;
        };

        if let Ok(st) = http_get(port, "/api/status") {
            if let Some(cams) = st.get("cameras").and_then(|c| c.as_array())
                && !cams.is_empty()
                && let Some(combo) = combo_lens_timer.upgrade()
            {
                let current_count = combo
                    .model()
                    .and_then(|m| m.downcast::<gtk4::StringList>().ok())
                    .map(|m| m.n_items())
                    .unwrap_or(0);
                let target_count = cams.len() as u32 + 1;
                if current_count != target_count {
                    let mut items = vec!["Default Camera".to_string()];
                    for c in cams {
                        let cid = c["id"].as_u64().unwrap_or(0);
                        let facing = c["facing"].as_str().unwrap_or("");
                        let res = c["resolution"].as_str().unwrap_or("");
                        items.push(format!("Camera {cid} ({facing}, {res})"));
                    }
                    let item_refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
                    let new_model = gtk4::StringList::new(&item_refs);
                    combo.set_model(Some(&new_model));
                }
            }
            if let Some(dev) = st.get("device") {
                let status = dev["status"].as_str().unwrap_or("NotFound");
                let brand = dev["brand"].as_str().unwrap_or("");
                let model = dev["model"].as_str().unwrap_or("");
                let serial = dev["serial"].as_str().unwrap_or("");

                if status == "Connected" {
                    row.set_title(&format!("{} {}", brand.to_uppercase(), model));
                    if serial.contains(':') || serial.contains('.') {
                        row.set_subtitle(&format!("● Connected via Wi-Fi ({serial})"));
                    } else {
                        row.set_subtitle(&format!("● Connected via USB ({serial})"));
                    }
                    if !serial.is_empty()
                        && ucfg_timer.borrow().last_device_serial.as_deref() != Some(serial)
                    {
                        ucfg_timer.borrow_mut().last_device_serial = Some(serial.to_string());
                        let _ = ucfg_timer.borrow().save();
                    }
                } else if status == "Unauthorized" {
                    row.set_title("Unauthorized Device");
                    row.set_subtitle("Tap 'Allow' on your phone to authorize USB debugging");
                } else {
                    row.set_title("No smartphone detected");
                    row.set_subtitle("Connect phone via USB-C &amp; Enable USB Debugging");
                }
            }

            if let Some(lbl) = bat_lbl_timer.upgrade() {
                if let Some(b) = st["device"]["battery_percent"].as_u64() {
                    let chg = st["device"]["is_charging"].as_bool().unwrap_or(false);
                    lbl.set_label(&format!("{b}%{}", if chg { " (Charging)" } else { "" }));
                } else {
                    lbl.set_label("");
                }
            }

            let streaming = st["is_streaming"].as_bool().unwrap_or(false);
            is_streaming_timer.store(streaming, Ordering::SeqCst);
            let dev_status = st
                .get("device")
                .and_then(|d| d["status"].as_str())
                .unwrap_or("NotFound");
            let dev_serial = st
                .get("device")
                .and_then(|d| d["serial"].as_str())
                .unwrap_or("");

            is_updating_timer.store(true, Ordering::SeqCst);
            update_ui_timer(streaming, dev_status, dev_serial);
            is_updating_timer.store(false, Ordering::SeqCst);
        }

        if let Ok(logs) = http_get(port, "/api/logs")
            && let (Some(row), Some(arr)) = (logs_row_timer.upgrade(), logs.as_array())
            && let Some(last) = arr.last().and_then(|v| v.as_str())
        {
            row.set_subtitle(&glib::markup_escape_text(last));
        }

        glib::ControlFlow::Continue
    });

    // Keyboard Shortcuts
    let key_controller = gtk4::EventControllerKey::new();
    let win_weak_key = window.downgrade();
    let btn_action_key = btn_stream_action.downgrade();
    key_controller.connect_key_pressed(move |_controller, key, _keycode, _modifier| match key {
        gtk4::gdk::Key::Escape | gtk4::gdk::Key::q => {
            if let Some(win) = win_weak_key.upgrade() {
                win.close();
            }
            glib::Propagation::Stop
        }
        gtk4::gdk::Key::space => {
            if let Some(btn) = btn_action_key.upgrade() {
                btn.emit_clicked();
            }
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
    window.add_controller(key_controller);

    // Auto-start webcam if configured
    if initial_cfg.auto_start {
        let btn_auto = btn_stream_action.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
            btn_auto.emit_clicked();
        });
    }

    // Window Close Request: ensure clean teardown of /dev/video10 and preview player
    let preview_child_close = preview_child.clone();
    window.connect_close_request(move |_| {
        let _ = &theme_monitors;
        if let Some(mut child) = preview_child_close.borrow_mut().take() {
            let _ = child.kill();
        }
        let _ = http_post(port, "/api/stop", None);
        glib::Propagation::Proceed
    });

    window
}

pub fn find_widget_by_name<W: glib::object::IsA<gtk4::Widget>>(
    root: &gtk4::Widget,
    name: &str,
) -> Option<W> {
    if root.widget_name() == name {
        return root.clone().downcast::<W>().ok();
    }
    let mut child = root.first_child();
    while let Some(c) = child {
        if let Some(found) = find_widget_by_name::<W>(&c, name) {
            return Some(found);
        }
        child = c.next_sibling();
    }
    None
}

pub fn http_get(port: u16, path: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        std::time::Duration::from_millis(400),
    )?;
    stream.set_read_timeout(Some(std::time::Duration::from_millis(400)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_millis(400)))?;

    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes())?;

    let mut resp = String::new();
    stream.read_to_string(&mut resp)?;

    if let Some(body_idx) = resp.find("\r\n\r\n") {
        let body = &resp[body_idx + 4..];
        let val: serde_json::Value = serde_json::from_str(body)?;
        return Ok(val);
    }
    Err("Invalid HTTP response".into())
}

pub fn http_post(
    port: u16,
    path: &str,
    body: Option<&serde_json::Value>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        std::time::Duration::from_millis(400),
    )?;
    stream.set_read_timeout(Some(std::time::Duration::from_millis(400)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_millis(400)))?;

    let body_str = body.map(|b| b.to_string()).unwrap_or_default();
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body_str.len(),
        body_str
    );
    stream.write_all(req.as_bytes())?;

    let mut resp = String::new();
    stream.read_to_string(&mut resp)?;

    if let Some(body_idx) = resp.find("\r\n\r\n") {
        let body = &resp[body_idx + 4..];
        let val: serde_json::Value =
            serde_json::from_str(body).unwrap_or(serde_json::json!({ "success": true }));
        return Ok(val);
    }
    Err("Invalid HTTP response".into())
}
