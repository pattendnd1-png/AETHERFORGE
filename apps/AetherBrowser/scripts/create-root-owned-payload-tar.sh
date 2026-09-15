#!/usr/bin/env bash
set -euo pipefail
PAYLOAD=${1:?payload-root required}
OUT=${2:?output tar required}
[[ -d "$PAYLOAD" ]] || { echo "AETHER_BROWSER_PAYLOAD_TAR=FAIL:missing:$PAYLOAD" >&2; exit 2; }
mkdir -p "$(dirname "$OUT")"
# Shared system directories must have canonical package metadata. Individual
# file modes are already set explicitly by install-current-tree.sh.
find "$PAYLOAD" -type d -exec chmod 0755 {} +
rm -f "$OUT"
tar --format=posix --numeric-owner --owner=0 --group=0 -C "$PAYLOAD" -cf "$OUT" .
[[ -s "$OUT" ]] || { echo 'AETHER_BROWSER_PAYLOAD_TAR=FAIL:empty' >&2; exit 3; }
echo 'AETHER_BROWSER_PAYLOAD_TAR=PASS:root-owned'
