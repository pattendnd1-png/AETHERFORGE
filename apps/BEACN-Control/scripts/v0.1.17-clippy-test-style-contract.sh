#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)}"
CORE="$ROOT/tests/core.rs"
fail(){ echo "AETHERFORGE_BEACN_V0_1_17_CLIPPY_TEST_STYLE=FAIL:$1"; exit 1; }
[[ -f "$CORE" ]] || fail missing_core_test
if grep -Fq 'let mut state = SoftwareDspState::default();' "$CORE" && grep -Fq 'state.mic_gain_db = 99.0;' "$CORE"; then
  fail field_reassign_with_default_regression
fi
grep -Fq 'let mut state = SoftwareDspState {' "$CORE" || fail missing_struct_initializer
grep -Fq 'mic_gain_db: 99.0,' "$CORE" || fail missing_inline_mic_gain
echo 'AETHERFORGE_BEACN_V0_1_17_CLIPPY_TEST_STYLE=PASS'
