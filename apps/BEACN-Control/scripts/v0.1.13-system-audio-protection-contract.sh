#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fail() { echo "AETHERFORGE_BEACN_V0_1_13_SYSTEM_AUDIO_PROTECTION=FAIL:$1"; exit 1; }

grep -q '^version = "0.1.13"$' "$ROOT/Cargo.toml" || fail version
grep -q 'BlockedToPreserveSystemAudio' "$ROOT/src/hardware.rs" || fail policy_missing
grep -q 'pub const fn direct_usb_claims_allowed() -> bool' "$ROOT/src/hardware.rs" || fail guard_missing
grep -q 'Protected pass-through' "$ROOT/src/hardware.rs" || fail protected_error_missing
if grep -q 'HardwareController::connect()' "$ROOT/src/main.rs"; then fail ui_direct_connect_present; fi
if grep -q 'app\.connect_hardware()' "$ROOT/src/main.rs"; then fail startup_direct_connect_present; fi
if grep -q 'Reconnect Hardware DSP' "$ROOT/src/main.rs"; then fail manual_direct_connect_present; fi
grep -q 'SYSTEM AUDIO PROTECTED' "$ROOT/src/main.rs" || fail protection_ui_missing
grep -q 'Direct USB DSP control is blocked' "$ROOT/src/main.rs" || fail explanation_missing
echo 'AETHERFORGE_BEACN_V0_1_13_SYSTEM_AUDIO_PROTECTION=PASS'
