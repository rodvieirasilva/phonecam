#!/usr/bin/env bash
# ==============================================================================
# scripts/install.sh - Install PhoneCam onto the local Omarchy Linux system
# Uses native `phonecam install` installer (matching SoundThis architecture)
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

if [ -f "target/release/phonecam" ]; then
    exec target/release/phonecam install "$@"
elif [ -f "phonecam" ]; then
    exec ./phonecam install "$@"
else
    echo "==> Building and installing PhoneCam via cargo..."
    exec cargo run --release -- install "$@"
fi
