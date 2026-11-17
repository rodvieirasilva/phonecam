use adw::prelude::*;
use gtk4::gio;
use gtk4::glib;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;

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
            background: "#18181b".to_string(),
            dark_background: "#121214".to_string(),
            darker_background: "#09090b".to_string(),
            lighter_background: "#27272a".to_string(),
            foreground: "#e4e4e7".to_string(),
            bright_foreground: "#f4f4f5".to_string(),
            green: "#10b981".to_string(),
            red: "#ef4444".to_string(),
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

    format!(
        r#"
@define-color window_bg_color {bg};
@define-color window_fg_color {fg};
@define-color view_bg_color {bg};
@define-color view_fg_color {fg};
@define-color headerbar_bg_color {dark_bg};
@define-color headerbar_fg_color {fg};
@define-color card_bg_color {card_bg};
@define-color card_fg_color {fg};
@define-color accent_color {accent};
@define-color accent_bg_color {accent};
@define-color accent_fg_color {accent_fg};

window, window.background, .background, preferencespage, scrolledwindow, viewport {{
    background-color: {bg};
    color: {fg};
}}

headerbar {{
    background-color: {dark_bg};
    color: {fg};
}}

.boxed-list, row {{
    background-color: {card_bg};
    color: {fg};
}}
"#,
        bg = palette.background,
        fg = palette.bright_foreground,
        dark_bg = palette.dark_background,
        card_bg = palette.lighter_background,
        accent = palette.accent,
        accent_fg = accent_fg,
    )
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
    let mut builder = adw::ApplicationWindow::builder()
        .title("PhoneCam — Smartphone Virtual Webcam")
        .icon_name("phonecam")
        .default_width(560)
        .default_height(780);
    if let Some(a) = app {
        builder = builder.application(a);
    }
    let window = builder.build();
    window.set_widget_name("window-main");

    let header = adw::HeaderBar::new();
    let title = adw::WindowTitle::new("PhoneCam", "Studio Smartphone Webcam for Omarchy");
    header.set_title_widget(Some(&title));

    let refresh_btn = gtk4::Button::builder()
        .name("btn-refresh")
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Refresh device & logs")
        .valign(gtk4::Align::Center)
        .build();
    header.pack_end(&refresh_btn);

    let page = adw::PreferencesPage::new();

    // 1. Device Status Group
    let device_group = adw::PreferencesGroup::builder()
        .title("Smartphone Device")
        .name("group-device")
        .build();

    let device_row = adw::ActionRow::builder()
        .name("text-device-name")
        .title("Detecting Smartphone...")
        .subtitle("Connect phone via USB-C &amp; Enable USB Debugging")
        .build();

    let battery_label = gtk4::Label::builder()
        .name("text-battery")
        .label("")
        .css_classes(["dim-label"])
        .valign(gtk4::Align::Center)
        .build();
    device_row.add_suffix(&battery_label);
    device_group.add(&device_row);
    page.add(&device_group);

    // 2. Wireless ADB over Wi-Fi Group
    let wifi_group = adw::PreferencesGroup::builder()
        .title("Wireless ADB over Wi-Fi")
        .description("Untethered studio camera streaming")
        .name("group-wifi")
        .build();

    let wifi_entry = gtk4::Entry::builder()
        .name("input-wifi-address")
        .placeholder_text("192.168.1.50:5555")
        .valign(gtk4::Align::Center)
        .hexpand(true)
        .build();

    let wifi_entry_row = adw::ActionRow::builder()
        .title("Phone IP:Port")
        .activatable_widget(&wifi_entry)
        .build();
    wifi_entry_row.add_suffix(&wifi_entry);
    wifi_group.add(&wifi_entry_row);

    let wifi_actions_row = adw::ActionRow::builder()
        .name("row-wifi-actions")
        .title("Wireless Pairing Actions")
        .build();

    let btn_wifi_connect = gtk4::Button::builder()
        .name("btn-wifi-connect")
        .label("📡 Connect Wi-Fi")
        .valign(gtk4::Align::Center)
        .css_classes(["suggested-action"])
        .build();

    let btn_wifi_tcpip = gtk4::Button::builder()
        .name("btn-wifi-tcpip")
        .label("🔌 Enable USB TCP/IP")
        .valign(gtk4::Align::Center)
        .tooltip_text("Switches USB-connected phone to port 5555 for wireless disconnection")
        .build();

    let wifi_btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    wifi_btn_box.append(&btn_wifi_tcpip);
    wifi_btn_box.append(&btn_wifi_connect);
    wifi_actions_row.add_suffix(&wifi_btn_box);
    wifi_group.add(&wifi_actions_row);

    let wifi_status_row = adw::ActionRow::builder()
        .name("text-wifi-status")
        .title("Wireless Status")
        .subtitle("💡 Connect phone via USB once and click 'Enable USB TCP/IP', then unplug cable and click 'Connect Wi-Fi'.")
        .build();
    wifi_group.add(&wifi_status_row);
    page.add(&wifi_group);

    // 3. Camera Selection & Sensor Group
    let camera_group = adw::PreferencesGroup::builder()
        .title("Camera Selection &amp; Sensor")
        .name("group-camera")
        .build();

    let facing_row = adw::ActionRow::builder()
        .title("Camera Facing")
        .subtitle("Select rear primary or front selfie camera")
        .build();

    let btn_camera_rear = gtk4::Button::builder()
        .name("btn-camera-rear")
        .label("📸 Rear (Back)")
        .valign(gtk4::Align::Center)
        .css_classes(["suggested-action"])
        .build();

    let btn_camera_front = gtk4::Button::builder()
        .name("btn-camera-front")
        .label("🤳 Selfie (Front)")
        .valign(gtk4::Align::Center)
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
    let combo_resolution = adw::ComboRow::builder()
        .name("combo-resolution")
        .title("Resolution (Resize)")
        .model(&res_strings)
        .selected(0)
        .build();
    geom_group.add(&combo_resolution);

    let crop_strings = gtk4::StringList::new(&[
        "Full Sensor (Native)",
        "16:9 Widescreen Crop",
        "4:3 Standard Crop",
        "1:1 Square (Avatar)",
        "Custom 1080p Center Cut",
    ]);
    let combo_crop = adw::ComboRow::builder()
        .name("combo-crop")
        .title("Framing / Cut (Crop)")
        .model(&crop_strings)
        .selected(0)
        .build();
    geom_group.add(&combo_crop);
    page.add(&geom_group);

    // 5. Transformations & Filters Group
    let fx_group = adw::PreferencesGroup::builder()
        .title("Transform, Invert &amp; Filters")
        .name("group-transforms")
        .build();

    let switch_flip_h = gtk4::Switch::builder()
        .name("switch-flip-h")
        .active(false)
        .valign(gtk4::Align::Center)
        .build();
    let row_flip_h = adw::ActionRow::builder()
        .title("Mirror (Flip Horizontal)")
        .subtitle("Natural webcam orientation")
        .activatable_widget(&switch_flip_h)
        .build();
    row_flip_h.add_suffix(&switch_flip_h);
    fx_group.add(&row_flip_h);

    let switch_flip_v = gtk4::Switch::builder()
        .name("switch-flip-v")
        .active(false)
        .valign(gtk4::Align::Center)
        .build();
    let row_flip_v = adw::ActionRow::builder()
        .title("Flip Vertical (180°)")
        .subtitle("Inverted mount correction")
        .activatable_widget(&switch_flip_v)
        .build();
    row_flip_v.add_suffix(&switch_flip_v);
    fx_group.add(&row_flip_v);

    let switch_invert = gtk4::Switch::builder()
        .name("switch-invert")
        .active(false)
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
        .active(false)
        .valign(gtk4::Align::Center)
        .build();
    let row_torch = adw::ActionRow::builder()
        .title("Camera Torch (Flashlight)")
        .subtitle("Rear lens studio lighting")
        .activatable_widget(&switch_torch)
        .build();
    row_torch.add_suffix(&switch_torch);
    fx_group.add(&row_torch);

    let switch_preview = gtk4::Switch::builder()
        .name("switch-preview")
        .active(false)
        .valign(gtk4::Align::Center)
        .build();
    let row_preview = adw::ActionRow::builder()
        .title("Live Desktop Preview")
        .subtitle("Open real-time monitor window")
        .activatable_widget(&switch_preview)
        .build();
    row_preview.add_suffix(&switch_preview);
    fx_group.add(&row_preview);
    page.add(&fx_group);

    // 6. Virtual Webcam Device Group
    let stream_group = adw::PreferencesGroup::builder()
        .title("Virtual Device /dev/video10")
        .name("group-stream")
        .build();

    let stream_row = adw::ActionRow::builder()
        .name("text-stream-status")
        .title("VIRTUAL CAMERA OFFLINE")
        .subtitle("Device node: /dev/video10 (v4l2loopback)")
        .build();

    let btn_preview = gtk4::Button::builder()
        .name("btn-preview")
        .label("👁️ Preview")
        .valign(gtk4::Align::Center)
        .build();

    let btn_stream_action = gtk4::Button::builder()
        .name("btn-stream-action")
        .label("🚀 Start Webcam")
        .valign(gtk4::Align::Center)
        .css_classes(["suggested-action"])
        .build();

    let stream_btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    stream_btn_box.append(&btn_preview);
    stream_btn_box.append(&btn_stream_action);
    stream_row.add_suffix(&stream_btn_box);
    stream_group.add(&stream_row);
    page.add(&stream_group);

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
    let switch_preview_weak = switch_preview.downgrade();
    let combo_res_weak = combo_resolution.downgrade();
    let combo_crop_weak = combo_crop.downgrade();

    // Start / Stop Stream Action
    let start_stop_handler = {
        let _is_updating = is_updating_clone.clone();
        let is_streaming = is_streaming_clone.clone();
        let btn_action_weak = btn_action_weak.clone();
        let stream_row_weak = stream_row_weak.clone();
        let switch_preview_weak = switch_preview_weak.clone();
        let combo_res_weak = combo_res_weak.clone();
        let combo_crop_weak = combo_crop_weak.clone();

        move || {
            let currently_streaming = is_streaming.load(Ordering::SeqCst);
            if currently_streaming {
                let _ = http_post(port, "/api/stop", None);
                is_streaming.store(false, Ordering::SeqCst);
                if let (Some(btn), Some(row)) =
                    (btn_action_weak.upgrade(), stream_row_weak.upgrade())
                {
                    btn.set_label("🚀 Start Webcam");
                    btn.remove_css_class("destructive-action");
                    btn.add_css_class("suggested-action");
                    row.set_title("VIRTUAL CAMERA OFFLINE");
                }
            } else {
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

                let preview = switch_preview_weak
                    .upgrade()
                    .map(|s| s.is_active())
                    .unwrap_or(false);

                let payload = serde_json::json!({
                    "video_nr": 10,
                    "card_label": "PhoneCam Virtual Camera",
                    "facing": "back",
                    "resolution": resolution,
                    "camera_ar": camera_ar,
                    "crop": crop,
                    "fps": 30,
                    "flip_h": false,
                    "flip_v": false,
                    "invert": false,
                    "preview": preview,
                    "torch": false,
                    "port": port
                });

                let _ = http_post(port, "/api/start", Some(&payload));
                is_streaming.store(true, Ordering::SeqCst);
                if let (Some(btn), Some(row)) =
                    (btn_action_weak.upgrade(), stream_row_weak.upgrade())
                {
                    btn.set_label("🛑 Stop & Detach");
                    btn.remove_css_class("suggested-action");
                    btn.add_css_class("destructive-action");
                    row.set_title("● VIRTUAL CAMERA ACTIVE");
                }
            }
        }
    };

    let ssh_clone = start_stop_handler.clone();
    btn_stream_action.connect_clicked(move |_| {
        ssh_clone();
    });

    let ssh_preview = start_stop_handler.clone();
    let switch_preview_for_btn = switch_preview.downgrade();
    btn_preview.connect_clicked(move |_| {
        if let Some(sw) = switch_preview_for_btn.upgrade() {
            sw.set_active(true);
        }
        ssh_preview();
    });

    // Switches
    let is_up_fliph = is_updating.clone();
    switch_flip_h.connect_active_notify(move |_| {
        if !is_up_fliph.load(Ordering::SeqCst) {
            let _ = http_post(port, "/api/toggle-flip-h", None);
        }
    });

    let is_up_flipv = is_updating.clone();
    switch_flip_v.connect_active_notify(move |_| {
        if !is_up_flipv.load(Ordering::SeqCst) {
            let _ = http_post(port, "/api/toggle-flip-v", None);
        }
    });

    let is_up_inv = is_updating.clone();
    switch_invert.connect_active_notify(move |_| {
        if !is_up_inv.load(Ordering::SeqCst) {
            let _ = http_post(port, "/api/toggle-invert", None);
        }
    });

    // Camera facing buttons
    let rear_btn_weak = btn_camera_rear.downgrade();
    let front_btn_weak = btn_camera_front.downgrade();
    btn_camera_rear.connect_clicked(move |_| {
        if let (Some(rear), Some(front)) = (rear_btn_weak.upgrade(), front_btn_weak.upgrade()) {
            rear.add_css_class("suggested-action");
            front.remove_css_class("suggested-action");
            let _ = http_post(
                port,
                "/api/set-facing",
                Some(&serde_json::json!({ "facing": "back" })),
            );
        }
    });

    let rear_btn_weak2 = btn_camera_rear.downgrade();
    let front_btn_weak2 = btn_camera_front.downgrade();
    btn_camera_front.connect_clicked(move |_| {
        if let (Some(rear), Some(front)) = (rear_btn_weak2.upgrade(), front_btn_weak2.upgrade()) {
            front.add_css_class("suggested-action");
            rear.remove_css_class("suggested-action");
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
    btn_wifi_connect.connect_clicked(move |_| {
        if let (Some(entry), Some(status_row)) =
            (wifi_entry_weak.upgrade(), wifi_status_weak.upgrade())
        {
            let addr = entry.text().trim().to_string();
            if addr.is_empty() {
                status_row.set_subtitle("❌ Please enter phone IP (e.g. 192.168.1.50:5555)");
                return;
            }
            status_row.set_subtitle(&format!("⏳ Connecting to {addr}..."));
            let full_addr = if addr.contains(':') {
                addr.clone()
            } else {
                format!("{addr}:5555")
            };
            match http_post(
                port,
                "/api/connect-wifi",
                Some(&serde_json::json!({ "address": full_addr })),
            ) {
                Ok(val) => {
                    let msg = val["message"].as_str().unwrap_or("Connected");
                    status_row.set_subtitle(&format!("✓ {msg}"));
                }
                Err(e) => {
                    status_row.set_subtitle(&format!("❌ Connection failed: {e}"));
                }
            }
        }
    });

    let wifi_status_tcpip = wifi_status_row.downgrade();
    btn_wifi_tcpip.connect_clicked(move |_| {
        if let Some(status_row) = wifi_status_tcpip.upgrade() {
            status_row.set_subtitle("⏳ Enabling TCP/IP mode on port 5555...");
            match http_post(port, "/api/enable-tcpip", None) {
                Ok(val) => {
                    let msg = val["message"].as_str().unwrap_or("TCP/IP enabled");
                    status_row.set_subtitle(&format!("✓ {msg}. You can now unplug USB!"));
                }
                Err(e) => {
                    status_row.set_subtitle(&format!("❌ Failed: {e}. Is phone plugged via USB?"));
                }
            }
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
                    row.set_subtitle(&format!("ADB Serial: {serial} (Connected)"));
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
                    lbl.set_label(&format!("🔋 {b}%{}", if chg { " ⚡" } else { "" }));
                } else {
                    lbl.set_label("");
                }
            }
        }
        if let Ok(logs) = http_get(port, "/api/logs")
            && let (Some(row), Some(arr)) = (logs_row_rf.upgrade(), logs.as_array())
            && let Some(last) = arr.last().and_then(|v| v.as_str())
        {
            row.set_subtitle(last);
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
    let btn_action_timer = btn_stream_action.downgrade();
    let stream_row_timer = stream_row.downgrade();
    let is_streaming_timer = is_streaming_state.clone();
    let is_updating_timer = is_updating.clone();

    glib::timeout_add_local(std::time::Duration::from_millis(1500), move || {
        let Some(row) = dev_row_timer.upgrade() else {
            return glib::ControlFlow::Break;
        };

        if let Ok(st) = http_get(port, "/api/status") {
            if let Some(dev) = st.get("device") {
                let status = dev["status"].as_str().unwrap_or("NotFound");
                let brand = dev["brand"].as_str().unwrap_or("");
                let model = dev["model"].as_str().unwrap_or("");
                let serial = dev["serial"].as_str().unwrap_or("");

                if status == "Connected" {
                    row.set_title(&format!("{} {}", brand.to_uppercase(), model));
                    row.set_subtitle(&format!("ADB Serial: {serial} (Connected)"));
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
                    lbl.set_label(&format!("🔋 {b}%{}", if chg { " ⚡" } else { "" }));
                } else {
                    lbl.set_label("");
                }
            }

            let streaming = st["is_streaming"].as_bool().unwrap_or(false);
            is_streaming_timer.store(streaming, Ordering::SeqCst);
            if let (Some(btn), Some(s_row)) =
                (btn_action_timer.upgrade(), stream_row_timer.upgrade())
            {
                is_updating_timer.store(true, Ordering::SeqCst);
                if streaming {
                    btn.set_label("🛑 Stop & Detach");
                    btn.remove_css_class("suggested-action");
                    btn.add_css_class("destructive-action");
                    s_row.set_title("● VIRTUAL CAMERA ACTIVE");
                } else {
                    btn.set_label("🚀 Start Webcam");
                    btn.remove_css_class("destructive-action");
                    btn.add_css_class("suggested-action");
                    s_row.set_title("VIRTUAL CAMERA OFFLINE");
                }
                is_updating_timer.store(false, Ordering::SeqCst);
            }
        }

        if let Ok(logs) = http_get(port, "/api/logs")
            && let (Some(row), Some(arr)) = (logs_row_timer.upgrade(), logs.as_array())
            && let Some(last) = arr.last().and_then(|v| v.as_str())
        {
            row.set_subtitle(last);
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

    // Window Close Request: ensure clean teardown of /dev/video10
    window.connect_close_request(move |_| {
        let _ = &theme_monitors;
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
