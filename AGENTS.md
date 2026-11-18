# AGENTS.md — Contributor & AI Agent Guidelines for PhoneCam

Welcome! This document provides architectural invariants, engineering standards, and developer instructions for AI agents and human contributors working on the **PhoneCam** repository.

---

## 1. Project Overview & Mission

**PhoneCam** is a high-performance smartphone webcam virtual device manager built specifically for **Omarchy Linux** (Arch Linux / Wayland).

- **Core Goal**: Expose any Android smartphone connected via USB as a zero-latency, studio-quality UVC webcam (`/dev/video10`) for all Linux applications (OBS, Discord, Zoom, Meet, Firefox, Chromium).
- **Core Technology**: Rust 2024 edition, `scrcpy 5.x`, `v4l2loopback`, and `GTK4 / Libadwaita`.
- **Interface**: Dual-mode UI:
  1. Modern native **GTK4 / Libadwaita** desktop window with Omarchy dark theme and live accent color (same pattern as SoundThis and Lan-Mouse).
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
5. **Single-Instance Exclusion**:
   - Only one PhoneCam process (`phonecam`, `--tui`, `--start`) may hold the hardware lock (`$XDG_RUNTIME_DIR/phonecam.lock`) at a time.
   - Secondary attempts must fail cleanly with exit code 1 and activate the existing window.
6. **Live Version Indicators**:
   - Both GTK4 window title bar and TUI header dashboard must dynamically source their version string from `env!("CARGO_PKG_VERSION")`.

---

## 3. Repository Layout & Component Responsibilities

```text
phonecam/
├── Cargo.toml                # Rust crate dependencies and build profiles
├── config.example.toml       # Template configuration file with all supported options
├── assets/
│   ├── phonecam.svg          # Primary vector logo
│   ├── phonecam.png          # 512x512 rendered raster icon
│   ├── screenshot-gui.png    # Live GTK4 desktop screenshot
│   └── screenshot-tui.png    # Live terminal dashboard screenshot
├── pkgbuild/
│   ├── PKGBUILD              # Arch Linux package specification for Omarchy
│   └── phonecam.desktop      # XDG Desktop application launcher
├── scripts/
│   └── capture-screenshot.sh # Pixel-perfect Wayland screenshot utility
├── src/
│   ├── main.rs               # Entry point, mode dispatcher
│   ├── cli.rs                # CLI argument parser (Clap)
│   ├── config.rs             # Configuration models, scrcpy flag generators
│   ├── installer.rs          # Native user & system installer (SoundThis pattern)
│   ├── instance.rs           # Single-instance mutual exclusion lock (flock)
│   ├── adb.rs                # ADB device detection, status polling, battery
│   ├── v4l2.rs               # Kernel module setup, permissions, teardown
│   ├── pipeline.rs           # scrcpy/ffmpeg stream lifecycle, filters, PIDs
│   ├── server.rs             # Internal HTTP REST IPC for frontend
│   ├── tui.rs                # Crossterm terminal UI with any-key auto-detach
│   ├── ui/                   # Unified desktop UI module
│   │   ├── mod.rs            # Desktop UI entry points
│   │   ├── gtk_app.rs        # Adwaita application, widgets, theming & event loop
│   │   ├── theme.css         # GTK4 / Libadwaita Omarchy theme template
│   │   └── Main.qml          # Declarative UI layout specification
│   └── util.rs               # Privilege escalation & Omarchy theme detection
├── tests/
│   ├── adb_tests.rs          # ADB device & camera parsing tests
│   ├── cli_tests.rs          # CLI flag & parser tests
│   ├── config_tests.rs       # Config defaults & serialization tests
│   ├── fake_phone_e2e.rs     # Mock phone end-to-end integration tests
│   ├── installer_tests.rs    # Native installer lifecycle tests
│   ├── instance_tests.rs     # Single-instance lock unit & lifecycle tests
│   ├── pipeline_tests.rs     # Stream argument & lifecycle tests
│   ├── server_tests.rs       # Axum REST API endpoint tests
│   ├── ui_test.rs            # GTK4 / Libadwaita UI & widget click simulation tests
│   ├── util_tests.rs         # Theme & environment utility tests
│   ├── v4l2_tests.rs         # Device node & module tests
│   ├── tst_ui.qml            # Qt Quick UI TestSuite with simulated button clicks & testIDs
│   ├── test-ui.sh            # UI test runner (GTK4 & QML)
│   ├── test-tui-e2e.sh       # Terminal TUI dashboard PTY & keypress E2E test
│   ├── test-ocr-e2e.sh       # Synthetic camera stream transformations OCR E2E verification
│   ├── test-perf-e2e.sh      # Memory usage (RSS), CPU benchmarks, and teardown E2E tests
│   ├── test-browser-e2e.sh   # Headless Brave/Chromium WebRTC video capture E2E test
│   ├── test-reconnect-e2e.sh # Cable disconnect & auto-reconnect fault-tolerance E2E test with OCR verification
│   ├── webcam-test.html      # Interactive WebRTC device verification test page
│   ├── reconnect_tests.rs    # Fault tolerance & reconnection unit tests
│   ├── test-e2e.sh           # CLI, desktop & asset E2E script
│   ├── test-package-e2e.sh   # Full package lifecycle test runner
│   └── test-android-emulator.sh # Android emulator / live hardware test script
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

### 4.2 Native GTK4 / Libadwaita Guidelines
- Built with `gtk4` and `libadwaita` (`adw`).
- Communicates with the internal REST API on `127.0.0.1:41789`.
- Dynamic theme synchronizes with Omarchy colors via inotify monitoring of `~/.local/state/omarchy/current/theme/colors.toml`.
- Keyboard shortcuts (`Escape`, `q`, `Space`) are active application-wide.
- Automatic polling periodically (1500ms) detects phone plug/unplug events dynamically.
- **UI Testability & TestIDs**:
  - Assign deterministic `widget_name` test IDs to every interactive control:
    - `"btn-camera-rear"`, `"btn-camera-front"`: Camera facing buttons
    - `"combo-camera-lens"`: Sensor lens dropdown
    - `"combo-resolution"`: Capture resolution dropdown
    - `"combo-crop"`: Crop/framing preset dropdown
    - `"combo-rotation"`: Rotation dropdown (0°, 90°, 180°, 270°)
    - `"switch-flip-h"`, `"switch-invert"`, `"switch-torch"`, `"switch-auto-start"`: Feature switches
    - `"btn-preview"`, `"btn-stream-action"`: Primary action buttons
    - `"btn-refresh"`: Status & logs refresh button
    - `"input-wifi-address"`: IP:port input for ADB over Wi-Fi
    - `"btn-wifi-connect"`: Connect Wi-Fi ADB button
    - `"btn-wifi-tcpip"`: Enable USB TCP/IP button
    - `"text-wifi-status"`: Status message text for wireless ADB
    - `"link-adb-docs"`: Setup guide link button
  - All UI tests run via `cargo test --test ui_test` and `./tests/test-ui.sh`.

---

## 5. Development & Verification Commands

PhoneCam supports both [mise](https://mise.jdx.dev) tasks and direct Cargo commands.

### Build & Check
```bash
# Verify formatting and strict linter
mise run check        # or: cargo fmt --check && cargo clippy --all-targets -- -D warnings

