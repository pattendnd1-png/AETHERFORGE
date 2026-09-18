#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
need(){ grep -Fq -- "$2" "$1" || { echo "CONTRACT_FAIL=$3"; exit 1; }; }
need "$ROOT/Cargo.toml" 'rev = "69041e41c0483fa520ac5386eeb29dbcfb7279cd"' pinned_beacn_lib
need "$ROOT/src/hardware.rs" 'pub fn set_mic_eq_gain' staged_mic_eq
need "$ROOT/src/hardware.rs" 'pub fn set_headphone_eq_gain' staged_headphone_eq
need "$ROOT/src/hardware.rs" 'pub const fn direct_usb_claims_allowed() -> bool' direct_usb_guard
need "$ROOT/src/software_dsp.rs" 'HEADPHONE_EQ_BAND_COUNT: usize = 10' software_headphone_eq_10
need "$ROOT/src/software_dsp.rs" 'pub de_esser:' software_de_esser
need "$ROOT/src/software_dsp.rs" 'pub exciter:' software_exciter
if grep -Fq 'HardwareController::connect()' "$ROOT/src/main.rs"; then echo 'CONTRACT_FAIL=ui_direct_usb_connect'; exit 1; fi
echo 'AETHERFORGE_BEACN_HARDWARE_DSP_CONTRACT=PASS'
