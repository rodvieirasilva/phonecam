#!/usr/bin/env bash
# ==============================================================================
# scripts/capture-screenshot.sh - Capture PhoneCam GUI & TUI screenshots
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "==> Building PhoneCam binary..."
cargo build --release

cleanup() {
    if [ -n "${APP_PID:-}" ] && kill -0 "$APP_PID" 2>/dev/null; then
        kill -TERM "$APP_PID" 2>/dev/null || true
        wait "$APP_PID" 2>/dev/null || true
    fi
    if [ -n "${TUI_PID:-}" ] && kill -0 "$TUI_PID" 2>/dev/null; then
        kill -TERM "$TUI_PID" 2>/dev/null || true
        wait "$TUI_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT INT TERM

# ------------------------------------------------------------------------------
# 1. Capture Native GTK4 GUI Window
# ------------------------------------------------------------------------------
echo "==> Step 1: Launching PhoneCam GUI in background..."
target/release/phonecam &
APP_PID=$!

sleep 2

echo "==> Finding PhoneCam GUI window geometry via hyprctl..."
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
    echo "==> Capturing GUI window at geometry: $GEO"
    grim -g "$GEO" assets/screenshot-gui.png
    echo "==> Saved clean screenshot to assets/screenshot-gui.png"
else
    echo "==> Warning: Could not detect window geometry via hyprctl, capturing full screen"
    grim assets/screenshot-gui.png
fi

# Terminate GUI instance and wait for lock to release
kill -TERM "$APP_PID" 2>/dev/null || true
wait "$APP_PID" 2>/dev/null || true
unset APP_PID
sleep 0.8

# ------------------------------------------------------------------------------
# 2. Capture Terminal TUI Dashboard
# ------------------------------------------------------------------------------
echo "==> Step 2: Capturing Terminal TUI Dashboard..."
TMP_TUI=$(mktemp -d /tmp/phonecam_tui_snap_XXXXXX)
MOCK_BIN="$TMP_TUI/bin"
mkdir -p "$MOCK_BIN"

cat << 'EOF' > "$MOCK_BIN/adb"
#!/usr/bin/env bash
if [[ "$*" == *"devices -l"* ]]; then
    echo "List of devices attached"
    echo "FAKESAM001            device usb:1-1 product:dm3q model:Galaxy_S24_Ultra device:dm3q"
    exit 0
elif [[ "$*" == *"getprop ro.product.model"* ]]; then
    echo "Galaxy S24 Ultra"
    exit 0
elif [[ "$*" == *"getprop ro.product.brand"* ]]; then
    echo "Samsung"
    exit 0
elif [[ "$*" == *"getprop ro.build.version.release"* ]]; then
    echo "15"
    exit 0
elif [[ "$*" == *"dumpsys battery"* ]]; then
    echo "status: 2"
    echo "level: 95"
    exit 0
fi
exit 0
EOF
chmod +x "$MOCK_BIN/adb"

cat << 'EOF' > "$MOCK_BIN/scrcpy"
#!/usr/bin/env bash
trap 'exit 0' SIGTERM SIGINT SIGHUP
while true; do
    sleep 0.1
done
EOF
chmod +x "$MOCK_BIN/scrcpy"

RUN_SCRIPT="$TMP_TUI/run.sh"
cat << EOF > "$RUN_SCRIPT"
#!/usr/bin/env bash
export PATH="$MOCK_BIN:\$PATH"
exec "$SCRIPT_DIR/target/release/phonecam" --tui
EOF
chmod +x "$RUN_SCRIPT"

if command -v foot >/dev/null 2>&1; then
    foot --app-id=phonecam-tui-capture --window-size-chars=76x22 "$RUN_SCRIPT" &
    TUI_PID=$!
    sleep 2.5

    TUI_GEO=$(python3 -c '
import subprocess, json, sys

try:
    clients = json.loads(subprocess.check_output(["hyprctl", "-j", "clients"]))
    for c in clients:
        if "phonecam-tui-capture" in c.get("initialClass", "") or "phonecam-tui-capture" in c.get("class", ""):
            x, y = c["at"]
            w, h = c["size"]
            print(f"{x},{y} {w}x{h}")
            sys.exit(0)
    sys.exit(1)
except Exception:
    sys.exit(1)
' || echo "")

    if [ -n "$TUI_GEO" ]; then
        echo "==> Capturing TUI window at geometry: $TUI_GEO"
        grim -g "$TUI_GEO" assets/screenshot-tui.png
        echo "==> Saved clean screenshot to assets/screenshot-tui.png"
    fi

    kill -TERM "$TUI_PID" 2>/dev/null || true
    wait "$TUI_PID" 2>/dev/null || true
    unset TUI_PID
fi
rm -rf "$TMP_TUI"

ls -lh assets/screenshot-gui.png assets/screenshot-tui.png
echo "==> All screenshots captured successfully!"
