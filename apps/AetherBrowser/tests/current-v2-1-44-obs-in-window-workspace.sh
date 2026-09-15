#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ENGINE="$ROOT/crates/aether-engine-servo/src/live.rs"
PAGES="$ROOT/crates/aether-native-pages/src/lib.rs"
INSTALL="$ROOT/scripts/install-current-tree.sh"
fail(){ echo "AETHER_BROWSER_V2_1_47_OBS_IN_WINDOW_WORKSPACE=FAIL:$1"; exit 1; }
grep -q 'OBS_EMBEDDED_URL' "$ENGINE" || fail obs-embedded-url-missing
grep -q 'aether://external-app/open?id=obs' "$ENGINE" || fail obs-target-url-missing
grep -q '"obs" => Some(ExternalAppTarget' "$ENGINE" || fail obs-external-target-missing
grep -q 'program: "obs"' "$ENGINE" || fail obs-program-missing
grep -q 'QT_QPA_PLATFORM' "$ENGINE" || fail obs-x11-backend-missing
grep -q '"xcb"' "$ENGINE" || fail obs-xcb-value-missing
grep -q '"obs" => "OBS Studio"' "$ENGINE" || fail obs-page-title-missing
grep -q 'fn known_external_app_targets()' "$ENGINE" || fail external-target-list-missing
grep -q 'OBS_EMBEDDED_URL' "$ENGINE" || fail obs-restart-adoption-list-missing
grep -q 'aether://external-app/open?id=obs' "$PAGES" || fail obs-native-page-cta-missing
grep -q 'Full OBS Workspace' "$PAGES" || fail full-workspace-copy-missing
grep -q "'obs-studio'" "$INSTALL" || fail obs-package-dependency-missing
# Full workspace must use the real OBS GUI while retaining WebSocket control-plane copy.
grep -q 'real installed OBS Studio' "$PAGES" || fail real-obs-copy-missing
grep -q 'OBS WebSocket v5' "$PAGES" || fail websocket-control-plane-copy-missing
echo 'AETHER_BROWSER_V2_1_47_OBS_IN_WINDOW_WORKSPACE=PASS'
