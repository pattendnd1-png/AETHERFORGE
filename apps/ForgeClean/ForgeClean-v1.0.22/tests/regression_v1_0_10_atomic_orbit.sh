#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

grep -Fq 'use crate::orbital::{local_storage_summary, read_status};' src/orbital_ui.rs
grep -Fq 'use crate::orbital_monitor_model::' src/orbital_ui.rs
grep -Fq 'pub mod orbital_ui;' src/lib.rs
grep -Fq 'Self::Orbital => "Orbital Sync"' src/gui.rs
grep -Fq 'Page::Orbital => crate::orbital_ui::show(ui)' src/gui.rs
if grep -RInE '^[[:space:]]*use[[:space:]]+forgeclean::' src \
  --include='*.rs' \
  --exclude='main.rs' \
  --exclude='gui_main.rs' \
  --exclude='forgeclean-system.rs'; then
  echo 'non-binary source contains external forgeclean:: import' >&2
  exit 1
fi
grep -Fq 'use forgeclean::' src/main.rs
grep -Fq 'use forgeclean::' src/gui_main.rs
grep -Fq 'use forgeclean::' src/bin/forgeclean-system.rs
echo 'FORGECLEAN_V1_0_10_ATOMIC_ORBIT_REGRESSION=PASS'
