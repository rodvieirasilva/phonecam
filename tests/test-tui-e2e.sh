#!/usr/bin/env bash
# ==============================================================================
# tests/test-tui-e2e.sh - TUI End-to-End Simulation & Auto-Detachment Test
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "=========================================================="
echo "  PhoneCam Terminal Dashboard (TUI) E2E Test"
echo "=========================================================="

# Ensure /dev/video10 is ready and writable if possible
if [ ! -c /dev/video10 ]; then
    sudo -n modprobe v4l2loopback video_nr=10 card_label="PhoneCam Virtual Camera" exclusive_caps=1 2>/dev/null || true
    sudo -n chmod 666 /dev/video10 2>/dev/null || true
fi

if [ ! -c /dev/video10 ]; then
    echo "[SKIP] /dev/video10 loopback device node is not available in this environment."
    exit 0
fi

# Determine binary path
if [ -f "target/release/phonecam" ]; then
    PHONECAM_BIN="target/release/phonecam"
elif [ -f "target/debug/phonecam" ]; then
    PHONECAM_BIN="target/debug/phonecam"
else
    cargo build
    PHONECAM_BIN="target/debug/phonecam"
fi
export PHONECAM_BIN

TMP_DIR=$(mktemp -d /tmp/phonecam_tui_test_XXXXXX)
MOCK_BIN_DIR="$TMP_DIR/bin"
mkdir -p "$MOCK_BIN_DIR"

cleanup() {
    killall -q phonecam 2>/dev/null || true
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# 1. Mock adb
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

# 2. Mock scrcpy (keeps running until killed by PhoneCam)
cat << 'EOF' > "$MOCK_BIN_DIR/scrcpy"
#!/usr/bin/env bash
trap "exit 0" SIGTERM SIGINT
while true; do
    sleep 0.1
done
EOF
chmod +x "$MOCK_BIN_DIR/scrcpy"

# Prepend mock bin dir to PATH
export PATH="$MOCK_BIN_DIR:$PATH"

python3 -c '
import pty, os, time, subprocess, sys

bin_path = os.environ.get("PHONECAM_BIN", "target/release/phonecam")
master, slave = pty.openpty()
p = subprocess.Popen([bin_path, "--tui"], stdin=slave, stdout=slave, stderr=slave)
os.close(slave)

time.sleep(1.8)

# Send "q" keypress to trigger auto-detachment on ANY KEY
os.write(master, b"q")

output = b""
for _ in range(25):
    try:
        data = os.read(master, 2048)
        if not data:
            break
        output += data
    except OSError:
        break
    time.sleep(0.1)

os.close(master)
try:
    p.wait(timeout=4)
except subprocess.TimeoutExpired:
    p.kill()
    sys.exit(1)

out_text = output.decode("utf-8", errors="ignore")
print("Process exit code:", p.returncode)

if "PHONECAM VIRTUAL WEBCAM" not in out_text:
    print("[FAIL] Missing TUI banner in terminal output", file=sys.stderr)
    print("Output received:\n", out_text, file=sys.stderr)
    sys.exit(1)

print("  [PASS] TUI dashboard rendered cleanly and auto-detached on keypress!")
'

echo "=========================================================="
echo "  [PASS] TUI E2E Test Passed Successfully!"
echo "=========================================================="
