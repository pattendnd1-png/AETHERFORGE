#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MEDIA="$ROOT/crates/aether-media-service/src/main.rs"
ENGINE="$ROOT/crates/aether-engine-servo/src/live.rs"

fail=0
if grep -q 'fn classify_drm_evidence' "$MEDIA"; then
  echo 'AETHER_BROWSER_V2_1_21_DEAD_DRM_HELPER=FAIL'
  fail=1
else
  echo 'AETHER_BROWSER_V2_1_21_DEAD_DRM_HELPER=PASS'
fi

main_line="$(grep -n '^fn main()' "$MEDIA" | cut -d: -f1 | head -1)"
test_line="$(grep -n '^#\[cfg(test)\]' "$MEDIA" | cut -d: -f1 | head -1)"
if [[ -n "$main_line" && -n "$test_line" && "$main_line" -lt "$test_line" ]]; then
  echo 'AETHER_BROWSER_V2_1_21_TEST_MODULE_ORDER=PASS'
else
  echo 'AETHER_BROWSER_V2_1_21_TEST_MODULE_ORDER=FAIL'
  fail=1
fi

if grep -q 'assert_eq!(right_edge.y, 629.0);' "$ENGINE"; then
  echo 'AETHER_BROWSER_V2_1_21_COMPACT_CHROME_POINTER_GEOMETRY=PASS'
else
  echo 'AETHER_BROWSER_V2_1_21_COMPACT_CHROME_POINTER_GEOMETRY=FAIL'
  fail=1
fi

exit "$fail"
