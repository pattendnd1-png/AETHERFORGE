#!/usr/bin/env bash
set -euo pipefail
main="${1:-src/main.rs}"
fail=0
if grep -q 'egui::SidePanel' "$main"; then
  echo 'FAIL: legacy egui::SidePanel API remains' >&2
  fail=1
fi
if grep -q 'egui::TopBottomPanel' "$main"; then
  echo 'FAIL: legacy egui::TopBottomPanel API remains' >&2
  fail=1
fi
if grep -q 'fn update(&mut self, ctx: &egui::Context' "$main"; then
  echo 'FAIL: legacy eframe::App::update implementation remains' >&2
  fail=1
fi
if ! grep -q 'fn ui(&mut self, ui: &mut egui::Ui' "$main"; then
  echo 'FAIL: eframe 0.36 App::ui implementation missing' >&2
  fail=1
fi
if ! grep -q 'egui::Panel::left("beacn-device-rail")' "$main"; then
  echo 'FAIL: eframe 0.36 left Panel API missing' >&2
  fail=1
fi
if ! grep -q 'egui::Panel::top("beacn-header")' "$main"; then
  echo 'FAIL: eframe 0.36 top Panel API missing' >&2
  fail=1
fi
exit "$fail"
