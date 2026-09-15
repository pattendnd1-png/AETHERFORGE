#!/usr/bin/env bash
set -euo pipefail
GUI=src/gui.rs

# Window controls must not capture `ui` inside a callback while the helper owns &mut Ui.
! grep -Fq 'round_window_button(ui, "—", "Minimize", || {' "$GUI"
! grep -Fq 'round_window_button(ui, "□", "Maximize", || {' "$GUI"
! grep -Fq 'round_window_button(ui, "×", "Close", || {' "$GUI"

# The helper returns click state; viewport commands execute after its mutable borrow ends.
grep -Fq 'fn round_window_button(ui: &mut egui::Ui, glyph: &str, tooltip: &str) -> bool' "$GUI"
grep -Fq 'if round_window_button(ui, "—", "Minimize") {' "$GUI"
grep -Fq 'if round_window_button(ui, "□", "Maximize") {' "$GUI"
grep -Fq 'if round_window_button(ui, "×", "Close") {' "$GUI"

printf 'FORGECLEAN_V0_6_3_TITLEBAR_BORROW=PASS\n'
