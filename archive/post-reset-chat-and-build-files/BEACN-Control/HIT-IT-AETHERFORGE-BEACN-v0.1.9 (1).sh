#!/usr/bin/env bash
set -euo pipefail
cd "$HOME/Downloads"
ARCHIVE="AetherForge-BEACN-Control-v0.1.9-SOURCE.tar.xz"
EXPECTED="2f7873631c4c4f3ec56af2dfd5ac3283b413a4ed924dc5e12d106e95f558fb89"
[[ -f "$ARCHIVE" ]] || { echo "Missing $HOME/Downloads/$ARCHIVE"; exit 1; }
ACTUAL="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || { echo "SOURCE_SHA256=FAIL"; exit 1; }
echo "SOURCE_SHA256=PASS"
rm -rf "AetherForge-BEACN-Control-v0.1.9"
tar -xJf "$ARCHIVE"
exec ./AetherForge-BEACN-Control-v0.1.9/INSTALL-AND-VERIFY.sh
