#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cd "$ROOT"
fail=0
if grep -R --line-number --fixed-strings 'MockProvider' apps --exclude-dir=target; then
  echo AETHERAI_NO_MOCK_RUNTIME=FAIL:MOCK_PROVIDER_IN_APP
  fail=1
fi
if grep -R --line-number --fixed-strings 'aetherai/mock' apps --exclude-dir=target; then
  echo AETHERAI_NO_MOCK_RUNTIME=FAIL:MOCK_MODEL_IN_APP
  fail=1
fi
if grep -R --line-number --fixed-strings 'AetherAI mock:' apps crates --exclude-dir=target; then
  echo AETHERAI_NO_MOCK_RUNTIME=FAIL:FABRICATED_MOCK_OUTPUT
  fail=1
fi
if (( fail )); then exit 1; fi
echo AETHERAI_NO_MOCK_RUNTIME=PASS
