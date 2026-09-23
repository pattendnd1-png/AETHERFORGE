#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
TEST="$ROOT/tests/coldpack_gc.rs"
MAIN="$ROOT/src/main.rs"
SYSTEM="$ROOT/src/system_scan.rs"
[[ -f "$TEST" ]]
[[ -f "$MAIN" ]]
[[ -f "$SYSTEM" ]]
! grep -Eq 'snapshot_files\([^\n]*\)\.len\(\)[[:space:]]*>[[:space:]]*0' "$TEST"
grep -Fq '!snapshot_files(&store.join("quarantine/3000000")).is_empty()' "$TEST"
grep -Fq 'const VERSION: &str' "$MAIN"
grep -Fq 'SYSTEM_MANIFEST_FILENAME' "$SYSTEM"
printf '%s\n' 'FORGECLEAN_V0_5_2_CLIPPY_TEST=PASS'
