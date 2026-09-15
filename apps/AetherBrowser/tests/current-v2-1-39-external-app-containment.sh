#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPAT="$ROOT/crates/aether-compat/src/lib.rs"
ENGINE="$ROOT/crates/aether-engine-servo/src/live.rs"
PAGES="$ROOT/crates/aether-native-pages/src/lib.rs"

fail() { echo "AETHER_BROWSER_V2_1_52_EXTERNAL_APP_CONTAINMENT=FAIL:$1"; exit 1; }

for path in "$COMPAT" "$ENGINE" "$PAGES"; do
  [[ -f "$path" ]] || fail "missing:${path#$ROOT/}"
done

grep -q 'pub struct ExternalAppSurface' "$COMPAT" || fail 'external-app-surface-missing'
grep -q 'pub struct ExternalAppHost' "$COMPAT" || fail 'external-app-host-missing'
grep -q '_NET_WM_PID' "$COMPAT" || fail 'external-app-pid-window-match-missing'
grep -q 'pub fn detach' "$COMPAT" || fail 'external-app-detach-missing'
grep -q 'pub fn reattach' "$COMPAT" || fail 'external-app-reattach-missing'
grep -q 'reparent_window' "$COMPAT" || fail 'x11-reparenting-missing'

grep -q 'ExternalAppHost' "$ENGINE" || fail 'engine-external-app-host-missing'
grep -q 'aether://external-app/open?id=obs' "$ENGINE" || fail 'obs-embedded-route-missing'
grep -q 'external_app: Option<ExternalAppSurface>' "$ENGINE" || fail 'tab-external-app-surface-missing'
grep -q 'AETHER_BROWSER_EXTERNAL_APP_SURFACE=PASS:IN_WINDOW' "$ENGINE" || fail 'in-window-pass-marker-missing'
grep -q 'AETHER_BROWSER_EXTERNAL_APP_DETACH=PASS' "$ENGINE" || fail 'detach-pass-marker-missing'
grep -q 'AETHER_BROWSER_EXTERNAL_APP_REATTACH=PASS' "$ENGINE" || fail 'reattach-pass-marker-missing'
grep -q 'KeyCode::KeyD' "$ENGINE" || fail 'explicit-detach-shortcut-missing'
grep -q 'KeyCode::KeyR' "$ENGINE" || fail 'explicit-reattach-shortcut-missing'

if grep -q 'INSTALLED // NATIVE WINDOW' "$PAGES"; then
  fail 'legacy-separate-window-default-still-advertised'
fi

grep -q '"obs" => Some(ExternalAppTarget' "$ENGINE" || fail 'obs-target-missing'
if grep -q 'const OBS_EMBEDDED_URL:.*mode=detached' "$ENGINE"; then
  fail 'obs-default-route-is-detached'
fi

echo 'AETHER_BROWSER_V2_1_52_EXTERNAL_APP_CONTAINMENT=PASS'
