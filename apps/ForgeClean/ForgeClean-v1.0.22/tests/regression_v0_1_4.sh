#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
grep -q 'pub mod system_scan;' "$ROOT/src/lib.rs"
grep -q 'SYSTEM_PACMAN_CACHE' "$ROOT/src/system_scan.rs"
grep -q 'PACMAN-BATCH.txt' "$ROOT/src/system_scan.rs"
grep -q '"scan-system"' "$ROOT/src/main.rs"
grep -q 'FORGECLEAN_SYSTEM_SCAN_UNIT_TEST' "$ROOT/build-and-verify.sh"
grep -q 'FORGECLEAN_SYSTEM_SCAN_E2E' "$ROOT/build-and-verify.sh"
grep -q 'system_scan_uses_fixed_pacman_cache' "$ROOT/tests/system_scan.rs"
