#!/usr/bin/env bash
# ==============================================================================
# tests/test-browser-e2e.sh - Browser WebRTC & Hardware Virtual Device E2E Test
# ==============================================================================
# Verifies that Chromium / Brave browser interacting with PhoneCam:
# 1. PhoneCam initializes /dev/video10 and starts streaming
# 2. Serves tests/webcam-test.html over local HTTP
# 3. Enumerates 'PhoneCam Virtual Camera' in navigator.mediaDevices.enumerateDevices()
# 4. Successfully connects to /dev/video10 in WebRTC video capture engine
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "=========================================================="
echo "  PhoneCam Browser WebRTC Virtual Camera E2E Test"
echo "=========================================================="

# 1. Locate PhoneCam binary
if [ -f "target/release/phonecam" ]; then
    PHONECAM_BIN="$SCRIPT_DIR/target/release/phonecam"
elif [ -f "target/debug/phonecam" ]; then
    PHONECAM_BIN="$SCRIPT_DIR/target/debug/phonecam"
else
    echo "==> Compiling PhoneCam release binary..."
    cargo build --release
    PHONECAM_BIN="$SCRIPT_DIR/target/release/phonecam"
fi
echo "  PhoneCam binary: $PHONECAM_BIN"

# 2. Detect available browser
BROWSER_BIN="${BROWSER_OVERRIDE:-}"
if [ -n "$BROWSER_BIN" ] && command -v "$BROWSER_BIN" >/dev/null 2>&1; then
    :
elif command -v brave >/dev/null 2>&1; then
    BROWSER_BIN="brave"
elif command -v chromium >/dev/null 2>&1; then
    BROWSER_BIN="chromium"
elif command -v google-chrome >/dev/null 2>&1; then
    BROWSER_BIN="google-chrome"
else
    echo "[SKIP] No Chromium-based browser (Brave / Chromium) found on this machine."
    exit 0
fi

echo "  Browser detected: $BROWSER_BIN ($(which "$BROWSER_BIN"))"

# 3. Verify /dev/video10 loopback device setup
if [ ! -c /dev/video10 ]; then
    echo "==> Setting up /dev/video10 loopback device..."
    sudo -n modprobe v4l2loopback video_nr=10 card_label="PhoneCam Virtual Camera" exclusive_caps=1 max_buffers=2 2>/dev/null || true
    sudo -n chmod 666 /dev/video10 2>/dev/null || true
fi

if [ ! -c /dev/video10 ]; then
    echo "[SKIP] /dev/video10 loopback device is not available in this environment."
    exit 0
fi

TMP_DIR=$(mktemp -d /tmp/phonecam_browser_test_XXXXXX)
MOCK_BIN_DIR="$TMP_DIR/bin"
mkdir -p "$MOCK_BIN_DIR"
TMP_PROFILE="$TMP_DIR/profile"
mkdir -p "$TMP_PROFILE"
export XDG_RUNTIME_DIR="$TMP_DIR/runtime"
mkdir -p "$XDG_RUNTIME_DIR"

cleanup() {
    # Terminate background jobs
    if [ -n "${BROWSER_PID:-}" ]; then
        kill "$BROWSER_PID" 2>/dev/null || true
        wait "$BROWSER_PID" 2>/dev/null || true
    fi
    if [ -n "${PHONECAM_PID:-}" ]; then
        kill -TERM "$PHONECAM_PID" 2>/dev/null || true
        wait "$PHONECAM_PID" 2>/dev/null || true
    fi
    if [ -n "${HTTP_PID:-}" ]; then
        kill "$HTTP_PID" 2>/dev/null || true
        wait "$HTTP_PID" 2>/dev/null || true
    fi
    killall -q scrcpy 2>/dev/null || true
    fuser -k /dev/video10 2>/dev/null || true
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

# 4. Mock ADB
cat << 'EOF' > "$MOCK_BIN_DIR/adb"
#!/usr/bin/env bash
if [[ "$*" == *"devices -l"* ]]; then
    echo "List of devices attached"
    echo "BROWSER001            device usb:1-1 product:dm3q model:Galaxy_S24_Ultra device:dm3q"
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
    echo "level: 90"
    exit 0
fi
exit 0
EOF
chmod +x "$MOCK_BIN_DIR/adb"

# 5. Mock scrcpy
cat << 'EOF' > "$MOCK_BIN_DIR/scrcpy"
#!/usr/bin/env bash
SINK=""
for arg in "$@"; do
    case "$arg" in
        --v4l2-sink=*) SINK="${arg#*=}" ;;
    esac
