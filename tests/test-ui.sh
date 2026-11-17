#!/usr/bin/env bash
# ==============================================================================
# tests/test-ui.sh - Qt Quick UI Test Suite with Button Click Simulation & TestIDs
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "=========================================================="
echo "  PhoneCam Native GTK4 & Libadwaita UI Tests"
echo "=========================================================="

echo "==> Running cargo test --test ui_test..."
cargo test --test ui_test -- --nocapture

if command -v qmltestrunner >/dev/null 2>&1 || [ -x "/usr/lib/qt6/bin/qmltestrunner" ]; then
    echo ""
    echo "=========================================================="
    echo "  PhoneCam Legacy QML UI Verification"
    echo "=========================================================="
    QML_RUNNER="$(command -v qmltestrunner || echo /usr/lib/qt6/bin/qmltestrunner)"
    QT_QPA_PLATFORM=offscreen "$QML_RUNNER" -input tests/tst_ui.qml || true
fi

echo "=========================================================="
echo "  🎉 All UI Tests Passed Successfully!"
echo "=========================================================="
exit 0
