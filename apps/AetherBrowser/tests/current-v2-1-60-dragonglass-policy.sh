#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_V2_1_60_DRAGONGLASS_POLICY=FAIL:$1"; exit 1; }
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
UI="$ROOT/crates/aether-ui/src/lib.rs"
TEST="$ROOT/crates/aether-ui/tests/dragonglass.rs"
grep -qF 'surface_transparency_percent: 90,' "$UI" || fail transparency-not-90
grep -qF 'smoky_surface_percent: 10,' "$UI" || fail smoky-not-10
grep -qF 'assert_eq!(tokens.surface_transparency_percent, 90);' "$TEST" || fail transparency-test-missing
grep -qF 'assert_eq!(tokens.smoky_surface_percent, 10);' "$TEST" || fail smoky-test-missing
if grep -qF 'surface_transparency_percent: 75,' "$UI"; then fail legacy-transparency-75; fi
if grep -qF 'smoky_surface_percent: 25,' "$UI"; then fail legacy-smoky-25; fi
echo 'AETHER_BROWSER_V2_1_60_DRAGONGLASS_POLICY=PASS'
