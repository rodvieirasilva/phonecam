#!/usr/bin/env bash
# ==============================================================================
# scripts/install.sh - Install PhoneCam onto the local Omarchy Linux system
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

PHONECAM_SRC=""
if [ -f "phonecam" ]; then
    PHONECAM_SRC="phonecam"
elif [ -f "target/release/phonecam" ]; then
    PHONECAM_SRC="target/release/phonecam"
elif command -v cargo >/dev/null 2>&1; then
    echo "==> Building PhoneCam (release mode)..."
    cargo build --release
    PHONECAM_SRC="target/release/phonecam"
else
    echo "Error: PhoneCam binary not found and cargo is not installed."
    exit 1
fi

# Determine target directories
BIN_TARGET="/usr/local/bin/phonecam"
USER_BIN="$HOME/.local/bin/phonecam"
SHARE_TARGET="/usr/local/share/phonecam"
USER_SHARE="$HOME/.local/share/phonecam"
DESKTOP_TARGET="/usr/share/applications/phonecam.desktop"
USER_DESKTOP="$HOME/.local/share/applications/phonecam.desktop"
ICON_SVG_SYSTEM="/usr/share/icons/hicolor/scalable/apps/phonecam.svg"
ICON_PNG_SYSTEM="/usr/share/icons/hicolor/512x512/apps/phonecam.png"
USER_ICONS="$HOME/.local/share/icons/hicolor"

SUDO_PASS=""
# If config doesn't disable credentials, and ~/.credentials is provided, use it
USE_CREDS="true"
if grep -q "use_credentials_file = false" "$HOME/.config/phonecam/config.toml" 2>/dev/null; then
    USE_CREDS="false"
fi

if [[ "$USE_CREDS" != "false" && -f "$HOME/.credentials" ]]; then
    # shellcheck disable=SC1090
    source "$HOME/.credentials"
    SUDO_PASS="${LINUX_PASSWORD:-}"
fi

run_sudo() {
    if [[ $(id -u) -eq 0 ]]; then
        "$@"
    elif sudo -n true 2>/dev/null; then
        sudo "$@"
    elif [[ -n "$SUDO_PASS" ]]; then
        echo "$SUDO_PASS" | sudo -S "$@"
    elif command -v pkexec >/dev/null 2>&1 && [ -n "${WAYLAND_DISPLAY:-${DISPLAY:-}}" ]; then
        pkexec "$@"
    elif command -v run0 >/dev/null 2>&1; then
        run0 "$@"
    else
        sudo "$@"
    fi
}

PHONECAM_SRC_ABS="$SCRIPT_DIR/$PHONECAM_SRC"
QML_SRC_ABS="$SCRIPT_DIR/src/ui/Main.qml"
if [ ! -f "$QML_SRC_ABS" ] && [ -f "$SCRIPT_DIR/ui/Main.qml" ]; then
    QML_SRC_ABS="$SCRIPT_DIR/ui/Main.qml"
fi
CONFIG_EXAMPLE_ABS="$SCRIPT_DIR/config.example.toml"
DESKTOP_SRC_ABS="$SCRIPT_DIR/pkgbuild/phonecam.desktop"
SVG_SRC_ABS="$SCRIPT_DIR/assets/phonecam.svg"
PNG_SRC_ABS="$SCRIPT_DIR/assets/phonecam.png"

echo "==> Installing PhoneCam binary..."
mkdir -p "$(dirname "$USER_BIN")"
cp -f "$PHONECAM_SRC_ABS" "$USER_BIN"
chmod +x "$USER_BIN"

if run_sudo install -Dm755 "$PHONECAM_SRC_ABS" "$BIN_TARGET" 2>/dev/null; then
    echo "  Installed system-wide: $BIN_TARGET"
fi

echo "==> Installing UI and resources..."
mkdir -p "$USER_SHARE/ui"
if [ -f "$QML_SRC_ABS" ]; then
    cp -f "$QML_SRC_ABS" "$USER_SHARE/ui/Main.qml"
fi

if run_sudo mkdir -p "$SHARE_TARGET/ui" 2>/dev/null; then
    if [ -f "$QML_SRC_ABS" ]; then
        run_sudo cp -f "$QML_SRC_ABS" "$SHARE_TARGET/ui/Main.qml"
    fi
    if [ -f "$CONFIG_EXAMPLE_ABS" ]; then
        run_sudo cp -f "$CONFIG_EXAMPLE_ABS" "$SHARE_TARGET/config.example.toml"
    fi
fi

echo "==> Installing configuration template..."
mkdir -p "$HOME/.config/phonecam"
cp -f "$CONFIG_EXAMPLE_ABS" "$HOME/.config/phonecam/config.example.toml"
if [[ ! -f "$HOME/.config/phonecam/config.toml" ]]; then
    cp -f "$CONFIG_EXAMPLE_ABS" "$HOME/.config/phonecam/config.toml"
fi

echo "==> Installing Desktop Launcher..."
if run_sudo cp -f "$DESKTOP_SRC_ABS" "$DESKTOP_TARGET" 2>/dev/null; then
    echo "  Installed desktop entry: $DESKTOP_TARGET"
    # Remove user-level desktop file if system-level exists to avoid duplicate menu entries
    rm -f "$USER_DESKTOP"
else
    mkdir -p "$(dirname "$USER_DESKTOP")"
    cp -f "$DESKTOP_SRC_ABS" "$USER_DESKTOP"
    echo "  Installed desktop entry: $USER_DESKTOP"
fi

echo "==> Installing Icons..."
mkdir -p "$USER_ICONS/scalable/apps" "$USER_ICONS/512x512/apps"
cp -f "$SVG_SRC_ABS" "$USER_ICONS/scalable/apps/phonecam.svg"
cp -f "$PNG_SRC_ABS" "$USER_ICONS/512x512/apps/phonecam.png"

