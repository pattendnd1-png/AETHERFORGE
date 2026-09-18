#!/usr/bin/env bash
set -euo pipefail
cd "$HOME/Downloads"
ARCHIVE="AetherForge-BEACN-Control-v0.1.12-SOURCE.tar.xz"
EXPECTED="cd1edea5a7b8d41d513d18d2d786e5274e39895189d3ce0a68473c09a5c8feae"
[[ -f "$ARCHIVE" ]] || { echo "Missing $HOME/Downloads/$ARCHIVE"; exit 1; }
ACTUAL="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || { echo "SOURCE_SHA256=FAIL"; exit 1; }
echo "SOURCE_SHA256=PASS"
rm -rf "AetherForge-BEACN-Control-v0.1.12"
tar -xJf "$ARCHIVE"
exec ./AetherForge-BEACN-Control-v0.1.12/INSTALL-AND-VERIFY.sh
