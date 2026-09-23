#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

# Canonical v1.0.16 production/build/install paths must not invoke a Python interpreter.
for path in build-and-verify.sh install-local.sh hit-it-template.sh \
  tests/regression_v1_0_10_atomic_orbit.sh \
  tests/regression_v1_0_11_clippy_gate.sh \
  tests/regression_v1_0_16_pre_rebase.sh; do
  if grep -Eiq 'python3|python[[:space:]]|#!/usr/bin/env[[:space:]]+python' "$path"; then
    echo "interpreter dependency found in canonical path: $path" >&2
    exit 1
  fi
done
if find src systemd tests -type f \( -name '*.py' -o -name '*.pyc' \) -print -quit | grep -q .; then
  echo 'Python file present in ForgeClean source tree' >&2
  exit 1
fi
if find . -type d -name '__pycache__' -print -quit | grep -q .; then
  echo '__pycache__ present in ForgeClean source tree' >&2
  exit 1
fi
grep -Fq 'pub mod pre_rebase;' src/lib.rs
grep -Fq 'cargo clippy --all-targets --all-features -- -D warnings' build-and-verify.sh
printf 'FORGECLEAN_V1_0_16_RUST_FIRST=PASS\n'
