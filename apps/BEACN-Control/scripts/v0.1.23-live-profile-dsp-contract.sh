#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
DSP="$ROOT/src/software_dsp.rs"; PROFILE="$ROOT/src/profile.rs"; MAIN="$ROOT/src/main.rs"; TESTS="$ROOT/tests/core.rs"; PRIVATE="$ROOT/src/private_audio.rs"
grep -Fq 'MIC_EQ_BAND_COUNT: usize = 10' "$DSP"
grep -Fq 'HEADPHONE_EQ_BAND_COUNT: usize = 10' "$DSP"
grep -Fq 'pub enum DspBackendState' "$DSP"
grep -Fq 'PRIVATE DSP STOPPED' "$DSP"
grep -Fq 'PRIVATE DSP LIVE' "$DSP"
grep -Fq 'pub de_esser:' "$DSP"
grep -Fq 'pub exciter:' "$DSP"
grep -Fq 'pub binaural_personalization:' "$DSP"
grep -Fq 'PROFILE_SCHEMA_VERSION: u32 = 2' "$PROFILE"
grep -Fq 'pub fn save_snapshot' "$PROFILE"
grep -Fq 'pub fn list_snapshots' "$PROFILE"
grep -Fq 'fn autosave_profile' "$MAIN"
grep -Fq 'fn load_snapshot' "$MAIN"
grep -Fq 'fn start_private_dsp' "$MAIN"
grep -Fq 'fn stop_private_dsp' "$MAIN"
grep -Fq 'pub fn update_profile' "$PRIVATE"
grep -Fq 'software_dsp_linked_headphone_eq_mirrors_selected_ear' "$TESTS"
grep -Fq 'legacy_profile_without_dsp_decodes_with_safe_defaults' "$TESTS"
if grep -Fq 'HardwareController::connect()' "$MAIN"; then echo 'DIRECT_USB_UI_PATH=FAIL'; exit 1; fi
if grep -Rqi 'AetherStream' "$ROOT/src" "$ROOT/Cargo.toml"; then echo 'SYSTEM_DSP_COUPLING=FAIL'; exit 1; fi
echo 'AETHERFORGE_BEACN_V0_1_23_LIVE_PROFILE_DSP_CONTRACT=PASS'
