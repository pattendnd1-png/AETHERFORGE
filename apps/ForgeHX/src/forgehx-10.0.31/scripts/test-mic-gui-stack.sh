#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
test -f "$root/crates/forgehx-gui/src/microphone.rs"
grep -q 'DeviceTab::InputDsp' "$root/crates/forgehx-gui/src/device_page.rs"
grep -q 'Capability::MicDsp' "$root/crates/forgehx-gui/src/device_page.rs"
grep -q 'ForgeHX Mic' "$root/crates/forgehx-gui/src/microphone.rs"
grep -q 'MicDspApply' "$root/crates/forgehx-gui/src/app.rs"
grep -q 'MicDspBypass' "$root/crates/forgehx-gui/src/app.rs"
grep -q 'MicDspLiveUpdate' "$root/crates/forgehx-gui/src/app.rs"
grep -q 'Automatic Voice Processing (VoicePilot)' "$root/crates/forgehx-gui/src/microphone.rs"
grep -q 'Bass Boost' "$root/crates/forgehx-gui/src/microphone.rs"
grep -q 'Treble Boost' "$root/crates/forgehx-gui/src/microphone.rs"
grep -q 'Multiband EQ' "$root/crates/forgehx-gui/src/microphone.rs"
grep -q 'MicDspApply(DeviceId, MicrophoneDspConfig)' "$root/crates/forgehx-gui/src/device_page.rs"
if grep -n 'return Some(MicControl::' "$root/crates/forgehx-gui/src/microphone.rs"; then
  echo 'microphone actions must escape egui closures through an outer action accumulator' >&2
  exit 1
fi
! grep -q 'Audio/Sink\|JamesDSP\|EasyEffects' "$root/crates/forgehx-gui/src/microphone.rs"
echo 'ForgeHX microphone GUI invariants passed.'
