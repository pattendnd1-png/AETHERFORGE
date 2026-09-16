#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if grep -RInE '^[[:space:]]*use[[:space:]]+forgeclean::' src \
  --include='*.rs' \
  --exclude='main.rs' \
  --exclude='gui_main.rs' \
  --exclude='forgeclean-system.rs'; then
  echo 'library source must use crate:: imports' >&2
  exit 1
fi
grep -Fq 'use crate::orbital::{local_storage_summary, read_status};' src/orbital_ui.rs
grep -Fq 'use crate::orbital_monitor_model::' src/orbital_ui.rs
grep -Fq 'pub mod orbital_ui;' src/lib.rs
grep -Fq 'Self::Orbital => "Orbital Sync"' src/gui.rs
grep -Fq 'Page::Orbital => crate::orbital_ui::show(ui)' src/gui.rs
echo 'FORGECLEAN_V1_0_7_IMPORT_SCOPE_REGRESSION=PASS'
