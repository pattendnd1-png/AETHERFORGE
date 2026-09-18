#!/usr/bin/env bash
set -euo pipefail
cd "$HOME/Downloads"
ARCHIVE="AetherForge-BEACN-Control-v0.1.14-SOURCE.tar.xz"
EXPECTED="4bafd0180c9d1d889129a4516210edd7eab1f8eb3efeabc7b6c563686443fe0a"
[[ -f "$ARCHIVE" ]] || { echo "Missing $HOME/Downloads/$ARCHIVE"; exit 1; }
ACTUAL="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || { echo "SOURCE_SHA256=FAIL"; exit 1; }
echo "SOURCE_SHA256=PASS"
rm -rf "AetherForge-BEACN-Control-v0.1.14"
tar -xJf "$ARCHIVE"
exec ./AetherForge-BEACN-Control-v0.1.14/INSTALL-AND-VERIFY.sh
