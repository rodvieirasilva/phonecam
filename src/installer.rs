use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const DESKTOP_TEMPLATE: &str = include_str!("../pkgbuild/phonecam.desktop");
pub const ICON_SVG: &str = include_str!("../assets/phonecam.svg");
pub const ICON_PNG: &[u8] = include_bytes!("../assets/phonecam.png");
pub const CONFIG_TEMPLATE: &str = include_str!("../config.example.toml");
pub const MAIN_QML: &str = include_str!("ui/Main.qml");

pub struct Installer;

impl Installer {
    /// Validates all embedded asset templates to prevent shipping corrupted files.
    pub fn validate_templates() -> Result<()> {
        if !DESKTOP_TEMPLATE.contains("[Desktop Entry]")
            || !DESKTOP_TEMPLATE.contains("Name=PhoneCam")
        {
            anyhow::bail!("Desktop template missing [Desktop Entry] or Name=PhoneCam");
        }
        if !ICON_SVG.contains("<svg") || !ICON_SVG.contains("</svg>") {
            anyhow::bail!("Icon SVG appears incomplete or corrupted");
        }
        if ICON_PNG.len() < 8 || &ICON_PNG[1..4] != b"PNG" {
            anyhow::bail!("Icon PNG is missing PNG magic header");
        }
        if !CONFIG_TEMPLATE.contains("facing =") || !CONFIG_TEMPLATE.contains("resolution =") {
            anyhow::bail!("Config template missing default configuration fields");
        }
        if !MAIN_QML.contains("Item")
            && !MAIN_QML.contains("Window")
            && !MAIN_QML.contains("ApplicationWindow")
        {
            anyhow::bail!("Main.qml template appears incomplete");
        }
        Ok(())
    }

    /// Returns the current user's home directory.
    pub fn home_dir() -> PathBuf {
        crate::util::home_dir()
    }

    /// Resolves the currently running PhoneCam executable or installed location.
    pub fn resolve_bin_path() -> PathBuf {
        if let Ok(exe) = std::env::current_exe()
            && exe.exists()
        {
            return exe;
        }
        let home = Self::home_dir();
        let local_bin = home.join(".local/bin/phonecam");
        if local_bin.exists() {
            return local_bin;
        }
        let usr_local = PathBuf::from("/usr/local/bin/phonecam");
        if usr_local.exists() {
            return usr_local;
        }
        PathBuf::from("phonecam")
    }

    /// Determines if running in a test or CI environment to prevent unwanted system changes.
    pub fn is_test_or_ci() -> bool {
        std::env::var("PHONECAM_TEST").is_ok() || std::env::var("CI").is_ok()
    }

    /// Executes complete local and optional system installation.
    pub fn install(manage_system: bool) -> Result<()> {
        let home = Self::home_dir();
        let bin_path = Self::resolve_bin_path();
        let manage = manage_system && !Self::is_test_or_ci();
        Self::install_to(&home, &bin_path, manage)?;

        println!("\n✅ PhoneCam successfully installed!");
        println!("   Desktop: Launch from application menu or run `phonecam`");
        println!("   TUI:     Run `phonecam --tui`");
        println!("   Device:  Virtual camera available at /dev/video10\n");

        Ok(())
    }