# Compile debug build
mise run build        # or: cargo build

# Compile optimized release binary
mise run build:release # or: cargo build --release

# Cross-compile optimized ARM64 (aarch64) release binary
mise run build:arm64
```

### Run Automated Tests
```bash
# Run all unit and integration tests
mise run test         # or: cargo test --all-targets

# Run mock phone end-to-end suite
cargo test --test fake_phone_e2e

# Run camera video stream transformation OCR suite
mise run test:ocr     # or: ./tests/test-ocr-e2e.sh

# Run memory RSS, CPU, and clean teardown performance suite
mise run test:perf    # or: ./tests/test-perf-e2e.sh

# Run terminal TUI dashboard PTY & keypress E2E suite
mise run test:tui     # or: ./tests/test-tui-e2e.sh

# Run Qt Quick UI click simulation & testID suite (offscreen)
mise run test:ui      # or: ./tests/test-ui.sh

# Run headless browser (Brave / Chromium) WebRTC camera capture E2E suite
mise run test:browser # or: ./tests/test-browser-e2e.sh

# Run cable disconnect & auto-reconnect fault-tolerance E2E suite (5 cycles, >= 5s, OCR verified)
mise run test:reconnect # or: ./tests/test-reconnect-e2e.sh

# Run complete system E2E validation
mise run test:e2e     # or: ./tests/test-package-e2e.sh
```

### Run Locally
```bash
# Run GUI mode
mise run gui          # or: cargo run

# Run Terminal TUI dashboard
mise run tui          # or: cargo run -- --tui

# Inspect phone cameras
phonecam --list-cameras

# Inspect status
phonecam --status
```

### System Installation & Packaging
```bash
# Run native installer via mise or direct binary (same pattern as SoundThis)
mise run install      # or: phonecam install (or: cargo run -- install)

# Run uninstaller
mise run uninstall    # or: phonecam uninstall

# Verify Arch PKGBUILD
cd pkgbuild && makepkg -fsi
```

---

## 6. Omarchy Menu & ASUS Provisioning Integration

PhoneCam is integrated into the official Omarchy setup:
- **Menu Shortcut**: Defined in `~/.config/omarchy/extensions/omarchy-menu.jsonc` under IDs `"phonecam"` and `"phonecam-tui"`.
- **System Provisioning**: Registered in `/home/rvs/dev/scripts/setup-asus.sh` so full machine re-provisioning automatically builds and installs PhoneCam.
