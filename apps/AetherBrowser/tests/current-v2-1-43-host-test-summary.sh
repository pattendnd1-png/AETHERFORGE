#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_V2_1_52_HOST_TEST_SUMMARY=FAIL:$1"; exit 1; }
VERIFY=scripts/verify.sh
HOST=scripts/package-consolidated.sh
[[ -f "$VERIFY" ]] || fail verify-script-missing
[[ -f "$HOST" ]] || fail host-wrapper-missing
# The aggregate test marker is derived only from retained component tests.
grep -qF "gate AETHER_BROWSER_HOME_NATIVE_TEST" "$VERIFY" || fail home-test-gate-missing
grep -qF "gate AETHER_BROWSER_VELORA_PROVIDER_TEST" "$VERIFY" || fail velora-test-gate-missing
if grep -qF "gate AETHER_BROWSER_DECK_TEST" "$VERIFY"; then fail removed-deck-test-gate-present; fi
grep -qF "AETHER_BROWSER_TEST=PASS" "$VERIFY" || fail aggregate-test-pass-marker-missing
grep -qF "AETHER_BROWSER_TEST=FAIL" "$VERIFY" || fail aggregate-test-fail-marker-missing
grep -qF 'for marker in CHECK CLIPPY TEST BUILD; do' "$HOST" || fail host-test-marker-contract-missing
echo "AETHER_BROWSER_V2_1_52_HOST_TEST_SUMMARY=PASS"
