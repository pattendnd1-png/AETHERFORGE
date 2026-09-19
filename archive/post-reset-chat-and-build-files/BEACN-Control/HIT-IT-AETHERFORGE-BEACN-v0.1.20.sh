#!/usr/bin/env bash
set -euo pipefail
cd "$HOME/Downloads"
ARCHIVE="AetherForge-BEACN-Control-v0.1.20-SOURCE.tar.xz"
EXPECTED="cee9ed144d07cbcf90a816a794249646f538fcb639cc1fdac05fdcd6fa8fb164"
[[ -f "$ARCHIVE" ]] || { echo "Missing $HOME/Downloads/$ARCHIVE"; exit 1; }
ACTUAL="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || { echo "SOURCE_SHA256=FAIL:expected=$EXPECTED:actual=$ACTUAL"; exit 1; }
echo "SOURCE_SHA256=PASS"
rm -rf "AetherForge-BEACN-Control-v0.1.20"
tar -xJf "$ARCHIVE"
exec ./AetherForge-BEACN-Control-v0.1.20/INSTALL-AND-VERIFY.sh
