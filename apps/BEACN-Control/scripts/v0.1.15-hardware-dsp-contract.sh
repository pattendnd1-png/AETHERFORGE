#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
need(){ grep -Fq -- "$2" "$1" || { echo "CONTRACT_FAIL=$3"; exit 1; }; }
reject(){ if grep -Fq -- "$2" "$1"; then echo "CONTRACT_FAIL=$3"; exit 1; fi; }

reject "$ROOT/Cargo.toml" 'beacn-lib' beacn_lib_transport_must_be_absent
need "$ROOT/src/hardware.rs" 'pub const fn direct_usb_claims_allowed() -> bool' direct_usb_guard
need "$ROOT/src/hardware.rs" 'BlockedToPreserveSystemAudio' direct_usb_policy
need "$ROOT/src/private_dsp.rs" 'pub struct PrivateDspEngine' private_dsp_engine
need "$ROOT/src/private_audio.rs" 'pub struct PrivateDspRuntime' private_audio_runtime
need "$ROOT/src/software_dsp.rs" 'HEADPHONE_EQ_BAND_COUNT: usize = 10' software_headphone_eq_10
need "$ROOT/src/software_dsp.rs" 'pub de_esser:' software_de_esser
need "$ROOT/src/software_dsp.rs" 'pub exciter:' software_exciter
if grep -Fq 'HardwareController::connect()' "$ROOT/src/main.rs"; then echo 'CONTRACT_FAIL=ui_direct_usb_connect'; exit 1; fi
if grep -Rqi 'AetherStream' "$ROOT/src" "$ROOT/Cargo.toml"; then echo 'CONTRACT_FAIL=system_dsp_runtime_reference'; exit 1; fi
echo 'AETHERFORGE_BEACN_HARDWARE_DSP_CONTRACT=PASS:MODEL_ONLY_PRIVATE_DSP'
