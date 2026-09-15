#!/usr/bin/env bash
set -euo pipefail
GUI=src/gui.rs

# Clippy -D warnings: keep WorkerEvent compact by boxing the large snapshot payload.
grep -Fq 'Snapshot(Box<Result<DashboardSnapshot, String>>),' "$GUI"
! grep -Fq 'Snapshot(Result<DashboardSnapshot, String>),' "$GUI"

# Clippy -D warnings: collapse the Open location click + parent guard into one if-let chain.
grep -Fq 'if action_button(ui, "Open location").clicked()' "$GUI"
grep -Fq '&& let Some(parent) = item.path.parent()' "$GUI"

printf 'FORGECLEAN_V0_6_4_GUI_CLIPPY=PASS\n'
