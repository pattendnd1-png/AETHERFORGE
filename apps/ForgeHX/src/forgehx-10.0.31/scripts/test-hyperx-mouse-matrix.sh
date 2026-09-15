#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

grep -q 'pub const IPC_PROTOCOL_VERSION: u32 = 11;' crates/forgehx-core/src/lib.rs
grep -q 'pub enum MouseProtocolFamily' crates/forgehx-core/src/lib.rs
grep -q 'pub struct MouseLimits' crates/forgehx-core/src/lib.rs
grep -q 'pub struct MouseModelInfo' crates/forgehx-core/src/lib.rs
grep -q 'MouseCapabilities' crates/forgehx-core/src/lib.rs

test -f crates/forgehx-mouse/src/registry.rs
test -f crates/forgehx-mouse/src/protocol/haste_v1.rs
test -f crates/forgehx-mouse/src/protocol/saga_pro.rs
for model in \
  'Pulsefire Surge' 'Pulsefire Raid' 'Pulsefire Core' 'Pulsefire FPS Pro' \
  'Pulsefire Dart' 'Pulsefire Haste' 'Pulsefire Haste Wireless' \
  'Pulsefire Haste 2' 'Pulsefire Haste 2 Wireless' 'Pulsefire Haste 2 Mini Wireless' \
  'Pulsefire Haste 2 Core Wireless' 'Pulsefire Haste 2 S Wireless' \
  'Pulsefire Fuse Wireless' 'Pulsefire Haste 2 Pro' 'Pulsefire Saga' 'Pulsefire Saga Pro'; do
  grep -q "$model" crates/forgehx-mouse/src/registry.rs
done
# Exact public identities we can safely bind today.
for id in '0x16d3' '0x0490' '0x16d7' '0x16de' '0x0d8f' '0x16e1' '0x068e' '0x16e2' '0x088e' '0x16e4' '0x1727' '0x0f8f' '0x028e' '0x048e' '0x0b97' '0x0f98' '0x04bf' '0x06bf'; do
  grep -qi "$id" crates/forgehx-mouse/src/registry.rs
done

grep -q 'SAGA_PRO_WIRED_PID: u16 = 0x04bf' crates/forgehx-mouse/src/protocol/saga_pro.rs
grep -q 'SAGA_PRO_WIRELESS_PID: u16 = 0x06bf' crates/forgehx-mouse/src/protocol/saga_pro.rs
grep -q 'rate_hz = 8000 / interval_code' crates/forgehx-mouse/src/protocol/saga_pro.rs
grep -q 'report(&\[0x50, 0x02\])' crates/forgehx-mouse/src/protocol/saga_pro.rs

grep -q 'forgehx mouse capabilities' README.md
grep -q 'MouseCapabilities' crates/forgehx-cli/src/main.rs
grep -q 'mouse_limits' crates/forgehx-gui/src/mouse.rs

grep -q 'ATTRS{idProduct}=="04bf"' packaging/udev/70-forgehx.rules
grep -q 'ATTRS{idProduct}=="06bf"' packaging/udev/70-forgehx.rules

grep -q 'name_match_score' crates/forgehx-mouse/src/registry.rs
grep -q 'name_only_matching_prefers_the_most_specific_haste_variant' crates/forgehx-mouse/src/registry.rs

echo 'ForgeHX 10.0.6 HyperX mouse matrix invariants passed.'
