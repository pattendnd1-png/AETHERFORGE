#!/usr/bin/env bash
set -euo pipefail
cd "$HOME/Downloads"
ARCHIVE="AetherForge-BEACN-Control-v0.1.21-SOURCE.tar.xz"
EXPECTED="1fee70123b805653bc807d09dc61468671ef9629204aebd8ca09f6b9be2f781f"
[[ -f "$ARCHIVE" ]] || { echo "Missing $HOME/Downloads/$ARCHIVE"; exit 1; }
ACTUAL="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || { echo "SOURCE_SHA256=FAIL:expected=$EXPECTED:actual=$ACTUAL"; exit 1; }
echo "SOURCE_SHA256=PASS"
rm -rf "AetherForge-BEACN-Control-v0.1.21"
tar -xJf "$ARCHIVE"
exec ./AetherForge-BEACN-Control-v0.1.21/INSTALL-AND-VERIFY.sh
