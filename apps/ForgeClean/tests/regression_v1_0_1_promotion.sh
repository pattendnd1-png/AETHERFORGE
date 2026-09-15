#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

grep -Fq 'version = "1.0.1"' Cargo.toml
grep -Fq 'const VERSION: &str = "1.0.1";' src/main.rs
grep -Fq 'ForgeClean-v1.0.1-BATCH.txt' src/main.rs
grep -Fq 'ForgeClean-v1.0.1-PACMAN-BATCH.txt' src/system_scan.rs
grep -Fq 'ForgeClean-v1.0.1-OFFLOAD-RESULT.txt' src/system_scan.rs
grep -Fq 'pub const GUI_VERSION: &str = "1.0.1";' src/gui_state.rs
grep -Fq 'if GUI_VERSION != "1.0.1"' src/gui.rs
grep -Fq 'ForgeClean GUI v1.0.1' src/gui_main.rs
grep -Fq 'FORGECLEAN_GUI_VERSION=1.0.1' src/gui_main.rs
grep -Fq 'VERSION="1.0.1"' build-and-verify.sh
grep -Fq 'VERSION="1.0.1"' install-local.sh
grep -Fq 'VERSION="1.0.1"' hit-it-template.sh
grep -Fq '# ForgeClean v1.0.1' README.md
grep -Fq 'FORGECLEAN_V1_0_1_PROMOTION' build-and-verify.sh
# Stable promotion must retain every historical compatibility/safety gate.
for gate in \
  regression_v0_6_0_gui.sh \
  regression_v0_6_1_permissions.sh \
  regression_v0_6_2_egui_api.sh \
  regression_v0_6_3_titlebar_borrow.sh \
  regression_v0_6_4_gui_clippy.sh; do
  grep -Fq "$gate" build-and-verify.sh
done
printf 'FORGECLEAN_V1_0_1_PROMOTION=PASS\n'
