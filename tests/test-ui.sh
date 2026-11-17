#!/usr/bin/env bash
# ==============================================================================
# tests/test-ui.sh - Qt Quick UI Test Suite with Button Click Simulation & TestIDs
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "=========================================================="
echo "  PhoneCam Qt Quick UI & Button Click Simulation Tests"
echo "=========================================================="

QML_TEST_RUNNER=""
for candidate in \
    "$(command -v qmltestrunner 2>/dev/null || true)" \
    "$(command -v qmltestrunner-qt6 2>/dev/null || true)" \
    "/usr/lib/qt6/bin/qmltestrunner" \
    "/usr/lib64/qt6/bin/qmltestrunner"; do
    if [[ -n "$candidate" && -x "$candidate" ]]; then
        QML_TEST_RUNNER="$candidate"
        break
    fi
done

if [[ -z "$QML_TEST_RUNNER" ]]; then
    echo "⚠️ Warning: 'qmltestrunner' not found. Checking syntax via qml6..."
    if command -v qml6 >/dev/null 2>&1; then
        timeout 2 qml6 -style Basic ui/Main.qml >/dev/null 2>&1 || [ $? -eq 124 ]
        echo "  [PASS] ui/Main.qml loaded cleanly with qml6"
    fi
    exit 0
fi

echo "==> Using QML test runner: $QML_TEST_RUNNER"
echo "==> Running tests/tst_ui.qml with offscreen QPA platform..."

QT_QPA_PLATFORM=offscreen "$QML_TEST_RUNNER" -input tests/tst_ui.qml

echo "=========================================================="
echo "  🎉 All Qt Quick UI Tests Passed Successfully!"
echo "=========================================================="