if run_sudo mkdir -p "$(dirname "$ICON_SVG_SYSTEM")" "$(dirname "$ICON_PNG_SYSTEM")" 2>/dev/null; then
    run_sudo cp -f "$SVG_SRC_ABS" "$ICON_SVG_SYSTEM"
    run_sudo cp -f "$PNG_SRC_ABS" "$ICON_PNG_SYSTEM"
fi

# Update icon cache if tool exists
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    run_sudo gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
fi

# Update desktop database
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
    run_sudo update-desktop-database /usr/share/applications 2>/dev/null || true
fi

echo "==> Configuring one-time root setup (autoload at boot & zero-root permissions)..."
# 1. Autoload module at boot
MODULES_LOAD="/etc/modules-load.d/v4l2loopback.conf"
TMP_MOD_LOAD="/tmp/phonecam_modules_load"
echo "v4l2loopback" > "$TMP_MOD_LOAD"
run_sudo cp "$TMP_MOD_LOAD" "$MODULES_LOAD" 2>/dev/null || true
rm -f "$TMP_MOD_LOAD"

# 2. Module default parameters
MODPROBE_CONF="/etc/modprobe.d/v4l2loopback.conf"
TMP_MOD_CONF="/tmp/phonecam_modprobe_conf"
echo 'options v4l2loopback devices=1 video_nr=10 card_label="PhoneCam Virtual Camera" exclusive_caps=0 max_buffers=2' > "$TMP_MOD_CONF"
run_sudo cp "$TMP_MOD_CONF" "$MODPROBE_CONF" 2>/dev/null || true
rm -f "$TMP_MOD_CONF"

# 3. Udev rule for non-root read/write permissions
UDEV_RULE="/etc/udev/rules.d/99-v4l2loopback.rules"
TMP_UDEV="/tmp/phonecam_udev"
echo 'KERNEL=="video*", SUBSYSTEM=="video4linux", ATTR{name}=="PhoneCam*", MODE="0666", GROUP="video"' > "$TMP_UDEV"
echo 'KERNEL=="video10", MODE="0666", GROUP="video"' >> "$TMP_UDEV"
run_sudo cp "$TMP_UDEV" "$UDEV_RULE" 2>/dev/null || true
rm -f "$TMP_UDEV"
run_sudo udevadm control --reload-rules 2>/dev/null || true
run_sudo udevadm trigger 2>/dev/null || true

# 4. Passwordless sudoers rule fallback
SUDOERS_FILE="/etc/sudoers.d/phonecam-v4l2loopback"
if [[ ! -f "$SUDOERS_FILE" ]]; then
    TMP_SUDOERS="/tmp/phonecam_sudoers"
    echo "$USER ALL=(ALL) NOPASSWD: /usr/bin/modprobe v4l2loopback *" > "$TMP_SUDOERS"
    echo "$USER ALL=(ALL) NOPASSWD: /usr/bin/modprobe -r v4l2loopback" >> "$TMP_SUDOERS"
    echo "$USER ALL=(ALL) NOPASSWD: /usr/bin/rmmod v4l2loopback" >> "$TMP_SUDOERS"
    echo "$USER ALL=(ALL) NOPASSWD: /usr/bin/chmod 666 /dev/video*" >> "$TMP_SUDOERS"
    chmod 0440 "$TMP_SUDOERS"
    run_sudo cp "$TMP_SUDOERS" "$SUDOERS_FILE" 2>/dev/null || true
    rm -f "$TMP_SUDOERS"
    echo "  Configured passwordless v4l2loopback management in $SUDOERS_FILE"
fi

# 5. Grant desktop camera portal permissions to camera preview apps (e.g. Snapshot)
if command -v busctl >/dev/null 2>&1; then
    busctl --user call org.freedesktop.impl.portal.PermissionStore /org/freedesktop/impl/portal/PermissionStore org.freedesktop.impl.portal.PermissionStore SetPermission sbssas "devices" true "camera" "org.gnome.Snapshot" 1 "yes" 2>/dev/null || true
    busctl --user call org.freedesktop.impl.portal.PermissionStore /org/freedesktop/impl/portal/PermissionStore org.freedesktop.impl.portal.PermissionStore SetPermission sbssas "devices" true "camera" "snapshot" 1 "yes" 2>/dev/null || true
fi

# Remove old samsung-webcam and duplicate local launchers
rm -f "$HOME/.local/share/applications/samsung-webcam.desktop" 2>/dev/null || true
rm -f "$HOME/.local/share/applications/phonecam.desktop" 2>/dev/null || true

echo "==> Consolidating Omarchy application launcher (single entry)..."
MENU_EXT="$HOME/.config/omarchy/extensions/omarchy-menu.jsonc"
if [[ -f "$MENU_EXT" ]]; then
    python3 -c '
path = "'"$MENU_EXT"'"
with open(path, "r") as f:
    content = f.read()
import re
# Strip out manual phonecam blocks to avoid duplicate entries in menu
cleaned = re.sub(r"\s*// PhoneCam[^\n]*\n\s*\"phonecam\":\s*\{[^}]*\},?", "", content)
cleaned = re.sub(r"\s*\"phonecam\":\s*\{[^}]*\},?", "", cleaned)
with open(path, "w") as f:
    f.write(cleaned)
print("  Verified single clean entry via /usr/share/applications/phonecam.desktop")
' || true
fi

echo "✨ PhoneCam has been successfully installed!"
echo "   Launch GUI: phonecam"
echo "   Launch TUI: phonecam --tui"
echo "   Omarchy Menu: Open Menu -> search 'PhoneCam' or 'webcam'"
