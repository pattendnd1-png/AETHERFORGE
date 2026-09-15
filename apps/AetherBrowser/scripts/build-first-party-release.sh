#!/usr/bin/env bash
set -euo pipefail
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
RELEASE_DIR="$TARGET_DIR/release"

cargo build --locked --release -p aether-browser --bin aether-browser
cargo build --locked --release -p aether-media-service --bin aether-media-service
cargo build --locked --release -p aether-stream-studio --bin aether-obs-verify

[[ -x "$RELEASE_DIR/aether-browser" ]] || { echo 'AETHER_BROWSER_FIRST_PARTY_OUTPUT=FAIL:aether-browser' >&2; exit 41; }
echo 'AETHER_BROWSER_FIRST_PARTY_OUTPUT=PASS:aether-browser'
[[ -x "$RELEASE_DIR/aether-media-service" ]] || { echo 'AETHER_BROWSER_FIRST_PARTY_OUTPUT=FAIL:aether-media-service' >&2; exit 42; }
echo 'AETHER_BROWSER_FIRST_PARTY_OUTPUT=PASS:aether-media-service'
[[ -x "$RELEASE_DIR/aether-obs-verify" ]] || { echo 'AETHER_BROWSER_FIRST_PARTY_OUTPUT=FAIL:aether-obs-verify' >&2; exit 45; }
echo 'AETHER_BROWSER_FIRST_PARTY_OUTPUT=PASS:aether-obs-verify'

