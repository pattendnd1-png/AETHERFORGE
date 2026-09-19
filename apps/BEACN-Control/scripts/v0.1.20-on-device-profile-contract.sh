#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
fail(){ echo "AETHERFORGE_BEACN_V0_1_20_ON_DEVICE_PROFILE=FAIL:$1"; exit 1; }

MAIN="$ROOT/src/main.rs"
READ="$ROOT/src/on_device.rs"
UI="$ROOT/src/ui_ux.rs"
[[ -f "$READ" ]] || fail on_device_module_missing
[[ -f "$UI" ]] || fail ui_ux_module_missing
grep -Fq 'pub mod on_device;' "$ROOT/src/lib.rs" || fail on_device_export_missing
grep -Fq 'pub mod ui_ux;' "$ROOT/src/lib.rs" || fail ui_ux_export_missing
grep -Fq 'Message::generate_fetch_message(DeviceType::BeacnMic, version)' "$READ" || fail generated_fetch_list_missing
grep -Fq 'device.handle_message(request)' "$READ" || fail read_dispatch_missing
grep -Fq 'scan_with_cache_fallback' "$READ" || fail cache_fallback_missing
grep -Fq 'ON_DEVICE_PROFILE_NAME' "$READ" || fail canonical_profile_name_missing
grep -Fq 'begin_on_device_startup_scan' "$MAIN" || fail startup_scan_missing
grep -Fq 'poll_on_device_startup_scan' "$MAIN" || fail startup_poll_missing
grep -Fq 'app.begin_on_device_startup_scan();' "$MAIN" || fail startup_invocation_missing
grep -Fq 'OnDeviceUiState::LocalEdit' "$MAIN" || fail local_edit_transition_missing
grep -Fq 'profile_memory_strip' "$MAIN" || fail mic_memory_ui_missing
grep -Fq 'READING MIC MEMORY' "$UI" || fail reading_state_missing
grep -Fq 'ON DEVICE ACTIVE' "$UI" || fail active_state_missing
grep -Fq 'MIC MEMORY' "$UI" || fail memory_strip_missing
if [[ "$(grep -Fc 'device.handle_message(' "$READ")" != "1" ]]; then fail unexpected_device_dispatch_count; fi
if grep -Fq 'set_value(' "$READ" || grep -Fq 'param_set(' "$READ"; then fail direct_setter_path_present; fi
echo 'AETHERFORGE_BEACN_V0_1_20_ON_DEVICE_PROFILE=PASS:GETTER_ONLY_STARTUP_IMPORT'
