#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
CAPTURE="$ROOT/crates/aether-capture/src/lib.rs"
OBS="$ROOT/crates/aether-stream-studio/src/obs.rs"
CREATOR="$ROOT/crates/aether-creator-integrations/src/lib.rs"
OBS_RUN="$ROOT/scripts/obs-runtime-test.sh"
SL_RUN="$ROOT/scripts/streamlabs-runtime-test.sh"
fail(){ echo "AETHER_BROWSER_V2_1_47_CAPTURE_SAFETY=FAIL:$1"; exit 1; }
for token in CaptureSafetyPolicy StrictNoRecursion clip_region_to_canvas is_recursive_capture_target; do
  grep -qF "$token" "$CAPTURE" || fail "capture-core:$token"
done
for token in enforce_embedded_capture_safety GetInputList GetInputSettings SetSceneItemEnabled AETHER_BROWSER_OBS_CAPTURE_SAFETY; do
  grep -qF "$token" "$OBS" || fail "obs-guard:$token"
done
grep -qF 'STRICT_NO_RECURSION' "$CREATOR" || fail streamlabs-policy-missing
grep -qF 'AETHER_BROWSER_OBS_INFINITY_MIRROR=BLOCKED' "$OBS_RUN" || fail obs-runtime-marker-missing
grep -qF 'AETHER_BROWSER_STREAMLABS_INFINITY_MIRROR=BLOCKED' "$SL_RUN" || fail streamlabs-runtime-marker-missing
grep -qF 'AETHER_BROWSER_OBS_SCENE_OVERFLOW=CLIPPED' "$OBS_RUN" || fail obs-overflow-marker-missing
grep -qF 'AETHER_BROWSER_STREAMLABS_SCENE_OVERFLOW=CLIPPED' "$SL_RUN" || fail streamlabs-overflow-marker-missing
echo 'AETHER_BROWSER_V2_1_47_CAPTURE_SAFETY=PASS'
