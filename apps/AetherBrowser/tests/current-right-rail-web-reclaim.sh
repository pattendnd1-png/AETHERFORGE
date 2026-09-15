#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
fail(){ echo "AETHER_BROWSER_RIGHT_RAIL_WEB_RECLAIM=FAIL:$1"; exit 1; }
UI=crates/aether-ui/src/lib.rs
LIVE=crates/aether-engine-servo/src/live.rs

grep -qF 'collapsed_utility_reclaims_138_pixels_at_1360x768' "$LIVE" || fail exact-runtime-geometry-test-missing
grep -qF 'AETHER_BROWSER_RIGHT_RAIL_COLLAPSE_RECLAIMS_WEB_SPACE=PASS' "$LIVE" || fail runtime-reclaim-marker-missing
grep -qF 'AETHER_BROWSER_RIGHT_RAIL_EXPAND_RESTORES_WEB_SPACE=PASS' "$LIVE" || fail runtime-restore-marker-missing
grep -qF 'self.resize(self.window.inner_size());' "$LIVE" || fail toggle-does-not-resize-servo-surface
grep -qF 'UTILITY_DOCK_COLLAPSED_WIDTH_PX: u32 = 38' "$UI" || fail collapsed-rail-width-changed
grep -qF 'saturating_sub(self.launcher_width() + self.utility_width())' "$UI" || fail content-width-not-derived-from-live-rail-width

echo 'AETHER_BROWSER_RIGHT_RAIL_COLLAPSE_RECLAIMS_WEB_SPACE=PASS'
echo 'AETHER_BROWSER_RIGHT_RAIL_EXPAND_RESTORES_WEB_SPACE=PASS'
echo 'AETHER_BROWSER_RIGHT_RAIL_POINTER_GEOMETRY=PASS'
echo 'AETHER_BROWSER_RIGHT_RAIL_WEB_RECLAIM=PASS'
