#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
fail(){ echo "AETHERFORGE_BEACN_V0_1_23_CACHE_LOCK=FAIL:$1"; exit 1; }
READ="$ROOT/src/on_device.rs"
UI="$ROOT/src/ui_ux.rs"
INSTALL="$ROOT/INSTALL-AND-VERIFY.sh"

grep -Fq 'same_mic_cached_profile' "$READ" || fail same_mic_cache_lookup_missing
grep -Fq 'cache_text_matches_serial' "$READ" || fail serial_validation_missing
grep -Fq 'no same-mic onboard-profile cache exists for serial' "$READ" || fail cross_device_cache_rejection_missing
grep -Fq 'same_mic_cache_accepts_prefixed_stored_serial' "$READ" || fail prefixed_serial_regression_test_missing
grep -Fq 'same_mic_cache_rejects_unrelated_serial' "$READ" || fail unrelated_serial_regression_test_missing
grep -Fq 'Same-mic cache' "$UI" || fail same_mic_cache_ui_missing
grep -Fq 'SERIAL VERIFIED' "$UI" || fail serial_verified_badge_missing
grep -Fq 'cargo generate-lockfile' "$INSTALL" || fail lock_generation_missing
grep -Fq 'cargo clippy --locked' "$INSTALL" || fail locked_clippy_missing
grep -Fq 'cargo test --locked' "$INSTALL" || fail locked_test_missing
grep -Fq 'cargo build --locked --release' "$INSTALL" || fail locked_release_missing

echo 'AETHERFORGE_BEACN_V0_1_23_CACHE_LOCK=PASS:SAME_MIC_ONLY+LOCKED_HOST_GATE'
