#!/usr/bin/env bash
# ==============================================================================
# tests/test-ocr-e2e.sh - Optical Character Recognition (OCR) Stream Verification
# ==============================================================================
# Tests PhoneCam end-to-end across multiple hardware/software transformations:
# 1. Standard orientation (phonecam --start)
# 2. Horizontal mirror (phonecam --start --mirror)
# 3. 90° clockwise rotation (phonecam --start --rotation 90)
# 4. 180° inverted mount (phonecam --start --rotation 180)
# 5. Inverted colors (phonecam --start --invert)
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "=========================================================="
echo "  PhoneCam Hardware OCR Video Stream Verification"
echo "=========================================================="

# Check prerequisites
if command -v magick >/dev/null 2>&1; then
    MAGICK="magick"
elif command -v convert >/dev/null 2>&1; then
    MAGICK="convert"
else
    echo "[SKIP] ImageMagick ('magick' or 'convert') is required for OCR test suite."
    exit 0
fi

for cmd in ffmpeg tesseract; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "[SKIP] '$cmd' is required for OCR test suite."
        exit 0
    fi
done

# Determine binary path
if [ -f "target/release/phonecam" ]; then
    PHONECAM_BIN="$SCRIPT_DIR/target/release/phonecam"
elif [ -f "target/debug/phonecam" ]; then
    PHONECAM_BIN="$SCRIPT_DIR/target/debug/phonecam"
else
    echo "==> Compiling PhoneCam binary..."
    cargo build
    PHONECAM_BIN="$SCRIPT_DIR/target/debug/phonecam"
fi

# Ensure /dev/video10 is ready and writable
if [ ! -c /dev/video10 ]; then
    echo "==> Setting up /dev/video10 loopback device..."
    sudo -n modprobe v4l2loopback video_nr=10 card_label="PhoneCam Virtual Camera" exclusive_caps=1 2>/dev/null || true
    sudo -n chmod 666 /dev/video10 2>/dev/null || true
fi

if [ ! -c /dev/video10 ]; then
    echo "[SKIP] /dev/video10 loopback device node is not available in this environment."
    exit 0
fi

TMP_DIR=$(mktemp -d /tmp/phonecam_ocr_test_XXXXXX)
MOCK_BIN_DIR="$TMP_DIR/bin"
mkdir -p "$MOCK_BIN_DIR"
export XDG_RUNTIME_DIR="$TMP_DIR/runtime"
mkdir -p "$XDG_RUNTIME_DIR"

cleanup() {
    # Kill any dangling phonecam or ffmpeg processes
    killall -q phonecam 2>/dev/null || true
    fuser -k /dev/video10 2>/dev/null || true
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

EXPECTED_TOKEN="7419"
BASE_IMG="$TMP_DIR/base_token.png"
CAPTURED_FRAME="$TMP_DIR/captured_frame.png"
REVERSED_FRAME="$TMP_DIR/reversed_frame.png"

# Locate a standard scalable font for magick rendering (fallback to default)
FONT_ARG=()
for f in \
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" \
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf" \
    "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf" \
    "/usr/share/fonts/dejavu/DejaVuSans.ttf" \
    "/usr/share/fonts/TTF/DejaVuSans.ttf"; do
    if [ -f "$f" ]; then
        FONT_ARG=(-font "$f")
        break
    fi
done

# Generate high-contrast base image with test token
"$MAGICK" -size 640x480 xc:white -fill black "${FONT_ARG[@]}" -pointsize 72 -gravity center -annotate +0+0 "$EXPECTED_TOKEN" "$BASE_IMG"

# Create mock adb in MOCK_BIN_DIR
cat << 'EOF' > "$MOCK_BIN_DIR/adb"
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
chmod +x "$MOCK_BIN_DIR/adb"

# Create mock scrcpy in MOCK_BIN_DIR that acts as the phone camera source
# transforming the test pattern according to the scrcpy capture flags passed by PhoneCam
cat << EOF > "$MOCK_BIN_DIR/scrcpy"
#!/usr/bin/env bash
set -e

ORIENTATION="0"
SINK=""
IS_PIPE=false

for arg in "\$@"; do
    case "\$arg" in
        --capture-orientation=*)
            ORIENTATION="\${arg#*=}"
            ;;
        --v4l2-sink=*)
            SINK="\${arg#*=}"
            ;;
        -r)
            IS_PIPE=true
            ;;
    esac
done

SRC_IMG="$BASE_IMG"
STAGE_IMG="$TMP_DIR/stage_\$ORIENTATION.png"

case "\$ORIENTATION" in
    0)
        cp "\$SRC_IMG" "\$STAGE_IMG"
        ;;
    flip0)
        "$MAGICK" "\$SRC_IMG" -flop "\$STAGE_IMG"
        ;;
    90)
        "$MAGICK" "\$SRC_IMG" -rotate 90 "\$STAGE_IMG"
        ;;
    180)
        "$MAGICK" "\$SRC_IMG" -rotate 180 "\$STAGE_IMG"
        ;;
    270)
        "$MAGICK" "\$SRC_IMG" -rotate 270 "\$STAGE_IMG"
        ;;
    flip90)
        "$MAGICK" "\$SRC_IMG" -flop -rotate 90 "\$STAGE_IMG"
        ;;
    flip180)
        "$MAGICK" "\$SRC_IMG" -flop -rotate 180 "\$STAGE_IMG"
        ;;
    flip270)
        "$MAGICK" "\$SRC_IMG" -flop -rotate 270 "\$STAGE_IMG"
        ;;
    *)
        cp "\$SRC_IMG" "\$STAGE_IMG"
        ;;
