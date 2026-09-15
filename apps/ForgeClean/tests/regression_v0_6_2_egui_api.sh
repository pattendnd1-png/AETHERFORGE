#!/usr/bin/env bash
set -euo pipefail

GUI=src/gui.rs

# egui 0.36 consolidated side/top/bottom panels under Panel.
! grep -Fq 'egui::TopBottomPanel::' "$GUI"
! grep -Fq 'egui::SidePanel::' "$GUI"
grep -Fq 'egui::Panel::top("forgeclean_title")' "$GUI"
grep -Fq 'egui::Panel::left("forgeclean_nav")' "$GUI"

# Panel exact sizing is unified in egui 0.36.
! grep -Fq '.exact_height(' "$GUI"
! grep -Fq '.exact_width(' "$GUI"
grep -Fq '.exact_size(42.0)' "$GUI"
grep -Fq '.exact_size(NAV_WIDTH)' "$GUI"

# Context::style_mut was removed; mutate the current global style explicitly.
! grep -Fq 'ctx.style_mut(' "$GUI"
grep -Fq 'ctx.global_style_mut(|style|' "$GUI"

printf 'FORGECLEAN_V0_6_2_EGUI_API=PASS\n'
