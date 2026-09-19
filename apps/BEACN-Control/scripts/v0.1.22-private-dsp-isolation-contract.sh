#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
fail(){ echo "AETHERFORGE_BEACN_V0_1_22_PRIVATE_DSP_ISOLATION=FAIL:$1"; exit 1; }

[[ -f "$ROOT/src/private_dsp.rs" ]] || fail private_dsp_module_missing
[[ -f "$ROOT/src/private_audio.rs" ]] || fail private_audio_module_missing
grep -Fq 'pub mod private_dsp;' "$ROOT/src/lib.rs" || fail private_dsp_export_missing
grep -Fq 'pub mod private_audio;' "$ROOT/src/lib.rs" || fail private_audio_export_missing
grep -Fq 'pub const PRIVATE_SOURCE_NAME' "$ROOT/src/private_audio.rs" || fail private_source_namespace_missing
grep -Fq 'aetherforge_beacn_private_dsp' "$ROOT/src/private_audio.rs" || fail private_source_name_missing
grep -Fq 'module-pipe-source' "$ROOT/src/private_audio.rs" || fail pipe_source_missing
grep -Fq 'pw-record' "$ROOT/src/private_audio.rs" || fail explicit_capture_missing
grep -Fq -- '--target' "$ROOT/src/private_audio.rs" || fail targeted_capture_missing
grep -Fq -- '--raw' "$ROOT/src/private_audio.rs" || fail raw_capture_missing
grep -Fq 'float32le' "$ROOT/src/private_audio.rs" || fail pipe_source_format_missing
grep -Fq 'DirectUsbControlPolicy::BlockedToPreserveSystemAudio' "$ROOT/src/hardware.rs" || fail direct_usb_block_missing

if grep -Rqi --exclude-dir=docs --exclude='README.md' --exclude='REFERENCE-INSTALLERS.txt' 'AetherStream' "$ROOT/src" "$ROOT/INSTALL-AND-VERIFY.sh" "$ROOT/Cargo.toml"; then
  fail system_dsp_reference_present
fi
grep -Fq 'beacn-lib = { git = "https://github.com/beacn-on-linux/beacn-lib.git", tag = "v0.4.3" }' "$ROOT/Cargo.toml" || fail read_only_beacn_lib_missing
if grep -Fq 'beacn_lib' "$ROOT/src/private_dsp.rs" "$ROOT/src/private_audio.rs"; then fail hardware_transport_inside_private_dsp; fi
if grep -RqiE 'wpctl[[:space:]]+set-default|pactl[[:space:]]+set-default-(source|sink)' "$ROOT/src" "$ROOT/INSTALL-AND-VERIFY.sh"; then
  fail default_device_mutation_present
fi
if grep -Fq 'HardwareController::connect()' "$ROOT/src/main.rs"; then fail ui_direct_usb_connect_present; fi

echo 'AETHERFORGE_BEACN_V0_1_22_PRIVATE_DSP_ISOLATION=PASS'
