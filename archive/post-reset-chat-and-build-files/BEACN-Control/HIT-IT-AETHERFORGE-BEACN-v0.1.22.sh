#!/usr/bin/env bash
set -euo pipefail
cd "$HOME/Downloads"
ARCHIVE="AetherForge-BEACN-Control-v0.1.22-SOURCE.tar.xz"
EXPECTED="6af06fa843a4879f34df64dcd3713ca974324c75fa564f41f310079171356972"
[[ -f "$ARCHIVE" ]] || { echo "Missing $HOME/Downloads/$ARCHIVE"; exit 1; }
ACTUAL="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || { echo "SOURCE_SHA256=FAIL:expected=$EXPECTED:actual=$ACTUAL"; exit 1; }
echo "SOURCE_SHA256=PASS"
rm -rf "AetherForge-BEACN-Control-v0.1.22"
tar -xJf "$ARCHIVE"
exec ./AetherForge-BEACN-Control-v0.1.22/INSTALL-AND-VERIFY.sh
