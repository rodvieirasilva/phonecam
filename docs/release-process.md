# PhoneCam Release & Versioning Guide

This document outlines the step-by-step process for cutting a new release of **PhoneCam**, publishing GitHub release assets, updating package definitions, and verifying live UI/TUI version indicators.

---

## 1. Quality Gates

Before cutting any release, every quality gate and test suite must pass 100% cleanly:

```bash
# 1. Format & strict linter checks
mise run check        # or: cargo fmt --check && cargo clippy --all-targets -- -D warnings

# 2. Unit & integration test suite (51 tests)
mise run test         # or: cargo test --all-targets

# 3. Cross-compile optimized ARM64 binary
mise run build:arm64

# 4. Mock ADB phone integration test
cargo test --test fake_phone_e2e

# 5. Synthetic camera hardware OCR verification (all rotations, mirror, invert)
mise run test:ocr     # or: ./tests/test-ocr-e2e.sh

# 6. Performance, memory (RSS < 50MB), CPU (< 35%), and clean teardown verification
mise run test:perf    # or: ./tests/test-perf-e2e.sh

# 7. Terminal TUI dashboard auto-detach & banner test
mise run test:tui     # or: ./tests/test-tui-e2e.sh

# 8. Full package and repository lifecycle verification
mise run test:e2e     # or: ./tests/test-package-e2e.sh
```

---

## 2. Version Bumping Checklist

When incrementing the version (e.g. from `0.1.0` to `0.2.0`):

| File | Field / Purpose | Verification |
|---|---|---|
| `Cargo.toml` | `[package] version = "X.Y.Z"` | Crates version |
| `src/cli.rs` | `version = "X.Y.Z"` in `#[command(...)]` | `phonecam --version` matches |
| `pkgbuild/PKGBUILD` | `pkgver=X.Y.Z`, `pkgrel=1` | Arch Linux packaging |
| `src/ui/gtk_app.rs` | Uses `env!("CARGO_PKG_VERSION")` | Window title: `PhoneCam vX.Y.Z` |
| `src/tui.rs` | Uses `env!("CARGO_PKG_VERSION")` | Header: `PHONECAM VIRTUAL WEBCAM vX.Y.Z` |
| `README.md` | Version badges / feature highlights | Documentation accuracy |
| `AGENTS.md` | Architecture, versioning & test standards | Contributor guidelines |
| `assets/*.png` | Regenerate with `./scripts/capture-screenshot.sh` | Live screenshots reflect version |

---

## 3. Screenshot & Print Asset Updates

Whenever UI or TUI banners change or a new release is cut, update the screenshot assets:

```bash
# Build the release binary
cargo build --release

# Capture pixel-perfect GUI and TUI screenshots in Hyprland
./scripts/capture-screenshot.sh

# Verify OCR and no shell syntax error artifacts in assets
tesseract assets/screenshot-tui.png stdout
```

---

## 4. Local Installation Verification

Verify local build and system installation before tagging:

```bash
# Run native installer (matches SoundThis pattern)
phonecam install      # or: mise run install (or: cargo run --release -- install)

# Test binary version
phonecam --version
```

---

## 5. Tagging & Publishing

```bash
# 1. Commit changes
git add -A
git commit -m "chore(release): bump version to v0.2.0"
git push origin master

# 2. Create annotated tag
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0
```

Pushing the `v*` tag automatically triggers the GitHub Actions `.github/workflows/release.yml` workflow, compiling release binaries for `x86_64` and `aarch64`, generating `.tar.gz` archives and `.sha256` checksums, and publishing the GitHub release.
