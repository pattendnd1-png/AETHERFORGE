#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
TEMPLATE="$ROOT/hit-it-template.sh"
[[ -f "$TEMPLATE" ]] || { echo 'missing hit-it-template.sh' >&2; exit 1; }
# Guarantee is structural: the verification file must be initialized before the
# first source/extract/regression gate, and an EXIT trap must always report it.
init_line="$(grep -nF ': > "$VERIFY"' "$TEMPLATE" | head -n1 | cut -d: -f1 || true)"
source_gate_line="$(grep -nF 'FORGECLEAN_SOURCE=FAIL:NOT_FOUND' "$TEMPLATE" | head -n1 | cut -d: -f1 || true)"
[[ -n "$init_line" && -n "$source_gate_line" && "$init_line" -lt "$source_gate_line" ]] || {
  echo 'verify file is not initialized before early-failure gates' >&2
  exit 1
}
grep -Fq 'trap finalize EXIT' "$TEMPLATE"
grep -Fq 'FORGECLEAN_HIT_IT=FAIL:' "$TEMPLATE"
grep -Fq 'FORGECLEAN_HIT_IT=PASS' "$TEMPLATE"
grep -Fq 'VERIFY_FILE=${VERIFY}' "$TEMPLATE"
grep -Fq 'run_stage FORGECLEAN_INSTALL ./install-local.sh' "$TEMPLATE"
echo 'FORGECLEAN_V0_4_2_VERIFY_ARTIFACT=PASS'
