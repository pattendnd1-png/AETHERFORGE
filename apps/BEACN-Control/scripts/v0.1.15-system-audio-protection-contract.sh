#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fail() { echo "AETHERFORGE_BEACN_V0_1_15_SYSTEM_AUDIO_PROTECTION=FAIL:$1"; exit 1; }

grep -q '^version = "0.1.15"$' "$ROOT/Cargo.toml" || fail version
grep -q 'BlockedToPreserveSystemAudio' "$ROOT/src/hardware.rs" || fail policy_missing
grep -q 'pub const fn direct_usb_claims_allowed() -> bool' "$ROOT/src/hardware.rs" || fail guard_missing
grep -q 'Protected pass-through' "$ROOT/src/hardware.rs" || fail protected_error_missing
if grep -q 'HardwareController::connect()' "$ROOT/src/main.rs"; then fail ui_direct_connect_present; fi
if grep -q 'app\.connect_hardware()' "$ROOT/src/main.rs"; then fail startup_direct_connect_present; fi
if grep -q 'Reconnect Hardware DSP' "$ROOT/src/main.rs"; then fail manual_direct_connect_present; fi
if grep -Fq 'beacn-lib' "$ROOT/Cargo.toml"; then fail beacn_lib_transport_present; fi
if grep -Rqi 'AetherStream' "$ROOT/src" "$ROOT/Cargo.toml"; then fail system_dsp_coupling_present; fi
grep -q 'SYSTEM DSP ISOLATED' "$ROOT/src/main.rs" || fail isolation_ui_missing
grep -q 'RAW MIC PRESERVED' "$ROOT/src/main.rs" || fail raw_mic_ui_missing
grep -q 'DIRECT USB CONTROL BLOCKED' "$ROOT/src/main.rs" || fail direct_usb_ui_missing
grep -q 'PRIVATE_SOURCE_NAME' "$ROOT/src/private_audio.rs" || fail private_source_missing
if grep -RqiE 'wpctl[[:space:]]+set-default|pactl[[:space:]]+set-default-(source|sink)' "$ROOT/src" "$ROOT/INSTALL-AND-VERIFY.sh"; then fail default_device_mutation; fi
echo 'AETHERFORGE_BEACN_V0_1_15_SYSTEM_AUDIO_PROTECTION=PASS'
