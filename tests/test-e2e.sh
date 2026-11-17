#!/usr/bin/env bash
# ==============================================================================
# tests/test-e2e.sh - End-to-End System, CLI & Package Lifecycle Verification
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "=========================================================="
echo "  PhoneCam End-to-End (E2E) Test Suite"
echo "=========================================================="

FAILED=0

pass() {
    echo "  [PASS] $1"
}

fail() {
    echo "  [FAIL] $1"
    FAILED=$((FAILED + 1))
}

# 1. Binary checks
echo "==> Step 1: Testing CLI interface..."
if cargo run -- --help >/dev/null 2>&1; then
    pass "CLI --help executed successfully"
else
    fail "CLI --help failed"
fi

VERSION_OUT=$(cargo run -- --version 2>/dev/null || true)
if echo "$VERSION_OUT" | grep -q "phonecam 0.1.0"; then
    pass "CLI --version reports 'phonecam 0.1.0'"
else
    fail "CLI --version mismatch: $VERSION_OUT"
fi

STATUS_OUT=$(cargo run -- --status 2>/dev/null || true)
if echo "$STATUS_OUT" | grep -q "Checking device connection"; then
    pass "CLI --status checked device connection cleanly"
else
    fail "CLI --status check failed: $STATUS_OUT"
fi

# 2. Desktop file validation
echo "==> Step 2: Testing Desktop Entry..."
if command -v desktop-file-validate >/dev/null 2>&1; then
    if desktop-file-validate pkgbuild/phonecam.desktop >/dev/null 2>&1; then
        pass "pkgbuild/phonecam.desktop passes desktop-file-validate"
    else
        fail "pkgbuild/phonecam.desktop failed validation"
    fi
else
    pass "desktop-file-validate not available (skipped)"
fi

# 3. Icon Asset validation
echo "==> Step 3: Testing Logo & Icon Assets..."
if [[ -f "assets/phonecam.svg" && -s "assets/phonecam.svg" ]]; then
    pass "assets/phonecam.svg exists and is non-empty"
else
    fail "assets/phonecam.svg missing"
fi

if [[ -f "assets/phonecam.png" && -s "assets/phonecam.png" ]]; then
    pass "assets/phonecam.png exists and is non-empty"
else
    fail "assets/phonecam.png missing"
fi

# 4. QML syntax validation
echo "==> Step 4: Testing QML UI Syntax..."
if command -v qml6 >/dev/null 2>&1; then
    # Test loading QML file non-interactively
    python3 -c '
import subprocess, sys

cmd = ["qml6", "ui/Main.qml"]
try:
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        p.wait(timeout=2)
    except subprocess.TimeoutExpired:
        # If it stayed open 2s without crashing with syntax error, it loaded properly
        p.terminate()
        p.kill()
        sys.exit(0)
    if p.returncode == 0:
        sys.exit(0)
    else:
        print("QML stderr:", p.stderr.read())
        sys.exit(1)
except Exception as e:
    print("Error:", e)
    sys.exit(1)
'
    pass "ui/Main.qml loaded cleanly in qml6 runtime"
else
    pass "qml6 runtime not available (skipped)"
fi

# 4b. Qt Quick UI Automated Click & TestID Test Suite
echo "==> Step 4b: Testing QML UI Button Clicks & TestIDs..."
if ./tests/test-ui.sh; then
    pass "Qt Quick UI button click simulation & testIDs passed"
else
    fail "Qt Quick UI button click simulation failed"
fi

# 5. Package structure verification
echo "==> Step 5: Testing Repository & Documentation Structure..."
REQUIRED_FILES=(
    "Cargo.toml"
    "LICENSE"
    "README.md"
    "AGENTS.md"
    "pkgbuild/PKGBUILD"
    "pkgbuild/phonecam.desktop"
    "scripts/install.sh"
    "scripts/uninstall.sh"
    "src/lib.rs"
    "src/main.rs"
    "ui/Main.qml"
    "tests/tst_ui.qml"
    "tests/test-ui.sh"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [[ -f "$file" ]]; then
        pass "Found $file"
    else
        fail "Missing required file: $file"
    fi
done

# 6. Sudoers rule syntax verification
echo "==> Step 6: Testing Sudoers syntax..."
TMP_SUDOERS=$(mktemp)
echo "ALL ALL=(ALL) NOPASSWD: /usr/bin/modprobe v4l2loopback *, /usr/bin/rmmod v4l2loopback, /usr/bin/chmod 666 /dev/video*" > "$TMP_SUDOERS"
if command -v visudo >/dev/null 2>&1; then
    if visudo -c -f "$TMP_SUDOERS" >/dev/null 2>&1; then
        pass "Sudoers syntax is valid"
    else
        fail "Sudoers syntax error"
    fi
fi
rm -f "$TMP_SUDOERS"

echo "=========================================================="
if [[ $FAILED -eq 0 ]]; then
    echo "  🎉 All E2E Tests Passed Successfully!"
    echo "=========================================================="
    exit 0
else
    echo "  ❌ $FAILED test(s) failed."
    echo "=========================================================="
    exit 1
fi
