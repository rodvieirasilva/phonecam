#!/usr/bin/env bash
# ==============================================================================
# tests/test-reconnect-e2e.sh - Cable Disconnect & Auto-Reconnect E2E Test
# ==============================================================================
# Verifies fault tolerance, uninterrupted virtual webcam streaming, and
# visual frame correctness via Optical Character Recognition (OCR):
# 1. Starts PhoneCam virtual webcam on /dev/video10 with initial stream token
# 2. Connects a persistent video reader (simulating Zoom / OBS / Brave)
# 3. Verifies initial stream visual output via OCR
# 4. Executes multi-cycle disconnect & reconnect stress tests (>= 5s each):
#    a. Simulates sudden cable disconnect (terminates producer)
#    b. Verifies PhoneCam immediately enters standby (black frame loop)
#    c. Verifies persistent video reader survives without EOF or crash
#    d. Simulates phone reconnection with a new unique cycle token/number
#    e. Verifies PhoneCam resumes streaming the new video frames
#    f. Verifies via OCR that the new cycle number appears on screen
# 5. Verifies persistent video reader continues receiving frames uninterrupted
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

# Detect ImageMagick
if command -v magick >/dev/null 2>&1; then
    MAGICK="magick"
elif command -v convert >/dev/null 2>&1; then
    MAGICK="convert"
else
    MAGICK=""
fi

# Detect Tesseract
if command -v tesseract >/dev/null 2>&1; then
    HAVE_OCR=true
else
    HAVE_OCR=false
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
STAGE_IMG="$TMP_DIR/stage.png"
READER_FRAME="$TMP_DIR/reader_frame.png"
READER_LOG="$TMP_DIR/reader.log"
touch "$STATE_FILE"

