#!/usr/bin/env bash
set -euo pipefail
cd "$HOME/Downloads"
ARCHIVE="AetherForge-BEACN-Control-v0.1.17-SOURCE.tar.xz"
EXPECTED="d3d2fe1638066f2a695461a65b3a8d250a8b4ed94181ca109f6acdb582bb4498"
[[ -f "$ARCHIVE" ]] || { echo "Missing $HOME/Downloads/$ARCHIVE"; exit 1; }
ACTUAL="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || { echo "SOURCE_SHA256=FAIL:expected=$EXPECTED:actual=$ACTUAL"; exit 1; }
echo "SOURCE_SHA256=PASS"
rm -rf "AetherForge-BEACN-Control-v0.1.17"
tar -xJf "$ARCHIVE"
exec ./AetherForge-BEACN-Control-v0.1.17/INSTALL-AND-VERIFY.sh
