#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
fail(){ echo "AETHERFORGE_BEACN_V0_1_20_EQ_MODEL_PARITY=FAIL:$1" >&2; exit 1; }
need(){ grep -Fq -- "$2" "$1" || fail "$3"; }

need "$ROOT/src/hardware.rs" 'pub const HARDWARE_EQ_BAND_COUNT: usize = 10;' hardware_band_count
need "$ROOT/src/hardware.rs" 'pub mic_eq: [EqBandState; HARDWARE_EQ_BAND_COUNT]' hardware_mic_eq_count
need "$ROOT/src/hardware.rs" 'pub headphone_eq_left: [EqBandState; HARDWARE_EQ_BAND_COUNT]' hardware_headphone_left_count
need "$ROOT/src/hardware.rs" 'pub headphone_eq_right: [EqBandState; HARDWARE_EQ_BAND_COUNT]' hardware_headphone_right_count
need "$ROOT/src/software_dsp.rs" 'pub const MIC_EQ_BAND_COUNT: usize = 10;' private_mic_eq_count
need "$ROOT/src/software_dsp.rs" 'pub const HEADPHONE_EQ_BAND_COUNT: usize = 10;' private_headphone_eq_count
need "$ROOT/tests/core.rs" 'fn hardware_defaults_match_ten_band_render_target_for_mic_and_headphones()' host_regression_test
need "$ROOT/tests/core.rs" 'assert_eq!(state.mic_eq.len(), 10);' host_mic_assert
need "$ROOT/tests/core.rs" 'assert_eq!(state.headphone_eq_left.len(), 10);' host_headphone_left_assert
need "$ROOT/tests/core.rs" 'assert_eq!(state.headphone_eq_right.len(), 10);' host_headphone_right_assert

echo 'AETHERFORGE_BEACN_V0_1_20_EQ_MODEL_PARITY=PASS'
