#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
need(){ grep -Fq -- "$2" "$1" || { echo "CONTRACT_FAIL=$3"; exit 1; }; }

need "$ROOT/Cargo.toml" 'beacn-lib = { git = "https://github.com/beacn-on-linux/beacn-lib.git", tag = "v0.4.3" }' beacn_lib_read_transport_missing
need "$ROOT/src/lib.rs" 'pub mod on_device;' on_device_module_missing
need "$ROOT/src/on_device.rs" 'Message::generate_fetch_message' generated_getters_missing
need "$ROOT/src/on_device.rs" 'device.handle_message(request)' getter_dispatch_missing
need "$ROOT/src/on_device.rs" 'never sends setter messages' getter_only_policy_missing
need "$ROOT/src/hardware.rs" 'pub const fn direct_usb_claims_allowed() -> bool' direct_usb_guard
need "$ROOT/src/hardware.rs" 'BlockedToPreserveSystemAudio' direct_usb_write_policy
need "$ROOT/src/private_dsp.rs" 'pub struct PrivateDspEngine' private_dsp_engine
need "$ROOT/src/private_audio.rs" 'pub struct PrivateDspRuntime' private_audio_runtime
need "$ROOT/src/software_dsp.rs" 'MIC_EQ_BAND_COUNT: usize = 10' software_mic_eq_10
need "$ROOT/src/software_dsp.rs" 'HEADPHONE_EQ_BAND_COUNT: usize = 10' software_headphone_eq_10
need "$ROOT/src/software_dsp.rs" 'pub de_esser:' software_de_esser
need "$ROOT/src/software_dsp.rs" 'pub exciter:' software_exciter
if [[ "$(grep -Fc 'device.handle_message(' "$ROOT/src/on_device.rs")" != "1" ]]; then echo 'CONTRACT_FAIL=unexpected_on_device_dispatch_count'; exit 1; fi
if grep -Fq 'HardwareController::connect()' "$ROOT/src/main.rs"; then echo 'CONTRACT_FAIL=ui_direct_usb_connect'; exit 1; fi
if grep -Rqi 'AetherStream' "$ROOT/src" "$ROOT/Cargo.toml"; then echo 'CONTRACT_FAIL=system_dsp_runtime_reference'; exit 1; fi
echo 'AETHERFORGE_BEACN_HARDWARE_DSP_CONTRACT=PASS:READ_ONLY_MIC_MEMORY+PRIVATE_DSP'
