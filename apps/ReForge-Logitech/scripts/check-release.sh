#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
VERSION="0.6.1"

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }

[[ "$(awk -F'"' '/^version = /{print $2; exit}' Cargo.toml)" == "$VERSION" ]] || fail "workspace version is not $VERSION"
grep -qx "pkgver=$VERSION" packaging/arch/PKGBUILD || fail "PKGBUILD pkgver is not $VERSION"
grep -q "VERSION=\"$VERSION\"" scripts/make-dist.sh || fail "make-dist version is not $VERSION"
grep -q "VERSION=\"$VERSION\"" scripts/install-arch.sh || fail "install version is not $VERSION"
grep -q "fwupd: official Logitech firmware updates through LVFS" packaging/arch/PKGBUILD || fail "fwupd optdepend missing"
grep -q 'TAG+="uaccess"' packaging/udev/70-reforge-logitech.rules || fail "uaccess tag missing"
! grep -q 'MODE="0666"' packaging/udev/70-reforge-logitech.rules || fail "unsafe 0666 hidraw permission found"
python - <<'PY2'
from pathlib import Path
s = Path("packaging/arch/PKGBUILD").read_text()
t = s.find("cargo test --workspace")
b = s.find("cargo build --release --workspace")
if t < 0 or b < 0 or t > b:
    raise SystemExit("FAIL: Arch package must test before release build")
PY2
! grep -Eq -- '--force|--allow-older|--allow-reinstall|--no-safety-check' crates/reforge-daemon/src/fwupd.rs || fail "unsafe fwupd override flag found"
grep -q "v0.6.1" RELEASE_NOTES.md || fail "release notes not v0.6.1"
grep -q "v0.6.1" INSTALL-UPGRADE.txt || fail "upgrade instructions not v0.6.1"
grep -q "## v0.6 Logitech Services" README.md || fail "README v0.6 services section missing"
! perl -0ne 'exit 0 if /shared\s*\.lock\(\)/; exit 1' crates/reforge-daemon/src/main.rs || fail "stale shared.lock() remains after AppState split"
python - <<'PY2'
from pathlib import Path
s = Path("crates/reforge-gui/src/app.rs").read_text()
bad = 'None => ui.label("The selected device has no active HID++ session."),'
if bad in s:
    raise SystemExit("FAIL: session-status match None arm returns egui::Response instead of ()")
PY2
printf 'PASS: release surfaces are aligned for v%s\n' "$VERSION"
