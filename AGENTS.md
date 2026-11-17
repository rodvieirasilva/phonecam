# AGENTS.md — Contributor & AI Agent Guidelines for PhoneCam

Welcome! This document provides architectural invariants, engineering standards, and developer instructions for AI agents and human contributors working on the **PhoneCam** repository.

---

## 1. Project Overview & Mission

**PhoneCam** is a high-performance smartphone webcam virtual device manager built specifically for **Omarchy Linux** (Arch Linux / Wayland).

- **Core Goal**: Expose any Android smartphone connected via USB as a zero-latency, studio-quality UVC webcam (`/dev/video10`) for all Linux applications (OBS, Discord, Zoom, Meet, Firefox, Chromium).
- **Core Technology**: Rust 2024 edition, `scrcpy 5.x`, `v4l2loopback`, and `Qt 6 Quick / QML`.
- **Interface**: Dual-mode UI:
  1. Modern **Qt Quick (QML)** desktop window with Omarchy dark theme and live accent color.
  2. Interactive **Terminal TUI** dashboard with auto-detachment on **ANY KEY** press.
- **Transformations**: Live switching between Front/Back cameras, Cut (Crop presets: 16:9, 4:3, 1:1, full), Resize (1080p, 720p, 4K), Invert (color negative filter), and Flip (horizontal mirror & vertical).

---

## 2. Architectural Invariants

Agents modifying or extending PhoneCam must adhere to the following core invariants:

1. **Clean Hardware Teardown**:
   - The virtual device `/dev/video10` must NEVER be left orphaned or in an unreadable state.
   - Any exit path (closing GUI window, pressing Escape/q in TUI, `SIGINT`, `SIGTERM`, panic) must invoke `StreamManager::stop()` and `detach_loopback_device()`.
2. **Device State Resilience**:
   - The phone might be unplugged, unauthorized, screen-locked, or in charging-only mode at any point.
   - The application must never panic on missing ADB devices; it must display clear, actionable guidance to the user.
3. **Privilege Boundary Separation**:
   - The application itself runs as an unprivileged user (`rvs`).
   - Kernel module loading (`modprobe v4l2loopback`) and device node permissions (`chmod 666 /dev/video*`) are handled via passwordless sudoers rules (`/etc/sudoers.d/phonecam-v4l2loopback`) or `~/.credentials` fallback.
   - Never run the GUI or scrcpy as root.
4. **Theme & Desktop Conformance**:
   - Always follow the Omarchy design guidelines: dark backgrounds (`#18181B`, `#27272A`), rounded corners (12px), and dynamic accent colors from `~/.local/state/omarchy/current/theme/colors.toml`.

---

## 3. Repository Layout & Component Responsibilities

```text
phonecam/
├── Cargo.toml                # Rust crate dependencies and build profiles
├── assets/
│   ├── phonecam.svg          # Primary vector logo
│   └── phonecam.png          # 512x512 rendered raster icon
├── pkgbuild/
│   ├── PKGBUILD              # Arch Linux package specification for Omarchy
│   └── phonecam.desktop      # XDG Desktop application launcher
├── scripts/
│   ├── install.sh            # Complete local build & system installation
│   └── uninstall.sh          # System uninstaller
├── src/
│   ├── main.rs               # CLI argument parser (Clap), mode dispatcher
│   ├── config.rs             # Configuration models, scrcpy flag generators
│   ├── adb.rs                # ADB device detection, status polling, battery
│   ├── v4l2.rs               # Kernel module setup, permissions, teardown
│   ├── pipeline.rs           # scrcpy/ffmpeg stream lifecycle, filters, PIDs
│   ├── server.rs             # Internal HTTP REST IPC for QML frontend
│   ├── tui.rs                # Crossterm terminal UI with any-key auto-detach
│   └── util.rs               # Privilege escalation & Omarchy theme detection
├── ui/
│   └── Main.qml              # Qt Quick ApplicationWindow layout & controls
├── README.md                 # User-facing documentation
├── AGENTS.md                 # Contributor & Agent instructions (this file)
└── LICENSE                   # MIT License
```

---

## 4. Engineering Standards

### 4.1 Rust Guidelines
- **Edition**: Rust 2024.
- **Error Handling**: Use `anyhow::Result` for application-level logic with descriptive `.context(...)`. Use `thiserror` for typed errors when programmatic branching is required.
- **Async Runtime**: `tokio` for background tasks, HTTP server, and IPC.
- **Terminal Handling**: `crossterm` in raw mode with `event::poll` and `event::read` for non-blocking key detection.
- **Process Management**: Store `std::process::Child` handles and ensure both stdout/stderr are logged to `/tmp/phonecam.log` or piped to filters.

### 4.2 Qt Quick (QML) Guidelines
- Built for `qml6` runtime (`Qt 6.12+`).
- Use `XMLHttpRequest` to communicate with the internal REST API on `127.0.0.1:41789`.
- Ensure keyboard shortcuts (`Escape`, `q`, `Space`) are active application-wide.
- Always poll `/api/status` periodically (2000ms) to detect phone plug/unplug events dynamically.

---

## 5. Development & Verification Commands

### Build & Check
```bash
# Verify compilation without warnings
cargo check

# Compile debug build
cargo build

# Compile optimized release binary
cargo build --release
```

### Run Locally
```bash
# Run GUI mode
cargo run

# Run Terminal TUI dashboard
cargo run -- --tui

# Inspect phone cameras
cargo run -- --list-cameras

# Inspect status
cargo run -- --status
```

### System Installation & Packaging
```bash
# Run installer (installs binary, desktop file, icon, sudoers rule, omarchy menu)
./scripts/install.sh

# Verify Arch PKGBUILD
cd pkgbuild && makepkg -fsi
```

---

## 6. Omarchy Menu & ASUS Provisioning Integration

PhoneCam is integrated into the official Omarchy setup:
- **Menu Shortcut**: Defined in `~/.config/omarchy/extensions/omarchy-menu.jsonc` under IDs `"phonecam"` and `"phonecam-tui"`.
- **System Provisioning**: Registered in `/home/rvs/dev/scripts/setup-asus.sh` so full machine re-provisioning automatically builds and installs PhoneCam.