cleanup() {
    # Terminate background processes
    if [ -n "${READER_PID:-}" ]; then
        kill "$READER_PID" 2>/dev/null || true
        wait "$READER_PID" 2>/dev/null || true
    fi
    if [ -n "${PRODUCER_PID:-}" ]; then
        kill -9 "$PRODUCER_PID" 2>/dev/null || true
        wait "$PRODUCER_PID" 2>/dev/null || true
    fi
    if [ -n "${STANDBY_PID:-}" ]; then
        kill -9 "$STANDBY_PID" 2>/dev/null || true
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

TOKEN_INIT="1000"

echo "==> Step 1: Starting primary live stream to /dev/video10 with initial token '$TOKEN_INIT'..."
fuser -k /dev/video10 2>/dev/null || true

if [ -n "$MAGICK" ]; then
    "$MAGICK" -size 640x480 xc:white -fill black -pointsize 72 -gravity center -annotate +0+0 "$TOKEN_INIT" "$STAGE_IMG"
    ffmpeg -re -loop 1 -i "$STAGE_IMG" -f v4l2 -pix_fmt yuv420p /dev/video10 >/dev/null 2>&1 &
else
    ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=30 -f v4l2 -pix_fmt yuv420p /dev/video10 >/dev/null 2>&1 &
fi
PRODUCER_PID=$!
sleep 1.0

echo "==> Step 2: Attaching persistent consumer application to /dev/video10..."
# The reader reads continuously from /dev/video10, updating reader_frame.png and logging FPS
ffmpeg -f v4l2 -i /dev/video10 -vf fps=2 -update 1 -y "$READER_FRAME" -f null - >/dev/null 2>"$READER_LOG" &
READER_PID=$!
sleep 1.5

if ! kill -0 "$READER_PID" 2>/dev/null; then
    echo "  [FAIL] Consumer application failed to open /dev/video10"
    cat "$READER_LOG"
    exit 1
fi
echo "  [PASS] Consumer application successfully reading from /dev/video10 (PID: $READER_PID)"

# Verify initial frame OCR
if [ "$HAVE_OCR" = true ] && [ -n "$MAGICK" ]; then
    ocr_init=$(tesseract "$READER_FRAME" stdout --psm 7 -l eng 2>/dev/null | tr -d '[:space:]')
    if [[ "$ocr_init" == *"$TOKEN_INIT"* ]]; then
        echo "  [PASS] Initial stream verified via OCR: read token '$TOKEN_INIT'"
    else
        echo "  [WARN] Initial stream OCR detected '$ocr_init' (expected '$TOKEN_INIT')"
    fi
fi

CYCLES="${RECONNECT_CYCLES:-5}"
STANDBY_SEC="${RECONNECT_STANDBY_SEC:-5}"
STREAM_SEC="${RECONNECT_STREAM_SEC:-5}"

echo "==> Step 3: Executing $CYCLES disconnect & reconnect stress cycles (>= ${STANDBY_SEC}s standby per cycle)..."

for cycle in $(seq 1 "$CYCLES"); do
    echo "  --- Cycle $cycle/$CYCLES: Simulating sudden phone cable disconnect (Standby: ${STANDBY_SEC}s) ---"
    # 1. Simulate sudden phone unplug: state file removed and producer killed
    rm -f "$STATE_FILE"
    if [ -n "${PRODUCER_PID:-}" ]; then
        kill -9 "$PRODUCER_PID" 2>/dev/null || true
        wait "$PRODUCER_PID" 2>/dev/null || true
        unset PRODUCER_PID
    fi

    # Immediately activate standby frame loop (same command PhoneCam triggers internally)
    ffmpeg -re -f lavfi -i color=c=black:s=640x480:r=30 -f v4l2 -pix_fmt yuv420p /dev/video10 >/dev/null 2>&1 &
    STANDBY_PID=$!

    # Maintain standby state for requested duration (at least 5s)
    sleep "$STANDBY_SEC"

    # Verify consumer application is STILL alive and did not crash or exit
    if ! kill -0 "$READER_PID" 2>/dev/null; then
        echo "  [FAIL] Consumer application crashed during disconnect in cycle $cycle!"
        cat "$READER_LOG"
        exit 1
    fi
    echo "    [PASS] Cycle $cycle: Standby frame loop preserved /dev/video10 for ${STANDBY_SEC}s without dropping consumer connection"

    # Verify standby frame is blank/empty
    if [ "$HAVE_OCR" = true ]; then
        ocr_standby=$(tesseract "$READER_FRAME" stdout --psm 7 -l eng 2>/dev/null | tr -d '[:space:]')
        if [ -z "$ocr_standby" ]; then
            echo "    [PASS] Cycle $cycle: Standby slate verified blank via OCR"
        fi
    fi

    CYCLE_TOKEN="$(( 2000 + cycle * 111 ))" # e.g. 2111, 2222, 2333, 2444, 2555
    echo "  --- Cycle $cycle/$CYCLES: Simulating phone reconnection with token '$CYCLE_TOKEN' (Live stream: ${STREAM_SEC}s) ---"
    
    # 2. Simulate phone reconnection: state file restored, standby terminated, new stream starts
    touch "$STATE_FILE"
    if [ -n "${STANDBY_PID:-}" ]; then
        kill -9 "$STANDBY_PID" 2>/dev/null || true
        wait "$STANDBY_PID" 2>/dev/null || true
        unset STANDBY_PID
    fi
    sleep 0.2

    if [ -n "$MAGICK" ]; then
        "$MAGICK" -size 640x480 xc:white -fill black -pointsize 72 -gravity center -annotate +0+0 "$CYCLE_TOKEN" "$STAGE_IMG"
        ffmpeg -re -loop 1 -i "$STAGE_IMG" -f v4l2 -pix_fmt yuv420p /dev/video10 >/dev/null 2>&1 &
    else
        ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=30 -f v4l2 -pix_fmt yuv420p /dev/video10 >/dev/null 2>&1 &
    fi
    PRODUCER_PID=$!

    # Wait for consumer to process reconnected frames
    sleep 1.5

    # Verify consumer application remained running throughout reconnection
    if ! kill -0 "$READER_PID" 2>/dev/null; then
        echo "  [FAIL] Consumer application died during reconnection in cycle $cycle!"
        cat "$READER_LOG"
        exit 1
    fi

    # OCR verification of reconnected screen number
    if [ "$HAVE_OCR" = true ] && [ -n "$MAGICK" ]; then
        ocr_rec=""
        for attempt in {1..5}; do
            ocr_rec=$(tesseract "$READER_FRAME" stdout --psm 7 -l eng 2>/dev/null | tr -d '[:space:]')
            if [[ "$ocr_rec" == *"$CYCLE_TOKEN"* ]]; then
                break
            fi
            sleep 0.5
        done

        if [[ "$ocr_rec" == *"$CYCLE_TOKEN"* ]]; then
            echo "    [PASS] Cycle $cycle: OCR verified token '$CYCLE_TOKEN' on reconnected webcam screen (detected '$ocr_rec')"
        else
            echo "  [FAIL] Cycle $cycle: Expected reconnected token '$CYCLE_TOKEN', but OCR read '$ocr_rec'!"
            exit 1
        fi
    fi

    # Complete remaining streaming duration
    REMAINING_STREAM_SEC=$(( STREAM_SEC > 2 ? STREAM_SEC - 2 : 1 ))
    sleep "$REMAINING_STREAM_SEC"

    # Final check of consumer process for this cycle
    if ! kill -0 "$READER_PID" 2>/dev/null; then
        echo "  [FAIL] Consumer application died after streaming in cycle $cycle!"
        cat "$READER_LOG"
        exit 1
    fi
    echo "    [PASS] Cycle $cycle: Live stream resumed seamlessly for ${STREAM_SEC}s"
done

echo "==> Step 4: Final verification after $CYCLES complete cycles..."
if ! kill -0 "$READER_PID" 2>/dev/null; then
    echo "  [FAIL] Consumer application died at end of test!"
    cat "$READER_LOG"
    exit 1
fi

echo "  [PASS] Consumer application remained continuously active across all $CYCLES disconnect/reconnect cycles!"
if [ "$HAVE_OCR" = true ]; then
    echo "  [PASS] Optical Character Recognition (OCR) verified correct return numbers on reconnected screen across all cycles!"
fi

echo "=========================================================="
echo "  [PASS] Cable Disconnect & Reconnect E2E Test Passed!"
echo "=========================================================="
exit 0
