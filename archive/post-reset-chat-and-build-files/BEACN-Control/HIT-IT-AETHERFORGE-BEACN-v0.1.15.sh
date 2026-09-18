#!/usr/bin/env bash
set -euo pipefail
cd "$HOME/Downloads"
ARCHIVE="AetherForge-BEACN-Control-v0.1.15-SOURCE.tar.xz"
EXPECTED="9046c8fdb92fc298fc915043134bcc97e70d3d658cde30f3bd0bfda4485bbd6b"
[[ -f "$ARCHIVE" ]] || { echo "Missing $HOME/Downloads/$ARCHIVE"; exit 1; }
ACTUAL="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$ACTUAL" == "$EXPECTED" ]] || { echo "SOURCE_SHA256=FAIL"; exit 1; }
echo "SOURCE_SHA256=PASS"
rm -rf "AetherForge-BEACN-Control-v0.1.15"
tar -xJf "$ARCHIVE"
exec ./AetherForge-BEACN-Control-v0.1.15/INSTALL-AND-VERIFY.sh
