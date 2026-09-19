#!/usr/bin/env bash
set -euo pipefail
cd "$HOME/Downloads"
ARCHIVE="AetherForge-BEACN-Control-v0.1.23-SOURCE.tar.xz"
EXPECTED="5771c7d7c0192b3471339d91795d7f2a445276c58e1ee26680524aead096f651"
[[ -f "$ARCHIVE" ]] || { echo "Missing $HOME/Downloads/$ARCHIVE" >&2; exit 1; }
echo "$EXPECTED  $ARCHIVE" | sha256sum -c -
rm -rf AetherForge-BEACN-Control-v0.1.23
tar -xJf "$ARCHIVE"
cd AetherForge-BEACN-Control-v0.1.23
exec ./INSTALL-AND-VERIFY.sh
