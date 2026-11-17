# PhoneCam Release & Versioning Guide

This document outlines the step-by-step process for cutting a new release of **PhoneCam**, publishing GitHub release assets, and updating package definitions.

---

## 1. Quality Gates

Before cutting a release, ensure all checks pass:

```bash
# 1. Format check
cargo fmt --check

# 2. Linter
cargo clippy --all-targets -- -D warnings

# 3. Unit, integration & E2E tests
cargo test --all-targets

# 4. Fake Phone E2E test suite
cargo test --test fake_phone_e2e

# 5. Full package lifecycle verification
./tests/test-package-e2e.sh
```

---

## 2. Version Bumping Checklist

When incrementing the version (e.g. from `0.1.0` to `0.2.0`):

| File | Field |
|---|---|
| `Cargo.toml` | `[package] version` |
| `src/cli.rs` | `version = "X.Y.Z"` |
| `pkgbuild/PKGBUILD` | `pkgver=X.Y.Z`, `pkgrel=1` |
| `README.md` | Version badges / references |

---

## 3. Tagging & Publishing

```bash
# 1. Commit changes
git add -A
git commit -m "chore(release): bump version to v0.1.0"
git push origin master

# 2. Create annotated tag
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

Pushing the `v*` tag automatically triggers the GitHub Actions `.github/workflows/release.yml` workflow, compiling binaries for `x86_64` and `aarch64`, generating `.tar.gz` archives and `.sha256` checksums, and publishing the release.
