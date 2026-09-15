#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
MAIN="$ROOT/src/main.rs"
SECTION="$(mktemp)"
trap 'rm -f -- "$SECTION"' EXIT
awk '
  /^fn command_organize_once\(/ { in_fn=1 }
  in_fn { print }
  in_fn && /^}$/ { exit }
' "$MAIN" > "$SECTION"
grep -Fq 'for action in &report.actions {' "$SECTION"
if grep -Fq 'for action in report.actions {' "$SECTION"; then
  echo 'FORGECLEAN_V0_3_1_PARTIAL_MOVE=FAIL:OWNING_ITERATION_PRESENT'
  exit 1
fi
grep -Fq 'if report.is_ok() {' "$SECTION"
echo 'FORGECLEAN_V0_3_1_PARTIAL_MOVE=PASS'
