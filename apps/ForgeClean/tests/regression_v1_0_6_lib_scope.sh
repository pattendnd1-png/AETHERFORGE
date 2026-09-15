#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

! grep -Fq 'use forgeclean::' src/orbital_ui.rs
grep -Fq 'use crate::orbital::{local_storage_summary, read_status};' src/orbital_ui.rs
grep -Fq 'use crate::orbital_monitor_model::' src/orbital_ui.rs
grep -Fq 'pub mod orbital_ui;' src/lib.rs
grep -Fq 'Self::Orbital => "Orbital Sync"' src/gui.rs
grep -Fq 'Page::Orbital => crate::orbital_ui::show(ui)' src/gui.rs
grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.6"' Cargo.toml

echo 'FORGECLEAN_V1_0_6_LIB_SCOPE_REGRESSION=PASS'
