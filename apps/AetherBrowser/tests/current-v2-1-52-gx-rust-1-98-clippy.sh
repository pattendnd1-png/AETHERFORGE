#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
UI="$ROOT/crates/aether-ui/src/lib.rs"
fail(){ echo "AETHER_BROWSER_V2_1_52_GX_RUST_1_98_CLIPPY=FAIL:$1"; exit 1; }
if grep -nE 'Stroke::new\(1\.0,' "$UI" >/dev/null; then
  fail unsuffixed-f32-stroke-width
fi
grep -qF 'Stroke::new(1.0_f32, GX_EDGE_GLOW)' "$UI" || fail gx-edge-stroke-suffix
grep -qF 'Stroke::new(1.0_f32, Color32::from_rgb' "$UI" || fail gx-secondary-stroke-suffix
echo 'AETHER_BROWSER_V2_1_52_GX_RUST_1_98_CLIPPY=PASS'
