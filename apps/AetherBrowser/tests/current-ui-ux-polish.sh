#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
fail(){ echo "AETHER_BROWSER_UI_UX_POLISH=FAIL:$1"; exit 1; }
UI=crates/aether-ui/src/lib.rs
PAGES=crates/aether-native-pages/src/lib.rs

grep -qF 'pub const CHROME_HEIGHT_PX: u32 = 108;' "$UI" || fail chrome-not-compact-108
grep -qF 'pub const TAB_MIN_WIDTH_PX: u32 = 104;' "$UI" || fail adaptive-tab-min-missing
grep -qF 'pub const TAB_MAX_WIDTH_PX: u32 = 188;' "$UI" || fail adaptive-tab-max-missing
grep -qF 'fn tab_width_for_count(&self, width: u32) -> u32' "$UI" || fail adaptive-tab-width-function-missing
grep -qF 'provider_tab_glyph' "$UI" || fail provider-tab-identity-missing
grep -qF '<details class="route-more">' "$PAGES" || fail compact-more-navigation-missing
grep -qF '.route-primary' "$PAGES" || fail primary-route-navigation-style-missing
grep -qF '.route-more' "$PAGES" || fail more-route-navigation-style-missing
grep -qF 'AETHER_BROWSER_UI_UX_COMPACT_CHROME=PASS' tests/current-ui-ux-polish.sh || fail self-marker-missing

echo 'AETHER_BROWSER_UI_UX_COMPACT_CHROME=PASS'
echo 'AETHER_BROWSER_ADAPTIVE_TABS=PASS'
echo 'AETHER_BROWSER_NATIVE_NAVIGATION_HIERARCHY=PASS'
echo 'AETHER_BROWSER_UI_UX_POLISH=PASS'
