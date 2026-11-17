#!/usr/bin/env bash
# ==============================================================================
# scripts/uninstall.sh - Clean uninstallation of PhoneCam
# ==============================================================================

set -euo pipefail

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

echo "==> Removing PhoneCam files..."
rm -f "$HOME/.local/bin/phonecam"
rm -rf "$HOME/.local/share/phonecam"
rm -f "$HOME/.local/share/applications/phonecam.desktop"
rm -f "$HOME/.local/share/icons/hicolor/scalable/apps/phonecam.svg"
rm -f "$HOME/.local/share/icons/hicolor/512x512/apps/phonecam.png"

run_sudo rm -f "/usr/local/bin/phonecam" "/usr/bin/phonecam" || true
run_sudo rm -rf "/usr/local/share/phonecam" "/usr/share/phonecam" || true
run_sudo rm -f "/usr/share/applications/phonecam.desktop" || true
run_sudo rm -f "/usr/share/icons/hicolor/scalable/apps/phonecam.svg" || true
run_sudo rm -f "/usr/share/icons/hicolor/512x512/apps/phonecam.png" || true
run_sudo rm -f "/etc/sudoers.d/phonecam-v4l2loopback" || true

echo "✓ PhoneCam uninstalled cleanly."
