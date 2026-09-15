#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
OBS="$ROOT/crates/aether-stream-studio/src/obs.rs"
VERIFY="$ROOT/crates/aether-stream-studio/src/bin/aether-obs-verify.rs"
fail(){ echo "AETHER_BROWSER_V2_1_34_OBS_PROBES=FAIL:$1"; exit 1; }
grep -q 'GetSceneTransitionList' "$OBS" || fail correct-transition-request-missing
if grep -q 'request_raw("GetTransitionList"' "$OBS"; then fail obsolete-transition-request-present; fi
grep -q 'virtual_camera_available' "$OBS" || fail virtual-camera-availability-state-missing
grep -q 'NOT_AVAILABLE' "$VERIFY" || fail virtual-camera-not-available-marker-missing
echo 'AETHER_BROWSER_V2_1_34_OBS_PROBES=PASS'
