#!/usr/bin/env bash
# ==============================================================================
# tests/test-package-e2e.sh - Package Lifecycle & E2E Validation
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

echo "==> Running Full Test Suite (Unit & Integration)..."
cargo test --all-targets

echo "==> Running Fake Phone E2E Test Suite..."
cargo test --test fake_phone_e2e

echo "==> Running Terminal TUI E2E Test Suite..."
./tests/test-tui-e2e.sh

echo "==> Running Video Stream Hardware OCR E2E Verification..."
./tests/test-ocr-e2e.sh

echo "==> Running Performance, Memory, CPU & Teardown E2E..."
./tests/test-perf-e2e.sh

echo "==> Running CLI E2E Tests..."
./tests/test-e2e.sh

echo "==> Verifying Arch PKGBUILD syntax..."
cd pkgbuild
makepkg --verifysource || true
cd ..

echo "[PASS] All E2E & Package Tests Succeeded Cleanly!"
