#!/usr/bin/env bash
set -euo pipefail
UI=crates/aether-ui/src/lib.rs
LIVE=crates/aether-engine-servo/src/live.rs
MAIN=crates/aether-browser/src/main.rs
fail(){ echo "AETHER_BROWSER_CURRENT_SHELL=FAIL:$1"; exit 1; }

[[ ! -e crates/aether-ui/assets/browser.html ]] || fail browser-html-present
grep -qF 'pub const NATIVE_BROWSER_SURFACE_REVISION' "$UI" || fail native-revision
grep -qF 'pub const LAUNCHER_WIDTH_PX: u32 = 52;' "$UI" || fail launcher-width
grep -qF 'pub const UTILITY_DOCK_WIDTH_PX: u32 = 176;' "$UI" || fail utility-width
grep -qF 'pub const STATUS_BAR_HEIGHT_PX: u32 = 30;' "$UI" || fail status-height
grep -qF 'pub struct NativeChromeRenderer' "$UI" || fail native-renderer
grep -qF 'pub const fn native_surface_owns_content(&self) -> bool' "$UI" || fail content-ownership-policy
grep -qF 'UtilityCollapseToggle' "$UI" || fail utility-collapse-target
grep -qF 'UTILITY_DOCK_COLLAPSED_WIDTH_PX: u32 = 38;' "$UI" || fail utility-collapsed-width
grep -qF 'let cards_top = content_top + hero_h - 2.0;' "$UI" || fail home-card-hit-geometry
grep -qF 'fn is_home_surface(&self)' "$LIVE" || fail home-surface-runtime
grep -qF 'actual_url: None' "$LIVE" || fail native-home-no-content-url
grep -qF 'webview: Option<WebView>' "$LIVE" || fail optional-content-webview
grep -qF 'AETHER_BROWSER_UI_OWNERSHIP=NATIVE_RUST' "$MAIN" || fail status-native-owner
grep -qF 'AETHER_BROWSER_UI_RENDERER=NATIVE_EGUI_GLOW' "$MAIN" || fail status-native-renderer
grep -qF 'AETHER_BROWSER_SERVO_SURFACES=INTERNAL_NON_HTTP_ONLY' "$MAIN" || fail status-content-only
grep -qF 'AETHER_BROWSER_HOME_SURFACE=NATIVE_RUST' "$MAIN" || fail status-native-home
grep -qF 'AETHER_BROWSER_HOME_BACKING_WEBVIEW=NONE' "$MAIN" || fail status-no-home-webview
for old in 'ChromeDelegate' 'chrome_webview' 'chrome_context' 'render_patch_script' 'render_document' 'UNIFIED_BROWSER_SURFACE' 'UNIFIED_UI|ACTIVE_WEB_CONTENT' 'RUNNING_NONCOMPOSITED'; do
  if grep -RqsF "$old" crates/aether-ui/src crates/aether-engine-servo/src crates/aether-browser/src; then fail "legacy:$old"; fi
done
if grep -RqiE 'PanelMode|panel[_ -]state|dragon.?glass.?panel' crates/aether-ui/src crates/aether-engine-servo/src crates/aether-browser/src; then fail legacy-panel-symbols; fi
python3 - <<'PY'
from pathlib import Path
import re
live=Path('crates/aether-engine-servo/src/live.rs').read_text()
ui=Path('crates/aether-ui/src/lib.rs').read_text()
refs=set(re.findall(r'ChromeHitTarget::([A-Za-z0-9_]+)', live))
block=re.search(r'pub enum ChromeHitTarget \{(.*?)\n\}', ui, re.S)
if not block: raise SystemExit('AETHER_BROWSER_CURRENT_SHELL=FAIL:missing-hit-target-enum')
variants=set(re.findall(r'^\s*([A-Za-z0-9_]+)(?:\([^\n]+\))?,?\s*$', block.group(1), re.M))
missing=sorted(refs-variants)
if missing: raise SystemExit('AETHER_BROWSER_CURRENT_SHELL=FAIL:missing-hit-target-variants:'+','.join(missing))
PY
echo 'AETHER_BROWSER_CURRENT_SHELL=PASS'
