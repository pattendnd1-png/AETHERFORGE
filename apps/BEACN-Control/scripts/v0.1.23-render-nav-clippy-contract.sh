#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-.}"
MAIN="$ROOT/src/main.rs"
fail(){ echo "AETHERFORGE_BEACN_V0_1_23_RENDER_NAV_CLIPPY=FAIL:$1" >&2; exit 1; }
[[ -f "$MAIN" ]] || fail missing_main
if grep -Eq '^[[:space:]]+(Mixer|Device),[[:space:]]*$' "$MAIN"; then
  fail obsolete_page_variant
fi
if grep -Eq 'Page::(Mixer|Device)' "$MAIN"; then
  fail obsolete_page_match_arm
fi
grep -Fq 'self.mixer_page(ui);' "$MAIN" || fail mixer_not_folded_into_render_nav
grep -Fq 'self.device_page(ui);' "$MAIN" || fail device_not_folded_into_render_nav
echo 'AETHERFORGE_BEACN_V0_1_23_RENDER_NAV_CLIPPY=PASS'
