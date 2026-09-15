#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
UI="$ROOT/crates/aether-ui/src/lib.rs"
fail(){ echo "AETHER_BROWSER_V2_1_60_OPERA_GX_DEB_REFIT=FAIL:$1"; exit 1; }
grep -qF 'OPERA_GX_REFERENCE_VERSION: &str = "135.0.5973.135"' "$UI" || fail version
grep -qF 'OPERA_GX_REFERENCE_SHA256' "$UI" || fail sha256-symbol
grep -qF '960bbce3c7a993d481568f0b3a32a70417b159f902b6083dcfa43b416ccab668' "$UI" || fail sha256-value
grep -qF 'OPERA_GX_REFERENCE_PACKAGE: &str = "opera-gx-stable_135.0.5973.135_amd64.deb"' "$UI" || fail package
grep -qF 'GX_REFERENCE_SIDEBAR_CONTROL' "$UI" || fail sidebar-control
grep -qF 'GX_REFERENCE_WORKSPACES' "$UI" || fail workspaces
grep -qF 'GX_REFERENCE_LEFT_TAB_STRIP' "$UI" || fail left-tab-strip
grep -qF 'GX_REFERENCE_PLAYER_SERVICE' "$UI" || fail player-service
grep -qF 'fn paint_workspace_switcher' "$UI" || fail workspace-switcher-function
grep -qF 'fn paint_sidebar_tool_stack' "$UI" || fail sidebar-tool-stack-function
grep -qF 'AetherForge Browser v2.1.60' "$UI" || fail title-version
if grep -qiE 'include_bytes!\([^)]*(opera|opera-gx)|opera-gx.*\.(png|svg|pak)' "$UI"; then
  fail vendor-asset-embedded
fi
echo 'AETHER_BROWSER_V2_1_60_OPERA_GX_DEB_REFIT=PASS'
