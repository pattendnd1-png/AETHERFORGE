#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

grep -Fq 'const VERSION: &str = env!("CARGO_PKG_VERSION");' src/main.rs
grep -Fq 'pub const GUI_VERSION: &str = env!("CARGO_PKG_VERSION");' src/gui_state.rs
grep -Fq 'FORGECLEAN_V1_0_1_PROMOTION' build-and-verify.sh
for gate in \
  regression_v0_6_0_gui.sh \
  regression_v0_6_1_permissions.sh \
  regression_v0_6_2_egui_api.sh \
  regression_v0_6_3_titlebar_borrow.sh \
  regression_v0_6_4_gui_clippy.sh; do
  grep -Fq "$gate" build-and-verify.sh
done
printf 'FORGECLEAN_V1_0_1_PROMOTION=PASS\n'
