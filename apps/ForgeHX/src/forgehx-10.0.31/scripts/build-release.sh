#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
command -v cargo >/dev/null || { echo 'cargo is required. Install Rust with rustup or pacman -S rust.' >&2; exit 1; }

RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo test --workspace
RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo build --workspace --release
python3 "$ROOT/scripts/check-v10.0.27-source.py"
python3 "$ROOT/scripts/test-10.0.27-always-on-dsp-activation.py"
python3 "$ROOT/scripts/test-10.0.27-live-noise-rejection.py"
python3 "$ROOT/scripts/test-10.0.27-aetherstream-bridge.py"
SOURCE_ARCHIVE="$($ROOT/scripts/create-source-package.sh)"

VERSION="10.0.27"
BUNDLE="$ROOT/dist/forgehx-$VERSION-linux-x86_64"
rm -rf "$BUNDLE"
mkdir -p "$BUNDLE/bin" "$BUNDLE/packaging"
install -m755 target/release/forgehx "$BUNDLE/bin/forgehx"
install -m755 target/release/forgehx-gui "$BUNDLE/bin/forgehx-gui"
install -m755 target/release/forgehx-daemon "$BUNDLE/bin/forgehx-daemon"
install -m755 target/release/forgehx-tray "$BUNDLE/bin/forgehx-tray"
cp -a packaging/. "$BUNDLE/packaging/"
cp README.md "$BUNDLE/README.md"
tar -C "$ROOT/dist" -czf "$ROOT/dist/forgehx-$VERSION-linux-x86_64.tar.gz" "forgehx-$VERSION-linux-x86_64"

echo "Source archive: $SOURCE_ARCHIVE"
echo "Binary bundle: $ROOT/dist/forgehx-$VERSION-linux-x86_64.tar.gz"