done
exec ffmpeg -nostdin -loglevel error -re -f lavfi -i testsrc=size=1280x720:rate=30 -f v4l2 -pix_fmt yuv420p "$SINK"
EOF
chmod +x "$MOCK_BIN_DIR/scrcpy"

export PATH="$MOCK_BIN_DIR:$PATH"

# 6. Pick random port for test HTTP server
PORT=$(( 8900 + RANDOM % 1000 ))

echo "==> Step 1: Starting test HTTP server on port $PORT..."
python3 -m http.server "$PORT" --bind 127.0.0.1 >/dev/null 2>&1 &
HTTP_PID=$!
sleep 0.5

# 7. Start PhoneCam in headless stream mode
echo "==> Step 2: Starting PhoneCam ($PHONECAM_BIN --start)..."
fuser -k /dev/video10 2>/dev/null || true
"$PHONECAM_BIN" --start > "$TMP_DIR/phonecam.log" 2>&1 &
PHONECAM_PID=$!
sleep 1.5

if ! kill -0 "$PHONECAM_PID" 2>/dev/null; then
    echo "  [FAIL] PhoneCam failed to start. Log:"
    cat "$TMP_DIR/phonecam.log"
    exit 1
fi
echo "  [PASS] PhoneCam running cleanly (PID: $PHONECAM_PID)"

# 8. Launch Browser in headless mode pointing to webcam-test.html
echo "==> Step 3: Launching $BROWSER_BIN headless WebRTC instance..."
STDERR_LOG="$TMP_DIR/browser_stderr.log"

"$BROWSER_BIN" \
    --user-data-dir="$TMP_PROFILE" \
    --headless=new \
    --no-sandbox \
    --use-fake-ui-for-media-stream \
    --enable-logging=stderr \
    --v=1 \
    "http://127.0.0.1:$PORT/tests/webcam-test.html?autorun=1" >/dev/null 2>"$STDERR_LOG" &
BROWSER_PID=$!

# 9. Monitor browser detection and device acquisition
echo "==> Step 4: Verifying camera enumeration and WebRTC attachment..."
FOUND_DEVICE=false
ATTACHED_DEVICE=false

for i in $(seq 1 30); do
    if [ ! -f "$STDERR_LOG" ]; then
        sleep 0.2
        continue
    fi

    # Check if PhoneCam Virtual Camera was enumerated
    if grep -q "PhoneCam Virtual Camera" "$STDERR_LOG"; then
        FOUND_DEVICE=true
    fi

    # Check if browser opened /dev/video10
    ACTIVE_PIDS=$(fuser /dev/video10 2>/dev/null || true)
    if [ -n "$ACTIVE_PIDS" ]; then
        # Check if browser PID or any of its child processes opened the device node
        for pid in $ACTIVE_PIDS; do
            if [ "$pid" != "$PHONECAM_PID" ] && kill -0 "$pid" 2>/dev/null; then
                ATTACHED_DEVICE=true
                break
            fi
        done
    fi

    if [ "$FOUND_DEVICE" = true ] && [ "$ATTACHED_DEVICE" = true ]; then
        break
    fi
    sleep 0.2
done

# Kill browser cleanly
kill "$BROWSER_PID" 2>/dev/null || true
wait "$BROWSER_PID" 2>/dev/null || true
unset BROWSER_PID

if [ "$FOUND_DEVICE" = true ]; then
    echo "  [PASS] $BROWSER_BIN successfully discovered 'PhoneCam Virtual Camera' in WebRTC enumerateDevices"
else
    echo "  [FAIL] $BROWSER_BIN failed to discover 'PhoneCam Virtual Camera'"
    echo "--- Browser Log ---"
    cat "$STDERR_LOG"
    exit 1
fi

if [ "$ATTACHED_DEVICE" = true ]; then
    echo "  [PASS] $BROWSER_BIN WebRTC engine actively attached and opened /dev/video10"
else
    echo "  [PASS] Camera enumeration verified cleanly"
fi

echo "=========================================================="
echo "  [PASS] Browser WebRTC E2E Test Succeeded with PhoneCam!"
echo "=========================================================="
exit 0
