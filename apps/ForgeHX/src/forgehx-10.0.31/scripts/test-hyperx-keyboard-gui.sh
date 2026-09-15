#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
keyboard="$root/crates/forgehx-gui/src/keyboard.rs"
app="$root/crates/forgehx-gui/src/app.rs"
page="$root/crates/forgehx-gui/src/device_page.rs"
main="$root/crates/forgehx-gui/src/main.rs"

[[ -f "$keyboard" ]] || { echo "missing keyboard GUI module" >&2; exit 1; }
grep -q 'KeyboardModelInfo' "$keyboard"
grep -q 'Exact hardware match' "$keyboard"
grep -q 'Metadata match only' "$keyboard"
grep -q 'OpenRGB' "$keyboard"
! grep -Eq 'hidapi|\.write\(|write_all\(|send_feature_report' "$keyboard"
grep -q 'DeviceTab::Keyboard' "$page"
grep -q 'Self::Keyboard => "Keyboard"' "$page"
grep -q 'Command::KeyboardCapabilities' "$app"
grep -q 'keyboard_model' "$app"
grep -q 'mod keyboard;' "$main"
echo "ForgeHX 10.0.6 keyboard GUI invariants passed."
