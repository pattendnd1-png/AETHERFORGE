#!/usr/bin/env bash
set -euo pipefail
cd "$HOME/Downloads"
ARCHIVE="AetherForge-BEACN-Control-v0.1.13-SOURCE.tar.xz"
EXPECTED="fcaa8e889bf20c7733d523c53622b6b0a8b67b9c71940e4b03fc6ae0f7a0ec08"
[[ -f "$ARCHIVE" ]] || { echo "Missing $HOME/Downloads/$ARCHIVE"; exit 1; }
ACTUAL="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || { echo "SOURCE_SHA256=FAIL"; exit 1; }
echo "SOURCE_SHA256=PASS"
rm -rf "AetherForge-BEACN-Control-v0.1.13"
tar -xJf "$ARCHIVE"
exec ./AetherForge-BEACN-Control-v0.1.13/INSTALL-AND-VERIFY.sh
