#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

grep -Fq 'pub mod orbital_ui;' src/lib.rs
grep -Fq 'Orbital,' src/gui.rs
grep -Fq 'Self::Orbital => "Orbital Sync"' src/gui.rs
grep -Fq 'Page::Orbital => crate::orbital_ui::show(ui)' src/gui.rs
grep -Fq 'pub fn show(ui: &mut egui::Ui)' src/orbital_ui.rs
! grep -Fq 'SidePanel::right("forgeclean_orbital_sync_panel")' src/orbital_ui.rs
grep -Fq 'const REFRESH: Duration = Duration::from_secs(1);' src/orbital_ui.rs
grep -Fq 'ctx.request_repaint_after(REFRESH);' src/orbital_ui.rs
grep -Fq 'pub const GUI_VERSION: &str = env!("CARGO_PKG_VERSION");' src/gui_state.rs
grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.5"' Cargo.toml

echo 'FORGECLEAN_V1_0_5_ORBITAL_UI_REGRESSION=PASS'
