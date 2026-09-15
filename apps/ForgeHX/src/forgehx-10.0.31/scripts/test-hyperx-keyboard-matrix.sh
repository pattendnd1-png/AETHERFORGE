#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
REG="$ROOT/crates/forgehx-keyboard/src/registry.rs"
LIB="$ROOT/crates/forgehx-keyboard/src/lib.rs"
CARGO="$ROOT/crates/forgehx-keyboard/Cargo.toml"
[[ -f "$REG" && -f "$LIB" && -f "$CARGO" ]] || { echo "missing forgehx-keyboard crate" >&2; exit 1; }
for marker in \
  'alloy-rise' 'alloy-rise-75' 'alloy-rise-75-wireless' \
  'origins-2-pro-65' 'origins-2-65' 'origins-2-1800' 'eve-1800' \
  'alloy-elite-rgb' 'alloy-fps-rgb' 'alloy-origins' 'alloy-origins-core' \
  'alloy-elite-2' 'alloy-origins-60' 'alloy-origins-65' 'alloy-mkw100'; do
  grep -q "$marker" "$REG" || { echo "missing keyboard model: $marker" >&2; exit 1; }
done
for id in '0x16e5' '0x0591' '0x16e6' '0x098f' '0x1734' '0x0c8e' '0x038f' '0x16be' '0x1711' '0x058f' '0x16dc'; do
  grep -qi "$id" "$REG" || { echo "missing verified keyboard product id: $id" >&2; exit 1; }
done
grep -q 'name_match_score' "$REG"
grep -q 'exact_hardware_match' "$REG"
grep -Eq 'native_driver:[[:space:]]*None' "$REG"
grep -q 'rise_75_wireless_name_beats_shorter_rise_alias' "$REG"
grep -q 'KeyboardCapabilities' "$ROOT/crates/forgehx-core/src/lib.rs"
grep -q 'fn keyboard_capabilities' "$ROOT/crates/forgehx-daemon/src/lib.rs"
grep -q 'KeyboardCommand::Capabilities' "$ROOT/crates/forgehx-cli/src/main.rs"
UDEV="$ROOT/packaging/udev/70-forgehx.rules"
for id in 0591 098f 0c8e 038f 058f; do
  grep -qi "idProduct}==\"$id\"" "$UDEV" || { echo "missing exact HP keyboard udev id $id" >&2; exit 1; }
done
if grep -Eq 'ATTRS\{idVendor\}=="03f0"[[:space:]]*,?[[:space:]]*TAG\+=' "$UDEV"; then
  echo 'blanket HP 03f0 udev rule is forbidden' >&2
  exit 1
fi
grep -q 'exact_legacy_keyboard_without_native_driver_does_not_gain_native_lighting' "$ROOT/crates/forgehx-daemon/src/lib.rs"
grep -q 'deterministic_openrgb_keyboard_match_owns_lighting' "$ROOT/crates/forgehx-daemon/src/lib.rs"
echo 'ForgeHX 10.0.6 HyperX keyboard matrix invariants passed.'