    /// Installs PhoneCam to the specified home directory and optional system targets.
    pub fn install_to(home: &Path, bin_path: &Path, manage_system: bool) -> Result<()> {
        Self::validate_templates()?;
        tracing::info!("Installing PhoneCam using binary: {}", bin_path.display());

        // 1. Install user binary
        let user_bin_dir = home.join(".local/bin");
        fs::create_dir_all(&user_bin_dir)
            .with_context(|| format!("Failed to create {}", user_bin_dir.display()))?;
        let user_bin = user_bin_dir.join("phonecam");

        if bin_path.exists() && bin_path != user_bin {
            fs::copy(bin_path, &user_bin)
                .with_context(|| format!("Failed to copy binary to {}", user_bin.display()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&user_bin, fs::Permissions::from_mode(0o755));
            }
            tracing::info!("Installed user binary: {}", user_bin.display());
        }

        // 2. Install shared resources
        let user_share_ui = home.join(".local/share/phonecam/ui");
        fs::create_dir_all(&user_share_ui)
            .with_context(|| format!("Failed to create {}", user_share_ui.display()))?;
        fs::write(user_share_ui.join("Main.qml"), MAIN_QML)?;

        // 3. Install configuration template
        let config_dir = home.join(".config/phonecam");
        fs::create_dir_all(&config_dir)
            .with_context(|| format!("Failed to create {}", config_dir.display()))?;
        let example_config = config_dir.join("config.example.toml");
        fs::write(&example_config, CONFIG_TEMPLATE)?;
        let user_config = config_dir.join("config.toml");
        if !user_config.exists() {
            fs::write(&user_config, CONFIG_TEMPLATE)?;
        }

        // 4. Install user icons
        let icons_svg_dir = home.join(".local/share/icons/hicolor/scalable/apps");
        let icons_png_dir = home.join(".local/share/icons/hicolor/512x512/apps");
        fs::create_dir_all(&icons_svg_dir)?;
        fs::create_dir_all(&icons_png_dir)?;
        let user_svg = icons_svg_dir.join("phonecam.svg");
        let user_png = icons_png_dir.join("phonecam.png");
        fs::write(&user_svg, ICON_SVG)?;
        fs::write(&user_png, ICON_PNG)?;

        // 5. Install user desktop entry (fallback or default)
        let apps_dir = home.join(".local/share/applications");
        fs::create_dir_all(&apps_dir)?;
        let user_desktop = apps_dir.join("phonecam.desktop");
        fs::write(&user_desktop, DESKTOP_TEMPLATE)?;

        let _ = Command::new("update-desktop-database")
            .arg(&apps_dir)
            .status();

        // 6. Grant desktop camera portal permissions to camera preview apps
        let _ = Command::new("busctl")
            .args([
                "--user",
                "call",
                "org.freedesktop.impl.portal.PermissionStore",
                "/org/freedesktop/impl/portal/PermissionStore",
                "org.freedesktop.impl.portal.PermissionStore",
                "SetPermission",
                "sbssas",
                "devices",
                "true",
                "camera",
                "org.gnome.Snapshot",
                "1",
                "yes",
            ])
            .status();
        let _ = Command::new("busctl")
            .args([
                "--user",
                "call",
                "org.freedesktop.impl.portal.PermissionStore",
                "/org/freedesktop/impl/portal/PermissionStore",
                "org.freedesktop.impl.portal.PermissionStore",
                "SetPermission",
                "sbssas",
                "devices",
                "true",
                "camera",
                "snapshot",
                "1",
                "yes",
            ])
            .status();

        // 7. Clean up manual Omarchy menu overrides if present to ensure single clean entry
        Self::clean_omarchy_menu(home);

        // 8. System-level setup when requested and running with privileges
        if manage_system {
            tracing::info!(
                "Configuring system-level files and permissions (single elevation batch)..."
            );

            let tmp_dir = std::env::temp_dir();
            let tmp_desktop = tmp_dir.join("phonecam_desktop_install.desktop");
            let tmp_svg = tmp_dir.join("phonecam_icon.svg");
            let tmp_png = tmp_dir.join("phonecam_icon.png");
            let _ = fs::write(&tmp_desktop, DESKTOP_TEMPLATE);
            let _ = fs::write(&tmp_svg, ICON_SVG);
            let _ = fs::write(&tmp_png, ICON_PNG);

            let current_user = std::env::var("USER").unwrap_or_else(|_| "rvs".into());
            let script_path = tmp_dir.join("phonecam_sysinstall.sh");
            let sys_script = format!(
                r#"#!/usr/bin/env bash
set -eu
if [ -f "{bin_path}" ]; then
    install -Dm755 "{bin_path}" /usr/local/bin/phonecam
fi
if [ -f "{tmp_desktop}" ]; then
    install -Dm644 "{tmp_desktop}" /usr/share/applications/phonecam.desktop
fi
if [ -f "{tmp_svg}" ]; then
    install -Dm644 "{tmp_svg}" /usr/share/icons/hicolor/scalable/apps/phonecam.svg
fi
if [ -f "{tmp_png}" ]; then
    install -Dm644 "{tmp_png}" /usr/share/icons/hicolor/512x512/apps/phonecam.png
fi
gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
update-desktop-database /usr/share/applications 2>/dev/null || true

mkdir -p /etc/modules-load.d /etc/modprobe.d /etc/udev/rules.d /etc/sudoers.d
echo "v4l2loopback" > /etc/modules-load.d/v4l2loopback.conf
echo 'options v4l2loopback devices=1 video_nr=10 card_label="PhoneCam Virtual Camera" exclusive_caps=0 max_buffers=2' > /etc/modprobe.d/v4l2loopback.conf
cat << 'EOF' > /etc/udev/rules.d/99-v4l2loopback.rules
KERNEL=="video*", SUBSYSTEM=="video4linux", ATTR{{name}}=="PhoneCam*", MODE="0666", GROUP="video"
KERNEL=="video10", MODE="0666", GROUP="video"
EOF
udevadm control --reload-rules 2>/dev/null || true
udevadm trigger 2>/dev/null || true

cat << 'EOF' > /etc/sudoers.d/phonecam-v4l2loopback
{current_user} ALL=(ALL) NOPASSWD: /usr/bin/modprobe v4l2loopback
{current_user} ALL=(ALL) NOPASSWD: /usr/bin/modprobe v4l2loopback *
{current_user} ALL=(ALL) NOPASSWD: /usr/bin/modprobe -r v4l2loopback
{current_user} ALL=(ALL) NOPASSWD: /usr/bin/rmmod v4l2loopback
{current_user} ALL=(ALL) NOPASSWD: /usr/bin/chmod 666 /dev/video*
EOF
chmod 0440 /etc/sudoers.d/phonecam-v4l2loopback
"#,
                bin_path = bin_path.display(),
                tmp_desktop = tmp_desktop.display(),
                tmp_svg = tmp_svg.display(),
                tmp_png = tmp_png.display(),
                current_user = current_user
            );

            if fs::write(&script_path, sys_script).is_ok() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755));
                }
                if crate::util::run_elevated("bash", &[&script_path.to_string_lossy()]).is_ok() {
                    // Remove user desktop entry to prevent duplicate entries in app launchers
                    let _ = fs::remove_file(&user_desktop);
                }
                let _ = fs::remove_file(&script_path);
            }
            let _ = fs::remove_file(&tmp_desktop);
            let _ = fs::remove_file(&tmp_svg);
            let _ = fs::remove_file(&tmp_png);
        }

        Ok(())
    }

    /// Uninstalls PhoneCam from local user and optional system directories.
    pub fn uninstall(manage_system: bool) -> Result<()> {
        let home = Self::home_dir();
        let manage = manage_system && !Self::is_test_or_ci();
        Self::uninstall_from(&home, manage)?;
        println!("\n🗑️ PhoneCam successfully uninstalled!\n");
        Ok(())
    }

    /// Removes PhoneCam files from the specified home directory and optional system locations.
    pub fn uninstall_from(home: &Path, manage_system: bool) -> Result<()> {
        tracing::info!("Uninstalling PhoneCam files...");

        // 1. Remove user binary
        let user_bin = home.join(".local/bin/phonecam");
        if user_bin.exists() {
            let _ = fs::remove_file(&user_bin);
        }

        // 2. Remove user share
        let user_share = home.join(".local/share/phonecam");
        if user_share.exists() {
            let _ = fs::remove_dir_all(&user_share);
        }

        // 3. Remove user desktop launcher
        let user_desktop = home.join(".local/share/applications/phonecam.desktop");
        if user_desktop.exists() {
            let _ = fs::remove_file(&user_desktop);
            let _ = Command::new("update-desktop-database")
                .arg(home.join(".local/share/applications"))
                .status();
        }

        // 4. Remove user icons
        let user_svg = home.join(".local/share/icons/hicolor/scalable/apps/phonecam.svg");
        if user_svg.exists() {
            let _ = fs::remove_file(&user_svg);
        }
        let user_png = home.join(".local/share/icons/hicolor/512x512/apps/phonecam.png");
        if user_png.exists() {
            let _ = fs::remove_file(&user_png);
        }

        // 5. System removals if manage_system is enabled
        if manage_system {
            let tmp_dir = std::env::temp_dir();
            let script_path = tmp_dir.join("phonecam_sysuninstall.sh");
            let sys_script = r#"#!/usr/bin/env bash
rm -f /usr/local/bin/phonecam /usr/bin/phonecam
rm -rf /usr/local/share/phonecam /usr/share/phonecam
rm -f /usr/share/applications/phonecam.desktop
rm -f /usr/share/icons/hicolor/scalable/apps/phonecam.svg
rm -f /usr/share/icons/hicolor/512x512/apps/phonecam.png
rm -f /etc/sudoers.d/phonecam-v4l2loopback
update-desktop-database /usr/share/applications 2>/dev/null || true
gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
"#;
            if fs::write(&script_path, sys_script).is_ok() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755));
                }
                let _ = crate::util::run_elevated("bash", &[&script_path.to_string_lossy()]);
                let _ = fs::remove_file(&script_path);
            }
        }

        Ok(())
    }

    /// Cleans duplicate menu entries in omarchy-menu.jsonc if present.
    fn clean_omarchy_menu(home: &Path) {
        let menu_ext = home.join(".config/omarchy/extensions/omarchy-menu.jsonc");
        if !menu_ext.exists() {
            return;
        }

        if let Ok(content) = fs::read_to_string(&menu_ext) {
            let mut new_lines = Vec::new();
            let mut skipping = false;
            let mut brace_depth: i32 = 0;

            for line in content.lines() {
                if line.contains("\"phonecam\":") {
                    skipping = true;
                    brace_depth =
                        line.matches('{').count() as i32 - line.matches('}').count() as i32;
                    if brace_depth <= 0 {
                        skipping = false;
                    }
                    continue;
                }
                if skipping {
                    brace_depth +=
                        line.matches('{').count() as i32 - line.matches('}').count() as i32;
                    if brace_depth <= 0 {
                        skipping = false;
                    }
                    continue;
                }
                new_lines.push(line);
            }
            let updated = new_lines.join("\n");
            let _ = fs::write(&menu_ext, updated);
        }
    }
}
