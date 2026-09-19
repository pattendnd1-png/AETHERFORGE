#!/usr/bin/env bash
set -euo pipefail
cd "$HOME/Downloads"
ARCHIVE="AetherForge-BEACN-Control-v0.1.19-SOURCE.tar.xz"
EXPECTED="419820f3f1d7537d03b501d968a2cef41e28294c2809caa5c356eb0a8bbfdf01"
[[ -f "$ARCHIVE" ]] || { echo "Missing $HOME/Downloads/$ARCHIVE"; exit 1; }
ACTUAL="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || { echo "SOURCE_SHA256=FAIL:expected=$EXPECTED:actual=$ACTUAL"; exit 1; }
echo "SOURCE_SHA256=PASS"
rm -rf "AetherForge-BEACN-Control-v0.1.19"
tar -xJf "$ARCHIVE"
exec ./AetherForge-BEACN-Control-v0.1.19/INSTALL-AND-VERIFY.sh
