#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
UI="$ROOT/crates/aether-ui/src/lib.rs"
LIVE="$ROOT/crates/aether-engine-servo/src/live.rs"
MAIN="$ROOT/crates/aether-browser/src/main.rs"
fail(){ echo "AETHER_BROWSER_COMPACT_INTERFACE=FAIL:$1"; exit 1; }
grep -qF 'pub const CHROME_HEIGHT_PX: u32 = 108;' "$UI" || fail chrome-height
grep -qF 'pub const LAUNCHER_WIDTH_PX: u32 = 52;' "$UI" || fail launcher-width
grep -qF 'pub const UTILITY_DOCK_WIDTH_PX: u32 = 176;' "$UI" || fail utility-expanded-width
grep -qF 'pub const UTILITY_DOCK_COLLAPSED_WIDTH_PX: u32 = 38;' "$UI" || fail utility-collapsed-width
grep -qF 'pub const STATUS_BAR_HEIGHT_PX: u32 = 30;' "$UI" || fail status-height
grep -qF 'UtilityCollapseToggle' "$UI" || fail collapse-hit-target
grep -qF 'utility_collapsed' "$UI" || fail collapse-layout-state
grep -qF 'toggle_utility_collapsed' "$LIVE" || fail collapse-runtime-toggle
grep -qF 'AETHER_BROWSER_UTILITY_DOCK_COLLAPSED_WIDTH_PX=38' "$MAIN" || fail status-collapse-width
echo 'AETHER_BROWSER_COMPACT_INTERFACE=PASS'
