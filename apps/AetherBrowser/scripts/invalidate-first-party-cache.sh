#!/usr/bin/env bash
set -euo pipefail
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
RELEASE_DIR="$TARGET_DIR/release"

# Keep Servo/GStreamer/third-party dependency artifacts, but never carry
# AetherForge-owned release executables or package fingerprints across versions.
rm -f \
  "$RELEASE_DIR/aether-browser" \
  "$RELEASE_DIR/aether-media-service"

if [[ -d "$RELEASE_DIR/deps" ]]; then
  find "$RELEASE_DIR/deps" -maxdepth 1 -type f \
    \( -name 'aether_browser-*' \
       -o -name 'libaether_browser*' \
       -o -name 'aether_media_service-*' \
       -o -name 'libaether_media_service*' \) \
    -delete
fi

if [[ -d "$RELEASE_DIR/.fingerprint" ]]; then
  find "$RELEASE_DIR/.fingerprint" -mindepth 1 -maxdepth 1 -type d \
    \( -name 'aether-browser-*' \
       -o -name 'aether-media-service-*' \) \
    -exec rm -rf {} +
fi
