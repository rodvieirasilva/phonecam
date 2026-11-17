#!/usr/bin/env bash
# ==============================================================================
# scripts/install.sh - Install PhoneCam onto the local Omarchy Linux system
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "==> Building PhoneCam (release mode)..."
cargo build --release

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
if [[ -f "$HOME/.credentials" ]]; then
    # shellcheck disable=SC1090
    source "$HOME/.credentials"
    SUDO_PASS="${LINUX_PASSWORD:-}"
fi

run_sudo() {
    if [[ $(id -u) -eq 0 ]]; then
        "$@"
    elif [[ -n "$SUDO_PASS" ]]; then
        echo "$SUDO_PASS" | sudo -S "$@"
    elif command -v run0 >/dev/null 2>&1; then
        run0 "$@"
    else
        sudo "$@"
    fi
}

echo "==> Installing PhoneCam binary..."
mkdir -p "$(dirname "$USER_BIN")"
cp -f "target/release/phonecam" "$USER_BIN"
chmod +x "$USER_BIN"

if run_sudo install -Dm755 "target/release/phonecam" "$BIN_TARGET" 2>/dev/null; then
    echo "  Installed system-wide: $BIN_TARGET"
fi

echo "==> Installing QML UI and resources..."
mkdir -p "$USER_SHARE/ui"
cp -f "ui/Main.qml" "$USER_SHARE/ui/Main.qml"

if run_sudo mkdir -p "$SHARE_TARGET/ui" 2>/dev/null; then
    run_sudo cp -f "ui/Main.qml" "$SHARE_TARGET/ui/Main.qml"
fi

echo "==> Installing Desktop Launchers..."
mkdir -p "$(dirname "$USER_DESKTOP")"
cp -f "pkgbuild/phonecam.desktop" "$USER_DESKTOP"

if run_sudo cp -f "pkgbuild/phonecam.desktop" "$DESKTOP_TARGET" 2>/dev/null; then
    echo "  Installed desktop entry: $DESKTOP_TARGET"
fi

echo "==> Installing Icons..."
mkdir -p "$USER_ICONS/scalable/apps" "$USER_ICONS/512x512/apps"
cp -f "assets/phonecam.svg" "$USER_ICONS/scalable/apps/phonecam.svg"
cp -f "assets/phonecam.png" "$USER_ICONS/512x512/apps/phonecam.png"

if run_sudo mkdir -p "$(dirname "$ICON_SVG_SYSTEM")" "$(dirname "$ICON_PNG_SYSTEM")" 2>/dev/null; then
    run_sudo cp -f "assets/phonecam.svg" "$ICON_SVG_SYSTEM"
    run_sudo cp -f "assets/phonecam.png" "$ICON_PNG_SYSTEM"
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
echo 'options v4l2loopback devices=1 video_nr=10 card_label="PhoneCam Virtual Camera" exclusive_caps=1' > "$TMP_MOD_CONF"
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

# Remove old samsung-webcam launcher if present
rm -f "$HOME/.local/share/applications/samsung-webcam.desktop" 2>/dev/null || true

echo "==> Updating Omarchy Menu configuration..."
MENU_EXT="$HOME/.config/omarchy/extensions/omarchy-menu.jsonc"
if [[ -f "$MENU_EXT" ]]; then
    python3 -c '
import json, re

path = "'"$MENU_EXT"'"
with open(path, "r") as f:
    content = f.read()

# Replace or add phonecam entries
if "phonecam" not in content:
    idx = content.rfind("}")
    if idx != -1:
        entry = """
  // PhoneCam - Smartphone Virtual Webcam (Rust & GTK4)
  "phonecam": {
    "icon": "",
    "label": "PhoneCam",
    "aliases": ["webcam", "camera", "phonecam"],
    "action": "phonecam"
  },
"""
        new_content = content[:idx] + entry + content[idx:]
        with open(path, "w") as f:
            f.write(new_content)
        print("  Added PhoneCam entry to omarchy-menu.jsonc")
else:
    print("  PhoneCam already present in omarchy-menu.jsonc")
' || true
fi

echo "✨ PhoneCam has been successfully installed!"
echo "   Launch GUI: phonecam"
echo "   Launch TUI: phonecam --tui"
echo "   Omarchy Menu: Open Menu -> search 'PhoneCam' or 'webcam'"
