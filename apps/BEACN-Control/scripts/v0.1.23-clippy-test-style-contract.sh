#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)}"
CORE="$ROOT/tests/core.rs"
fail(){ echo "AETHERFORGE_BEACN_V0_1_23_CLIPPY_TEST_STYLE=FAIL:$1"; exit 1; }
[[ -f "$CORE" ]] || fail missing_core_test
if grep -Fq 'let mut state = SoftwareDspState::default();' "$CORE" && grep -Fq 'state.mic_gain_db = 99.0;' "$CORE"; then
  fail field_reassign_with_default_regression
fi
grep -Fq 'let mut state = SoftwareDspState {' "$CORE" || fail missing_struct_initializer
grep -Fq 'mic_gain_db: 99.0,' "$CORE" || fail missing_inline_mic_gain
echo 'AETHERFORGE_BEACN_V0_1_23_CLIPPY_TEST_STYLE=PASS'

ON_DEVICE="$ROOT/src/on_device.rs"
UI_UX="$ROOT/src/ui_ux.rs"
[[ -f "$ON_DEVICE" ]] || fail missing_on_device_source
[[ -f "$UI_UX" ]] || fail missing_ui_ux_source
if grep -Eq 'if processor_mode_from_eq\(mode\) == dsp\.mic_eq_mode \{[[:space:]]*$' "$ON_DEVICE"; then
  # The Type arm intentionally binds an intermediate index. The other four arms must use let-chains.
  nested_count="$(grep -Ec 'if processor_mode_from_eq\(mode\) == dsp\.mic_eq_mode \{[[:space:]]*$' "$ON_DEVICE" || true)"
  [[ "$nested_count" -le 1 ]] || fail collapsible_if_regression
fi
grep -Fq '#[derive(Debug, Default, Clone, PartialEq, Eq)]' "$UI_UX" || fail ui_state_default_not_derived
if grep -Fq 'impl Default for OnDeviceUiState' "$UI_UX"; then
  fail derivable_impl_regression
fi
echo 'AETHERFORGE_BEACN_V0_1_23_HOST_CLIPPY_FIXES=PASS'