esac

if [ "\$IS_PIPE" = true ]; then
    # Pipe MKV stream to stdout (PhoneCam reads this and pipes through ffmpeg negate)
    exec ffmpeg -nostdin -loglevel error -re -loop 1 -i "\$STAGE_IMG" -c:v libx264 -preset ultrafast -tune zerolatency -pix_fmt yuv420p -t 10 -f matroska -
else
    # Output directly to v4l2loopback sink device configured by PhoneCam
    exec ffmpeg -nostdin -loglevel error -re -loop 1 -i "\$STAGE_IMG" -t 10 -f v4l2 -pix_fmt yuv420p "\$SINK"
fi
EOF
chmod +x "$MOCK_BIN_DIR/scrcpy"

# Prepend mock bin dir to PATH so PhoneCam invokes our mock camera environment
export PATH="$MOCK_BIN_DIR:$PATH"

verify_phonecam_stream() {
    local test_name="$1"
    local post_process_filter="$2"
    shift 2
    local phonecam_args=("$@")

    echo "==> Testing: $test_name..."
    echo "    Command: $PHONECAM_BIN ${phonecam_args[*]}"

    fuser -k /dev/video10 2>/dev/null || true
    sleep 0.3

    if [ ! -c /dev/video10 ]; then
        sudo -n modprobe v4l2loopback video_nr=10 card_label="PhoneCam Virtual Camera" exclusive_caps=1 2>/dev/null || true
        sudo -n chmod 666 /dev/video10 2>/dev/null || true
    fi

    local log_file="$TMP_DIR/phonecam_${test_name// /_}.log"
    # Launch PhoneCam in background
    "$PHONECAM_BIN" "${phonecam_args[@]}" > "$log_file" 2>&1 &
    local pc_pid=$!

    # Allow stream to initialize and fill v4l2 buffer
    sleep 2.0

    local captured=false
    for _ in {1..20}; do
        rm -f "$CAPTURED_FRAME" "$REVERSED_FRAME"
        if ffmpeg -nostdin -loglevel error -y -f v4l2 -i /dev/video10 -frames:v 1 -update 1 "$CAPTURED_FRAME" >/dev/null 2>&1; then
            if [ -f "$CAPTURED_FRAME" ] && [ -s "$CAPTURED_FRAME" ]; then
                captured=true
                break
            fi
        fi
        sleep 0.3
    done

    # Terminate PhoneCam cleanly
    kill -TERM "$pc_pid" 2>/dev/null || true
    wait "$pc_pid" 2>/dev/null || true
    fuser -k /dev/video10 2>/dev/null || true

    if [ "$captured" != true ] || [ ! -f "$CAPTURED_FRAME" ] || [ ! -s "$CAPTURED_FRAME" ]; then
        echo "  [FAIL] Frame capture from /dev/video10 failed (empty or missing frame)"
        echo "--- PhoneCam Log ---"
        cat "$log_file" 2>/dev/null || true
        return 1
    fi

    # Apply inverse transform if needed
    if [ -n "$post_process_filter" ]; then
        "$MAGICK" "$CAPTURED_FRAME" $post_process_filter "$REVERSED_FRAME"
    else
        cp "$CAPTURED_FRAME" "$REVERSED_FRAME"
    fi

    # Run Tesseract OCR on reversed frame
    local ocr_result
    ocr_result=$((tesseract "$REVERSED_FRAME" stdout --psm 7 -l eng 2>/dev/null || true) | tr -d '[:space:]')

    if [[ "$ocr_result" == *"$EXPECTED_TOKEN"* ]]; then
        echo "  [PASS] OCR verified token '$EXPECTED_TOKEN' (detected '$ocr_result')"
    else
        echo "  [FAIL] Expected token '$EXPECTED_TOKEN' but OCR read '$ocr_result'"
        return 1
    fi
}

# 1. Standard Orientation via PhoneCam
verify_phonecam_stream "Standard Video Stream" "" --start

# 2. Horizontal Mirror (Flip-H) via PhoneCam
verify_phonecam_stream "Horizontal Mirror (Flip-H)" "-flop" --start --mirror

# 3. 90° Clockwise Rotation via PhoneCam
verify_phonecam_stream "90° Clockwise Rotation" "-rotate -90" --start --rotation 90

# 4. 180° Inverted Mount via PhoneCam
verify_phonecam_stream "180° Inverted Mount" "-rotate 180" --start --rotation 180

# 5. Invert Colors (Negative Filter) via PhoneCam
verify_phonecam_stream "Invert Colors (Negative Filter)" "-negate" --start --invert

echo "=========================================================="
echo "  [PASS] All PhoneCam OCR Video Stream Tests Passed!"
echo "=========================================================="
