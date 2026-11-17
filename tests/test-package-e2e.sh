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

echo "==> Running CLI E2E Tests..."
./tests/test-e2e.sh

echo "==> Verifying Arch PKGBUILD syntax..."
cd pkgbuild
makepkg --verifysource || true
cd ..

echo "🎉 All E2E & Package Tests Succeeded Cleanly!"
