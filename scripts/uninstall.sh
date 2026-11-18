#!/usr/bin/env bash
# ==============================================================================
# scripts/uninstall.sh - Clean uninstallation of PhoneCam
# Uses native `phonecam uninstall` installer (matching SoundThis architecture)
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

if command -v phonecam >/dev/null 2>&1; then
    exec phonecam uninstall "$@"
elif [ -f "target/release/phonecam" ]; then
    exec target/release/phonecam uninstall "$@"
else
    exec cargo run --release -- uninstall "$@"
fi
