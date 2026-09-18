#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
DSP="$ROOT/src/software_dsp.rs"; PROFILE="$ROOT/src/profile.rs"; MAIN="$ROOT/src/main.rs"; TESTS="$ROOT/tests/core.rs"
grep -Fq 'HEADPHONE_EQ_BAND_COUNT: usize = 10' "$DSP"
grep -Fq 'pub enum DspBackendState' "$DSP"
grep -Fq 'DSP BACKEND UNAVAILABLE' "$DSP"
grep -Fq 'pub de_esser:' "$DSP"
grep -Fq 'pub exciter:' "$DSP"
grep -Fq 'pub binaural_personalization:' "$DSP"
grep -Fq 'PROFILE_SCHEMA_VERSION: u32 = 2' "$PROFILE"
grep -Fq 'pub fn save_snapshot' "$PROFILE"
grep -Fq 'pub fn list_snapshots' "$PROFILE"
grep -Fq 'fn autosave_profile' "$MAIN"
grep -Fq 'fn load_snapshot' "$MAIN"
grep -Fq 'software_dsp_linked_headphone_eq_mirrors_selected_ear' "$TESTS"
grep -Fq 'legacy_profile_without_dsp_decodes_with_safe_defaults' "$TESTS"
if grep -Fq 'HardwareController::connect()' "$MAIN"; then echo 'DIRECT_USB_UI_PATH=FAIL'; exit 1; fi
echo 'AETHERFORGE_BEACN_V0_1_13_LIVE_PROFILE_DSP_CONTRACT=PASS'
