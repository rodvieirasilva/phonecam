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

python3 -c '
import pty, os, time, subprocess, sys

master, slave = pty.openpty()
p = subprocess.Popen(["target/release/phonecam", "--tui"], stdin=slave, stdout=slave, stderr=slave)
os.close(slave)

time.sleep(1.8)

# Send "q" keypress to trigger auto-detachment on ANY KEY
os.write(master, b"q")

output = b""
for _ in range(15):
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
    sys.exit(1)

if "Output:" not in out_text and "/dev/video10" not in out_text:
    print("[FAIL] Missing device /dev/video10 in output", file=sys.stderr)
    sys.exit(1)

print("  [PASS] TUI dashboard rendered cleanly and auto-detached on keypress!")
sys.exit(p.returncode)
'

echo "=========================================================="
echo "  [PASS] TUI E2E Test Passed Successfully!"
echo "=========================================================="
