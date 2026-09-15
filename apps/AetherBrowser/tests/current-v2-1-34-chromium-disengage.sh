#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
COMPAT="$ROOT/crates/aether-compat/src/lib.rs"
LIVE="$ROOT/crates/aether-engine-servo/src/live.rs"
fail(){ echo "AETHER_BROWSER_V2_1_34_CHROMIUM_DISENGAGE=FAIL:$1"; exit 1; }
grep -q 'get_window_attributes' "$COMPAT" || fail map-state-verification-missing
grep -q 'MapState::UNMAPPED' "$COMPAT" || fail unmapped-state-check-missing
grep -q -- '-32768' "$COMPAT" || fail offscreen-hide-guard-missing
grep -q 'AETHER_BROWSER_COMPAT_DISENGAGE=PASS' "$LIVE" || fail disengage-pass-marker-missing
grep -q 'AETHER_BROWSER_COMPAT_DISENGAGE=FAIL' "$LIVE" || fail disengage-fail-marker-missing
echo 'AETHER_BROWSER_V2_1_34_CHROMIUM_DISENGAGE=PASS'
