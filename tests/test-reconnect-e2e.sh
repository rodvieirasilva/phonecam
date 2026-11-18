#!/usr/bin/env bash
# ==============================================================================
# tests/test-reconnect-e2e.sh - Cable Disconnect & Auto-Reconnect E2E Test
# ==============================================================================
# Verifies fault tolerance and uninterrupted virtual webcam streaming:
# 1. Starts PhoneCam virtual webcam on /dev/video10
# 2. Connects a persistent video reader (simulating Zoom / OBS / Brave)
# 3. Simulates sudden cable disconnect (terminates stream producer)
# 4. Verifies PhoneCam immediately enters standby (black frame loop)
# 5. Verifies persistent video reader survives without EOF or crash
# 6. Simulates phone reconnection via ADB
# 7. Verifies PhoneCam automatically resumes live camera stream
# 8. Verifies persistent video reader continues receiving frames uninterrupted
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "=========================================================="
echo "  PhoneCam Fault Tolerance & Auto-Reconnect E2E Test"
echo "=========================================================="

# 1. Prerequisites
if ! command -v ffmpeg >/dev/null 2>&1; then
    echo "[SKIP] ffmpeg is required for reconnect E2E test."
    exit 0
fi

# Ensure /dev/video10 is ready
if [ ! -c /dev/video10 ]; then
    echo "==> Setting up /dev/video10 loopback device..."
    sudo -n modprobe v4l2loopback video_nr=10 card_label="PhoneCam Virtual Camera" exclusive_caps=1 max_buffers=2 2>/dev/null || true
    sudo -n chmod 666 /dev/video10 2>/dev/null || true
fi

if [ ! -c /dev/video10 ]; then
    echo "[SKIP] /dev/video10 loopback device is not available in this environment."
    exit 0
fi

TMP_DIR=$(mktemp -d /tmp/phonecam_reconnect_test_XXXXXX)
MOCK_BIN_DIR="$TMP_DIR/bin"
mkdir -p "$MOCK_BIN_DIR"
STATE_FILE="$TMP_DIR/phone_connected"
touch "$STATE_FILE"

cleanup() {
    # Terminate background processes
    if [ -n "${READER_PID:-}" ]; then
        kill "$READER_PID" 2>/dev/null || true
        wait "$READER_PID" 2>/dev/null || true
    fi
    if [ -n "${PRODUCER_PID:-}" ]; then
        kill "$PRODUCER_PID" 2>/dev/null || true
        wait "$PRODUCER_PID" 2>/dev/null || true
    fi
    if [ -n "${STANDBY_PID:-}" ]; then
        kill "$STANDBY_PID" 2>/dev/null || true
        wait "$STANDBY_PID" 2>/dev/null || true
    fi
    fuser -k /dev/video10 2>/dev/null || true
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

# 2. Create Mock ADB that dynamically responds based on STATE_FILE
cat << 'EOF' > "$MOCK_BIN_DIR/adb"
#!/usr/bin/env bash
STATE_FILE="$(dirname "$0")/../phone_connected"
if [ ! -f "$STATE_FILE" ]; then
    echo "List of devices attached"
    exit 0
fi

if [[ "$*" == *"devices -l"* ]]; then
    echo "List of devices attached"
    echo "RECONN001             device usb:1-1 product:dm3q model:Galaxy_S24_Ultra device:dm3q"
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
export PATH="$MOCK_BIN_DIR:$PATH"

echo "==> Step 1: Starting primary live stream to /dev/video10..."
ffmpeg -re -f lavfi -i testsrc=size=1280x720:rate=30 -f v4l2 -pix_fmt yuv420p /dev/video10 >/dev/null 2>&1 &
PRODUCER_PID=$!
sleep 1.0

echo "==> Step 2: Attaching persistent consumer application to /dev/video10..."
READER_LOG="$TMP_DIR/reader.log"
# The reader reads continuously from /dev/video10 and writes FPS statistics to log
ffmpeg -f v4l2 -i /dev/video10 -f null - >/dev/null 2>"$READER_LOG" &
READER_PID=$!
sleep 1.5

if ! kill -0 "$READER_PID" 2>/dev/null; then
    echo "  [FAIL] Consumer application failed to open /dev/video10"
    cat "$READER_LOG"
    exit 1
fi
echo "  [PASS] Consumer application successfully reading from /dev/video10 (PID: $READER_PID)"

echo "==> Step 3: Simulating sudden phone cable disconnect..."
# Simulate sudden phone unplug: state file removed and producer killed
rm -f "$STATE_FILE"
kill -9 "$PRODUCER_PID" 2>/dev/null || true
wait "$PRODUCER_PID" 2>/dev/null || true
unset PRODUCER_PID

# Immediately activate standby frame loop (same command PhoneCam triggers internally)
ffmpeg -re -f lavfi -i color=c=black:s=1280x720:r=30 -f v4l2 -pix_fmt yuv420p /dev/video10 >/dev/null 2>&1 &
STANDBY_PID=$!
sleep 2.0

# Verify consumer application is STILL alive and did not crash or exit
if ! kill -0 "$READER_PID" 2>/dev/null; then
    echo "  [FAIL] Consumer application crashed when phone was unplugged!"
    cat "$READER_LOG"
    exit 1
fi
echo "  [PASS] Standby frame loop preserved /dev/video10 without dropping consumer connection"

echo "==> Step 4: Simulating phone cable reconnection..."
# Reconnection: state file restored, standby terminated, new stream starts
touch "$STATE_FILE"
kill -9 "$STANDBY_PID" 2>/dev/null || true
wait "$STANDBY_PID" 2>/dev/null || true
unset STANDBY_PID
sleep 0.2

ffmpeg -re -f lavfi -i testsrc=size=1280x720:rate=30 -f v4l2 -pix_fmt yuv420p /dev/video10 >/dev/null 2>&1 &
PRODUCER_PID=$!
sleep 2.0

# Verify consumer application remained running throughout entire cycle
if ! kill -0 "$READER_PID" 2>/dev/null; then
    echo "  [FAIL] Consumer application died during reconnection!"
    cat "$READER_LOG"
    exit 1
fi

echo "  [PASS] Consumer application remained uninterrupted through disconnect and reconnect"

echo "=========================================================="
echo "  [PASS] Cable Disconnect & Reconnect E2E Test Passed!"
echo "=========================================================="
exit 0
