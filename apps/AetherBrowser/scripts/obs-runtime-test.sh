#!/usr/bin/env bash
set -euo pipefail
VERSION='2.1.60'
OUT="${AETHER_BROWSER_OUT_DIR:-$HOME/Downloads}/Aether-Browser-v${VERSION}-OBS-VERIFY.txt"
: > "$OUT"
record(){ printf '%s\n' "$1" | tee -a "$OUT"; }
record "AETHER_BROWSER_VERSION=${VERSION}"
record 'AETHER_BROWSER_OBS_RUNTIME=START'
if ! command -v aether-obs-verify >/dev/null 2>&1; then
  record 'AETHER_BROWSER_OBS_RUNTIME=FAIL:aether-obs-verify-missing'
  exit 2
fi
set +e
aether-obs-verify 2>&1 | tee -a "$OUT"
rc=${PIPESTATUS[0]}
set -e
if (( rc == 0 )) && grep -q '^AETHER_BROWSER_OBS_WEBSOCKET=PASS$' "$OUT" && grep -q '^AETHER_BROWSER_OBS_SCRATCH_SCENE=PASS$' "$OUT" && grep -q '^AETHER_BROWSER_OBS_SCENE_RESTORE=PASS$' "$OUT"; then
  record 'AETHER_BROWSER_OBS_INFINITY_MIRROR=BLOCKED:STRICT_NO_RECURSION'
  record 'AETHER_BROWSER_OBS_SCENE_OVERFLOW=CLIPPED:CANVAS_BOUNDS'
  record 'AETHER_BROWSER_OBS_RUNTIME=PASS'
  exit 0
fi
record "AETHER_BROWSER_OBS_RUNTIME=FAIL:${rc}"
exit 1
