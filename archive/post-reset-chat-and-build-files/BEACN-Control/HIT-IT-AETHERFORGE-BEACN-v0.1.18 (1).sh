#!/usr/bin/env bash
set -euo pipefail
cd "$HOME/Downloads"
ARCHIVE="AetherForge-BEACN-Control-v0.1.18-SOURCE.tar.xz"
EXPECTED="59314f0a516f77e07b2b567f168eabad69b61fd43197d9051cfb45c883f104e4"
[[ -f "$ARCHIVE" ]] || { echo "Missing $HOME/Downloads/$ARCHIVE"; exit 1; }
ACTUAL="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || { echo "SOURCE_SHA256=FAIL:expected=$EXPECTED:actual=$ACTUAL"; exit 1; }
echo "SOURCE_SHA256=PASS"
rm -rf "AetherForge-BEACN-Control-v0.1.18"
tar -xJf "$ARCHIVE"
exec ./AetherForge-BEACN-Control-v0.1.18/INSTALL-AND-VERIFY.sh
