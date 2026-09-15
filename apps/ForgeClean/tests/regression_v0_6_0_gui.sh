#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

grep -Fq 'name = "forgeclean-gui"' Cargo.toml
grep -Fq 'eframe = { version = "0.36.2"' Cargo.toml
[[ -f src/gui.rs ]]
[[ -f src/gui_main.rs ]]
[[ -f src/gui_state.rs ]]
[[ -f forgeclean.desktop.in ]]
grep -Fq 'DRAGONGLASS_WINDOW_ALPHA: u8 = 26' src/gui.rs
# All DragonGlass background surfaces stay at the canonical 10% smoky/glassy opacity.
if grep -Eq 'glass_frame\((70|[3-9][0-9]|1[0-9][0-9]|2[7-9])\)' src/gui.rs; then
  echo 'noncanonical DragonGlass frame alpha' >&2
  exit 1
fi
if grep -Eq 'fill\(glass_color\([^)]*,[[:space:]]*(70|9[0-9]|1[0-9][0-9])\)\)' src/gui.rs; then
  echo 'noncanonical DragonGlass control fill alpha' >&2
  exit 1
fi
grep -Fq '.fill(glass_color(128, 35, 88, DRAGONGLASS_WINDOW_ALPHA))' src/gui.rs
grep -Fq '90% transparent / 10% smoky-glassy' README.md
grep -Fq 'forgeclean-gui' install-local.sh
grep -Fq 'FORGECLEAN_GUI_SELF_TEST' build-and-verify.sh
grep -Fq 'ExecStart=%h/.local/bin/forgeclean watch' forgeclean-organizer.service.in
grep -Fq 'pub mod gui_state;' src/lib.rs
grep -Fq 'command.current_dir(parent);' src/gui_state.rs
grep -Fq 'relocated_build_artifact_runs_from_its_canonical_build_folder' tests/gui_state.rs
grep -Fq 'notify-send' src/gui.rs
printf 'FORGECLEAN_V0_6_0_GUI=PASS\n'
