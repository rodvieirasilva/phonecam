#!/usr/bin/env bash
# ==============================================================================
# scripts/capture-screenshot.sh - Capture pixel-perfect PhoneCam window screenshot
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "==> Building PhoneCam binary..."
cargo build --release

echo "==> Launching PhoneCam in background..."
target/release/phonecam &
APP_PID=$!

cleanup() {
    if kill -0 "$APP_PID" 2>/dev/null; then
        kill -TERM "$APP_PID" 2>/dev/null || true
        wait "$APP_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

# Wait for window to map to Wayland compositor
sleep 2

echo "==> Finding PhoneCam window geometry via hyprctl..."
GEO=$(python3 -c '
import subprocess, json, sys

try:
    clients = json.loads(subprocess.check_output(["hyprctl", "-j", "clients"]))
    for c in clients:
        c_class = c.get("class", "").lower()
        c_init = c.get("initialClass", "").lower()
        if "phonecam" in c_class or "phonecam" in c_init:
            x, y = c["at"]
            w, h = c["size"]
            print(f"{x},{y} {w}x{h}")
            sys.exit(0)
    sys.exit(1)
except Exception:
    sys.exit(1)
' || echo "")

if [ -n "$GEO" ]; then
    echo "==> Capturing window at geometry: $GEO"
    grim -g "$GEO" assets/screenshot-gui.png
    echo "==> Saved clean screenshot to assets/screenshot-gui.png"
else
    echo "==> Warning: Could not detect window geometry via hyprctl, capturing full screen"
    grim assets/screenshot-gui.png
fi

ls -lh assets/screenshot-gui.png
echo "==> Screenshot capture completed successfully!"
