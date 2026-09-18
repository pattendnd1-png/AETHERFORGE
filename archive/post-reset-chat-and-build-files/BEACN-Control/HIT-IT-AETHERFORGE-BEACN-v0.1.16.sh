#!/usr/bin/env bash
set -euo pipefail
cd "$HOME/Downloads"
ARCHIVE="AetherForge-BEACN-Control-v0.1.16-SOURCE.tar.xz"
EXPECTED="bdfc8c29851659db86fe23aef029f2a3539cd24474c7ef619a8090392982ac6a"
[[ -f "$ARCHIVE" ]] || { echo "Missing $HOME/Downloads/$ARCHIVE"; exit 1; }
ACTUAL="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || { echo "SOURCE_SHA256=FAIL:expected=$EXPECTED:actual=$ACTUAL"; exit 1; }
echo "SOURCE_SHA256=PASS"
rm -rf "AetherForge-BEACN-Control-v0.1.16"
tar -xJf "$ARCHIVE"
exec ./AetherForge-BEACN-Control-v0.1.16/INSTALL-AND-VERIFY.sh
