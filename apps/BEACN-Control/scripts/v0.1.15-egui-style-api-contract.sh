#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
MAIN="$ROOT/src/main.rs"
fail() { echo "AETHERFORGE_BEACN_V0_1_15_EGUI_STYLE_API=FAIL:$1"; exit 1; }
grep -q '^version = "0.1.15"$' "$ROOT/Cargo.toml" || fail version
grep -q 'ctx\.style_mut_of(egui::Theme::Dark' "$MAIN" || fail missing_style_mut_of_dark
if grep -q 'ctx\.style_mut(' "$MAIN"; then fail obsolete_context_style_mut; fi
echo 'AETHERFORGE_BEACN_V0_1_15_EGUI_STYLE_API=PASS'
