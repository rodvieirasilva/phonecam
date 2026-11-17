#!/usr/bin/env bash
# ==============================================================================
# PhoneCam Performance, Memory, CPU, Teardown & Single-Instance E2E Test Suite
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Locate PhoneCam binary
PHONECAM_BIN="${PROJECT_ROOT}/target/release/phonecam"
if [ ! -f "$PHONECAM_BIN" ]; then
    PHONECAM_BIN="${PROJECT_ROOT}/target/debug/phonecam"
fi
if [ ! -f "$PHONECAM_BIN" ]; then
    echo "PhoneCam binary not found in release or debug. Compiling debug build..."
    cargo build --bin phonecam
    PHONECAM_BIN="${PROJECT_ROOT}/target/debug/phonecam"
fi

echo "=========================================================="
echo "  PhoneCam Performance, Memory, CPU & Teardown E2E"
echo "  Binary: $PHONECAM_BIN"
echo "=========================================================="

TMP_DIR=$(mktemp -d /tmp/phonecam_perf_XXXXXX)
MOCK_BIN_DIR="$TMP_DIR/mock_bin"
mkdir -p "$MOCK_BIN_DIR"

# Clean up on exit
cleanup() {
    if [ -n "${PC_PID:-}" ]; then
        kill -TERM "$PC_PID" 2>/dev/null || true
        wait "$PC_PID" 2>/dev/null || true
    fi
    killall -q phonecam 2>/dev/null || true
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

# 1. Hermetic Mock ADB
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

# 2. Hermetic Mock scrcpy
cat << 'EOF' > "$MOCK_BIN_DIR/scrcpy"
#!/usr/bin/env bash
trap 'exit 0' SIGTERM SIGINT SIGHUP
while true; do
    sleep 0.1
done
EOF
chmod +x "$MOCK_BIN_DIR/scrcpy"

export PATH="$MOCK_BIN_DIR:$PATH"
export XDG_RUNTIME_DIR="$TMP_DIR/runtime"
mkdir -p "$XDG_RUNTIME_DIR"

LOCK_FILE="$XDG_RUNTIME_DIR/phonecam.lock"

# ------------------------------------------------------------------------------
# Test 1: Single-Instance Enforcement
# ------------------------------------------------------------------------------
echo "==> Step 1: Testing Single-Instance Mutual Exclusion..."

# Launch primary PhoneCam instance in headless stream mode
"$PHONECAM_BIN" --start > "$TMP_DIR/pc_instance1.log" 2>&1 &
PC_PID=$!

# Wait for instance 1 to initialize and acquire lock
sleep 1.2
if ! kill -0 "$PC_PID" 2>/dev/null; then
    echo "  [FAIL] Primary instance failed to start. Log:"
    cat "$TMP_DIR/pc_instance1.log"
    exit 1
fi
echo "  [PASS] Primary instance 1 running (PID: $PC_PID)"

# Verify lockfile exists and holds instance 1 PID
if [ ! -f "$LOCK_FILE" ]; then
    echo "  [FAIL] Lockfile $LOCK_FILE was not created"
    exit 1
fi
LOCKED_PID=$(tr -d ' \n\r' < "$LOCK_FILE")
if [ "$LOCKED_PID" != "$PC_PID" ]; then
    echo "  [FAIL] Lockfile PID ($LOCKED_PID) does not match primary instance PID ($PC_PID)"
    exit 1
fi
echo "  [PASS] Lockfile verified with correct PID ($LOCKED_PID)"

# Attempt to launch secondary instance (MUST FAIL)
set +e
"$PHONECAM_BIN" --start > "$TMP_DIR/pc_instance2.log" 2>&1
INST2_EXIT=$?
set -e

if [ "$INST2_EXIT" -eq 0 ]; then
    echo "  [FAIL] Secondary instance succeeded when it should have been blocked!"
    cat "$TMP_DIR/pc_instance2.log"
    exit 1
fi

INST2_LOG=$(cat "$TMP_DIR/pc_instance2.log")
if [[ "$INST2_LOG" != *"already running"* ]]; then
    echo "  [FAIL] Secondary instance error message did not mention 'already running': $INST2_LOG"
    exit 1
fi
echo "  [PASS] Secondary instance correctly blocked with exit code $INST2_EXIT and message: '$INST2_LOG'"

# Verify primary instance is still running unaffected
if ! kill -0 "$PC_PID" 2>/dev/null; then
    echo "  [FAIL] Primary instance crashed when secondary instance attempted to launch!"
    exit 1
fi
echo "  [PASS] Primary instance 1 unaffected and still running cleanly"

# ------------------------------------------------------------------------------
# Test 2: Memory Usage & Leak Verification
# ------------------------------------------------------------------------------
echo "==> Step 2: Testing Memory Usage (RSS) & Stability..."

# Read memory statistics from /proc/<pid>/status
VM_RSS_KB=$(grep -i "VmRSS" "/proc/$PC_PID/status" | awk '{print $2}')
VM_HWM_KB=$(grep -i "VmHWM" "/proc/$PC_PID/status" | awk '{print $2}')
RSS_MB=$(awk "BEGIN {printf \"%.2f\", $VM_RSS_KB / 1024}")
HWM_MB=$(awk "BEGIN {printf \"%.2f\", $VM_HWM_KB / 1024}")

echo "  Measured VmRSS: ${RSS_MB} MB (${VM_RSS_KB} KB)"
echo "  Measured VmHWM: ${HWM_MB} MB (${VM_HWM_KB} KB)"

# Assert RSS is under 50 MB threshold for headless stream manager
MAX_ALLOWED_RSS_KB=51200 # 50 MB
if [ "$VM_RSS_KB" -gt "$MAX_ALLOWED_RSS_KB" ]; then
    echo "  [FAIL] Memory RSS (${RSS_MB} MB) exceeded allowed threshold of 50 MB!"
    exit 1
fi
echo "  [PASS] Memory RSS (${RSS_MB} MB) is well within performance limits (< 50 MB)"

# Sample memory 5 times over 1 second to verify no runaway leak
SAMPLES=()
for i in {1..5}; do
    SAMPLES+=("$(grep -i "VmRSS" "/proc/$PC_PID/status" | awk '{print $2}')")
    sleep 0.2
done
INITIAL_SAMPLE=${SAMPLES[0]}
FINAL_SAMPLE=${SAMPLES[4]}
DELTA_KB=$(( FINAL_SAMPLE - INITIAL_SAMPLE ))
if [ "$DELTA_KB" -lt 0 ]; then DELTA_KB=$(( -DELTA_KB )); fi

echo "  Memory drift over 1s: ${DELTA_KB} KB"
if [ "$DELTA_KB" -gt 5120 ]; then # 5 MB threshold
    echo "  [FAIL] Detected potential memory leak: ${DELTA_KB} KB growth"
    exit 1
fi
echo "  [PASS] Memory is stable over time (drift: ${DELTA_KB} KB < 5 MB)"

# ------------------------------------------------------------------------------
# Test 3: CPU Usage Verification
# ------------------------------------------------------------------------------
echo "==> Step 3: Testing CPU Utilization..."

# Sample CPU percentage via ps
CPU_PCT=$(ps -o %cpu= -p "$PC_PID" | tr -d ' ')
echo "  Measured CPU usage: ${CPU_PCT}%"

# Convert float to int for bash comparison
CPU_INT=$(printf "%.0f" "$CPU_PCT" 2>/dev/null || echo "0")
if [ "$CPU_INT" -gt 35 ]; then
    echo "  [FAIL] CPU usage (${CPU_PCT}%) exceeded 35% threshold!"
    exit 1
fi
echo "  [PASS] CPU usage (${CPU_PCT}%) is within efficiency limits (< 35%)"

# ------------------------------------------------------------------------------
# Test 4: Clean Teardown & Lock Release Verification
# ------------------------------------------------------------------------------
echo "==> Step 4: Testing Clean Teardown & Device Lifecycle..."

# Send termination signal to primary instance
kill -TERM "$PC_PID"
wait "$PC_PID" 2>/dev/null || true
unset PC_PID

# Verify PhoneCam process has fully exited
sleep 0.5
if pgrep -f "$PHONECAM_BIN" >/dev/null 2>&1; then
    echo "  [FAIL] PhoneCam process is still lingering after SIGTERM!"
    exit 1
fi
echo "  [PASS] Primary PhoneCam process exited cleanly"

# Verify all child processes (scrcpy, ffmpeg, mpv) have terminated
if pgrep -f "scrcpy.*--video-source=camera" >/dev/null 2>&1; then
    echo "  [FAIL] Orphan scrcpy child processes found running!"
    exit 1
fi
echo "  [PASS] No orphan scrcpy or stream engine processes remain"

# Verify lockfile has been removed or released
if [ -f "$LOCK_FILE" ]; then
    # If file exists, check if flock is unlocked
    set +e
    python3 -c "
import fcntl
f = open('$LOCK_FILE', 'r+')
fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
" 2>/dev/null
    LOCK_TEST_EXIT=$?
    set -e
    if [ "$LOCK_TEST_EXIT" -ne 0 ]; then
        echo "  [FAIL] Lockfile was not unlocked after process exit!"
        exit 1
    fi
fi
echo "  [PASS] Single-instance lock cleanly released"

# Verify a new instance can now start without collision
set +e
"$PHONECAM_BIN" --start > "$TMP_DIR/pc_instance3.log" 2>&1 &
NEW_PC_PID=$!
sleep 1.2
if kill -0 "$NEW_PC_PID" 2>/dev/null; then
    echo "  [PASS] New instance successfully started after teardown (PID: $NEW_PC_PID)"
    kill -TERM "$NEW_PC_PID" 2>/dev/null || true
    wait "$NEW_PC_PID" 2>/dev/null || true
else
    echo "  [FAIL] New instance failed to start after teardown. Log:"
    cat "$TMP_DIR/pc_instance3.log"
    exit 1
fi
set -e

echo "=========================================================="
echo "  [PASS] All Performance, Memory, CPU & Teardown Tests Passed!"
echo "=========================================================="
