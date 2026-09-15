#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
UI="$ROOT/crates/aether-ui/src/lib.rs"
fail(){ echo "AETHER_BROWSER_V2_1_60_GX_DRAGON_GLASS_SHELL=FAIL:$1"; exit 1; }
grep -qF 'OPERA_GX_REFERENCE_VERSION: &str = "135.0.5973.135"' "$UI" || fail reference-version-missing
grep -qF 'AETHERFORGE_GX_DRAGONGLASS' "$UI" || fail shell-contract-missing
grep -qF 'GX_EDGE_GLOW' "$UI" || fail edge-glow-token-missing
grep -qF 'ACTIVE_RAIL' "$UI" || fail active-rail-token-missing
grep -qF 'AetherForge Browser v2.1.60' "$UI" || fail title-version-missing
if grep -qiE 'include_bytes!\(.*opera|opera-gx.*(png|svg|pak)' "$UI"; then
  fail vendor-asset-embedded
fi
echo 'AETHER_BROWSER_V2_1_60_GX_DRAGON_GLASS_SHELL=PASS'
